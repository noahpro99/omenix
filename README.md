# Omenix

![Icon](assets/icon.png)

Fan control application for HP Omen laptops with system tray integration.

![Image](readme-assets/image.png)

## Features

- **Fan Control**: Auto, Max Performance, or BIOS Default modes
- **Performance Modes**: Balanced and Performance profiles
- **System Tray**: Easy access via system tray icon
- **Temperature Monitoring**: Real-time CPU temperature display
- **Daemon Architecture**: Background service with GUI frontend

## Quick Start

### NixOS Users

Add to your system configuration:

```nix
{
  inputs.omenix.url = "github:noahpro99/omenix";

  # In your system configuration:
  packages.omenix.enable = true;
}
```

Run the GUI:

```bash
omenix
```

If you have a desktop environment like hyprland:

```
exec-once = omenix
```

### Non-NixOS with Nix Package Manager

Install and run:

```bash
nix profile install github:noahpro99/omenix#omenix
nix profile install github:noahpro99/omenix#omenix-daemon

sudo omenix-daemon
omenix
```

### Non-NixOS without Nix Package Manager

Download the latest AppImage release from the [Releases page](https://github.com/noahpro99/omenix/releases).

```bash
chmod +x omenix*.AppImage

sudo ./omenix-daemon.AppImage
./omenix-gui.AppImage
```

some distributions may require `fuse` to be installed.
