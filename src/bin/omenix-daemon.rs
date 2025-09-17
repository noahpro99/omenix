use clap::{CommandFactory, Parser};
use clap_config::ClapConfig;
use std::fs;
use std::io::{self, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tracing::{debug, error, info, instrument, warn};

use omenix::client::DAEMON_SOCKET_PATH;
use omenix::types::{FanMode, HardwareFanMode, PerformanceMode};

const TEMP_SENSOR_PATH: &str = "/sys/class/thermal/thermal_zone*/temp";
const FAN_CONTROL_PATH: &str = "/sys/devices/platform/hp-wmi/hwmon/hwmon*/pwm1_enable";
const PERFORMANCE_PROFILE_PATH: &str = "/sys/firmware/acpi/platform_profile";
const CONFIG_FILE_PATH: &str = "/etc/omenix-daemon.yaml";

#[derive(ClapConfig, Parser, Debug, Clone)]
pub struct AppConfig {
    /// Temperature threshold in Celsius to trigger max fan mode in Auto mode
    #[clap(long, default_value = "75")]
    temp_threshold_high: i32,
    /// Temperature threshold in Celsius to allow switching back to BIOS control when fans are on max
    #[clap(long, default_value = "70")]
    temp_threshold_low: i32,
    /// Number of consecutive high temperature readings to trigger max fan mode in Auto mode
    #[clap(long, default_value = "3")]
    consecutive_high_temp_limit: u32,
    /// Number of consecutive low temperature readings to switch back to BIOS control when fans are on max
    #[clap(long, default_value = "3")]
    consecutive_low_temp_limit: u32,
    /// Interval in seconds to check temperature
    #[clap(long, default_value = "5")]
    temp_check_interval: u64,
    /// Interval in seconds to rewrite max fan mode in Max mode
    #[clap(long, default_value = None)]
    max_fan_write_interval: Option<u64>,
}

#[derive(Debug)]
pub struct DaemonState {
    pub user_mode: FanMode,
    pub actual_mode: HardwareFanMode,
    pub performance_mode: PerformanceMode,
    pub last_fan_write: Option<Instant>,
    pub consecutive_high_temps: u32,
    pub consecutive_low_temps: u32,
    pub temp_monitoring_active: bool,
    pub current_temp: Option<i32>,
    pub config: AppConfig,
}

impl DaemonState {
    pub fn new(config: &AppConfig) -> Self {
        let state = Self {
            user_mode: FanMode::Auto,
            actual_mode: HardwareFanMode::Bios,
            performance_mode: PerformanceMode::Performance,
            last_fan_write: None,
            consecutive_high_temps: 0,
            consecutive_low_temps: 0,
            temp_monitoring_active: false,
            current_temp: None,
            config: config.clone(),
        };
        info!("DaemonState initialized: {:?}", state);
        state
    }
}

#[instrument(level = "debug")]
fn write_fan_mode(mode: HardwareFanMode) -> Result<(), io::Error> {
    let value = match mode {
        HardwareFanMode::Max => "0",
        HardwareFanMode::Bios => "2",
    };

    info!("Writing fan mode: {:?} (value: {})", mode, value);

    // Find the actual fan control file
    let paths: Vec<_> = glob::glob(FAN_CONTROL_PATH)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?
        .filter_map(Result::ok)
        .collect();

    if paths.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "No fan control file found",
        ));
    }

    let fan_path = &paths[0];
    debug!("Writing to fan control file: {:?}", fan_path);

    let mut file = fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(fan_path)?;

    file.write_all(value.as_bytes())?;
    file.flush()?;

    info!("Successfully wrote fan mode: {:?}", mode);
    Ok(())
}

fn write_performance_mode(mode: PerformanceMode) -> Result<(), io::Error> {
    let value = mode.to_string(); // "balanced" or "performance"

    info!("Writing performance mode: {:?} (value: {})", mode, value);

    let mut file = fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(PERFORMANCE_PROFILE_PATH)?;

    file.write_all(value.as_bytes())?;
    file.flush()?;

    info!("Successfully wrote performance mode: {:?}", mode);
    Ok(())
}

#[instrument(level = "debug")]
fn read_temperature() -> Result<i32, io::Error> {
    let paths: Vec<_> = glob::glob(TEMP_SENSOR_PATH)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?
        .filter_map(Result::ok)
        .collect();

    if paths.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
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
            io::Error::new(io::ErrorKind::InvalidData, "Failed to read temperature")
        })?;

    debug!("Max temperature read: {}°C", max_temp / 1000);
    Ok(max_temp)
}

