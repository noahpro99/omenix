use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use tracing::{debug, error, info, warn};

use crate::types::{FanMode, PerformanceMode, SystemState};

pub const DAEMON_SOCKET_PATH: &str = "/tmp/omenix-daemon.sock";

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
    pub fn set_fan_mode(&self, mode: FanMode) -> Result<(), std::io::Error> {
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

    /// Set performance mode via daemon
    pub fn set_performance_mode(&self, mode: PerformanceMode) -> Result<(), std::io::Error> {
        info!("Setting performance mode to: {:?}", mode);

        let command = format!("set_performance {}", mode);
        let response = self.send_command(&command)?;

        if response.starts_with("OK:") {
            info!("Successfully set performance mode to: {:?}", mode);
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

    /// Get current system state from daemon
    pub fn get_current_state(&self) -> Result<SystemState, std::io::Error> {
        debug!("Getting current state from daemon");

        let response = self.send_command("status")?;

        if response.starts_with("OK:") {
            let status_data = response.strip_prefix("OK:").unwrap_or(&response).trim();

            // Parse the status response: "Mode: Auto, Actual: Max, Performance: balanced, Temp: 45°C"
            let fan_mode = if status_data.contains("Mode: Max") {
                FanMode::Max
            } else if status_data.contains("Mode: Auto") {
                FanMode::Auto
            } else {
                FanMode::Bios
            };

            // Parse performance mode from status
            let performance_mode = if status_data.contains("Performance: performance") {
                PerformanceMode::Performance
            } else {
                PerformanceMode::Balanced
            };

            // Extract temperature if available
            let temperature = None; // Would be parsed from daemon response

            Ok(SystemState {
                fan_mode,
                performance_mode,
                temperature,
                error_message: None,
            })
        } else if response.starts_with("ERROR:") {
            let error_msg = response.strip_prefix("ERROR:").unwrap_or(&response).trim();
            error!("Error getting state: {}", error_msg);
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Error getting state: {}", error_msg),
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
