{
  lib,
  rustPlatform,
  pkg-config,
  fontconfig,
  freetype,
  libxkbcommon,
  libxcb,
  wayland,
  vulkan-loader,
  libGL,
}:

rustPlatform.buildRustPackage (finalAttrs: {
  pname = "mulu";
  version = (lib.importTOML ../Cargo.toml).package.version;

  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ../Cargo.toml
      ../Cargo.lock
      ../src
      ../locales
      ../linux-icons
      ../mulu.desktop
    ];
  };

  cargoHash = "sha256-ljsimHGGmFvwZHVKifDF63WDhhSEBf827zosIX/akhw=";

  # The in-app updater pings GitHub releases; Nix users update through Nix.
  buildNoDefaultFeatures = true;

  # Upstream has no test suite; `cargo test` would only rebuild the crate graph.
  doCheck = false;

  nativeBuildInputs = [ pkg-config ];

  buildInputs = [
    fontconfig
    freetype
    libxkbcommon
    libxcb
  ];

  postInstall = ''
    install -Dm644 mulu.desktop -t $out/share/applications
    for icon in linux-icons/*.png; do
      install -Dm644 "$icon" "$out/share/icons/hicolor/$(basename "$icon" .png)/apps/mulu.png"
    done
  '';

  # GPUI dlopens the graphics and windowing libraries at runtime.
  postFixup = ''
    patchelf $out/bin/mulu --add-rpath ${
      lib.makeLibraryPath [
        libGL
        vulkan-loader
        wayland
      ]
    }
  '';

  meta = {
    description = "Lightweight MTP client for browsing files on Android phones and cameras";
    homepage = "https://github.com/yangkx1024/mulu";
    changelog = "https://github.com/yangkx1024/mulu/releases/tag/v${finalAttrs.version}";
    license = with lib.licenses; [
      mit
      asl20
    ];
    maintainers = [ ];
    mainProgram = "mulu";
    platforms = lib.platforms.linux;
  };
})
