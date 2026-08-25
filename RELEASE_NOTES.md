# Oxide Editor B1.3.4 · Build 2

B1.3.4 continues **The Compatibility Update** with a proper automatic updater for Linux `.deb` installations and responsive fixes for shorter laptop displays.

## Linux automatic `.deb` updates

- Oxide now detects when the running Linux release came from a Debian package.
- The signed Linux update ZIP contains both the AppImage and `.deb` payloads.
- Oxide downloads and verifies the signed package before requesting any elevated access.
- `.deb` installs use Linux's normal **Polkit/pkexec** authorization flow; Oxide never asks for or receives the user's password itself.
- After authorization, the update helper installs the staged package with `dpkg --install` and reopens Oxide.
- AppImage updates continue to replace the user-owned AppImage directly without elevation.
- Unpackaged development builds do not attempt system-level self-installation.

## Smaller-screen layout repair

- The editor/workspace can no longer expand over the Build Bay when lesson content is tall.
- Project, Cargo, and Tutorial panels now obey the available workspace height.
- The Tutorial body scrolls vertically on shorter screens.
- Tutorial navigation/actions remain reachable with a sticky action area.
- Build Bay height scales down on short laptop displays before sacrificing editor space.
- Additional width scaling keeps Tutorial Mode usable on narrower laptop windows.

## Rust syntax highlighting

- Added Oxide-native Rust syntax highlighting in the real editor, including tutorial projects.
- Rust keywords and macros use Oxide rust/orange, strings use green, declared variables use copper-red, types use steel-blue, numbers use amber, and comments use a muted gray.
- Highlighting is layered behind the existing textarea so editing, selections, diagnostics, indentation, and Rust Code Analyzer/Completer behavior stay on the same editor path.

## Retained from B1.3.3

- Rust Code Analyzer/Completer powered by rust-analyzer.
- Node.js 24 CI/tooling baseline.
- Windows updater warning cleanup.
- Windows and Linux signed package update feeds.
- 26-lesson hands-on Rust tutorial.

- Fixed compact-screen editor layout where the inline Rust diagnostic banner could expand over most of the editor and push the code area toward the Build Bay.
- Rust Code Analyzer/Completer popups now clamp themselves to the available editor area and switch to a compact suggestions-only layout when space is tight.
- GitHub Actions maintained JavaScript actions now use Node.js 24-native major versions (`checkout@v7`, `setup-node@v7`, `upload-artifact@v7`).

## Internal build-number updates

- Added an internal build number separate from the public Oxide version.
- B1.3.4 Build 2 can be detected as newer than B1.3.4 Build 1.
- Update feeds now publish both `release_version` and `build`.
- Signed update package filenames include the build number.
- The update dialog and About/status UI show build identity where useful.
- Added `npm run release:build` to increment a rebuild without changing the public version.
