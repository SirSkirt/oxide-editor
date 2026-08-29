# Rivet B1.3.6 · Build 9

**Rivet — Rust Development Environment**

Build 9 is the first Android application milestone. It deliberately ships the **editor before the Android Rust toolchain backend** so the mobile shell, file/project workflow, themes, layouts and Android packaging can be tested independently from PRoot/rustc/Cargo work.

## Android editor preview

The Android build can:

- launch Rivet as a native Tauri Android application;
- use Mobile Layout or Desktop Layout from **Tools → Settings**;
- use all built-in/custom presentation themes;
- create a normal Rust/Cargo-shaped project without invoking Cargo (Rivet writes `Cargo.toml` and `src/main.rs` itself);
- open projects stored in Rivet's private Android workspace;
- browse project files, open multiple tabs, edit and save text files;
- edit Cargo.toml metadata/dependencies through Rivet's existing manifest tools;
- use lexical Semantic Readability highlighting even without rust-analyzer.

The Android build intentionally cannot yet:

- run Cargo or rustc;
- run live Cargo diagnostics;
- use rust-analyzer semantic intelligence/completion;
- build/run/test/clean Rust programs;
- use LLDB debugging;
- run the interactive tutorial, which depends on a real Rust toolchain;
- browse arbitrary shared-device storage through Android's Storage Access Framework.

Those controls remain visible where appropriate but are disabled with a clear **Android backend pending** explanation. This keeps the real desktop workflow recognizable while avoiding fake compiler behavior.

## Mobile code separation

Mobile work is no longer mixed into the main desktop files. Android/mobile-specific code lives under:

```text
src/mobile/
  mobile.css
  layout.js
  android-preview.js

src-tauri/src/mobile/
  mod.rs

src-tauri/tauri.android.conf.json
src-tauri/capabilities/mobile.json
```

`src/main.js` only talks to the small mobile/layout interfaces it needs. Desktop updater support remains desktop-only.

## Android workspace

Until Android Storage Access Framework support is implemented, Rivet creates/opens projects inside its app-private **RIVET WORKSPACE**. This lets Build 9 prove reliable editing and saving without mixing storage-permission work into the toolchain milestone.

## Android packaging

GitHub Actions now verifies an ARM64 Android APK in addition to Windows/Linux. The GitHub release workflow also publishes an **Android preview APK**. Build 9 uses the Android preview application id `com.rivet.rde.preview`; production Android signing/package identity will be established before Rivet Android is treated as a stable distribution target.

The Android `versionCode` is build-specific so internal B1.3.6 builds can advance independently from the public SemVer string.

## Existing desktop features retained

Build 9 preserves the Windows/Linux debugger, updater, theme system, Semantic Readability Colors, resizable Build Bay, Settings, and both workspace layouts. Desktop behavior is not intentionally changed by the Android preview work.
