# Mulu App

[English](./README.md) | [简体中文](./README.zh-CN.md)

[![Build Status](https://github.com/yangkx1024/mulu/actions/workflows/ci.yml/badge.svg)](https://github.com/yangkx1024/mulu/actions/workflows/ci.yml)

A cross-platform MTP client built with [gpui](https://gpui.rs), [gpui-component](https://gpui-kit.com) and [mtp-rs](https://github.com/vdavid/mtp-rs). Simple, easy to use, and extremely lightweight.

<img alt="Icon" src="./screenshots/app_screenshot.webp" />

# Install

## macOS

Download the latest signed and notarized `Mulu_*.dmg` (Apple Silicon) from the [Releases page](https://github.com/yangkx1024/mulu/releases/latest), open it, and drag `Mulu.app` into `/Applications`.

## Debian / Ubuntu (apt)

An apt repository is published at https://yangkx1024.github.io/mulu/ with `Release` signed by a dedicated GPG key.

```sh
sudo install -d -m 0755 /etc/apt/keyrings
curl -fsSL https://yangkx1024.github.io/mulu/pubkey.asc | sudo gpg --dearmor -o /etc/apt/keyrings/mulu.gpg
echo "deb [signed-by=/etc/apt/keyrings/mulu.gpg] https://yangkx1024.github.io/mulu stable main" | sudo tee /etc/apt/sources.list.d/mulu.list
sudo apt update
sudo apt install mulu
```

`apt upgrade` will pull future releases automatically.

## Arch Linux (pacman)

Each release ships a `PKGBUILD` alongside a `mulu_*.tar.gz` source tarball. Download both from the [Releases page](https://github.com/yangkx1024/mulu/releases/latest) into the same directory, then build and install with `makepkg`:

```sh
makepkg -si
```

## Nix / NixOS

The repository is a flake for `x86_64-linux` and `aarch64-linux`:

```sh
# Run without installing
nix run github:yangkx1024/mulu

# Install into your user profile
nix profile install github:yangkx1024/mulu
```

On NixOS, add `github:yangkx1024/mulu` as a flake input and install `inputs.mulu.packages.${pkgs.stdenv.hostPlatform.system}.default`, or apply `inputs.mulu.overlays.default` and use `pkgs.mulu`. Updates come through Nix, so the in-app update check is disabled in this build.

Mulu needs permission to open the phone's USB device. If the phone is not detected or fails to open, add libmtp's udev rules, which grant access to the logged-in user:

```nix
services.udev.packages = [ pkgs.libmtp ];
```

If Mulu reports the device as busy, another MTP client (such as gvfs or KDE's kio-mtp) holds the session; unmount the phone in your file manager first.

## Verify Linux releases

Linux packages (`.deb`, `.tar.gz`) are also signed with [minisign](https://jedisct1.github.io/minisign/) for file-level verification. The public key lives at [`minisign.pub`](./minisign.pub).

The `.sig` files attached to releases are base64-wrapped; decode them first, then verify with [`rsign2`](https://crates.io/crates/rsign2):

```sh
base64 -d mulu_X.Y.Z_amd64.deb.sig > mulu_X.Y.Z_amd64.deb.minisig
rsign verify mulu_X.Y.Z_amd64.deb -p minisign.pub -x mulu_X.Y.Z_amd64.deb.minisig
```

# Build

1. Run `cargo run --release` directly. Or
2. `cargo packager --release` (requires [`cargo-packager`](https://crates.io/crates/cargo-packager) to be installed) — produces an app bundle under `build/release/`.
3. With Nix: `nix develop` opens a shell with the toolchain and system libraries; `nix build` builds the package into `./result`.

# License

MIT OR Apache-2.0.
