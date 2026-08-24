# Oxide Editor B1.3.2 — The Compatibility Update

B1.3.2 expands Oxide beyond Windows and starts its cross-platform native package updater.

- Added Linux x86_64 support with Pop!_OS / Ubuntu / Debian-family systems as the first target.
- GitHub Actions now verifies and releases both Windows and Linux builds.
- Linux releases include both `.deb` and AppImage packages.
- AppImage builds use Oxide's own signed package updater and rollback-aware Linux update helper.
- `.deb` builds are supported for package-manager installation; automatic self-update currently targets the AppImage distribution so Oxide does not overwrite root-owned package files.
- Oxide now finds rustup's `~/.cargo/bin` toolchain when launched from a Linux desktop environment that does not inherit shell PATH configuration.
- Linux file paths are treated as case-sensitive.
- Update feeds are platform-specific: `oxide-latest-windows-x86_64.json` and `oxide-latest-linux-x86_64.json`.
- Windows keeps the B1.3.1 legacy `latest.json` bridge and the no-background-console release behavior.