fn handle_client_request(request: &str, state: Arc<Mutex<DaemonState>>) -> Result<String, String> {
    let parts: Vec<&str> = request.split_whitespace().collect();

    match parts.as_slice() {
        ["set", mode_str] => {
            let mode = mode_str
                .parse::<FanMode>()
                .map_err(|_| "Invalid fan mode")?;
            set_fan_mode(state, mode)?;
            Ok(format!("Fan mode set to: {}", mode))
        }
        ["set_performance", mode_str] => {
            let mode = mode_str
                .parse::<PerformanceMode>()
                .map_err(|_| "Invalid performance mode")?;
            set_performance_mode(state, mode)?;
            Ok(format!("Performance mode set to: {}", mode))
        }
        ["status"] => {
            let state_guard = state.lock().unwrap();
            let temp_str = match state_guard.current_temp {
                Some(temp) => format!("{}°C", temp / 1000),
                None => "Unknown".to_string(),
            };
            Ok(format!(
                "Mode: {}, Actual: {:?}, Performance: {}, Temp: {}",
                state_guard.user_mode,
                state_guard.actual_mode,
                state_guard.performance_mode,
                temp_str
            ))
        }
        _ => Err(
            "Invalid command. Use 'set <mode>', 'set_performance <mode>', or 'status'".to_string(),
        ),
    }
}

fn set_fan_mode(state: Arc<Mutex<DaemonState>>, new_mode: FanMode) -> Result<(), String> {
    info!("Setting fan mode to: {:?}", new_mode);

    let temp_threshold = {
        let state_guard = state.lock().unwrap();
        state_guard.config.temp_threshold_high * 1000
    };

    let actual_mode_to_set = match new_mode {
        FanMode::Max => HardwareFanMode::Max,
        FanMode::Auto => {
            // For Auto mode, check current temperature
            match read_temperature() {
                Ok(temp) if temp > temp_threshold => HardwareFanMode::Max,
                _ => HardwareFanMode::Bios,
            }
        }
        FanMode::Bios => HardwareFanMode::Bios,
    };

    // Update state
    {
        let mut state_guard = state.lock().unwrap();
        let old_state = format!("{:?}", *state_guard);

        state_guard.user_mode = new_mode;
        state_guard.actual_mode = actual_mode_to_set;
        state_guard.consecutive_high_temps = 0;

        match new_mode {
            FanMode::Max => {
                state_guard.last_fan_write = Some(Instant::now());
                state_guard.temp_monitoring_active = false;
                info!("Max mode: Set last_fan_write and disabled temp monitoring");
            }
            FanMode::Auto => {
                state_guard.temp_monitoring_active = true;
                state_guard.last_fan_write = None;
                info!("Auto mode: Enabled temp monitoring and cleared last_fan_write");
            }
            FanMode::Bios => {
                state_guard.temp_monitoring_active = false;
                state_guard.last_fan_write = None;
                info!("BIOS mode: Disabled temp monitoring and cleared last_fan_write");
            }
        }

        let new_state = format!("{:?}", *state_guard);
        debug!(
            "State transition:\n  From: {}\n  To: {}",
            old_state, new_state
        );
    }

    // Write to hardware
    write_fan_mode(actual_mode_to_set).map_err(|e| format!("Failed to write fan mode: {}", e))?;

    info!("Successfully set fan mode to: {:?}", actual_mode_to_set);
    Ok(())
}

fn set_performance_mode(
    state: Arc<Mutex<DaemonState>>,
    new_mode: PerformanceMode,
) -> Result<(), String> {
    info!("Setting performance mode to: {:?}", new_mode);

    // Update state
    {
        let mut state_guard = state.lock().unwrap();
        state_guard.performance_mode = new_mode;
    }

    // Write to platform profile
    write_performance_mode(new_mode)
        .map_err(|e| format!("Failed to write performance mode: {}", e))?;

    info!("Successfully set performance mode to: {:?}", new_mode);
    Ok(())
}

