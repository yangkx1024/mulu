{
  description = "Mulu - a lightweight MTP client";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

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
      packages = forAllSystems (
        pkgs:
        let
          mulu = pkgs.callPackage ./nix/package.nix { };
        in
        {
          inherit mulu;
          default = mulu;
        }
      );

      overlays.default = final: _prev: {
        mulu = final.callPackage ./nix/package.nix { };
      };
    };
}
