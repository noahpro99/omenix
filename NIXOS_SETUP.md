# NixOS Configuration for Omenix

To enable Omenix fan control on NixOS, add the following to your `configuration.nix`:

## Basic Setup

```nix
{
  # Import the Omenix flake
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    omenix.url = "github:noahpro99/omenix";
  };

  outputs = { nixpkgs, omenix, ... }: {
    nixosConfigurations.your-hostname = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [
        # Import the Omenix module
        omenix.nixosModules.default

        {
          # Enable Omenix service
          services.omenix.enable = true;

          # Ensure your user is in the wheel group for polkit authentication
          users.users.your-username.extraGroups = [ "wheel" ];

          # Optional: Install the package for all users
          environment.systemPackages = [ omenix.packages.x86_64-linux.default ];
        }
      ];
    };
  };
}
```

## What this configuration does:

1. **Enables polkit**: Automatically enables the polkit service
2. **Creates setuid wrapper**: Creates `/run/wrappers/bin/pkexec` with proper setuid permissions
3. **Installs policy**: Installs the polkit policy for fan control authentication
4. **Sets environment**: Sets `PKEXEC_PATH` to use the wrapper
5. **Helper script**: Installs the fan control helper script

## Manual Installation (if not using flakes)

If you're not using flakes, you can manually configure the required components:

```nix
# configuration.nix
{
  # Enable polkit
  security.polkit.enable = true;

  # Add polkit rule for omenix
  security.polkit.extraConfig = ''
    polkit.addRule(function(action, subject) {
        if (action.id == "com.omenix.fancontrol.setfanmode" &&
            subject.isInGroup("wheel")) {
            return polkit.Result.YES;
        }
    });
  '';

  # Ensure pkexec is setuid root
  security.wrappers.pkexec = {
    owner = "root";
    group = "root";
    setuid = true;
    setgid = false;
    source = "${pkgs.polkit}/bin/pkexec";
  };

  # Set environment variable for omenix
  environment.variables.PKEXEC_PATH = "/run/wrappers/bin/pkexec";

  # Add your user to wheel group
  users.users.your-username.extraGroups = [ "wheel" ];
}
```

## Verification

After rebuilding your system (`sudo nixos-rebuild switch`), verify the setup:

```bash
# Check if pkexec wrapper exists and is setuid
ls -la /run/wrappers/bin/pkexec
# Should show: -rwsr-xr-x (note the 's' for setuid)

# Check environment variable
echo $PKEXEC_PATH
# Should show: /run/wrappers/bin/pkexec

# Test polkit authentication
pkexec echo "Test successful"
# Should prompt for password and then print "Test successful"

# Check if you're in wheel group
groups $USER | grep wheel
```

## Troubleshooting

### If fan control still doesn't work:

1. **Restart polkit service**:

   ```bash
   sudo systemctl restart polkit
   ```

2. **Check polkit authentication agent**:

   ```bash
   systemctl --user status polkit-kde-agent
   # or
   systemctl --user status polkit-gnome-authentication-agent-1
   ```

3. **Check HP WMI interface**:
   ```bash
   ls /sys/devices/platform/hp-wmi/hwmon/
   ```

### If authentication prompts don't appear:

Make sure a polkit authentication agent is running. On most desktop environments this happens automatically, but you can start one manually:

```bash
# For KDE
systemctl --user start polkit-kde-agent

# For GNOME
systemctl --user start polkit-gnome-authentication-agent-1

# For Hyprland (as you're using)
systemctl --user start hyprpolkitagent
```