fn start_temperature_monitor(state: Arc<Mutex<DaemonState>>) {
    info!("Starting temperature monitoring thread");
    thread::spawn(move || {
        let config = {
            let state_guard = state.lock().unwrap();
            state_guard.config.clone()
        };
        info!("Temperature monitoring thread started");
        loop {
            thread::sleep(Duration::from_secs(config.temp_check_interval));

            // Read current temperature
            let current_temp = match read_temperature() {
                Ok(temp) => {
                    {
                        let mut state_guard = state.lock().unwrap();
                        state_guard.current_temp = Some(temp);
                    }
                    temp
                }
                Err(e) => {
                    error!("Failed to read temperature: {}", e);
                    continue;
                }
            };

            let mut should_handle_max_mode = false;
            let mut should_handle_auto_mode = false;
            let user_mode;

            // Check what we need to do
            {
                let state_guard = state.lock().unwrap();
                user_mode = state_guard.user_mode;

                if user_mode == FanMode::Max {
                    if let Some(interval) = config.max_fan_write_interval {
                        if let Some(last_write) = state_guard.last_fan_write {
                            let max_fan_write_interval = Duration::from_secs(interval);
                            let elapsed = last_write.elapsed();
                            if elapsed >= max_fan_write_interval {
                                should_handle_max_mode = true;
                            }
                        } else {
                            should_handle_max_mode = true;
                        }
                    }
                    // If max_fan_write_interval is None, don't rewrite max mode
                }

                if user_mode == FanMode::Auto && state_guard.temp_monitoring_active {
                    should_handle_auto_mode = true;
                }
            }

            // Handle max mode timing - CRITICAL: Must rewrite every 100 seconds
            if should_handle_max_mode {
                info!("Handling max mode timing - rewriting to maintain max fans");
                if let Err(e) = write_fan_mode(HardwareFanMode::Max) {
                    error!("Failed to set max fan mode: {}", e);
                } else {
                    let mut state_guard = state.lock().unwrap();
                    state_guard.last_fan_write = Some(Instant::now());
                    state_guard.actual_mode = HardwareFanMode::Max;
                    info!("✓ Max fan mode rewritten successfully");
                }
            }

            // Handle auto mode temperature monitoring
            if should_handle_auto_mode {
                debug!("Handling auto mode temperature check");
                let temp_celsius = current_temp / 1000;
                debug!(
                    "Temperature check: {}°C (high_threshold: {}°C, low_threshold: {}°C)",
                    temp_celsius, config.temp_threshold_high, config.temp_threshold_low
                );

                let mut state_guard = state.lock().unwrap();

                if state_guard.actual_mode != HardwareFanMode::Max {
                    // Not in MAX mode - check for going to MAX
                    if current_temp > config.temp_threshold_high * 1000 {
                        state_guard.consecutive_high_temps += 1;
                        info!(
                            "High temperature detected: {}°C (high_count: {})",
                            temp_celsius, state_guard.consecutive_high_temps
                        );

                        if state_guard.consecutive_high_temps >= config.consecutive_high_temp_limit
                        {
                            info!("Temperature consistently high, switching to max fans");
                            drop(state_guard);
                            if let Err(e) = write_fan_mode(HardwareFanMode::Max) {
                                error!("Failed to set max fan mode: {}", e);
                            } else {
                                let mut state_guard = state.lock().unwrap();
                                state_guard.actual_mode = HardwareFanMode::Max;
                                state_guard.last_fan_write = Some(Instant::now()); // Track for 100s rule
                                state_guard.consecutive_high_temps = 0; // Reset counter after switching
                            }
                        }
                    } else {
                        // Temperature not high enough - reset high counter
                        if state_guard.consecutive_high_temps > 0 {
                            state_guard.consecutive_high_temps = 0;
                            debug!("Temperature normal, resetting high temperature counter");
                        }
                    }
                } else {
                    // In MAX mode - first maintain write interval rule if configured, then check for going back to BIOS
                    let should_rewrite_max = if let Some(interval) = config.max_fan_write_interval {
                        if let Some(last_write) = state_guard.last_fan_write {
                            let max_fan_write_interval = Duration::from_secs(interval);
                            last_write.elapsed() >= max_fan_write_interval
                        } else {
                            false
                        }
                    } else {
                        false // Don't rewrite if interval is not configured
                    };

                    if should_rewrite_max {
                        drop(state_guard);
                        info!("Auto mode: Rewriting max fans to maintain 100s rule");
                        if let Err(e) = write_fan_mode(HardwareFanMode::Max) {
                            error!("Failed to maintain max fan mode: {}", e);
                        } else {
                            let mut state_guard = state.lock().unwrap();
                            state_guard.last_fan_write = Some(Instant::now());
                        }
                        continue; // Skip the rest of this iteration
                    }

                    // Now check for low temperature
                    if current_temp <= config.temp_threshold_low * 1000 {
                        state_guard.consecutive_low_temps += 1;
                        info!(
                            "Low temperature detected: {}°C (low_count: {})",
                            temp_celsius, state_guard.consecutive_low_temps
                        );

                        if state_guard.consecutive_low_temps >= config.consecutive_low_temp_limit {
                            info!("Temperature consistently low, switching back to BIOS control");
                            drop(state_guard);
                            if let Err(e) = write_fan_mode(HardwareFanMode::Bios) {
                                error!("Failed to set BIOS fan mode: {}", e);
                            } else {
                                let mut state_guard = state.lock().unwrap();
                                state_guard.actual_mode = HardwareFanMode::Bios;
                                state_guard.last_fan_write = None; // Clear since not in max mode
                                state_guard.consecutive_low_temps = 0; // Reset counter after switching
                            }
                        }
                    } else {
                        // Temperature not low enough - reset low counter
                        if state_guard.consecutive_low_temps > 0 {
                            state_guard.consecutive_low_temps = 0;
                            debug!("Temperature not low enough, resetting low temperature counter");
                        }
                    }
                }
            }
        }
    });
}

