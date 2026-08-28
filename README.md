# Rivet B1.3.6 · Build 7


**Rivet** is the product name; **Rust Development Environment** is its tagline. The default presentation theme remains named **Oxide**.
B1.3.6 Build 7 is a **material-texture pass** for Rivet's industrial themes. Metallic now behaves as a forged **Iron** material rather than a gray palette swap, with visible directional brushing, subtle cross-scratches, mottled metal variation, cold edge highlights, recessed seams, and raised iron controls. Rust is now explicitly **Rusty Iron**: it inherits the same forged depth model and adds irregular oxidation, patina, worn corners/seams, and darker exposed-metal variation. The actual code canvas remains intentionally quiet so material texture never competes with Semantic Readability.

Build 4 introduced the **composable presentation system**. Layout and functionality remain unchanged; themes are recipes assembled from independent material, UI palette, control-treatment, and Semantic Readability components.

The five built-ins still ship as **Oxide**, **Metallic**, **Rust**, **Modern (Dark)**, and **Modern (Light)**, but they now use the same recipe model as user-created themes. **View → Theme → Theme Workshop…** can create, edit, save, delete, and apply custom themes by mixing the available presentation components. Custom themes persist locally and can be selected directly from the View → Theme menu.

Material treatment is no longer treated as a color swap. **Metallic** is the forged **Iron** presentation: directional brushing, cross-scratches, mottled metal, edge highlights, recessed depth, and raised iron controls. **Rust** is **Rusty Iron**: the same manufactured geometry aged with irregular oxidation, patina, worn seams/corners, and exposed darker iron. Build 7 makes these material cues visibly stronger across Rivet chrome and panels while keeping the source-code surface low-noise. **Modern** material intentionally removes most industrial texture.

**Semantic Readability Colors remain independently selectable.** rust-analyzer still supplies semantic meaning; the active theme recipe supplies the readability palette for variables, functions, types, macros, strings, numbers, comments, keywords, punctuation, and now a neutral/fallback source role. Theme Workshop includes a live Rust readability preview and warns when a semantic preset is designed for the opposite editor-surface tone.

Rivet is a Rust-first desktop IDE built with Tauri 2. The frontend is vanilla HTML/CSS/JavaScript; filesystem access, Cargo, compiler diagnostics, project creation, dependency editing, tutorial evaluation, process I/O, language intelligence, debugging, and update orchestration are handled by Rust.

B1.3.5 Build 3 is a **workbench-polish and reliability build**. It keeps Rivet's existing forged-metal layout and workflow, but tightens the visual hierarchy of panels, tabs, command controls, project rows, Cargo/dependency cards, the editor chrome, and the Build Bay. It does not replace the current UI with a new layout.

The **Build Bay is now vertically resizable** from its forged top-edge grip. Its height is clamped so the output remains usable without swallowing the editor, is remembered between sessions, and can be reset by double-clicking the grip, pressing Home while it is focused, or using View → Reset Layout.

Build 3 also hardens **Windows and Linux automatic updates around Rivet's real identity: `(release_version, build)`**. A machine on B1.3.5 Build 1 can therefore see B1.3.5 Build 2/3 even though the public SemVer is still `1.3.5`. Update requests bypass stale cache reuse, release feeds are validated before publication, every build gets its own GitHub Release tag such as `v1.3.5-b3`, and the release stays draft until both Windows and Linux assets are ready.

The B1.3.5 Build 2 LLDB/DAP debugger expansion and **Semantic Readability Colors** remain intact. Semantic colors use rust-analyzer tokens when available with the lexical highlighter as fallback; ordinary variables/identifiers stay steel blue, and red remains reserved for actual errors/problems.

## Themes

All B1.3.6 themes use the **same DOM, grid, controls, commands, panel locations, and workflows**. A theme is not a workspace preset. Build 4 separates presentation into four independently composable parts:

1. **Material** — surface texture, physical depth, panel treatment, seams, and patina.
2. **Color Palette** — UI surface, text, border, accent, editor, and status colors.
3. **Control Treatment** — button/tab edge treatment, bevel/pressed behavior, and modern-vs-industrial control styling.
4. **Semantic Readability** — theme-aware code colors for Rust semantic categories.

Built-in recipes:

- **Oxide** — Oxide Iron + Oxide palette + Oxide Industrial controls + Oxide Readability
- **Metallic** — Forged Iron + Metallic palette + Forged controls + Metallic Readability
- **Rust** — Rusty Iron + Rust palette + Weathered controls + Rust Readability
- **Modern (Dark)** — Modern Flat + Modern Dark palette + Modern controls + Modern Dark Readability
- **Modern (Light)** — Modern Flat + Modern Light palette + Modern controls + Modern Light Readability

### Theme Workshop / custom themes

Use **View → Theme → Theme Workshop…** to create a custom recipe. A custom theme can mix components independently—for example Forged Iron material with the Rust UI palette, Modern controls, and Oxide Semantic Readability Colors. The Workshop can preview the unsaved recipe, and closing/canceling restores the active theme. Saved custom themes appear in the Theme menu and persist between sessions.

