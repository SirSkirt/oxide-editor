# Oxide Package Updater Setup

B1.3.3 continues the platform-specific Oxide package feeds introduced by B1.3.2 **The Compatibility Update**, while retaining the old Windows installer feed as a migration bridge.

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

The Linux automatic-update path initially targets **AppImage** installations.

Linux packages contain:

```text
oxide-editor.AppImage
update-package.json
```

Oxide copies its small Linux update helper out of the running AppImage, downloads and verifies the signed ZIP, and then exits. The helper:

1. validates and extracts the package;
2. waits for the old Oxide process to finish;
3. keeps the original AppImage as a rollback copy;
4. replaces it with the new AppImage;
5. restores executable permissions;
6. relaunches Oxide;
7. restores the backup if replacement/relaunch fails.

### `.deb` installations

The `.deb` package is intended for users who prefer normal Debian/Ubuntu/Pop!_OS package installation. B1.3.2 intentionally does not modify root-owned `.deb` files behind the package manager's back.

A `.deb` installation can detect a newer release, but Oxide will ask the user to install the new `.deb` manually. The AppImage is the Linux distribution with automatic Oxide package updates in B1.3.2.

## GitHub Actions

Repository secrets required:

```text
TAURI_SIGNING_PRIVATE_KEY
TAURI_SIGNING_PRIVATE_KEY_PASSWORD   # only if the key has a password
```

Then use:

**Repository → Actions → Build and publish Oxide → Run workflow**

or push a matching SemVer tag:

```powershell
git tag v1.3.3
git push origin v1.3.3
```

The release pipeline runs separate Windows and Linux jobs after creating the shared GitHub Release.

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
