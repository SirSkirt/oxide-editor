# Oxide Editor B1.3.5 · Build 4

B1.3.5 Build 4 is the **Rust Intelligence & Editing build**. It expands Oxide's existing persistent rust-analyzer connection into traditional IDE navigation/refactoring features while keeping the Rust-heavy architecture and normal Cargo project model.

Build 4 adds **Go to Definition (F12), Find References (Shift+F12), Semantic Rename (F2), and Code Actions / Quick Fixes (Ctrl+.)**. It also repairs the Rust Code Analyzer/Completer UX: the popup stays below the line being typed, Escape dismisses it for the remainder of the current word, and completion is allowed again when the next word starts. Auto-close pairs and bracket matching add a smaller typing-quality pass.

The resizable Build Bay, Build 3 workbench-polish preview, Semantic Readability Colors, LLDB/DAP debugger, and Windows/Linux `(release_version, build)` updater behavior remain intact.
Oxide is a Rust-first desktop IDE built with Tauri 2. The frontend is vanilla HTML/CSS/JavaScript; filesystem access, Cargo, compiler diagnostics, project creation, dependency editing, tutorial evaluation, process I/O, language intelligence, debugging, and update orchestration are handled by Rust.

B1.3.5 Build 3 is a **workbench-polish and reliability build**. It keeps Oxide's existing forged-metal layout and workflow, but tightens the visual hierarchy of panels, tabs, command controls, project rows, Cargo/dependency cards, the editor chrome, and the Build Bay. It does not replace the current UI with a new layout.

The **Build Bay is now vertically resizable** from its forged top-edge grip. Its height is clamped so the output remains usable without swallowing the editor, is remembered between sessions, and can be reset by double-clicking the grip, pressing Home while it is focused, or using View → Reset Layout.

Build 3 also hardens **Windows and Linux automatic updates around Oxide's real identity: `(release_version, build)`**. A machine on B1.3.5 Build 1 can therefore see B1.3.5 Build 2/3 even though the public SemVer is still `1.3.5`. Update requests bypass stale cache reuse, release feeds are validated before publication, every build gets its own GitHub Release tag such as `v1.3.5-b3`, and the release stays draft until both Windows and Linux assets are ready.

The B1.3.5 Build 2 LLDB/DAP debugger expansion and **Semantic Readability Colors** remain intact. Semantic colors use rust-analyzer tokens when available with the lexical highlighter as fallback; ordinary variables/identifiers stay steel blue, and red remains reserved for actual errors/problems.

## Windows + Linux

Supported release targets in B1.3.5:

- **Windows x86_64** — NSIS installer plus Oxide's signed native ZIP update packages
- **Linux x86_64 AppImage** — portable build with Oxide automatic package updates
- **Linux x86_64 .deb** — native Debian/Ubuntu/Pop!_OS package with Polkit-authorized automatic updates

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

B1.3.5 continues to use the platform-specific signed update packages introduced in B1.3.2.

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
oxide-update-windows-x86_64-1.3.5-b3.zip
├── oxide-editor.exe
├── oxide-updater.exe
└── update-package.json
```

The temporary Oxide Update Service backs up the installed runtime, replaces it, rolls back on failure, and restarts Oxide.

### Linux update package

```text
oxide-update-linux-x86_64-1.3.5-b3.zip
├── oxide-editor.AppImage
├── oxide-editor.deb
└── update-package.json
```

For AppImage installs, Oxide downloads and verifies the package, launches the Linux update helper, exits, replaces the original AppImage with rollback protection, restores executable permissions, and relaunches Oxide.

For `.deb` installs, Oxide first verifies the same signed package as the normal user. After Oxide exits, the Linux helper invokes `pkexec` to ask the desktop's registered Polkit authentication agent for permission, then runs `dpkg --install` on the staged `.deb`. Oxide never receives or stores the user's password. After the package manager succeeds, the helper relaunches Oxide.

Unpackaged development builds deliberately do not self-install system packages.

### Compatibility bridge

`latest.json` is still published for B1.3.1 and older updater-enabled **Windows** builds. It points to the signed NSIS installer so older installations can cross the bridge into B1.3.2. B1.3.2 and newer use the platform-specific Oxide package feeds above.

## Compact-screen / laptop layout

B1.3.4 treats the window height as a hard layout constraint. The workspace uses a zero-minimum grid row, the Build Bay owns its own non-overlapping row, and the Project/Cargo/Tutorial panels all use independently scrollable bodies. The Tutorial action controls are sticky at the bottom of the lesson scroller so **Next Step**, **Complete Lesson**, and navigation controls remain reachable on shorter displays.

The Build Bay also scales down on short windows instead of pushing the editor outside the application frame.

## GitHub Actions

Every normal build now verifies both platforms:

```text
Windows x64 verification
Linux x64 verification (Ubuntu 22.04 / Pop!_OS baseline)
```

A release produces both Windows and Linux assets in the same GitHub Release.

Typical B1.3.5 assets:

```text
Windows
├── Oxide Editor ... setup.exe
├── oxide-update-windows-x86_64-1.3.5-b3.zip
├── oxide-update-windows-x86_64-1.3.5-b3.zip.sig
├── oxide-latest-windows-x86_64.json
└── latest.json

