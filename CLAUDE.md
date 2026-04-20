# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

Mulu is a macOS/Linux desktop app that browses files on MTP devices (Android phones, cameras). It is a single-binary Rust app built on GPUI (Zed's UI toolkit) and `mtp-rs`.

## Build / run

- `cargo run` — debug build and launch the app.
- `cargo build --release` — release binary at `target/release/mulu`.
- `cargo check` / `cargo clippy` — typecheck / lints. There is no test suite.
- `./package-mac.sh` — produces a signed+notarized `.app` / `.dmg` via `cargo-packager`. Requires `.env` with `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID`, `APPLE_SIGNING_IDENTITY`, and the signing identity present in the keychain. The script rewrites `Cargo.toml` temporarily to inject `signing-identity` into `[package.metadata.packager.macos]` because `cargo-packager -c` replaces rather than merges.

## Required sibling checkout

`Cargo.toml` contains `[patch.crates-io] nusb = { path = "../nusb" }`. A sibling `../nusb` checkout **must exist** for the build to resolve — this patch pins `core-foundation = 0.10.0` to match GPUI's pin. `gpui` and `gpui-component` are also pulled from git (Zed and Longbridge's forks), so the first build is slow and network-dependent.

## Architecture

The app is one window hosting the `MtpBrowser` root view (`src/mtp_browser/mod.rs`). Everything else is a subsystem of that view.

**UI layer — `src/mtp_browser/`** (GPUI, synchronous, runs on the main thread):
- `mod.rs` owns `MtpBrowser` state (devices list, current `Session`, selected row, table state, status line) and routes `TableEvent`s (row select / double-click to descend).
- `sidebar.rs`, `toolbar.rs`, `table.rs`, `status_bar.rs` render their sections.
- `actions.rs` holds all async operations (open device, list folder, import, export, delete, new folder) and GPUI `Action`s for context menus. This is where most behavior lives.

**MTP layer — `src/mtp/`** (async, runs on a dedicated tokio runtime):
- `runtime.rs` — lazy static 2-worker tokio runtime (`tokio_rt()`) + **`spawn_mtp(cx, fut, done)`** — the critical bridge. It runs `fut` on tokio and calls `done(&mut view, result, cx)` back on the GPUI thread. All MTP I/O MUST go through this helper; never block the GPUI thread on MTP calls.
- `client.rs` — `MtpClient` wraps `mtp_rs::MtpDevice` with a single active `StorageId`. Contains an Android quirk workaround: some devices reject `parent=None` at storage root and require `ObjectHandle::ALL` (0xFFFFFFFF) — detected lazily on `InvalidObjectHandle`, cached in `root_uses_all_handle: Arc<AtomicBool>`, and retried once. `MtpOpError` maps `is_exclusive_access()` to a dedicated `Busy` variant so the UI can surface "device busy" (another app holds the MTP session).
- `hotplug.rs` — spawns `nusb::watch_devices()` on tokio, forwards events through `mpsc::channel(1)` with `try_send` (drops bursts), then trailing-edge debounces 300ms on the GPUI side before calling `list_devices`. The channel capacity of 1 is load-bearing — coalescing happens at the consumer.
- `types.rs` — `FileEntry`, `StorageSummary`, `DeviceSummary` (plain data for the UI).

**Model — `src/model.rs`:** `Session` holds the open `MtpClient`, the device's storages, and a breadcrumb `Vec<Crumb>` where the first element is the storage root (`handle: None`) and each subsequent `Crumb` stores the folder's own object handle (used as `parent` when listing children). `current_parent()`, `push_folder()`, `pop()`, `truncate_to()`, and `reset_to_storage()` are the only mutators.

**Formatting — `src/format.rs`:** size / datetime / file-kind strings, all i18n-aware.

## Concurrency model

- Two execution domains: GPUI's main-thread executor and a shared 2-worker tokio runtime.
- `spawn_mtp` is the **only** way to cross from GPUI into MTP work and back. Don't call `tokio::spawn` directly or block on `.await` in view code.
- `MtpClient` is `Clone` (internally `Arc`-based) — clone it into `spawn_mtp` closures rather than borrowing from the view.
- Folder navigation, import, export, delete, and mkdir all follow the same pattern: set a loading status, `spawn_mtp`, then in the callback either `load_current_folder` or `set_status` on error.

## i18n

- `rust-i18n` with YAML at `locales/app.yml`. Initialized in `main.rs` via `rust_i18n::i18n!("locales", fallback = "en")`.
- Supported locales: `en`, `zh-CN`, `zh-HK`, `ja`. `detect_system_locale()` in `main.rs` picks one from `sys_locale`; the toolbar exposes a manual switch.
- When adding UI strings, always add all four translations in `locales/app.yml` and use `t!("key")` or `t!("key", arg = val)`. Keys with `.` are nested paths. The format-string helpers in `src/format.rs` also go through `t!`.

## Conventions

- Rust edition **2024**. `gpui::*` and `gpui_component::*` are wildcard-imported throughout the UI code — match that style.
- View event handlers take `&mut self, event, &mut Window, &mut Context<Self>`. Use `cx.notify()` after mutating state that affects rendering.
- Errors bubble up as `MtpOpError`; call `.user_message()` for display (already translated). Don't `unwrap()` on MTP results — surface them to the status bar.
- Android root-parent quirk: when adding new write operations that take a `parent: Option<ObjectHandle>`, route through `self.root_parent(parent)` and the same retry-on-`InvalidObjectHandle` pattern as `create_folder` / `upload_file`.
