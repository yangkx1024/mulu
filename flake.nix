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

      devShells = forAllSystems (pkgs: {
        default = pkgs.mkShell {
          inputsFrom = [ self.packages.${pkgs.stdenv.hostPlatform.system}.mulu ];

          packages = with pkgs; [
            clippy
            rustfmt
            rust-analyzer
          ];

          RUST_SRC_PATH = pkgs.rustPlatform.rustLibSrc;

          # `cargo run` binaries lack the rpath that package.nix patches in.
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (
            with pkgs;
            [
              libGL
              vulkan-loader
              wayland
            ]
          );
        };
      });

      overlays.default = final: _prev: {
        mulu = final.callPackage ./nix/package.nix { };
      };
    };
}
