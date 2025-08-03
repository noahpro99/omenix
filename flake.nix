{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";
  };

  outputs = { nixpkgs, ... }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
    in
    {
      devShells.${system}.default = pkgs.mkShell
        {
          buildInputs = with pkgs; [
            # Runtime libraries
            gtk3
            xdotool
            libappindicator-gtk3
            libayatana-appindicator

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

          nativeBuildInputs = with pkgs; [
            # Build tools

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
          '';

        };
    };
}

