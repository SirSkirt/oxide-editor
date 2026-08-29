# Rivet B1.3.6 · Build 10

## Build 10 CI repairs

- Fixed the Settings/layout validator so it uses stable menu markers instead of LF-only newline sequences. Windows CRLF checkouts no longer fail with `Could not inspect Tools menu`.
- Android CI now explicitly provisions the Android SDK command-line tools with `android-actions/setup-android@v4.0.1`, so `sdkmanager` is guaranteed to be available to the workflow.
- Android CI now uses NDK r28c (`28.2.13676358`) and explicitly exports `ANDROID_HOME`, `ANDROID_SDK_ROOT`, and `NDK_HOME` for Tauri. NDK r28+ also gives the preview a clean path toward Android 16 KB page-size requirements.
- Android feature scope is unchanged: this remains the editor-only preview. No Cargo/rustc/PRoot backend is introduced in Build 10.

Build 10 begins Rivet's Android port with an **editor-first preview**.

### Added
- Tauri Android packaging configuration for an ARM64 preview APK.
- Android verification and release jobs in GitHub Actions.
- Android app-private `RIVET WORKSPACE` for creating/opening projects.
- Dedicated `src/mobile/` frontend modules and stylesheet.
- Dedicated `src-tauri/src/mobile/` Android backend hooks.
- Platform-specific Tauri configuration/capabilities so desktop updater sidecars are not pulled into Android.
- Build-specific Android version code.

### Android preview behavior
- Editing, tabs, saves, project tree, Cargo.toml inspector, themes, Settings and both layouts remain usable.
- Project creation does not require Cargo; Rivet writes the standard project skeleton itself.
- Cargo/rustc, rust-analyzer, debugger, Build/Run/Test/Clean and the interactive tutorial are intentionally unavailable until the Android toolchain/runtime backend is implemented.
- Lexical Rust highlighting remains available offline.

### Architecture
Mobile-specific layout/runtime work is physically separated from desktop code to prevent the desktop files from becoming filled with Android-only branches.
