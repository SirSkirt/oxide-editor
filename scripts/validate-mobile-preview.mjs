import fs from 'node:fs';

const pkg = JSON.parse(fs.readFileSync('package.json', 'utf8'));
const android = JSON.parse(fs.readFileSync('src-tauri/tauri.android.conf.json', 'utf8'));
const build = Number(pkg.buildNumber || 0);
if (build !== 10) throw new Error(`Expected Build 10, got ${build}`);
if (!fs.existsSync('src/mobile/mobile.css')) throw new Error('Mobile CSS must stay in src/mobile/mobile.css');
if (!fs.existsSync('src/mobile/layout.js')) throw new Error('Mobile layout logic must stay in src/mobile/layout.js');
if (!fs.existsSync('src/mobile/android-preview.js')) throw new Error('Android preview behavior must stay in src/mobile/android-preview.js');
if (!fs.existsSync('src-tauri/src/mobile/mod.rs')) throw new Error('Android Rust hooks must stay in src-tauri/src/mobile/mod.rs');
if (android.bundle?.externalBin?.length) throw new Error('Android preview must not bundle the desktop updater sidecar');
if (!String(android.identifier || '').includes('preview')) throw new Error('Build 10 Android package must use a preview application id until production signing is established');

const buildWorkflow = fs.readFileSync('.github/workflows/build.yml', 'utf8');
const releaseWorkflow = fs.readFileSync('.github/workflows/release.yml', 'utf8');
for (const [name, workflow] of [['build.yml', buildWorkflow], ['release.yml', releaseWorkflow]]) {
  const setupIndex = workflow.indexOf('android-actions/setup-android@v4.0.1');
  const configureIndex = workflow.indexOf('Configure Android SDK/NDK environment');
  if (setupIndex < 0 || configureIndex < 0 || configureIndex <= setupIndex) {
    throw new Error(`${name} must set up Android SDK command-line tools before configuring the SDK/NDK environment`);
  }
  if (!workflow.includes('ndk;28.2.13676358')) {
    throw new Error(`${name} must provision the pinned Android NDK r28c toolchain`);
  }
}

const main = fs.readFileSync('src/main.js', 'utf8');
if (!main.includes("./mobile/layout.js") || !main.includes("./mobile/android-preview.js")) throw new Error('main.js must use the isolated mobile modules');
const css = fs.readFileSync('src/styles.css', 'utf8');
if (css.includes(':root[data-layout="mobile"] .workspace')) throw new Error('Mobile workspace CSS leaked back into the desktop stylesheet');
console.log('Rivet Android editor-preview separation checks passed.');
