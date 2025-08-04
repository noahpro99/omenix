use std::fs;
use std::io::Read;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tracing::{debug, error, info, instrument, warn};

const TEMP_SENSOR_PATH: &str = "/sys/class/thermal/thermal_zone*/temp";
const TEMP_THRESHOLD: i32 = 75000; // 75°C in millicelsius
const MAX_FAN_WRITE_INTERVAL: Duration = Duration::from_secs(100); // <120 seconds
const TEMP_CHECK_INTERVAL: Duration = Duration::from_secs(5);
const CONSECUTIVE_HIGH_TEMP_LIMIT: u32 = 3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FanStatus {
    Max,
    Auto,
    Bios,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ActualFanMode {
    Max,  // Writing 0 to device
    Bios, // Writing 2 to device
}

#[derive(Debug)]
pub struct AppState {
    pub user_mode: FanStatus,       // What the user selected
    pub actual_mode: ActualFanMode, // What's actually written to device
    pub last_fan_write: Option<Instant>,
    pub consecutive_high_temps: u32,
    pub temp_monitoring_active: bool,
}

impl AppState {
    pub fn new() -> Self {
        let state = Self {
            user_mode: FanStatus::Bios,
            actual_mode: ActualFanMode::Bios,
            last_fan_write: None,
            consecutive_high_temps: 0,
            temp_monitoring_active: false,
        };
        info!("AppState initialized: {:?}", state);
        state
    }
}

impl std::fmt::Display for FanStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FanStatus::Max => write!(f, "Max"),
            FanStatus::Auto => write!(f, "Auto"),
            FanStatus::Bios => write!(f, "Bios"),
        }
    }
}

impl FromStr for FanStatus {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "max" => Ok(FanStatus::Max),
            "auto" => Ok(FanStatus::Auto),
            "bios" => Ok(FanStatus::Bios),
            _ => Err(()),
        }
    }
}

#[instrument(level = "debug")]
fn set_fan_mode(mode: ActualFanMode) -> Result<(), std::io::Error> {
    let value = match mode {
        ActualFanMode::Max => "0",
        ActualFanMode::Bios => "2",
    };

    info!(
        "Setting actual fan mode to: {:?} (writing value: {})",
        mode, value
    );

    // Get the helper script path from environment variable or use default
    let helper_path = std::env::var("OMENIX_HELPER_PATH")
        .unwrap_or_else(|_| "/etc/omenix/omenix-fancontrol".to_string());

    // Get pkexec path - try environment variable first, then system paths
    let pkexec_path = std::env::var("PKEXEC_PATH")
        .or_else(|_| {
            // Check common system locations for setuid pkexec
            for path in ["/usr/bin/pkexec", "/bin/pkexec", "/usr/local/bin/pkexec"] {
                if let Ok(metadata) = std::fs::metadata(path) {
                    // Check if file is setuid (mode & 0o4000 != 0)
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        if metadata.permissions().mode() & 0o4000 != 0 {
                            debug!("Found setuid pkexec at: {}", path);
                            return Ok(path.to_string());
                        } else {
                            warn!("Found pkexec at {} but it's not setuid", path);
                        }
                    }
                    #[cfg(not(unix))]
                    {
                        return Ok(path.to_string());
                    }
                }
            }
            Err(std::env::VarError::NotPresent)
        })
        .unwrap_or_else(|_| {
            warn!("No setuid pkexec found, falling back to PATH");
            "pkexec".to_string()
        });

    debug!("Using pkexec at: {}", pkexec_path);
    debug!("Using helper script at: {}", helper_path);

    // Verify helper script exists
    if !std::path::Path::new(&helper_path).exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Helper script not found at: {}", helper_path),
        ));
    }

    // Use polkit to authenticate and run the fan control script
    let output = std::process::Command::new(&pkexec_path)
        .arg(&helper_path)
        .arg(value)
        .output()?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        info!("Successfully set fan mode, output: {}", stdout.trim());
        Ok(())
    } else {
        let error_msg = String::from_utf8_lossy(&output.stderr);
        error!("Failed to set fan mode: {}", error_msg);

        // Check if the error is due to polkit cancellation
        if error_msg.contains("Request dismissed") || error_msg.contains("Operation was cancelled")
        {
            warn!("User cancelled the authentication request");
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "Authentication cancelled by user",
            ));
        }

        // Check if pkexec is not setuid root
        if error_msg.contains("pkexec must be setuid root") {
            error!("pkexec is not properly configured - it must be setuid root");
            error!("Current pkexec: {}", pkexec_path);
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "pkexec is not setuid root. Please install system polkit or use NixOS module.",
            ));
        }

        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to set fan mode: {}", error_msg),
        ))
    }
}