Custom themes are stored in a versioned JSON schema under `oxide.appearance.customThemes`; the active theme ID remains in `oxide.appearance.theme`. The schema already reserves separate palette/semantic override maps so later builds can add granular user-defined colors or import/export without replacing the theme engine.

## Windows + Linux

Supported release targets in B1.3.6:

- **Windows x86_64** — NSIS installer plus Rivet's signed native ZIP update packages
- **Linux x86_64 AppImage** — portable build with Rivet automatic package updates
- **Linux x86_64 .deb** — native Debian/Ubuntu/Pop!_OS package with Polkit-authorized automatic updates

### Oxide → Rivet Debian package migration

B1.3.6 Build 3 adds `Provides`, `Replaces`, and `Conflicts` metadata for the former `oxide-editor` Debian package. This allows a machine with an existing Oxide `.deb` installation to install/update to the new `rivet` package even though both generations intentionally use the compatibility executable path `/usr/bin/oxide-editor`.

The Linux CI build runs on Ubuntu 22.04 so the produced binaries target a reasonably old WebKitGTK/glibc baseline instead of accidentally requiring the newest Ubuntu release.

### Linux Rust toolchain discovery

Linux desktop launchers do not necessarily inherit PATH additions from `.bashrc`, `.profile`, or other interactive shell configuration. Rivet now resolves Rust tools from PATH **and** the standard rustup location:

```text
~/.cargo/bin/cargo
~/.cargo/bin/rustc
```

This prevents a desktop-launched Rivet from reporting that Cargo is missing when it works normally in a terminal.

### Linux path handling

Rivet now treats Linux paths as case-sensitive. Files such as `Thing.rs` and `thing.rs` are no longer normalized as though they were the same file.

## Rivet Package Update System

B1.3.6 continues to use the platform-specific signed update packages introduced in B1.3.2.

Rivet checks:

```text
oxide-latest-{{target}}-{{arch}}.json
```

which resolves to feeds such as:

```text
oxide-latest-windows-x86_64.json
oxide-latest-linux-x86_64.json
```

The downloaded package is still cryptographically verified with Rivet's existing Tauri updater signing key before Rivet hands it to its own updater logic.

### Windows update package

```text
oxide-update-windows-x86_64-1.3.6-b1.zip
├── oxide-editor.exe
├── oxide-updater.exe
└── update-package.json
```

The temporary Rivet Update Service backs up the installed runtime, replaces it, rolls back on failure, and restarts Rivet.

### Linux update package

```text
oxide-update-linux-x86_64-1.3.6-b1.zip
├── oxide-editor.AppImage
├── oxide-editor.deb
└── update-package.json
```

For AppImage installs, Rivet downloads and verifies the package, launches the Linux update helper, exits, replaces the original AppImage with rollback protection, restores executable permissions, and relaunches Rivet.

For `.deb` installs, Rivet first verifies the same signed package as the normal user. After Rivet exits, the Linux helper invokes `pkexec` to ask the desktop's registered Polkit authentication agent for permission, then runs `dpkg --install` on the staged `.deb`. Rivet never receives or stores the user's password. After the package manager succeeds, the helper relaunches Rivet.

Unpackaged development builds deliberately do not self-install system packages.

### Compatibility bridge

`latest.json` is still published for B1.3.1 and older updater-enabled **Windows** builds. It points to the signed NSIS installer so older installations can cross the bridge into B1.3.2. B1.3.2 and newer use the platform-specific Rivet package feeds above.

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

Typical B1.3.6 assets:

```text
Windows
├── Rivet ... setup.exe
├── oxide-update-windows-x86_64-1.3.6-b1.zip
├── oxide-update-windows-x86_64-1.3.6-b1.zip.sig
├── oxide-latest-windows-x86_64.json
└── latest.json

Linux
├── Rivet ... amd64.deb
├── Rivet ... amd64.AppImage
├── oxide-update-linux-x86_64-1.3.6-b1.zip
├── oxide-update-linux-x86_64-1.3.6-b1.zip.sig
└── oxide-latest-linux-x86_64.json
```

## Linux development on Pop!_OS / Ubuntu

Run the included helper once (it also installs LLDB for Rivet debugging and the rust-analyzer rustup component when rustup is available):

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

Windows release builds use the GUI subsystem, so Rivet does not leave a background Command Prompt open.

## Rust Code Analyzer/Completer

Rivet includes its own Visual Studio-style Rust completion experience, backed by the real **rust-analyzer** language server.

As you type Rust, Rivet requests context-aware suggestions from rust-analyzer and filters/ranks them against the current prefix. The popup can include local variables, functions, methods, fields, modules, structs, enums, traits, associated items, macros, and Rust keywords. A detail pane shows the symbol kind, signature/detail text, and documentation when rust-analyzer provides it.

Controls:

```text
Type normally     automatic completion
Ctrl+Space        request completion manually
Up / Down         select a suggestion
Enter / Tab       accept
Escape            dismiss
```

