use std::io;
use tracing::{debug, error, info, warn};

/// Handle authentication and privilege escalation for fan control operations
pub struct AuthHandler {
    helper_path: String,
    pkexec_path: String,
}

impl AuthHandler {
    pub fn new() -> Self {
        let helper_path = std::env::var("OMENIX_HELPER_PATH")
            .unwrap_or_else(|_| "/etc/omenix/omenix-fancontrol".to_string());

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

        Self {
            helper_path,
            pkexec_path,
        }
    }

    /// Execute a privileged command using polkit authentication
    pub fn execute_privileged(&self, fan_mode_value: &str) -> Result<(), io::Error> {
        info!("Executing privileged fan control command with value: {}", fan_mode_value);

        // Verify helper script exists
        if !std::path::Path::new(&self.helper_path).exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Helper script not found at: {}", self.helper_path),
            ));
        }

        // Use polkit to authenticate and run the fan control script
        let output = std::process::Command::new(&self.pkexec_path)
            .arg(&self.helper_path)
            .arg(fan_mode_value)
            .output()?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            info!("Successfully executed privileged command, output: {}", stdout.trim());
            Ok(())
        } else {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            error!("Failed to execute privileged command: {}", error_msg);

            // Check if the error is due to polkit cancellation
            if error_msg.contains("Request dismissed") || error_msg.contains("Operation was cancelled") {
                warn!("User cancelled the authentication request");
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "Authentication cancelled by user",
                ));
            }

            // Check if pkexec is not setuid root
            if error_msg.contains("pkexec must be setuid root") {
                error!("pkexec is not properly configured - it must be setuid root");
                error!("Current pkexec: {}", self.pkexec_path);
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "pkexec is not setuid root. Please install system polkit or use NixOS module.",
                ));
            }

            Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to execute privileged command: {}", error_msg),
            ))
        }
    }
}

impl Default for AuthHandler {
    fn default() -> Self {
        Self::new()
    }
}
