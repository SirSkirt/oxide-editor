# Oxide Editor B1.3.2

Oxide is a Rust-first desktop editor built with Tauri 2. The frontend is vanilla HTML/CSS/JavaScript; filesystem, Cargo, compiler diagnostics, project creation, dependency editing, tutorial evaluation, process I/O, and update orchestration are handled by Rust.

## B1.3.2 — Oxide Package Update System

B1.3.2 begins Oxide's own update path instead of using the Windows installer as the normal update mechanism.

### New update flow

1. Oxide checks `oxide-latest.json` on the latest GitHub Release.
2. Tauri's updater transport downloads the announced ZIP and verifies its signature with Oxide's existing embedded public key.
3. Oxide stages the verified ZIP under the user's temporary directory.
4. Oxide copies and launches the installed `oxide-updater.exe` helper from that temporary directory.
5. The editor exits so its installed executable can be replaced.
6. **Oxide Update Service** opens in an Oxide-styled window, extracts the package to a staging directory, creates rollback copies, replaces the runtime files, and restarts Oxide.
7. If file replacement fails, the helper restores the previous runtime files before reporting the failure.

The package format is intentionally simple in B1.3.2:

```text
oxide-update-win-x64-1.3.2.zip
├── oxide-editor.exe
├── oxide-updater.exe
└── update-package.json
```

The full runtime is shipped instead of differential patches. This is larger than a delta update but substantially simpler to verify, recover, and maintain.

### GitHub Release assets

The release workflow now publishes both the normal installer and the package updater assets:

```text
Oxide Editor B1.3.2
├── Oxide Editor ... setup.exe        # first install / repair / legacy updater bridge
├── ...setup.exe.sig
├── oxide-update-win-x64-1.3.2.zip    # B1.3.2+ package updater
├── oxide-update-win-x64-1.3.2.zip.sig
├── latest.json                       # compatibility feed for B1.3.1 and older
└── oxide-latest.json                 # native Oxide package feed
```

### Migration from the old updater

B1.3.1 and older updater-enabled versions are already configured to read `latest.json`. That feed remains available and points to the signed NSIS installer, so those builds can still update to B1.3.2 using the old mechanism.

B1.3.2 switches to `oxide-latest.json`. Future B1.3.2+ releases can therefore use the Oxide ZIP updater without breaking older installed versions that have not crossed the B1.3.2 bridge yet.

The signing keypair does **not** change. The same private key stored in GitHub Actions signs both compatibility installer artifacts and Oxide update packages; the public key remains embedded in Oxide. Tauri's updater download API verifies the package signature before returning the bytes to Oxide's update engine.

### Updater helper build

The updater is a second small Tauri/Rust executable under `updater/`. It has its own compact Oxide-styled UI and is bundled with the main installer as a Tauri sidecar.

`npm run tauri dev` and `npm run tauri build` invoke:

```text
npm run prepare:updater
```

which builds the helper and stages the correctly target-suffixed sidecar in `src-tauri/binaries/` before Tauri bundles the main editor.

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
- Windows release builds use the GUI subsystem, so no background Command Prompt appears

## Run Oxide

Requirements:

- Node.js / npm
- current Rust toolchain with Cargo
- Tauri 2 Windows prerequisites

```powershell
npm install
npm run tauri dev
```

Release build:

```powershell
npm run tauri build
```

The first build may take longer because Oxide also compiles the Update Service sidecar.

## Versioning

- Package/build version: `1.3.2`
- User-facing version: **B1.3.2**

To move both the editor and updater helper to a future version:

```powershell
npm run release:version -- 1.3.3 B1.3.3
```

## Release

See [UPDATER_SETUP.md](UPDATER_SETUP.md) for signing-key and GitHub Actions details. The normal release workflow builds the installer, builds the native update package, signs both paths, creates both update feeds, and uploads the assets to GitHub Releases.
