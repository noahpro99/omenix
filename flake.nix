{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.11";
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
            cargo
            rustc
            rust-analyzer
            clippy
            openssl
            pkg-config
            bashInteractive
          ];
        };
    };
}

