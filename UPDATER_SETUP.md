# Oxide Package Updater Setup

B1.3.5 continues the platform-specific Oxide package feeds introduced by B1.3.2 **The Compatibility Update**, while retaining the old Windows installer feed as a migration bridge.

## Signing key

The existing Tauri/minisign keypair remains the root of trust on both Windows and Linux.

- Public key: embedded in `src-tauri/tauri.conf.json`; safe to commit.
- Private key: **never commit it**.
- GitHub repository secret: `TAURI_SIGNING_PRIVATE_KEY`
- Optional password secret: `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`

If the keypair has not been created yet, run from PowerShell:

```powershell
Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass
.\scripts\setup-updater.ps1
```

The setup script writes JSON as UTF-8 **without a BOM**.

## Update feeds

### Windows legacy bridge

```text
latest.json
```

B1.3.1 and earlier updater-enabled Windows builds already know this feed. It remains pointed at the signed NSIS installer so those versions can cross into B1.3.2.

### B1.3.2+ platform feeds

The B1.3.2 application uses:

```text
https://github.com/SirSkirt/oxide-editor/releases/latest/download/oxide-latest-{{target}}-{{arch}}.json
```

The first targets are:

```text
oxide-latest-windows-x86_64.json
oxide-latest-linux-x86_64.json
```

Keep `latest.json` available for older Windows installs even after B1.3.2 is released.

### Build-aware release identity

Oxide does not treat the public SemVer string as the whole release identity. Platform feeds publish both:

```text
release_version: 1.3.5
build: 3
```

The editor compares release version first, then build number when the public version is equal. Therefore `1.3.5 Build 4` is newer than `1.3.5 Build 3`. Build 3+ requests updater feeds with no-cache headers and publishes each internal build as a distinct GitHub Release tag so GitHub's `releases/latest/download/...` route cannot remain pinned to an older same-version release.

## Package verification

The main application uses the Rust `tauri-plugin-updater` API for release discovery and downloading. The package signature is verified against Oxide's embedded public key before the downloaded bytes are handed to Oxide's own installer logic.

## Windows package updates

Windows packages contain:

```text
oxide-editor.exe
oxide-updater.exe
update-package.json
```

The styled Windows Update Service is copied to a temporary directory, backs up the currently installed runtime, replaces the files, rolls back on failure, and restarts Oxide.

## Linux package updates

Linux release packages contain both supported installation payloads:

```text
oxide-editor.AppImage
oxide-editor.deb
update-package.json
```

For AppImage installs, Oxide copies its small Linux update helper out of the running AppImage, downloads and verifies the signed ZIP, and then exits. The helper validates the package, waits for the old process, keeps a rollback copy, replaces the AppImage, restores executable permissions, and relaunches Oxide.

### `.deb` installations

Debian/Ubuntu/Pop!_OS installs use the operating system's normal privilege boundary. Oxide downloads and verifies the signed package as the user, exits, then the Linux helper invokes `pkexec` so the desktop's Polkit agent can authorize `dpkg --install`. Oxide never displays a password box and never receives the user's password.

## GitHub Actions

Repository secrets required:

```text
TAURI_SIGNING_PRIVATE_KEY
TAURI_SIGNING_PRIVATE_KEY_PASSWORD   # only if the key has a password
```

Then use:

**Repository → Actions → Build and publish Oxide → Run workflow**

or push the build-specific release tag produced by the release workflow convention:

```powershell
git tag v1.3.5-b3
git push origin v1.3.5-b3
```

The public Oxide version is still `1.3.5`; the `-b4` tag (and the same `-bN` pattern for every build) makes each internal build a distinct GitHub Release so `releases/latest/download/...` advances for Build 1 → Build 2 → Build 3 → Build 4 updates. The platform feed remains authoritative and carries both `release_version` and `build`.

The pipeline creates the build-specific release as a **draft**, runs the Windows and Linux jobs, uploads and validates both platform feeds, and only then publishes the release. If either platform job fails, the incomplete draft does not replace the previous working `releases/latest` target.

### Windows job

```text
build Windows Update Service
→ build Oxide NSIS installer
→ build/sign Windows Oxide ZIP
→ generate oxide-latest-windows-x86_64.json
→ generate legacy latest.json
→ upload assets
```

### Linux job

Built on **Ubuntu 22.04**:

```text
install Tauri Linux build dependencies
→ build Linux update helper
→ build .deb + AppImage
→ package/sign the AppImage update ZIP
→ generate oxide-latest-linux-x86_64.json
→ upload assets
```

## First B1.3.2 release

- Existing B1.3.1 Windows installs reach B1.3.2 through `latest.json` and NSIS.
- Fresh Linux users download either the B1.3.2 `.deb` or AppImage from GitHub Releases.
- B1.3.2 AppImage installs use the Linux Oxide package feed for subsequent automatic updates.
