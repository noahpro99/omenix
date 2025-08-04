use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use tracing::{debug, error, info, warn};

const DAEMON_SOCKET_PATH: &str = "/tmp/omenix-daemon.sock";

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FanStatus {
    Max,
    Auto,
    Bios,
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

impl std::str::FromStr for FanStatus {
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

/// Client for communicating with the daemon
pub struct DaemonClient;

impl DaemonClient {
    pub fn new() -> Self {
        Self
    }

    /// Send a command to the daemon and get response
    fn send_command(&self, command: &str) -> Result<String, std::io::Error> {
        debug!("Connecting to daemon at: {}", DAEMON_SOCKET_PATH);

        let mut stream = UnixStream::connect(DAEMON_SOCKET_PATH).map_err(|e| {
            error!("Failed to connect to daemon: {}. Is the daemon running?", e);
            std::io::Error::new(
                std::io::ErrorKind::ConnectionRefused,
                format!(
                    "Cannot connect to daemon: {}. Make sure omenix-daemon is running as root.",
                    e
                ),
            )
        })?;

        debug!("Sending command: {}", command);
        stream.write_all(command.as_bytes())?;

        let mut response = String::new();
        stream.read_to_string(&mut response)?;

        debug!("Received response: {}", response.trim());
        Ok(response)
    }

    /// Set fan mode via daemon
    pub fn set_fan_mode(&self, mode: FanStatus) -> Result<(), std::io::Error> {
        info!("Setting fan mode to: {:?}", mode);

        let command = format!("set {}", mode.to_string().to_lowercase());
        let response = self.send_command(&command)?;

        if response.starts_with("OK:") {
            info!("Successfully set fan mode to: {:?}", mode);
            Ok(())
        } else if response.starts_with("ERROR:") {
            let error_msg = response.strip_prefix("ERROR:").unwrap_or(&response).trim();
            error!("Daemon error: {}", error_msg);
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Daemon error: {}", error_msg),
            ))
        } else {
            warn!("Unexpected response from daemon: {}", response);
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Unexpected response from daemon",
            ))
        }
    }

    /// Get current status from daemon
    pub fn get_status(&self) -> Result<String, std::io::Error> {
        debug!("Getting status from daemon");

        let response = self.send_command("status")?;

        if response.starts_with("OK:") {
            let status = response.strip_prefix("OK:").unwrap_or(&response).trim();
            debug!("Current status: {}", status);
            Ok(status.to_string())
        } else if response.starts_with("ERROR:") {
            let error_msg = response.strip_prefix("ERROR:").unwrap_or(&response).trim();
            warn!("Error getting status: {}", error_msg);
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Error getting status: {}", error_msg),
            ))
        } else {
            warn!("Unexpected response from daemon: {}", response);
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Unexpected response from daemon",
            ))
        }
    }

    /// Check if daemon is running
    pub fn is_daemon_running(&self) -> bool {
        self.get_status().is_ok()
    }
}

impl Default for DaemonClient {
    fn default() -> Self {
        Self::new()
    }
}