fn start_unix_socket_server(state: Arc<Mutex<DaemonState>>) -> Result<(), io::Error> {
    use std::os::unix::net::UnixListener;

    // Remove existing socket file if it exists
    if Path::new(DAEMON_SOCKET_PATH).exists() {
        fs::remove_file(DAEMON_SOCKET_PATH)?;
    }

    let listener = UnixListener::bind(DAEMON_SOCKET_PATH)?;
    info!("Daemon listening on socket: {}", DAEMON_SOCKET_PATH);

    // Set socket permissions so regular users can connect
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(DAEMON_SOCKET_PATH)?.permissions();
        perms.set_mode(0o666); // rw-rw-rw-
        fs::set_permissions(DAEMON_SOCKET_PATH, perms)?;
    }

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let state_clone = state.clone();
                thread::spawn(move || {
                    if let Err(e) = handle_client(stream, state_clone) {
                        error!("Error handling client: {}", e);
                    }
                });
            }
            Err(e) => {
                error!("Error accepting connection: {}", e);
            }
        }
    }

    Ok(())
}

fn handle_client(mut stream: UnixStream, state: Arc<Mutex<DaemonState>>) -> Result<(), io::Error> {
    let mut buffer = [0; 1024];
    let bytes_read = stream.read(&mut buffer)?;
    let request = String::from_utf8_lossy(&buffer[..bytes_read]);

    debug!("Received request: {}", request.trim());

    let response = match handle_client_request(&request, state) {
        Ok(resp) => format!("OK: {}\n", resp),
        Err(err) => format!("ERROR: {}\n", err),
    };

    stream.write_all(response.as_bytes())?;
    Ok(())
}

fn main() {
    // Initialize tracing subscriber for structured logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    let config_path = PathBuf::from(CONFIG_FILE_PATH);
    let config_opt = if config_path.exists() {
        let config_str = fs::read_to_string(&config_path).unwrap();
        Some(serde_yaml::from_str(&config_str).unwrap())
    } else {
        None
    };
    info!("Using config file: {:?}", config_path);
    info!("Using config: {:?}", config_opt);
    let matches = <AppConfig as CommandFactory>::command().get_matches();
    let opts = AppConfig::from_merged(matches, config_opt);

    info!(
        "Daemon starting with config: temp_threshold_high={}°C, temp_threshold_low={}°C, consecutive_high_temp_limit={}, consecutive_low_temp_limit={}, temp_check_interval={}s, max_fan_write_interval={:?}",
        opts.temp_threshold_high,
        opts.temp_threshold_low,
        opts.consecutive_high_temp_limit,
        opts.consecutive_low_temp_limit,
        opts.temp_check_interval,
        opts.max_fan_write_interval
    );
    info!("Starting Omenix Fan Control Daemon");

    // Check if running as root
    if unsafe { libc::geteuid() } != 0 {
        error!("Daemon must be run as root to access fan controls");
        std::process::exit(1);
    }

    let state = Arc::new(Mutex::new(DaemonState::new(&opts)));

    // Apply initial fan mode (Auto) during startup
    info!("Applying initial Auto fan mode during daemon startup");
    if let Err(e) = set_fan_mode(state.clone(), FanMode::Auto) {
        error!("Failed to set initial fan mode: {}", e);
    } else {
        info!("Successfully applied initial Auto fan mode");
    }

    // Start temperature monitoring thread
    start_temperature_monitor(state.clone());

    // Start Unix socket server
    if let Err(e) = start_unix_socket_server(state) {
        error!("Failed to start socket server: {}", e);
        std::process::exit(1);
    }
}
