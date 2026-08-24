# Oxide Editor B1.3.3

B1.3.3 adds Oxide's first language-intelligence system and refreshes the release toolchain.

## Rust Code Analyzer/Completer

- Added **Rust Code Analyzer/Completer**, Oxide's rust-analyzer-backed completion system.
- Suggestions are requested from the real Cargo project through rust-analyzer/LSP; there is no hardcoded tutorial-only completion list.
- Completion automatically filters and ranks as you type.
- `.` and `::` contexts can surface relevant methods, fields, associated items, modules, types, variables, functions, macros, and keywords.
- The popup includes a Visual Studio-style details pane for symbol kind, signature/detail text, and rust-analyzer documentation when available.
- Arrow keys move through suggestions, Enter/Tab accepts, Escape dismisses, and Ctrl+Space manually requests completion.
- rust-analyzer text edits and additional import edits are applied when provided.
- Basic function signature/parameter help appears while entering call arguments.
- Oxide warms the analyzer when a Cargo project opens and reports analyzer state in the status rail.
- If rust-analyzer is missing, Oxide explains how to install it with `rustup component add rust-analyzer`.

## Tooling maintenance

- Removed the unused `Read` import from the Windows Oxide Update Service.
- GitHub Actions now uses **Node.js 24** for Windows and Linux verification/release jobs.
- `package.json` now declares Node 24+ as the development tooling baseline.

## Carried forward from B1.3.2

- Windows + Linux support, including Pop!_OS/Ubuntu `.deb` and AppImage builds.
- Oxide-native signed package updates with platform-specific feeds.
- B1.3.1 legacy updater bridge for Windows installs.
- 26-lesson hands-on Rust tutorial, floating interactive Run Terminal, live rustc diagnostics, Cargo GUI, and multi-file editor tabs.
