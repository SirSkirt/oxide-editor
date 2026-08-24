# Oxide Editor B1.2.2-U

## B1.2.2 editor indentation patch

- Added automatic indentation in the Oxide code editor for normal projects and tutorial projects.
- Enter inherits the current line indentation.
- Pressing Enter after an opening `{` adds one four-space indentation level.
- Pressing Enter between `{` and `}` creates an indented blank line while keeping the closing brace aligned.
- Typing `}` on an otherwise blank indented line automatically outdents one level.
- Manual Tab indentation continues to use four spaces.

## B1.2.1 tutorial readability patch

- Enlarged the tutorial explanation text and syntax breakdown text.
- Increased the tutorial panel width on desktop so explanations no longer read like footnotes beside the code example.
- Increased the size of objective, feedback, Learn More, and tutorial action text.
- Kept example code prominent without letting it visually overpower the explanation.
- No lesson logic or objective behavior changed in this patch.

Oxide is a Rust-first desktop editor built with Tauri 2. The frontend is vanilla HTML/CSS/JavaScript; filesystem, Cargo, compiler diagnostics, project creation, dependency editing, and interactive process I/O are handled by Rust.

## Core highlights

- First-run / no-project onboarding with **New Project**, **Open Project**, and **Tutorial**.
- Oxide-native filesystem browser; no Windows file-picker dependency for project workflows.
- New-project wizard:
  - choose or create a destination folder;
  - choose a Cargo package name;
  - version defaults to `0.0.1`;
  - creates a normal Cargo project using Rust edition 2024;
  - opens directly into `src/main.rs` containing the standard Hello World example.
- Multi-file editor tabs with independent dirty state, cursor position, selection, and scroll position.
- Cargo Check / Build / Run / Test / Clean.
- Cargo.toml package/dependency inspector and dependency add/remove controls.
- **Live Rust Check** using `cargo check --message-format=json`:
  - errors and warnings are parsed by the Rust backend;
  - gutter markers identify affected lines;
  - an in-editor diagnostic banner shows the current file's first problem and compiler hint;
  - the Problems pane shows compiler labels, error codes, and Rust-provided help/note messages;
  - clicking a problem opens the source file and jumps to its location.
- Run-mode chooser:
  - **Run in Oxide Terminal** for command-line applications;
  - **Run as GUI / Native Window** for applications that create their own UI.
- Interactive Oxide Run Terminal:
  - Oxide builds with Cargo in Build Bay, then launches the produced executable directly;
  - the terminal shows the program's stdout/stderr instead of `cargo run` compilation chatter;
  - streams stdout/stderr without waiting for newline boundaries;
  - accepts stdin from the editor UI;
  - Stop control and Ctrl+C support;
  - after a program exits, shows `Press any key to exit...`.
- Friendly and raw Cargo build output remain available.

## Run Oxide

Requirements:

- Node.js / npm
- Rust toolchain with Cargo
- Tauri 2 system prerequisites for your OS

From the project folder:

```powershell
npm install
npm run tauri dev
```

Release build:

```powershell
npm run tauri build
```

## Versioning

- Package/build version: `1.2.3`
- User-facing version: **B1.2.2-U**

## Notes on live checking

B1.0.0 deliberately uses the real Rust compiler through Cargo rather than implementing a second parser in JavaScript. Live Rust Check debounces edits, saves dirty open buffers, then asks Cargo/rustc for structured JSON diagnostics. It can be disabled from **Tools → Live Rust Check**.

The current editor surface is still a lightweight textarea-based implementation. The diagnostic gutter and Problems system are designed so a later code-editor engine or rust-analyzer integration can replace the text surface without moving compiler/project logic out of Rust.

## B1.0.0 UI hotfix

The startup layout now uses an explicitly single-column application grid. The onboarding screen fully replaces the editor workspace until a project is opened, preventing the hidden workspace from being auto-placed into an implicit second CSS Grid column.


## B1.2.2 tutorial teaching standard

