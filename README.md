# Omenix

![Icon](assets/icon.png)

Fan control application for HP Omen laptops with system tray integration.

![Image](https://github.com/user-attachments/assets/13c1e324-f3e6-4423-8b02-354f7462cf72)

## Features

- **Fan Control**: Auto, Max Performance, or BIOS Default modes
- **System Tray**: Easy access via system tray icon
- **Daemon Architecture**: Background service with GUI frontend
- Sets max fans every 2 mins to avoid BIOS resetting it on some laptops

## Requirements

### System Tray Support

Omenix requires system tray support to display its icon and menu. Most desktop environments have this built-in, but some require additional setup:

#### GNOME (including Bazzite, Fedora, Ubuntu)

GNOME removed native system tray support. You need to install one of these extensions:

**Option 1: AppIndicator Support (Recommended)**
```bash
# For Fedora/Bazzite (using Flatpak)
flatpak install flathub org.gnome.Extensions
# Then install from: https://extensions.gnome.org/extension/615/appindicator-support/
```

**Option 2: Tray Icons: Reloaded**
```bash
# Install from: https://extensions.gnome.org/extension/2890/tray-icons-reloaded/
```

After installing, enable the extension in the GNOME Extensions app and restart your session (log out and back in).

#### Other Desktop Environments

- **KDE Plasma**: System tray built-in, no setup needed
- **XFCE**: System tray built-in, no setup needed
- **Cinnamon**: System tray built-in, no setup needed
- **MATE**: System tray built-in, no setup needed

### Hardware Support

In order for Omenix to work, you need to have `hp-wmi` kernel module loaded which should be the case for most HP laptops. You can check if it's loaded with `lsmod | grep hp_wmi`. Setting the fans to max with `echo 0 | sudo tee /sys/devices/platform/hp-wmi/hwmon/hwmon*/pwm1_enable` also needs to work. If it doesn't, your laptop may not be supported see the note below.

> You can check your board dmi by running `dmidecode` in your terminal and then look for `Product Name: 8BAB` or similar.
> If your board dmi as found by dmidecode is in the [hp-wmi driver](https://github.com/torvalds/linux/blob/37816488247ddddbc3de113c78c83572274b1e2e/drivers/platform/x86/hp/hp-wmi.c#L65C3-L65C49) it should work fine.
> If it is not there, you can patch the kernel module to add support for your board manually. I did this for my board you can read about it [here](https://noahpro99.github.io/content/how-i-ended-up-sending-in-my-first-linux-kernel-patch).

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

## Troubleshooting

### System Tray Icon Not Appearing

If the Omenix icon doesn't appear in your system tray:

1. **Check if the application is running**: Run `./omenix.AppImage` in a terminal and look for error messages
2. **GNOME users**: You MUST install a system tray extension (see Requirements section above)
3. **Verify the daemon is running**: Check that `omenix-daemon` is running with `ps aux | grep omenix-daemon`
4. **Check logs**: Run `./omenix.AppImage` and look for messages about tray icon creation
5. **Restart your session**: After installing GNOME extensions, log out and back in
6. **Verify extension is enabled**: Open "Extensions" app and ensure the tray extension is turned on

Common error messages:

- `Failed to create system tray icon`: Your desktop doesn't support system tray icons
  - Solution: Install the required GNOME extension (see Requirements)
- `Cannot connect to daemon`: The daemon isn't running
  - Solution: Run `sudo ./omenix-daemon.AppImage` in a separate terminal first

### Running from Terminal

Always run the GUI from a terminal to see error messages:

```bash
# Terminal 1 - Run daemon
sudo ./omenix-daemon.AppImage

# Terminal 2 - Run GUI (you should see logs here)
./omenix.AppImage
```
