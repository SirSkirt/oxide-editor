import fs from 'node:fs';

const pkg = JSON.parse(fs.readFileSync('package.json', 'utf8'));
const android = JSON.parse(fs.readFileSync('src-tauri/tauri.android.conf.json', 'utf8'));
const build = Number(pkg.buildNumber || 0);
if (build !== 9) throw new Error(`Expected Build 9, got ${build}`);
if (!fs.existsSync('src/mobile/mobile.css')) throw new Error('Mobile CSS must stay in src/mobile/mobile.css');
if (!fs.existsSync('src/mobile/layout.js')) throw new Error('Mobile layout logic must stay in src/mobile/layout.js');
if (!fs.existsSync('src/mobile/android-preview.js')) throw new Error('Android preview behavior must stay in src/mobile/android-preview.js');
if (!fs.existsSync('src-tauri/src/mobile/mod.rs')) throw new Error('Android Rust hooks must stay in src-tauri/src/mobile/mod.rs');
if (android.bundle?.externalBin?.length) throw new Error('Android preview must not bundle the desktop updater sidecar');
if (!String(android.identifier || '').includes('preview')) throw new Error('Build 9 Android package must use a preview application id until production signing is established');
const main = fs.readFileSync('src/main.js', 'utf8');
if (!main.includes("./mobile/layout.js") || !main.includes("./mobile/android-preview.js")) throw new Error('main.js must use the isolated mobile modules');
const css = fs.readFileSync('src/styles.css', 'utf8');
if (css.includes(':root[data-layout="mobile"] .workspace')) throw new Error('Mobile workspace CSS leaked back into the desktop stylesheet');
console.log('Rivet Android editor-preview separation checks passed.');