#[instrument(level = "debug")]
fn read_temperature() -> Result<i32, std::io::Error> {
    let paths: Vec<_> = glob::glob(TEMP_SENSOR_PATH)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?
        .filter_map(Result::ok)
        .collect();

    if paths.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "No temperature sensor found",
        ));
    }

    let max_temp = paths
        .iter()
        .filter_map(|path| {
            let mut file = fs::File::open(path).ok()?;
            let mut contents = String::new();
            file.read_to_string(&mut contents).ok()?;
            contents.trim().parse::<i32>().ok()
        })
        .max()
        .ok_or_else(|| {
            error!("Failed to read temperature from any sensor");
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Failed to read temperature",
            )
        })?;

    debug!("Max temperature read: {:?}°C", max_temp / 1000);
    Ok(max_temp)
}

pub fn start_fan_control_thread(state: Arc<Mutex<AppState>>) {
    info!("Starting fan control background thread");
    thread::spawn(move || {
        info!("Fan control thread started, entering main loop");
        loop {
            thread::sleep(TEMP_CHECK_INTERVAL);

            let mut should_handle_max_mode = false;
            let mut should_handle_auto_mode = false;
            let user_mode;
            let state_snapshot;

            // Read current state
            {
                let state_guard = state.lock().unwrap();
                user_mode = state_guard.user_mode;
                state_snapshot = format!("{:?}", *state_guard);
                debug!("Current state: {}", state_snapshot);

                if user_mode == FanStatus::Max {
                    if let Some(last_write) = state_guard.last_fan_write {
                        let elapsed = last_write.elapsed();
                        if elapsed >= MAX_FAN_WRITE_INTERVAL {
                            debug!(
                                "Max mode interval reached: {:?} >= {:?}",
                                elapsed, MAX_FAN_WRITE_INTERVAL
                            );
                            should_handle_max_mode = true;
                        } else {
                            debug!(
                                "Max mode interval not reached: {:?} < {:?}",
                                elapsed, MAX_FAN_WRITE_INTERVAL
                            );
                        }
                    } else {
                        debug!("Max mode: no previous write, will write now");
                        should_handle_max_mode = true;
                    }
                }

                if user_mode == FanStatus::Auto && state_guard.temp_monitoring_active {
                    debug!("Auto mode: temperature monitoring active");
                    should_handle_auto_mode = true;
                } else if user_mode == FanStatus::Auto {
                    debug!("Auto mode: temperature monitoring NOT active");
                }
            }

            // Handle max mode timing
            if should_handle_max_mode {
                info!("Handling max mode timing - writing to device");
                if let Err(e) = set_fan_mode(ActualFanMode::Max) {
                    error!("Failed to set max fan mode: {}", e);
                } else {
                    let mut state_guard = state.lock().unwrap();
                    state_guard.last_fan_write = Some(Instant::now());
                    state_guard.actual_mode = ActualFanMode::Max;
                    info!(
                        "Max mode write successful, updated state: {:?}",
                        *state_guard
                    );
                }
            }

            // Handle auto mode temperature monitoring
            if should_handle_auto_mode {
                debug!("Handling auto mode temperature check");
                match read_temperature() {
                    Ok(temp) => {
                        let temp_celsius = temp / 1000;
                        let threshold_celsius = TEMP_THRESHOLD / 1000;
                        debug!(
                            "Temperature check: {}°C (threshold: {}°C)",
                            temp_celsius, threshold_celsius
                        );

                        let mut state_guard = state.lock().unwrap();

                        if temp > TEMP_THRESHOLD {
                            state_guard.consecutive_high_temps += 1;
                            warn!(
                                "High temperature detected: {}°C (count: {}/{})",
                                temp_celsius,
                                state_guard.consecutive_high_temps,
                                CONSECUTIVE_HIGH_TEMP_LIMIT
                            );

                            if state_guard.consecutive_high_temps >= CONSECUTIVE_HIGH_TEMP_LIMIT {
                                info!("Temperature threshold exceeded, switching to max fans");
                                state_guard.actual_mode = ActualFanMode::Max;
                                state_guard.consecutive_high_temps = 0;
                                state_guard.last_fan_write = Some(Instant::now());
                                drop(state_guard);

                                if let Err(e) = set_fan_mode(ActualFanMode::Max) {
                                    error!("Failed to set max fan mode: {}", e);
                                } else {
                                    info!(
                                        "Successfully switched to max fan mode due to high temperature"
                                    );
                                }
                            }
                        } else if state_guard.consecutive_high_temps > 0 {
                            info!(
                                "Temperature normal: {}°C, switching back to BIOS",
                                temp_celsius
                            );
                            state_guard.actual_mode = ActualFanMode::Bios;
                            state_guard.consecutive_high_temps = 0;
                            drop(state_guard);

                            if let Err(e) = set_fan_mode(ActualFanMode::Bios) {
                                error!("Failed to set bios fan mode: {}", e);
                            } else {
                                info!("Successfully switched back to BIOS fan mode");
                            }
                        } else {
                            state_guard.consecutive_high_temps = 0;
                            debug!(
                                "Temperature normal: {}°C, staying in current mode",
                                temp_celsius
                            );
                        }
                    }
                    Err(e) => {
                        error!("Failed to read temperature: {}", e);
                    }
                }
            }
        }
    });
}

