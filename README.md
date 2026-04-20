# Mulu App

[English](./README.md) | [简体中文](./README.zh-CN.md)

[![Build Status](https://github.com/yangkx1024/mulu/actions/workflows/ci.yml/badge.svg)](https://github.com/yangkx1024/mulu/actions/workflows/ci.yml)

A cross-platform MTP client built with [gpui](https://gpui.rs), [gpui-component](https://longbridge.github.io/gpui-component) and [mtp-rs](https://github.com/vdavid/mtp-rs). Simple, easy to use, and extremely lightweight.

<img alt="Icon" src="./screenshots/app_screenshot.webp" />

# Build

1. Run `cargo run --release` directly. Or
2. `cargo packager --release` (requires [`cargo-packager`](https://crates.io/crates/cargo-packager) to be installed) — produces an app bundle under `build/release/`.

# License

MIT OR Apache-2.0.
