{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";
  };

  outputs = { nixpkgs, ... }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};

      # Helper script for fan control
      omenix-fancontrol = pkgs.writeShellScript "omenix-fancontrol" ''
        #!/bin/bash
        # Omenix Fan Control Helper Script
        # This script is used with polkit to manage fan modes

        if [ "$#" -ne 1 ]; then
            echo "Usage: $0 <fan_mode>"
            echo "fan_mode: 0 (max) or 2 (bios)"
            exit 1
        fi

        FAN_MODE="$1"

        # Validate fan mode
        if [ "$FAN_MODE" != "0" ] && [ "$FAN_MODE" != "2" ]; then
            echo "Error: Invalid fan mode. Use 0 (max) or 2 (bios)"
            exit 1
        fi

        # Find the fan control path
        FAN_PATH=$(find /sys/devices/platform/hp-wmi/hwmon/hwmon*/pwm1_enable 2>/dev/null | head -n1)

        if [ -z "$FAN_PATH" ]; then
            echo "Error: Fan control path not found"
            exit 1
        fi

        # Write the fan mode
        echo "$FAN_MODE" > "$FAN_PATH"

        if [ $? -eq 0 ]; then
            echo "Successfully set fan mode to $FAN_MODE"
            exit 0
        else
            echo "Error: Failed to set fan mode"
            exit 1
        fi
      '';

      # Polkit policy file
      polkit-policy = pkgs.writeTextFile {
        name = "com.omenix.fancontrol.policy";
        text = ''
          <?xml version="1.0" encoding="UTF-8"?>
          <!DOCTYPE policyconfig PUBLIC
           "-//freedesktop//DTD PolicyKit Policy Configuration 1.0//EN"
           "http://www.freedesktop.org/standards/PolicyKit/1/policyconfig.dtd">
          <policyconfig>
            <vendor>Omenix</vendor>
            <vendor_url>https://github.com/noahpro99/omenix</vendor_url>
            
            <action id="com.omenix.fancontrol.setfanmode">
              <description>Set fan control mode</description>
              <message>Authentication is required to change the fan control mode</message>
              <defaults>
                <allow_any>auth_admin</allow_any>
                <allow_inactive>auth_admin</allow_inactive>
                <allow_active>auth_admin</allow_active>
              </defaults>
              <annotate key="org.freedesktop.policykit.exec.path">${omenix-fancontrol}</annotate>
            </action>
          </policyconfig>
        '';
        destination = "/share/polkit-1/actions/com.omenix.fancontrol.policy";
      };

      # Main Omenix package
      omenix = pkgs.rustPlatform.buildRustPackage {
        pname = "omenix";
        version = "0.1.0";

        src = ./.;

        cargoLock = {
          lockFile = ./Cargo.lock;
        };

        nativeBuildInputs = with pkgs; [
          pkg-config
          wrapGAppsHook3
        ];

        buildInputs = with pkgs; [
          gtk3
          libappindicator-gtk3
          libayatana-appindicator
          xdotool
          polkit
        ];

        # Install additional files
        postInstall = ''
          # Install the helper script
          install -Dm755 ${omenix-fancontrol} $out/bin/omenix-fancontrol
          
          # Install polkit policy
          install -Dm644 ${polkit-policy}/share/polkit-1/actions/com.omenix.fancontrol.policy \
                         $out/share/polkit-1/actions/com.omenix.fancontrol.policy
          
          # Install icon
          install -Dm644 assets/icon.png $out/share/icons/hicolor/256x256/apps/omenix.png
          
          # Create desktop file
          mkdir -p $out/share/applications
          cat > $out/share/applications/omenix.desktop << EOF
          [Desktop Entry]
          Name=Omenix Fan Control
          Comment=Control HP Omen laptop fan modes
          Exec=$out/bin/omenix
          Icon=omenix
          Type=Application
          Categories=System;Settings;
          StartupNotify=false
          NoDisplay=true
          EOF
        '';

        # Fix the fan control path in the binary
        postFixup = ''
          wrapProgram $out/bin/omenix \
            --prefix PATH : ${pkgs.lib.makeBinPath [ pkgs.polkit ]} \
            --set OMENIX_HELPER_PATH $out/bin/omenix-fancontrol \
            --set PKEXEC_PATH ${pkgs.polkit}/bin/pkexec
        '';

        meta = with pkgs.lib; {
          description = "HP Omen laptop fan control with GUI authentication";
          homepage = "https://github.com/noahpro99/omenix";
          license = licenses.mit;
          maintainers = [ ];
          platforms = platforms.linux;
        };
      };

    in
    {
      packages.${system} = {
        default = omenix;
        inherit omenix;
      };

      # NixOS module for system integration
      nixosModules.default = { config, lib, pkgs, ... }:
        let
          cfg = config.services.omenix;

          # Helper script for fan control (defined here to avoid circular dependency)
          omenix-fancontrol-module = pkgs.writeShellScript "omenix-fancontrol" ''
            #!/bin/bash
            # Omenix Fan Control Helper Script
            # This script is used with polkit to manage fan modes

            if [ "$#" -ne 1 ]; then
                echo "Usage: $0 <fan_mode>"
                echo "fan_mode: 0 (max) or 2 (bios)"
                exit 1
            fi

            FAN_MODE="$1"

            # Validate fan mode
            if [ "$FAN_MODE" != "0" ] && [ "$FAN_MODE" != "2" ]; then
                echo "Error: Invalid fan mode. Use 0 (max) or 2 (bios)"
                exit 1
            fi

            # Find the fan control path
            FAN_PATH=$(find /sys/devices/platform/hp-wmi/hwmon/hwmon*/pwm1_enable 2>/dev/null | head -n1)

            if [ -z "$FAN_PATH" ]; then
                echo "Error: Fan control path not found"
                exit 1
            fi

            # Write the fan mode
            echo "$FAN_MODE" > "$FAN_PATH"

            if [ $? -eq 0 ]; then
                echo "Successfully set fan mode to $FAN_MODE"
                exit 0
            else
                echo "Error: Failed to set fan mode"
                exit 1
            fi
          '';

          # Polkit policy file for the module
          polkit-policy-module = pkgs.writeTextFile {
            name = "com.omenix.fancontrol.policy";
            text = ''
              <?xml version="1.0" encoding="UTF-8"?>
              <!DOCTYPE policyconfig PUBLIC
               "-//freedesktop//DTD PolicyKit Policy Configuration 1.0//EN"
               "http://www.freedesktop.org/standards/PolicyKit/1/policyconfig.dtd">
              <policyconfig>
                <vendor>Omenix</vendor>
                <vendor_url>https://github.com/noahpro99/omenix</vendor_url>
                
                <action id="com.omenix.fancontrol.setfanmode">
                  <description>Set fan control mode</description>
                  <message>Authentication is required to change the fan control mode</message>
                  <defaults>
                    <allow_any>auth_admin</allow_any>
                    <allow_inactive>auth_admin</allow_inactive>
                    <allow_active>auth_admin</allow_active>
                  </defaults>
                  <annotate key="org.freedesktop.policykit.exec.path">${omenix-fancontrol-module}</annotate>
                </action>
              </policyconfig>
            '';
            destination = "/share/polkit-1/actions/com.omenix.fancontrol.policy";
          };
        in
        {
          options.services.omenix = {
            enable = lib.mkEnableOption "Omenix fan control service";

            package = lib.mkOption {
              type = lib.types.package;
              default = omenix;
              description = "The Omenix package to use";
            };
          };

          config = lib.mkIf cfg.enable {
            # Ensure polkit is enabled
            security.polkit.enable = true;

            # Install polkit policy system-wide
            security.polkit.extraConfig = ''
              polkit.addRule(function(action, subject) {
                  if (action.id == "com.omenix.fancontrol.setfanmode" &&
                      subject.isInGroup("wheel")) {
                      return polkit.Result.YES;
                  }
              });
            '';

            # Install the package and policy
            environment.systemPackages = [ cfg.package ];

            # Ensure polkit policy is installed (using module version to avoid circular dependency)
            environment.etc."polkit-1/actions/com.omenix.fancontrol.policy".source =
              "${polkit-policy-module}/share/polkit-1/actions/com.omenix.fancontrol.policy";

            # Create a symlink for the helper script in the expected location
            environment.etc."omenix/omenix-fancontrol".source = omenix-fancontrol-module;

            # Ensure pkexec is setuid root - this is the key fix!
            security.wrappers.pkexec = {
              owner = "root";
              group = "root";
              setuid = true;
              setgid = false;
              source = "${pkgs.polkit}/bin/pkexec";
            };

            # Set environment variable to use the wrapped pkexec
            environment.variables.PKEXEC_PATH = "/run/wrappers/bin/pkexec";
          };
        };

      devShells.${system}.default = pkgs.mkShell {
        buildInputs = with pkgs; [
          # Runtime libraries
          gtk3
          xdotool
          libappindicator-gtk3
          libayatana-appindicator
          polkit
        ];

        nativeBuildInputs = with pkgs; [
          # Build tools
          cargo
          rustc
          rust-analyzer
          clippy
          bashInteractive
          gcc
          openssl
          pkg-config
          libiconv
        ];

        shellHook = ''
          # Make sure dynamic linker can find the GTK/AppIndicator .so files
          export LD_LIBRARY_PATH="${
            pkgs.lib.makeLibraryPath [
              pkgs.libayatana-appindicator
              pkgs.libappindicator-gtk3
              pkgs.gtk3
            ]
          }:$LD_LIBRARY_PATH"

          # Helpful for some GTK apps so schemas/icons resolve
          export XDG_DATA_DIRS="${pkgs.gsettings-desktop-schemas}/share:${pkgs.hicolor-icon-theme}/share:$XDG_DATA_DIRS"
          
          # Set the helper path for development
          export OMENIX_HELPER_PATH="${omenix-fancontrol}"
          
          # Prefer system pkexec over Nix store version (system pkexec is setuid root)
          SYSTEM_PKEXEC=""
          for path in /run/wrappers/bin/pkexec /usr/bin/pkexec /bin/pkexec /usr/local/bin/pkexec; do
            if [ -f "$path" ] && [ -u "$path" ]; then
              SYSTEM_PKEXEC="$path"
              break
            fi
          done
          
          if [ -n "$SYSTEM_PKEXEC" ]; then
            export PKEXEC_PATH="$SYSTEM_PKEXEC"
            echo "✓ Found setuid pkexec at: $SYSTEM_PKEXEC"
          else
            export PKEXEC_PATH="pkexec"
            echo "⚠️  Warning: No setuid pkexec found. Fan control may not work."
            echo "   Available pkexec: $(which pkexec)"
            echo "   Permissions: $(ls -la $(which pkexec) 2>/dev/null)"
            echo ""
            echo "   To fix this issue:"
            echo "   1. Install polkit on your system (outside of Nix)"
            echo "   2. Ensure pkexec is setuid root"
            echo "   3. Or use NixOS with: services.omenix.enable = true;"
          fi
          
          echo ""
          echo "Development environment loaded!"
          echo "Helper script: $OMENIX_HELPER_PATH"
          echo "Using pkexec: $PKEXEC_PATH"
          echo ""
          echo "Commands:"
          echo "  cargo build    - Build the project"
          echo "  nix build      - Build the full package"
          echo ""
          echo "Requirements for fan control:"
          echo "  1. System polkit with setuid pkexec"
          echo "  2. User in 'wheel' group: $(groups | grep -o wheel || echo 'NOT FOUND')"
          echo "  3. HP WMI interface: $(ls /sys/devices/platform/hp-wmi/hwmon/ 2>/dev/null || echo 'NOT FOUND')"
        '';
      };
    };
}