#[instrument(level = "debug", fields(new_mode = ?new_mode))]
pub fn set_fan_status(
    state: Arc<Mutex<AppState>>,
    new_mode: FanStatus,
) -> Result<(), std::io::Error> {
    info!("Setting fan status to: {:?}", new_mode);

    let actual_mode_to_set = match new_mode {
        FanStatus::Max => ActualFanMode::Max,
        FanStatus::Auto => match read_temperature() {
            Ok(temp) if temp > TEMP_THRESHOLD => ActualFanMode::Max,
            _ => ActualFanMode::Bios,
        },
        FanStatus::Bios => ActualFanMode::Bios,
    };

    info!("Will set actual mode to: {:?}", actual_mode_to_set);

    {
        let mut state_guard = state.lock().unwrap();
        let old_state = format!("{:?}", *state_guard);

        state_guard.user_mode = new_mode;
        state_guard.actual_mode = actual_mode_to_set;
        state_guard.consecutive_high_temps = 0;

        match new_mode {
            FanStatus::Max => {
                state_guard.last_fan_write = Some(Instant::now());
                state_guard.temp_monitoring_active = false;
                info!("Max mode: Set last_fan_write and disabled temp monitoring");
            }
            FanStatus::Auto => {
                state_guard.temp_monitoring_active = true;
                state_guard.last_fan_write = None;
                info!("Auto mode: Enabled temp monitoring and cleared last_fan_write");
            }
            FanStatus::Bios => {
                state_guard.temp_monitoring_active = false;
                state_guard.last_fan_write = None;
                info!("BIOS mode: Disabled temp monitoring and cleared last_fan_write");
            }
        }

        let new_state = format!("{:?}", *state_guard);
        debug!(
            "State transition:\n  From: {}\n  To:   {}",
            old_state, new_state
        );
    }

    let result = set_fan_mode(actual_mode_to_set);
    if let Err(ref e) = result {
        error!("Failed to set fan mode: {}", e);
    } else {
        info!("Successfully set fan mode to: {:?}", actual_mode_to_set);
    }
    result
}

pub fn fan_status_string(state: Arc<Mutex<AppState>>) -> String {
    let state_guard = state.lock().unwrap();
    let status_string = format!("Fans: {}", state_guard.user_mode);
    debug!(
        "Fan status string: '{}' (state: {:?})",
        status_string, *state_guard
    );
    status_string
}