Function-call signature help is also requested while entering arguments. Rivet applies rust-analyzer's replacement edits and additional import edits when they are supplied.

Install/verify the analyzer with:

```text
rustup component add rust-analyzer
rust-analyzer --version
```

Rivet starts one persistent rust-analyzer session per active Cargo project instead of spawning a new process for every keystroke. The same session supplies semantic tokens for **Semantic Readability Colors**, while Rivet keeps its lexical Rust highlighter as a fallback during analyzer startup. In B1.3.6 the semantic category and its presentation color are separated: rust-analyzer supplies meaning and the active theme supplies the palette. The status rail reports **ANALYZER: READY**, **NOT FOUND**, or **ERROR**.

### Semantic Readability Colors

The following is the **Oxide theme** palette. Metallic, Rust, Modern (Dark), and Modern (Light) remap the same semantic roles for readability within their own materials.

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

Red is not used as an ordinary source-code category; Rivet keeps it reserved for actual errors/problems.

## Interactive Rust Tutorial

The Beginner course contains **26 real Cargo lessons / 81 hands-on activities**, covering:

- Hello World, variables, warnings/errors, mutability, and basic data types
- functions, parameters, return values, conditions, and loops
- Strings, vectors, structs, enums, and `match`
- ownership, borrowing, string slices, and mutable references
- methods, `Option`, `Result`, `HashMap`, and modules
- real interactive stdin through Rivet's Run Terminal
- combined mini-projects including Calculator and Scoreboard

Tutorial teaching follows:

**small example → one short explanation for each new syntax item → Now You Try → real compiler/run verification**

Longer explanations remain optional behind **Learn More**. Challenge steps accept multiple valid solutions when the required concept and observable result are demonstrated, and output matching is case-insensitive unless capitalization itself is the objective.

## Core editor highlights

- no-project onboarding with **New Project**, **Open Project**, and **Tutorial**
- Rivet-native project/file browser
- normal Cargo project creation with version `0.0.1` by default
- multi-file tabs with independent dirty/cursor/scroll state
- automatic brace-aware indentation
- **Rust Code Analyzer/Completer** powered by rust-analyzer/LSP
- **Rivet Debugger** powered by LLDB/DAP with target/thread selection, conditional/log breakpoints, stepping/restart, expandable stack/locals, watches, and Debug Console
- **Semantic Readability Colors** backed by rust-analyzer semantic tokens with a lexical fallback
- five built-in presentation themes plus persistent user-created recipes from **Theme Workshop**
- context-aware completion popup with details and signature help
- Cargo Check / Build / Run / Test / Clean
- Cargo.toml package/dependency GUI
- live rustc diagnostics using `cargo check --message-format=json`
- Friendly and Raw Cargo views
- Problems pane and clickable compiler diagnostics
- floating interactive Run Terminal with real stdin/stdout
- GUI/native-window run mode
- signed Rivet-native update packages

## Versioning

- Release version: `1.3.6`
- User-facing version: **B1.3.6**
- Current internal build number: **6**
- Full installed identity: **Rivet B1.3.6 · Build 7**
- B1.3.2 foundation codename: **The Compatibility Update**

To move all editor/updater components to a future version:

```powershell
npm run release:version -- 1.3.6 B1.3.6 1
```

The version helper updates the main editor, Windows Update Service, and Linux Update Service together.


### Build numbers

Rivet keeps the public release version and the internal build number separate. This lets a rebuilt release update an existing installation without forcing a public version bump.

For example:

```text
B1.3.6 · Build 5
        ↓
B1.3.6 · Build 6
```

The signed update feed carries `release_version` plus `build`, and Rivet compares both. GitHub update packages include the build in their filename. Every internal build gets its own release tag (`v1.3.6-b1`, `v1.3.6-b2`, and so on), so GitHub's `releases/latest/download/...` route advances even when the public Rivet version stays the same. For compatibility with older updater-enabled Rivet builds, the feed's required SemVer `version` is a monotonic updater sequence while `release_version` remains the actual Rivet release version.

To increment only the build number:

```powershell
npm run release:build
```

To begin a new public version, reset the build to 1:

```powershell
npm run release:version -- 1.3.6 B1.3.6 1
```

## Release

See [UPDATER_SETUP.md](UPDATER_SETUP.md) for signing-key and GitHub Actions details.

### Updater dialog reliability

B1.3.2 fixes an updater UI wiring bug inherited from B1.3.1: the **Later** and **Download & Update** controls are now application-level controls that are bound once at startup, independent of editor/project state. Unexpected frontend updater failures are also surfaced in the dialog instead of appearing as dead buttons.


### B1.3.4 compact-screen/editor fixes

- Inline diagnostics remain a single compact row instead of consuming the editor workspace.
- Rust Code Analyzer/Completer UI is constrained to the editor viewport on smaller laptop displays.
- GitHub-maintained JavaScript actions in CI/release workflows use Node.js 24-native current majors.