- Tutorial lessons now follow a consistent **Example → Explain → You Try** flow.
- New syntax should be demonstrated with a very small code example before the learner is asked to use it.
- Each new syntax element gets a short, one-sentence explanation in the main lesson flow.
- The former **Why?** control is now **Learn More**. It contains the longer conceptual/technical explanation for learners who want the deeper reasoning.
- Existing activities follow the short teaching standard, with deeper detail behind **Learn More**.
- A completed activity now waits for the learner to press **Next Step**, so the explanation remains on screen for re-reading.
- Completed lessons expose a direct **Next Lesson** action, and active lessons retain **Tutorial Home** and checkpoint restore controls.
- Beginner now includes: Hello Rust, Variables, Warnings vs Errors, Mutability, Basic Data Types, and Functions.
- The Warnings vs Errors lesson intentionally demonstrates that warnings still build while errors block compilation.

The intended pacing is: **see it, understand the minimum needed, use it immediately**. Longer explanations are optional rather than blocking the activity.

---

## B1.2.2-U — updater infrastructure

B1.2.2-U adds the release/update pipeline for the public repository:

`https://github.com/SirSkirt/oxide-editor`

### Included

- automatic update check shortly after Oxide launches
- Oxide-styled update prompt with release notes and download progress
- `Help -> Check for Updates...` manual check
- Tauri updater + process plugins
- signed updater artifacts for release builds (`src-tauri/tauri.release.conf.json` enables `createUpdaterArtifacts`)
- GitHub Releases endpoint: `releases/latest/download/latest.json`
- NSIS-first Windows release/update path
- passive Windows updater installation
- GitHub Actions release workflow that builds, signs and publishes the installer, signatures and `latest.json`
- GitHub Actions verification workflow for normal pushes and pull requests
- one-time updater signing-key setup helper
- release version synchronization helper

### Version note

The visible build is **B1.2.2-U**. Its internal SemVer is **1.2.3** so Windows and Tauri can treat it as newer than the existing 1.2.2 installation. Future updater releases must continue increasing the internal numeric SemVer.

## One-time updater setup

Tauri requires updater signatures. The private signing key must never be committed.

From PowerShell in the project root:

```powershell
.\scripts\setup-updater.ps1
```

The helper:

1. generates `~\.tauri\oxide-editor.key` if one does not already exist;
2. leaves the private key outside the repository;
3. copies only the public key into `src-tauri/tauri.conf.json`;
4. prints the GitHub Actions secret names you need to configure.

In the GitHub repository, add:

- `TAURI_SIGNING_PRIVATE_KEY` — complete contents of the private `.key` file
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` — only if you gave the key a password

Back up the private key. Existing Oxide installations trust the corresponding public key, so losing the private key means those installations cannot verify future automatic updates signed with a replacement key.

## Publishing through GitHub Actions

The repository contains:

- `.github/workflows/build.yml` — verifies frontend + Rust builds on `main` and pull requests
- `.github/workflows/release.yml` — builds the Windows NSIS release and creates a GitHub Release

The release workflow can be started manually from GitHub Actions, or by pushing a numeric version tag such as:

```powershell
git tag v1.2.3
git push origin v1.2.3
```

`tauri-action` uploads the NSIS installer, updater signature and `latest.json`. Installed updater-enabled copies of Oxide read that `latest.json` automatically.

Normal local `npm run tauri build` builds do **not** require the updater private key. The GitHub release workflow merges `src-tauri/tauri.release.conf.json` only for signed release builds.

For later versions, the included helper keeps the three internal version declarations synchronized:

```powershell
npm run release:version -- 1.2.4 B1.2.3
```

Then commit, tag and push.

## Windows installer/update behavior

B1.2.2-U intentionally builds the **NSIS** installer rather than both NSIS and MSI. Tauri's updater reuses the NSIS setup executable as the update package and invokes the installer's update mode; Oxide uses the updater's `passive` Windows install mode so the update is applied without the old manual uninstall/reinstall workflow.

A manually downloaded NSIS installer can still be used for clean installations or repair/reinstall scenarios. The in-app updater is the normal upgrade path once B1.2.2-U is installed.

### Updater signing vs Windows code signing

Tauri updater signing verifies that an update came from Oxide and was not modified. It is separate from Windows Authenticode/code signing. Until a Windows code-signing certificate is configured, Windows may still show an Unknown Publisher/SmartScreen warning on a freshly downloaded installer.
