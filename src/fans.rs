use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::thread;
use std::fs;
use std::io::Read;

const TEMP_SENSOR_PATH: &str = "/sys/class/thermal/thermal_zone0/temp";
const TEMP_THRESHOLD: i32 = 75000; // 75°C in millicelsius
const MAX_FAN_WRITE_INTERVAL: Duration = Duration::from_secs(90);
const TEMP_CHECK_INTERVAL: Duration = Duration::from_secs(2);
const CONSECUTIVE_HIGH_TEMP_LIMIT: u32 = 3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FanStatus {
    Max,
    Auto,
    Bios,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ActualFanMode {
    Max,    // Writing 0 to device
    Bios,   // Writing 2 to device
}

#[derive(Debug)]
pub struct AppState {
    pub user_mode: FanStatus,           // What the user selected
    pub actual_mode: ActualFanMode,     // What's actually written to device
    pub last_fan_write: Option<Instant>,
    pub consecutive_high_temps: u32,
    pub temp_monitoring_active: bool,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            user_mode: FanStatus::Bios,
            actual_mode: ActualFanMode::Bios,
            last_fan_write: None,
            consecutive_high_temps: 0,
            temp_monitoring_active: false,
        }
    }
}

impl std::fmt::Display for FanStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FanStatus::Max => write!(f, "Fans: Max"),
            FanStatus::Auto => write!(f, "Fans: Auto"),
            FanStatus::Bios => write!(f, "Fans: Bios"),
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

fn set_fan_mode(mode: ActualFanMode) -> Result<(), std::io::Error> {
    let value = match mode {
        ActualFanMode::Max => "0",
        ActualFanMode::Bios => "2",
    };
    
    println!("Setting actual fan mode to: {:?}", mode);
    
    // Use shell expansion to find the correct path
    let command = format!("echo {} | sudo tee /sys/devices/platform/hp-wmi/hwmon/hwmon*/pwm1_enable", value);
    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(&command)
        .output()?;
    
    if output.status.success() {
        println!("Successfully set fan mode");
        Ok(())
    } else {
        let error_msg = String::from_utf8_lossy(&output.stderr);
        Err(std::io::Error::new(
            std::io::ErrorKind::Other, 
            format!("Failed to set fan mode: {}", error_msg)
        ))
    }
}

fn read_temperature() -> Result<i32, std::io::Error> {
    let mut file = fs::File::open(TEMP_SENSOR_PATH)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    contents.trim().parse().map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

pub fn start_fan_control_thread(state: Arc<Mutex<AppState>>) {
    thread::spawn(move || {
        loop {
            thread::sleep(TEMP_CHECK_INTERVAL);
            
            let mut should_handle_max_mode = false;
            let mut should_handle_auto_mode = false;
            let user_mode;
            
            // Read current state
            {
                let state_guard = state.lock().unwrap();
                user_mode = state_guard.user_mode;
                
                if user_mode == FanStatus::Max {
                    if let Some(last_write) = state_guard.last_fan_write {
                        if last_write.elapsed() >= MAX_FAN_WRITE_INTERVAL {
                            should_handle_max_mode = true;
                        }
                    } else {
                        should_handle_max_mode = true;
                    }
                }
                
                if user_mode == FanStatus::Auto && state_guard.temp_monitoring_active {
                    should_handle_auto_mode = true;
                }
            }
            
            // Handle max mode timing
            if should_handle_max_mode {
                if let Err(e) = set_fan_mode(ActualFanMode::Max) {
                    eprintln!("Failed to set max fan mode: {}", e);
                } else {
                    let mut state_guard = state.lock().unwrap();
                    state_guard.last_fan_write = Some(Instant::now());
                    state_guard.actual_mode = ActualFanMode::Max;
                }
            }
            
            // Handle auto mode temperature monitoring
            if should_handle_auto_mode {
                match read_temperature() {
                    Ok(temp) => {
                        let mut state_guard = state.lock().unwrap();
                        
                        if temp > TEMP_THRESHOLD {
                            state_guard.consecutive_high_temps += 1;
                            println!("High temperature detected: {}°C (count: {})", 
                                   temp / 1000, state_guard.consecutive_high_temps);
                            
                            if state_guard.consecutive_high_temps >= CONSECUTIVE_HIGH_TEMP_LIMIT {
                                println!("Temperature threshold exceeded, switching to max fans");
                                state_guard.actual_mode = ActualFanMode::Max;
                                state_guard.consecutive_high_temps = 0;
                                state_guard.last_fan_write = Some(Instant::now());
                                drop(state_guard);
                                
                                if let Err(e) = set_fan_mode(ActualFanMode::Max) {
                                    eprintln!("Failed to set max fan mode: {}", e);
                                }
                            }
                        } else if state_guard.consecutive_high_temps > 0 {
                            println!("Temperature normal: {}°C, switching back to BIOS", temp / 1000);
                            state_guard.actual_mode = ActualFanMode::Bios;
                            state_guard.consecutive_high_temps = 0;
                            drop(state_guard);
                            
                            if let Err(e) = set_fan_mode(ActualFanMode::Bios) {
                                eprintln!("Failed to set bios fan mode: {}", e);
                            }
                        } else {
                            state_guard.consecutive_high_temps = 0;
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to read temperature: {}", e);
                    }
                }
            }
        }
    });
}

pub fn set_fan_status(state: Arc<Mutex<AppState>>, new_mode: FanStatus) -> Result<(), std::io::Error> {
    let actual_mode_to_set = match new_mode {
        FanStatus::Max => ActualFanMode::Max,
        FanStatus::Auto => ActualFanMode::Max, // Auto starts with max fans for immediate response
        FanStatus::Bios => ActualFanMode::Bios,
    };
    
    {
        let mut state_guard = state.lock().unwrap();
        state_guard.user_mode = new_mode;
        state_guard.actual_mode = actual_mode_to_set;
        state_guard.consecutive_high_temps = 0;
        
        match new_mode {
            FanStatus::Max => {
                state_guard.last_fan_write = Some(Instant::now());
                state_guard.temp_monitoring_active = false;
            }
            FanStatus::Auto => {
                state_guard.temp_monitoring_active = true;
                state_guard.last_fan_write = None;
            }
            FanStatus::Bios => {
                state_guard.temp_monitoring_active = false;
                state_guard.last_fan_write = None;
            }
        }
    }
    
    set_fan_mode(actual_mode_to_set)
}

pub fn fan_status_string(state: Arc<Mutex<AppState>>) -> String {
    let state_guard = state.lock().unwrap();
    format!("Fan State: {}", state_guard.user_mode)
}