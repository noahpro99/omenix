# Omenix Fan Control

A system tray application for controlling HP Omen laptop fan modes with GUI authentication using polkit, built with Nix for easy deployment.

## Features

- System tray integration with fan status display
- Three fan modes:
  - **Max**: Maximum fan speed for performance
  - **Auto**: Automatic temperature-based switching between Max and BIOS modes
  - **BIOS**: Default BIOS-controlled fan behavior
- GUI password prompt for secure privilege escalation via polkit
- Automatic temperature monitoring and logging
- Dark theme support
- Full Nix integration for declarative system management

## Prerequisites

- HP Omen laptop with supported fan control interface (`/sys/devices/platform/hp-wmi/hwmon/`)
- Linux system with polkit installed and properly configured
- Nix package manager or NixOS
- User account in the `wheel` group for polkit authentication

### Polkit Requirements

The application requires polkit for secure privilege escalation. Ensure:

1. **Polkit is installed**: Most modern Linux distributions include this
2. **pkexec is setuid root**: Usually configured automatically by the package manager
3. **User permissions**: Your user must be in the `wheel` group
4. **Authentication agent**: A polkit authentication agent must be running (part of most desktop environments)

Check your setup:
```bash
# Verify polkit is installed
which pkexec

# Check if pkexec is setuid root
ls -la $(which pkexec)
# Should show: -rwsr-xr-x (note the 's' in permissions)

# Verify you're in the wheel group
groups $USER | grep wheel

# Test polkit authentication
pkexec echo "Polkit test successful"
```

## Installation

### Option 1: Nix Flakes (Recommended)

#### Quick Install
```bash
# Install directly from GitHub
nix profile install github:noahpro99/omenix

# Run the application
omenix
```

#### NixOS Configuration

Add to your NixOS configuration:

```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    omenix.url = "github:noahpro99/omenix";
  };

  outputs = { nixpkgs, omenix, ... }: {
    nixosConfigurations.your-hostname = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [
        omenix.nixosModules.default
        {
          services.omenix.enable = true;
          
          # Optional: Add your user to wheel group for polkit access
          users.users.your-username.extraGroups = [ "wheel" ];
        }
      ];
    };
  };
}
```

#### Home Manager Integration

```nix
{
  inputs = {
    home-manager.url = "github:nix-community/home-manager";
    omenix.url = "github:noahpro99/omenix";
  };
  
  outputs = { home-manager, omenix, ... }: {
    homeConfigurations.your-username = home-manager.lib.homeManagerConfiguration {
      # ... your existing config ...
      modules = [
        {
          home.packages = [ omenix.packages.x86_64-linux.default ];
          
          # Auto-start with desktop session (optional)
          systemd.user.services.omenix = {
            Unit = {
              Description = "Omenix Fan Control";
              After = [ "graphical-session.target" ];
            };
            Service = {
              ExecStart = "${omenix.packages.x86_64-linux.default}/bin/omenix";
              Restart = "always";
            };
            Install = {
              WantedBy = [ "graphical-session.target" ];
            };
          };
        }
      ];
    };
  };
}
```

### Option 2: Development Setup

```bash
# Clone the repository
git clone https://github.com/noahpro99/omenix.git
cd omenix

# Enter development shell
nix develop

# Build and run
cargo build --release
./target/release/omenix
```

### Option 3: Build Package Locally

```bash
# Clone and build
git clone https://github.com/noahpro99/omenix.git
cd omenix

# Build the Nix package
nix build

# Install to profile
nix profile install .

# Run
omenix
```

## Usage

1. **System Tray Icon**: Look for the Omenix icon in your system tray
2. **Fan Modes**: Right-click the icon to access fan mode options:
   - **Fans Max**: Sets fans to maximum speed
   - **Fans Auto**: Enables automatic temperature-based control
   - **Fans BIOS**: Uses default BIOS fan control
3. **Authentication**: When changing modes, enter your password in the GUI polkit prompt

## Temperature Thresholds

- **Auto Mode**: Switches to Max when CPU temperature exceeds 75°C
- **Monitoring Interval**: Checks temperature every 5 seconds
- **Max Mode Refresh**: Rewrites max fan setting every 100 seconds

## Architecture

The application uses a secure architecture:

1. **Main Application**: Runs as user, handles GUI and tray interactions
2. **Polkit Policy**: Defines authentication requirements for fan control
3. **Helper Script**: Executes with elevated privileges only for fan mode changes
4. **Nix Integration**: Declaratively manages all components and dependencies

## Troubleshooting

### Permission Issues
- Ensure your user is in the `wheel` group: `groups $USER`
- On NixOS, add yourself to wheel group in configuration
- Verify polkit is running: `systemctl status polkit`

### pkexec setuid Issues
If you see "pkexec must be setuid root":

```bash
# Check pkexec permissions
ls -la $(which pkexec)
# Should show: -rwsr-xr-x (note the 's')

# If not setuid, fix it (as root):
sudo chmod u+s $(which pkexec)

# On some systems, you may need to reinstall polkit:
# Ubuntu/Debian: sudo apt reinstall policykit-1
# Fedora: sudo dnf reinstall polkit
# Arch: sudo pacman -S polkit
```

### NixOS-specific Issues
For NixOS users, ensure your configuration includes:
```nix
{
  security.polkit.enable = true;
  users.users.your-username.extraGroups = [ "wheel" ];
}
```

### Fan Control Not Working
- Check HP WMI interface: `ls /sys/devices/platform/hp-wmi/hwmon/`
- Verify hardware support: `cat /sys/devices/platform/hp-wmi/hwmon/hwmon*/pwm1_enable`
- Check system logs: `journalctl -f` while running the application

### GUI Authentication Issues
- Ensure a polkit authentication agent is running
- On minimal setups, install: `polkit-gnome`, `lxpolkit`, or equivalent
- For NixOS: `security.polkit.enable = true;` and ensure desktop environment includes auth agent

### Nix-specific Issues
- Update flake inputs: `nix flake update`
- Clear build cache: `nix store gc`
- Rebuild with verbose output: `nix build --verbose`

## Development

### Environment Setup
```bash
nix develop
```

### Logging
```bash
# Debug level
RUST_LOG=omenix=debug ./target/release/omenix

# Trace level  
RUST_LOG=trace cargo run
```

### Testing Changes
```bash
# Build and test locally
nix build

# Install test version
nix profile install .
```

## Security Model

- **Principle of Least Privilege**: Only the minimal helper script runs with elevated privileges
- **User Authentication**: Polkit ensures user authentication for each fan mode change
- **Input Validation**: Helper script validates all inputs before system calls
- **Audit Trail**: All operations are logged for security auditing

## Contributing

1. Fork the repository
2. Create a feature branch
3. Test with `nix build` and `nix develop`
4. Ensure NixOS module works correctly
5. Submit a pull request

## License

MIT License - see LICENSE file for details

## Support

- **Issues**: Report bugs on GitHub Issues
- **Discussions**: Use GitHub Discussions for questions
- **Wiki**: Check the GitHub Wiki for additional documentation
