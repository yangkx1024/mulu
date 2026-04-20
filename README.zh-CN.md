# Mulu（目录）App

[English](./README.md) | [简体中文](./README.zh-CN.md)

[![Build Status](https://github.com/yangkx1024/mulu/actions/workflows/ci.yml/badge.svg)](https://github.com/yangkx1024/mulu/actions/workflows/ci.yml)

基于 [gpui](https://gpui.rs) ， [gpui-component](https://longbridge.github.io/gpui-component) 和 [mtp-rs](https://github.com/vdavid/mtp-rs) 构建的跨平台MTP客户端。简单易用且极致轻量。

<img alt="Icon" src="./screenshots/app_screenshot.webp" />

# 本地编译

1. 直接`cargo run --release`运行二进制。或者
2. `cargo packager --release` （需要先安装 [`cargo-packager`](https://crates.io/crates/cargo-packager)），会在build/release目录下生成app文件。

# License

MIT 或者 Apache-2.0.
