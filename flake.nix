{
  description = "Omenix Fan Control for HP Omen laptops";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";

  outputs =
    { nixpkgs, ... }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };

      libs = nixpkgs.lib.makeLibraryPath (
        with pkgs;
        [
          libayatana-appindicator
          libappindicator-gtk3
          gtk3
        ]
      );

      version = "0.1.0";
      src = ./.;
      cargoLock.lockFile = ./Cargo.lock;

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
        wrapProgram $out/bin/* \
         --set OMENIX_ASSETS_DIR "$out/share/omenix/assets" \
         --prefix LD_LIBRARY_PATH : "${libs}"
      '';

      meta = with nixpkgs.lib; {
        description = "Fan control application for HP Omen laptops";
        homepage = "https://github.com/noahpro99/omenix";
        license = licenses.mit;
        platforms = platforms.linux;
      };

      omenix = pkgs.rustPlatform.buildRustPackage {
        pname = "omenix";
        inherit
          version
          src
          cargoLock
          nativeBuildInputs
          buildInputs
          postInstall
          ;
        cargoBuildFlags = [
          "--bin"
          "omenix"
        ];
        meta = meta // {
          mainProgram = "omenix";
        };
      };

      omenix-daemon = pkgs.rustPlatform.buildRustPackage {
        pname = "omenix-daemon";
        inherit
          version
          src
          cargoLock
          nativeBuildInputs
          buildInputs
          postInstall
          ;
        cargoBuildFlags = [
          "--bin"
          "omenix-daemon"
        ];
        meta = meta // {
          mainProgram = "omenix-daemon";
        };
      };
    in
    {
      packages.${system} = { inherit omenix omenix-daemon; };

      apps.${system} = {
        omenix = {
          type = "app";
          program = "${omenix}/bin/omenix";
          inherit (omenix) meta;
        };

        omenix-daemon = {
          type = "app";
          program = "${omenix-daemon}/bin/omenix-daemon";
          inherit (omenix-daemon) meta;
        };
      };

      nixosModules.default =
        { config, lib, ... }:
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

      devShells.${system}.default = pkgs.mkShell {
        inherit buildInputs;
        nativeBuildInputs = with pkgs; [
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
          export LD_LIBRARY_PATH="${libs}:$LD_LIBRARY_PATH"
          export OMENIX_ASSETS_DIR="$(pwd)/assets"
          echo "Development environment loaded!"
          echo "Assets directory: $OMENIX_ASSETS_DIR"
        '';

      };
      formatter.x86_64-linux = pkgs.nixfmt-tree;
    };
}
