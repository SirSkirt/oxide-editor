# Rivet B1.3.6 · Build 3

B1.3.6 Build 3 is a **rename-compatibility and startup reliability fix** for the first Rivet-branded release.

## Startup / project controls

Build 2 could stop frontend initialization while applying the active theme because the theme path called a removed function name (`syncSyntaxHighlight`) instead of the current syntax renderer (`updateSyntaxHighlight`). The failure happened after top-menu handlers were attached but before the Welcome-screen project controls and automatic toolchain detection were attached.

That produced a very specific failure pattern on launch:

- Cargo and rustc remained at `unknown` / `Checking…`.
- **Tools → Refresh Toolchain** still worked because menu handlers had already been registered.
- After refresh, Cargo/rustc appeared correctly.
- **New Project**, **Open Project**, and **Tutorial** remained unresponsive because their click handlers were never reached during startup.

Build 3 corrects the theme startup call so full frontend initialization completes and automatic Cargo/rustc discovery runs normally again.

## Linux Oxide → Rivet package migration

Build 2's Rivet `.deb` used the new Debian package name `rivet`, while an existing Oxide installation was owned by package `oxide-editor`. Both packages contained `/usr/bin/oxide-editor`, so Debian correctly refused to overwrite a file owned by the other package.

Build 3 now post-processes the release `.deb` with explicit migration metadata:

- `Provides: oxide-editor`
- `Replaces: oxide-editor`
- `Conflicts: oxide-editor`

This lets Debian/Ubuntu/Pop!_OS replace the old `oxide-editor` package with the new `rivet` package during a normal Rivet update instead of requiring the user to manually uninstall Oxide first. The release workflow validates this metadata before publishing the Linux assets.

Rivet still retains legacy internal executable/update identifiers such as `/usr/bin/oxide-editor`, `com.oxide.editor`, `oxide-*` update feeds, and updater signing identity where changing them would break installed-update compatibility.

## Branding and themes

The product remains **Rivet — Rust Development Environment**. The default visual theme remains named **Oxide**. The five B1.3.6 themes and theme-aware Semantic Readability Colors are otherwise unchanged.

## Version

- Release version: `1.3.6`
- Display version: **B1.3.6**
- Build: **3**
- Full identity: **Rivet B1.3.6 · Build 3**
