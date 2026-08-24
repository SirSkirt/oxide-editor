import fs from 'node:fs';

const [internalVersion, displayVersion] = process.argv.slice(2);
if (!/^\d+\.\d+\.\d+$/.test(internalVersion || '')) {
  console.error('Usage: npm run release:version -- 1.3.2 B1.3.2');
  process.exit(1);
}
if (!displayVersion) {
  console.error('A user-facing display version is required, for example B1.3.2.');
  process.exit(1);
}

const packagePath = 'package.json';
const packageJson = JSON.parse(fs.readFileSync(packagePath, 'utf8'));
packageJson.version = internalVersion;
packageJson.displayVersion = displayVersion;
fs.writeFileSync(packagePath, `${JSON.stringify(packageJson, null, 2)}\n`);

for (const configPath of ['src-tauri/tauri.conf.json', 'updater/src-tauri/tauri.conf.json']) {
  const config = JSON.parse(fs.readFileSync(configPath, 'utf8'));
  config.version = internalVersion;
  fs.writeFileSync(configPath, `${JSON.stringify(config, null, 2)}\n`);
}

for (const cargoPath of ['src-tauri/Cargo.toml', 'updater/src-tauri/Cargo.toml']) {
  let cargo = fs.readFileSync(cargoPath, 'utf8');
  cargo = cargo.replace(/(^\[package\][\s\S]*?^version\s*=\s*")[^"]+("$)/m, `$1${internalVersion}$2`);
  fs.writeFileSync(cargoPath, cargo);
}

const mainPath = 'src/main.js';
let main = fs.readFileSync(mainPath, 'utf8');
main = main.replace(/OXIDE EDITOR · B[^<`'\"]+/g, `OXIDE EDITOR · ${displayVersion}`);
main = main.replace(/OXIDE B[^<`'\"]+/g, `OXIDE ${displayVersion}`);
main = main.replace(/Beta B[^<`'\"]+/g, `Beta ${displayVersion}`);
main = main.replace(/CURRENT B\d+\.\d+\.\d+/g, `CURRENT ${displayVersion}`);
fs.writeFileSync(mainPath, main);

console.log(`Oxide release version updated: internal ${internalVersion}, display ${displayVersion}`);
