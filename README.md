# Omenix

![Icon](assets/icon.png)

Fan control application for HP Omen laptops with system tray integration.

![Image](readme-assets/image.png)

## Features

- **Fan Control**: Auto, Max Performance, or BIOS Default modes
- **System Tray**: Easy access via system tray icon
- **Daemon Architecture**: Background service with GUI frontend
- Sets max fans every 2 mins to avoid BIOS resetting it on some laptops

## Quick Start

In order for Omenix to work, you need to have `hp-wmi` kernel module loaded which should be the case for most HP laptops. You can check if it's loaded with `lsmod | grep hp_wmi`. Setting the fans with `echo 2 | sudo tee /sys/devices/platform/hp-wmi/hwmon/hwmon*/pwm1_enable` also needs to work. If it doesn't, your laptop may not be supported.

> You can see it by running dmidecode look for `Product Name: 8BAB` or similar.
>
> If your board dmi as found by dmidecode is in the [hp-wmi driver](https://github.com/torvalds/linux/blob/37816488247ddddbc3de113c78c83572274b1e2e/drivers/platform/x86/hp/hp-wmi.c#L65C3-L65C49) it should work fine.
> If not you can patch the kernel module to add support for your board. I did this for my board you can read about it [here](https://noahpro99.github.io/content/how-i-ended-up-sending-in-my-first-linux-kernel-patch).

## Configuration

Omenix can optionally be configured via a YAML configuration file at `/etc/omenix-daemon.yaml`. The daemon supports the following options:

Defaults and example configuration file:

```yaml
temp_threshold_high: 75 # Temperature in Celsius to trigger max fan mode
temp_threshold_low: 70 # Hysteresis to avoid rapid switching
consecutive_high_temp_limit: 3 # Number of consecutive high temp readings to trigger max fan mode
consecutive_low_temp_limit: 3 # Number of consecutive low temp readings to switch back to BIOS control
temp_check_interval: 5 # Check temperature every x seconds
# max_fan_write_interval: 120 # Set to 120 seconds to rewrite max fan mode every 2 minutes to avoid BIOS resetting it if needed (this is off by default)
```

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
./omenix.AppImage
```

Some distributions may require `fuse` to be installed such as Arch Linux.
