# Mulu App

[English](./README.md) | [简体中文](./README.zh-CN.md)

[![Build Status](https://github.com/yangkx1024/mulu/actions/workflows/ci.yml/badge.svg)](https://github.com/yangkx1024/mulu/actions/workflows/ci.yml)

A cross-platform MTP client built with [gpui](https://gpui.rs), [gpui-component](https://longbridge.github.io/gpui-component) and [mtp-rs](https://github.com/vdavid/mtp-rs). Simple, easy to use, and extremely lightweight.

<img alt="Icon" src="./screenshots/app_screenshot.webp" />

# Build

1. Run `cargo run --release` directly. Or
2. `cargo packager --release` (requires [`cargo-packager`](https://crates.io/crates/cargo-packager) to be installed) — produces an app bundle under `build/release/`.

# Install on Debian/Ubuntu via apt

An apt repository is published at https://yangkx1024.github.io/mulu/ with `Release` signed by a dedicated GPG key.

```sh
sudo install -d -m 0755 /etc/apt/keyrings
curl -fsSL https://yangkx1024.github.io/mulu/pubkey.asc | sudo gpg --dearmor -o /etc/apt/keyrings/mulu.gpg
echo "deb [signed-by=/etc/apt/keyrings/mulu.gpg] https://yangkx1024.github.io/mulu stable main" | sudo tee /etc/apt/sources.list.d/mulu.list
sudo apt update
sudo apt install mulu
```

`apt upgrade` will pull future releases automatically.

# Verify Linux releases

Linux packages (`.deb`, `.tar.gz`) are also signed with [minisign](https://jedisct1.github.io/minisign/) for file-level verification. The public key lives at [`minisign.pub`](./minisign.pub).

The `.sig` files attached to releases are base64-wrapped; decode them first, then verify with [`rsign2`](https://crates.io/crates/rsign2):

```sh
base64 -d mulu_X.Y.Z_amd64.deb.sig > mulu_X.Y.Z_amd64.deb.minisig
rsign verify mulu_X.Y.Z_amd64.deb -p minisign.pub -x mulu_X.Y.Z_amd64.deb.minisig
```

# License

MIT OR Apache-2.0.
