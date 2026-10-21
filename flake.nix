{
  description = "A systemd GUI to manage service, timer, socket and other units";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs =
    { self, nixpkgs }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
    in
    {
      packages = forAllSystems (pkgs: rec {
        sysd-manager = pkgs.callPackage ./packaging/nix/package.nix { };
        default = sysd-manager;
      });

      # Lets `nix flake check` (run in CI) build the package and its tests.
      checks = forAllSystems (pkgs: {
        inherit (self.packages.${pkgs.stdenv.hostPlatform.system}) sysd-manager;
      });
    };
}
