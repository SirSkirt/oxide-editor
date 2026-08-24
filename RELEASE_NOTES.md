# Oxide Editor B1.3.4

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

## Retained from B1.3.3

- Rust Code Analyzer/Completer powered by rust-analyzer.
- Node.js 24 CI/tooling baseline.
- Windows updater warning cleanup.
- Windows and Linux signed package update feeds.
- 26-lesson hands-on Rust tutorial.
