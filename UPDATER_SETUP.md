# Oxide updater setup

The code is already wired to the GitHub release feed for `SirSkirt/oxide-editor`, but the first release requires your own signing keypair.

## 1. Generate the keypair

Run in PowerShell from the Oxide project root:

```powershell
.\scripts\setup-updater.ps1
```

Do not commit the private key.

## 2. Add GitHub Actions secrets

Repository → **Settings** → **Secrets and variables** → **Actions** → **New repository secret**.

Add:

- `TAURI_SIGNING_PRIVATE_KEY`
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` if the key is password protected

The public key written into `src-tauri/tauri.conf.json` is safe to commit.

## 3. Push Oxide to the repository

The GitHub repository is expected to contain the project root (`package.json`, `src/`, `src-tauri/`, and `.github/`).


## Local builds vs release builds

Normal local builds use `src-tauri/tauri.conf.json` and do not create signed updater artifacts. The GitHub release workflow adds `src-tauri/tauri.release.conf.json`, which enables `createUpdaterArtifacts` only when publishing a signed release.

This means `npm run tauri build` still works locally without exposing the updater private key as an environment variable.

## 4. Publish

Either run **Build and publish Oxide** manually under GitHub Actions or push a SemVer tag:

```powershell
git tag v1.2.3
git push origin v1.2.3
```

The workflow creates the GitHub Release and `latest.json` automatically.

## 5. Test the updater

The first updater-enabled build must be installed manually. After that, publish a release with a higher internal SemVer. Launch the older installed build. Oxide should detect the newer release and show its own update dialog.
