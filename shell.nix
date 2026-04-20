{pkgs ? import <nixpkgs> {}}:
pkgs.mkShell {
  buildInputs = with pkgs; [
    rustc
    cargo
    rustfmt
    clippy
    # Add system dependencies here (e.g., openssl, pkg-config)
    pkg-config
    openssl
    wayland
    vulkan-loader
    mesa
    fontconfig
    udev
    libxkbcommon
    libxcb

    alsa-lib
    expat
    zlib
  ];

  # Optional: Set environment variables
  RUST_SRC_PATH = pkgs.rustPlatform.rustLibSrc;

  LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [
    pkgs.openssl
    pkgs.fontconfig
    pkgs.udev
    pkgs.wayland
    pkgs.libxkbcommon
    pkgs.vulkan-loader
    pkgs.mesa

    pkgs.alsa-lib
    pkgs.expat
    pkgs.zlib
  ];

  shellHook = ''
    export PATH=$PATH:$HOME/.cargo/bin
  '';
}
