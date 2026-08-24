# Oxide Editor B1.3.2 — The Compatibility Update

Oxide is a Rust-first desktop editor built with Tauri 2. The frontend is vanilla HTML/CSS/JavaScript; filesystem access, Cargo, compiler diagnostics, project creation, dependency editing, tutorial evaluation, process I/O, and update orchestration are handled by Rust.

B1.3.2 is internally dubbed **The Compatibility Update**. It starts Oxide's native package-update system and adds the first Linux desktop target, with **Pop!_OS / Ubuntu / Debian-family x86_64** as the initial compatibility baseline.

## Windows + Linux

Supported release targets in B1.3.2:

- **Windows x86_64** — NSIS installer plus Oxide's signed native ZIP update packages
- **Linux x86_64 AppImage** — portable build with Oxide automatic package updates
- **Linux x86_64 .deb** — native Debian/Ubuntu/Pop!_OS package

The Linux CI build runs on Ubuntu 22.04 so the produced binaries target a reasonably old WebKitGTK/glibc baseline instead of accidentally requiring the newest Ubuntu release.

### Linux Rust toolchain discovery

Linux desktop launchers do not necessarily inherit PATH additions from `.bashrc`, `.profile`, or other interactive shell configuration. Oxide now resolves Rust tools from PATH **and** the standard rustup location:

```text
~/.cargo/bin/cargo
~/.cargo/bin/rustc
```

This prevents a desktop-launched Oxide from reporting that Cargo is missing when it works normally in a terminal.

### Linux path handling

Oxide now treats Linux paths as case-sensitive. Files such as `Thing.rs` and `thing.rs` are no longer normalized as though they were the same file.

## Oxide Package Update System

B1.3.2 uses platform-specific signed update packages published on GitHub Releases.

Oxide checks:

```text
oxide-latest-{{target}}-{{arch}}.json
```

which resolves to feeds such as:

```text
oxide-latest-windows-x86_64.json
oxide-latest-linux-x86_64.json
```

The downloaded package is still cryptographically verified with Oxide's existing Tauri updater signing key before Oxide hands it to its own updater logic.

### Windows update package

```text
oxide-update-windows-x86_64-1.3.2.zip
├── oxide-editor.exe
├── oxide-updater.exe
└── update-package.json
```

The temporary Oxide Update Service backs up the installed runtime, replaces it, rolls back on failure, and restarts Oxide.

### Linux AppImage update package

```text
oxide-update-linux-x86_64-1.3.2.zip
├── oxide-editor.AppImage
└── update-package.json
```

For AppImage installs, Oxide downloads and verifies the package, launches a small Linux update helper, exits, replaces the original AppImage with rollback protection, restores executable permissions, and relaunches Oxide.

### Linux .deb updates

The `.deb` build is supported for normal installation on Pop!_OS/Ubuntu, but B1.3.2 does **not** silently overwrite root-owned package-manager files. If Oxide is running from a `.deb` installation, it can still detect new releases but tells the user to install the newer `.deb` package.

Use the AppImage build if you want Oxide's automatic self-update path on Linux.

### Compatibility bridge

`latest.json` is still published for B1.3.1 and older updater-enabled **Windows** builds. It points to the signed NSIS installer so older installations can cross the bridge into B1.3.2. B1.3.2 and newer use the platform-specific Oxide package feeds above.

## GitHub Actions

Every normal build now verifies both platforms:

```text
Windows x64 verification
Linux x64 verification (Ubuntu 22.04 / Pop!_OS baseline)
```

A release produces both Windows and Linux assets in the same GitHub Release.

Typical B1.3.2 assets:

```text
Windows
├── Oxide Editor ... setup.exe
├── oxide-update-windows-x86_64-1.3.2.zip
├── oxide-update-windows-x86_64-1.3.2.zip.sig
├── oxide-latest-windows-x86_64.json
└── latest.json

Linux
├── Oxide Editor ... amd64.deb
├── Oxide Editor ... amd64.AppImage
├── oxide-update-linux-x86_64-1.3.2.zip
├── oxide-update-linux-x86_64-1.3.2.zip.sig
└── oxide-latest-linux-x86_64.json
```

## Linux development on Pop!_OS / Ubuntu

Run the included helper once:

```bash
./scripts/setup-linux.sh
```

Then:

```bash
npm install
npm run tauri dev
```

Build local Linux bundles:

```bash
npm run tauri build -- --bundles deb,appimage
```

## Windows development

Requirements:

- Node.js / npm
- current Rust toolchain with Cargo
- Tauri 2 Windows prerequisites

```powershell
npm install
npm run tauri dev
```

Release-style Windows bundle:

```powershell
npm run tauri build -- --bundles nsis
```

Windows release builds use the GUI subsystem, so Oxide does not leave a background Command Prompt open.

## Interactive Rust Tutorial

The Beginner course contains **26 real Cargo lessons / 81 hands-on activities**, covering:

- Hello World, variables, warnings/errors, mutability, and basic data types
- functions, parameters, return values, conditions, and loops
- Strings, vectors, structs, enums, and `match`
- ownership, borrowing, string slices, and mutable references
- methods, `Option`, `Result`, `HashMap`, and modules
- real interactive stdin through Oxide's Run Terminal
- combined mini-projects including Calculator and Scoreboard

Tutorial teaching follows:

**small example → one short explanation for each new syntax item → Now You Try → real compiler/run verification**

Longer explanations remain optional behind **Learn More**. Challenge steps accept multiple valid solutions when the required concept and observable result are demonstrated, and output matching is case-insensitive unless capitalization itself is the objective.

## Core editor highlights

- no-project onboarding with **New Project**, **Open Project**, and **Tutorial**
- Oxide-native project/file browser
- normal Cargo project creation with version `0.0.1` by default
- multi-file tabs with independent dirty/cursor/scroll state
- automatic brace-aware indentation
- Cargo Check / Build / Run / Test / Clean
- Cargo.toml package/dependency GUI
- live rustc diagnostics using `cargo check --message-format=json`
- Friendly and Raw Cargo views
- Problems pane and clickable compiler diagnostics
- floating interactive Run Terminal with real stdin/stdout
- GUI/native-window run mode
- signed Oxide-native update packages

## Versioning

- Package/build version: `1.3.2`
- User-facing version: **B1.3.2**
- Internal update name: **The Compatibility Update**

To move all editor/updater components to a future version:

```powershell
npm run release:version -- 1.3.3 B1.3.3
```

The version helper updates the main editor, Windows Update Service, and Linux Update Service together.

## Release

See [UPDATER_SETUP.md](UPDATER_SETUP.md) for signing-key and GitHub Actions details.

### Updater dialog reliability

B1.3.2 fixes an updater UI wiring bug inherited from B1.3.1: the **Later** and **Download & Update** controls are now application-level controls that are bound once at startup, independent of editor/project state. Unexpected frontend updater failures are also surfaced in the dialog instead of appearing as dead buttons.
