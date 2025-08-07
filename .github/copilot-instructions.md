# Omenix Fan Control

Always reference these instructions first and fallback to search or bash commands only when you encounter unexpected information that does not match the information here.

Omenix is a Rust-based fan control application for HP Omen laptops with GTK3 GUI and system tray integration. The application consists of a daemon that controls the hardware and a GUI frontend that communicates via Unix sockets.

## Working Effectively

### Bootstrap and Build
Run these commands in sequence to set up the development environment:

```bash
# Install required system dependencies
sudo apt update
sudo apt install -y libgtk-3-dev pkg-config libssl-dev build-essential libxdo-dev

# Build the project
cargo check        # Takes ~2m 44s on first run. NEVER CANCEL. Set timeout to 5+ minutes.
cargo build        # Takes ~3s after dependencies are built
cargo build --release  # Takes ~1m 50s. NEVER CANCEL. Set timeout to 5+ minutes.
```

**CRITICAL TIMING WARNINGS:**
- Initial `cargo check` takes **2 minutes 44 seconds** - NEVER CANCEL, set timeout to 5+ minutes
- Release build takes **1 minute 50 seconds** - NEVER CANCEL, set timeout to 5+ minutes
- Subsequent builds are much faster (~3 seconds) after initial dependency compilation

### Testing and Validation
```bash
# Run tests (currently none exist - 0 tests will run)
cargo test

# Check code formatting (should pass)
cargo fmt --check

# Run linter (will show 16 warnings about format strings - this is expected)
cargo clippy -- -D warnings  # Will fail due to known format string warnings
cargo clippy  # Run without -D warnings to see issues without failing
```

**Known Issues:**
- Clippy shows 16 warnings about `uninlined_format_args` and `io_other_error` - these are style warnings, not blocking issues
- No unit tests exist in the codebase currently

### Runtime Execution
```bash
# Set assets directory for development
export OMENIX_ASSETS_DIR="$(pwd)/assets"

# Run the GUI (requires X11/Wayland display)
./target/debug/omenix
# Note: Will fail with "Failed to initialize GTK" in headless environments - this is expected

# Run the daemon (requires root privileges)
sudo ./target/debug/omenix-daemon
# Note: Will fail with "Daemon must be run as root" if not run as root - this is expected
```

**Runtime Requirements:**
- Daemon MUST be run as root to access HP Omen hardware controls
- GUI requires X11 or Wayland display server
- Both binaries look for assets in `OMENIX_ASSETS_DIR` environment variable or fallback to relative paths

### For NixOS Users
If Nix is available:
```bash
# Run directly from GitHub
nix run github:noahpro99/omenix

# Or install to profile  
nix profile install github:noahpro99/omenix
omenix

# Development shell with all dependencies
nix develop
```

## Validation Scenarios

**CRITICAL**: Always validate your changes using these specific scenarios:

### Build Validation
```bash
# Clean build test
cargo clean
cargo build --release  # Must complete in ~1m 50s without errors
```

### Runtime Validation  
```bash
# Test daemon startup (expect root privilege error unless run as root)
./target/debug/omenix-daemon 2>&1 | grep -q "Daemon must be run as root"

# Test GUI startup (expect GTK error in headless environment)  
./target/debug/omenix 2>&1 | grep -q "Failed to initialize GTK"

# Test assets loading
OMENIX_ASSETS_DIR="$(pwd)/assets" ./target/debug/omenix 2>&1
```

### Code Quality Validation
```bash
# Format check must pass
cargo fmt --check

# Build must succeed even with clippy warnings
cargo clippy  # Shows warnings but should not fail compilation
```

## Project Structure

### Key Files and Directories
```
/
├── src/
│   ├── bin/
│   │   ├── omenix.rs          # Main GUI application
│   │   └── omenix-daemon.rs   # Background daemon (requires root)
│   ├── lib.rs                 # Shared library modules
│   ├── client.rs              # Daemon communication client
│   ├── tray.rs                # System tray integration
│   └── types.rs               # Shared types (FanMode, PerformanceMode)
├── assets/
│   └── icon.png               # System tray icon
├── Cargo.toml                 # Rust package configuration
├── flake.nix                  # Nix package configuration
└── README.md                  # Project documentation
```

### Key Components
- **GUI (`omenix`)**: GTK3-based system tray application for user interaction
- **Daemon (`omenix-daemon`)**: Root-privileged service that controls HP Omen hardware
- **Communication**: Unix socket at `/tmp/omenix-daemon.sock`
- **Assets**: Icon files loaded via `OMENIX_ASSETS_DIR` environment variable

## Common Development Tasks

### Adding New Features
1. Always ensure daemon runs as root for hardware access
2. GUI components use GTK3 - check GTK documentation for widgets
3. Communication between GUI and daemon uses Unix sockets with simple string protocol
4. Fan modes: `Max`, `Auto`, `Bios`
5. Performance modes: `Balanced`, `Performance`

### Debugging Issues
```bash
# Check daemon logs (it uses tracing for structured logging)
RUST_LOG=debug ./target/debug/omenix-daemon

# Check GUI logs  
RUST_LOG=debug ./target/debug/omenix

# Verify socket communication
ls -la /tmp/omenix-daemon.sock  # Should exist when daemon is running
```

### Before Submitting Changes
Always run this complete validation sequence:
```bash
# Code quality
cargo fmt --check           # Must pass
cargo clippy                # Check warnings (16 format string warnings expected)

# Build validation  
cargo clean
cargo build --release       # Must complete in ~1m 50s - NEVER CANCEL

# Runtime smoke test
export OMENIX_ASSETS_DIR="$(pwd)/assets"
timeout 5s ./target/debug/omenix-daemon 2>&1 | grep -q "must be run as root"
timeout 5s ./target/debug/omenix 2>&1 | grep -q "Failed to initialize GTK"
```

## Dependencies and Environment

### System Dependencies
These packages are required and have been validated to work:
```bash
sudo apt install -y \
  libgtk-3-dev \
  pkg-config \
  libssl-dev \
  build-essential \
  libxdo-dev
```

### Rust Dependencies
Managed automatically by Cargo - see `Cargo.toml` for complete list. Key dependencies:
- `gtk = "0.18.2"` - GUI framework
- `tray-icon = "0.21.1"` - System tray integration  
- `tracing = "0.1"` - Structured logging
- `libxdo = "0.6.0"` - X11 automation library

### Environment Variables
- `OMENIX_ASSETS_DIR`: Directory containing icon.png and other assets
- `RUST_LOG`: Controls logging verbosity (debug, info, warn, error)

## Hardware Requirements

This application is specifically designed for HP Omen laptops and requires:
- HP Omen laptop hardware with fan control support
- Linux operating system  
- Root access for hardware control
- X11 or Wayland for GUI display

**Note**: The application will build and partially run on non-HP hardware but hardware control features will not function.