# Mulu（目录）App

[English](./README.md) | [简体中文](./README.zh-CN.md)

[![Build Status](https://github.com/yangkx1024/mulu/actions/workflows/ci.yml/badge.svg)](https://github.com/yangkx1024/mulu/actions/workflows/ci.yml)

基于 [gpui](https://gpui.rs) ， [gpui-component](https://longbridge.github.io/gpui-component) 和 [mtp-rs](https://github.com/vdavid/mtp-rs) 构建的跨平台MTP客户端。简单易用且极致轻量。

<img alt="Icon" src="./screenshots/app_screenshot.webp" />

# 本地编译

1. 直接`cargo run --release`运行二进制。或者
2. `cargo packager --release` （需要先安装 [`cargo-packager`](https://crates.io/crates/cargo-packager)），会在build/release目录下生成app文件。

# 通过 apt 安装（Debian / Ubuntu）

已在 https://yangkx1024.github.io/mulu/ 发布 apt 软件源，`Release` 文件由独立的 GPG 密钥签名。

```sh
sudo install -d -m 0755 /etc/apt/keyrings
curl -fsSL https://yangkx1024.github.io/mulu/pubkey.asc | sudo gpg --dearmor -o /etc/apt/keyrings/mulu.gpg
echo "deb [signed-by=/etc/apt/keyrings/mulu.gpg] https://yangkx1024.github.io/mulu stable main" | sudo tee /etc/apt/sources.list.d/mulu.list
sudo apt update
sudo apt install mulu
```

后续版本发布后直接 `apt upgrade` 即可获取。

# 校验 Linux 发行版

Linux 安装包（`.deb`、`.tar.gz`）同时使用 [minisign](https://jedisct1.github.io/minisign/) 对文件级签名，公钥保存在 [`minisign.pub`](./minisign.pub)。

Release 附带的 `.sig` 文件经过 base64 包装，先解码再用 [`rsign2`](https://crates.io/crates/rsign2) 校验：

```sh
base64 -d mulu_X.Y.Z_amd64.deb.sig > mulu_X.Y.Z_amd64.deb.minisig
rsign verify mulu_X.Y.Z_amd64.deb -p minisign.pub -x mulu_X.Y.Z_amd64.deb.minisig
```

# License

MIT 或者 Apache-2.0.
