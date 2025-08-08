# Omenix Copilot Instructions

## Project Overview

Omenix is a fan control application for HP Omen laptops built in Rust with a daemon-client architecture. The system consists of two binaries:

- **omenix-daemon**: Root-privileged background service that manages hardware (fans, temperature, performance modes)
- **omenix**: GTK-based GUI client with system tray integration

## Architecture & Communication

### Daemon-Client Pattern

- Communication via Unix domain socket at `/tmp/omenix-daemon.sock`
- Daemon must run as root for hardware access, GUI runs as user
- Protocol: simple text commands (`set max`, `status`, `set_performance balanced`)
- See `src/client.rs` for command format and `src/bin/omenix-daemon.rs` for handlers

### Key Components

- `src/types.rs`: Shared enums (FanMode, PerformanceMode, HardwareFanMode) with string conversion
- `src/client.rs`: DaemonClient handles socket communication and error handling
- `src/tray.rs`: TrayManager with GTK integration and menu state management
- `src/bin/omenix-daemon.rs`: Hardware control logic with temperature monitoring

### Hardware Integration

- Fan control: `/sys/devices/platform/hp-wmi/hwmon/hwmon*/pwm1_enable` (0=max, 2=bios)
- Temperature: `/sys/class/thermal/thermal_zone*/temp` (millicelsius)
- Performance: `/sys/firmware/acpi/platform_profile` (balanced/performance)
- Uses glob patterns for hardware file discovery

## Development Workflows

### Building & Testing

```bash
# Development builds
cargo build --bin omenix-daemon
cargo build --bin omenix

# Run daemon (requires root)
sudo target/debug/omenix-daemon

# Run GUI client
target/debug/omenix
```

### Nix Development

```bash
# Nix builds (preferred)
nix build .#omenix
nix build .#omenix-daemon

# Development shell
nix develop

# AppImage bundling
nix bundle --bundler github:ralismark/nix-appimage .#packages.x86_64-linux.omenix
```

## Project-Specific Patterns

### Error Handling Convention

- Use `tracing` crate for structured logging with levels (debug, info, warn, error)
- Hardware operations return `io::Error` with descriptive messages
- Client errors include daemon connectivity context

### State Management

- Daemon maintains `DaemonState` with user preferences vs hardware state
- Auto mode implements temperature-based control with thresholds (`TEMP_THRESHOLD: 75°C`)
- Tray menu caches state to avoid frequent daemon queries

### Threading Architecture

- GUI uses mpsc channels for message passing between tray and daemon communication
- Background temperature monitoring thread in daemon
- Separate quit signal handling to ensure clean shutdown

### Asset Management

- Icons loaded via `OMENIX_ASSETS_DIR` environment variable (set by Nix)
- Fallback to relative paths for development: `assets/icon.png`
- GTK dark theme preference set programmatically

## Critical Integration Points

### Nix Build System

- `flake.nix` defines separate packages for daemon and GUI
- Assets copied to Nix store with wrapper setting `OMENIX_ASSETS_DIR`
- Library path wrapping for GTK dependencies
- AppImage generation for non-Nix distributions

### Hardware Safety

- Fan writes limited to 100-second intervals to prevent hardware damage
- Temperature monitoring with consecutive high temp counting
- Graceful fallback to BIOS control on errors

### System Tray Integration

- Uses `tray-icon` crate with platform-specific behavior
- Menu rebuilding on state changes to reflect current modes
- Cross-platform icon handling (Linux focus)

When making changes, always consider the root/user privilege boundary and ensure proper error propagation through the client-daemon interface.
