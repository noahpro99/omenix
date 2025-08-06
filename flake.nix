{
  description = "Omenix Fan Control for HP Omen laptops";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";
  };

  outputs = { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};

      omenix = pkgs.rustPlatform.buildRustPackage {
        pname = "omenix";
        version = "0.1.0";
        src = ./.;
        cargoLock = {
          lockFile = ./Cargo.lock;
        };

        nativeBuildInputs = with pkgs; [
          pkg-config
          makeWrapper
        ];

        buildInputs = with pkgs; [
          gtk3
          libappindicator-gtk3
          libayatana-appindicator
          openssl
          xdotool
        ];

        postInstall = ''
          mkdir -p $out/share/omenix/assets
          cp -r assets/* $out/share/omenix/assets/
          
          # Wrap binaries with environment variable and runtime library paths
          wrapProgram $out/bin/omenix \
            --set OMENIX_ASSETS_DIR "$out/share/omenix/assets" \
            --prefix LD_LIBRARY_PATH : "${pkgs.lib.makeLibraryPath [
              pkgs.libayatana-appindicator
              pkgs.libappindicator-gtk3
              pkgs.gtk3
            ]}"
          
          wrapProgram $out/bin/omenix-daemon \
            --set OMENIX_ASSETS_DIR "$out/share/omenix/assets" \
            --prefix LD_LIBRARY_PATH : "${pkgs.lib.makeLibraryPath [
              pkgs.libayatana-appindicator
              pkgs.libappindicator-gtk3
              pkgs.gtk3
            ]}"
        '';

        meta = with pkgs.lib; {
          description = "Fan control application for HP Omen laptops";
          homepage = "https://github.com/noahpro99/omenix";
          license = licenses.mit;
          platforms = platforms.linux;
          mainProgram = "omenix";
        };
      };
    in
    {
      packages.${system} = {
        default = omenix;
        omenix = omenix;
      };

      apps.${system} = {
        default = {
          type = "app";
          program = "${omenix}/bin/omenix";
          meta = {
            description = "Omenix Fan Control GUI";
            mainProgram = "omenix";
          };
        };

        omenix-daemon = {
          type = "app";
          program = "${omenix}/bin/omenix-daemon";
          meta = {
            description = "Omenix Fan Control Daemon";
            mainProgram = "omenix-daemon";
          };
        };
      };

      nixosModules.default = { config, lib, pkgs, ... }:
        with lib;
        let
          cfg = config.services.omenix;
        in
        {
          options.services.omenix = {
            enable = mkEnableOption "Omenix fan control daemon";

            package = mkOption {
              type = types.package;
              default = omenix;
              description = "The omenix package to use.";
            };
          };

          config = mkIf cfg.enable {
            systemd.services.omenix-daemon = {
              description = "Omenix Fan Control Daemon";
              wantedBy = [ "multi-user.target" ];
              after = [ "multi-user.target" ];

              serviceConfig = {
                Type = "simple";
                ExecStart = "${cfg.package}/bin/omenix-daemon";
                Restart = "on-failure";
                RestartSec = 5;
                User = "root";
              };
            };

            environment.systemPackages = [ cfg.package ];
          };
        };

      devShells.${system}.default = pkgs.mkShell
        {
          buildInputs = with pkgs; [
            # Runtime libraries
            gtk3
            xdotool
            libappindicator-gtk3
            libayatana-appindicator
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
            
            # Set assets directory for development
            export OMENIX_ASSETS_DIR="$(pwd)/assets"
            
            echo "Development environment loaded!"
            echo "Assets directory: $OMENIX_ASSETS_DIR"
          '';

        };
    };
}

