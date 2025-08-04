/// Shared types used across the daemon and client
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FanMode {
    Max,  // Force fans to maximum speed
    Auto, // Temperature-based automatic control
    Bios, // Let BIOS handle fan control
}

impl fmt::Display for FanMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FanMode::Max => write!(f, "Max"),
            FanMode::Auto => write!(f, "Auto"),
            FanMode::Bios => write!(f, "Bios"),
        }
    }
}

impl std::str::FromStr for FanMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "max" => Ok(FanMode::Max),
            "auto" => Ok(FanMode::Auto),
            "bios" => Ok(FanMode::Bios),
            _ => Err(format!("Invalid fan mode: {}", s)),
        }
    }
}

/// Hardware-level fan modes (what actually gets written to device)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HardwareFanMode {
    Max,  // Writing 0 to device
    Bios, // Writing 2 to device
}

/// Status information from the daemon
#[derive(Debug, Clone)]
pub struct FanStatus {
    pub user_mode: FanMode,
    pub hardware_mode: HardwareFanMode,
    pub temperature: Option<i32>, // in millicelsius
}

impl fmt::Display for FanStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let temp_str = match self.temperature {
            Some(temp) => format!("{}°C", temp / 1000),
            None => "Unknown".to_string(),
        };
        write!(
            f,
            "Mode: {}, Hardware: {:?}, Temp: {}",
            self.user_mode, self.hardware_mode, temp_str
        )
    }
}

/// Messages sent from GUI to daemon
pub enum TrayMessage {
    SetMode(FanMode),
    Exit,
}