Linux
├── Oxide Editor ... amd64.deb
├── Oxide Editor ... amd64.AppImage
├── oxide-update-linux-x86_64-1.3.5-b3.zip
├── oxide-update-linux-x86_64-1.3.5-b3.zip.sig
└── oxide-latest-linux-x86_64.json
```

## Linux development on Pop!_OS / Ubuntu

Run the included helper once (it also installs LLDB for the B1.3.5 debugger and the rust-analyzer rustup component when rustup is available):

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

- Node.js 24+ / npm
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

## Rust Code Analyzer/Completer

Oxide includes its own Visual Studio-style Rust completion experience, backed by the real **rust-analyzer** language server.

As you type Rust, Oxide requests context-aware suggestions from rust-analyzer and filters/ranks them against the current prefix. The popup can include local variables, functions, methods, fields, modules, structs, enums, traits, associated items, macros, and Rust keywords. A detail pane shows the symbol kind, signature/detail text, and documentation when rust-analyzer provides it.

Controls:

```text
Type normally     automatic completion
Ctrl+Space        request completion manually
Up / Down         select a suggestion
Enter / Tab       accept
Escape            dismiss
```

Function-call signature help is also requested while entering arguments. Oxide applies rust-analyzer's replacement edits and additional import edits when they are supplied.

Install/verify the analyzer with:

```text
rustup component add rust-analyzer
rust-analyzer --version
```

Oxide starts one persistent rust-analyzer session per active Cargo project instead of spawning a new process for every keystroke. The same session now supplies semantic tokens for **Semantic Readability Colors**, while Oxide keeps its lexical Rust highlighter as a fallback during analyzer startup. The status rail reports **ANALYZER: READY**, **NOT FOUND**, or **ERROR**.

### Semantic Readability Colors

```text
Rust keywords          #D87941  rust orange
Variables/identifiers  #83A6B8  steel blue
Strings                #8FAF72  sage green
Numbers/booleans       #D3A95F  amber/gold
Types                  #C4A45F  brass/gold
Macros                 #E99A62  bright copper/orange
Functions/methods      #DDD0BF  warm cream
Comments               #70786E  muted gray-green
Operators/punctuation  neutral light gray/off-white
```

Red is not used as an ordinary source-code category; Oxide keeps it reserved for actual errors/problems.

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
- **Rust Code Analyzer/Completer** powered by rust-analyzer/LSP
- **Oxide Debugger** powered by LLDB/DAP with target/thread selection, conditional/log breakpoints, stepping/restart, expandable stack/locals, watches, and Debug Console
- **Semantic Readability Colors** backed by rust-analyzer semantic tokens with a lexical fallback
- context-aware completion popup with details and signature help
- Cargo Check / Build / Run / Test / Clean
- Cargo.toml package/dependency GUI
- live rustc diagnostics using `cargo check --message-format=json`
- Friendly and Raw Cargo views
- Problems pane and clickable compiler diagnostics
- floating interactive Run Terminal with real stdin/stdout
- GUI/native-window run mode
- signed Oxide-native update packages

## Versioning

- Release version: `1.3.5`
- User-facing version: **B1.3.5**
- Current internal build number: **3**
- Full installed identity: **B1.3.5 · Build 4**
- B1.3.2 foundation codename: **The Compatibility Update**

To move all editor/updater components to a future version:

```powershell
npm run release:version -- 1.3.5 B1.3.5 1
```

The version helper updates the main editor, Windows Update Service, and Linux Update Service together.


### Build numbers

Oxide keeps the public release version and the internal build number separate. This lets a rebuilt release update an existing installation without forcing a public version bump.

For example:

```text
B1.3.5 · Build 2
        ↓
B1.3.5 · Build 4
```

The signed update feed carries `release_version` plus `build`, and Oxide compares both. GitHub update packages include the build in their filename. Build 3 also gives every build its own release tag (`v1.3.5-b3`, `v1.3.5-b4`, and so on), so GitHub's `releases/latest/download/...` route advances even when the public Oxide version stays the same. For compatibility with older updater-enabled Oxide builds, the feed's required SemVer `version` is a monotonic updater sequence while `release_version` remains the actual Oxide release version.

To increment only the build number:

```powershell
npm run release:build
```

To begin a new public version, reset the build to 1:

```powershell
npm run release:version -- 1.3.5 B1.3.5 1
```

## Release

See [UPDATER_SETUP.md](UPDATER_SETUP.md) for signing-key and GitHub Actions details.

### Updater dialog reliability

B1.3.2 fixes an updater UI wiring bug inherited from B1.3.1: the **Later** and **Download & Update** controls are now application-level controls that are bound once at startup, independent of editor/project state. Unexpected frontend updater failures are also surfaced in the dialog instead of appearing as dead buttons.


### B1.3.4 compact-screen/editor fixes

- Inline diagnostics remain a single compact row instead of consuming the editor workspace.
- Rust Code Analyzer/Completer UI is constrained to the editor viewport on smaller laptop displays.
- GitHub-maintained JavaScript actions in CI/release workflows use Node.js 24-native current majors.
