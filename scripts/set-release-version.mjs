import fs from 'node:fs';

const [internalVersion, displayVersion, requestedBuild] = process.argv.slice(2);
if (!internalVersion) {
  console.error('Usage: npm run release:version -- 1.3.6 B1.3.6 [buildNumber]');
  process.exit(1);
}
if (!displayVersion) {
  console.error('A user-facing display version is required, for example B1.3.6.');
  process.exit(1);
}

const buildNumber = requestedBuild ? Number(requestedBuild) : 1;
if (!Number.isInteger(buildNumber) || buildNumber < 1) {
  console.error('Build number must be a positive integer.');
  process.exit(1);
}

const packagePath = 'package.json';
const packageJson = JSON.parse(fs.readFileSync(packagePath, 'utf8'));
packageJson.version = internalVersion;
packageJson.displayVersion = displayVersion;
packageJson.buildNumber = buildNumber;
fs.writeFileSync(packagePath, `${JSON.stringify(packageJson, null, 2)}\n`);

for (const configPath of ['src-tauri/tauri.conf.json', 'updater/src-tauri/tauri.conf.json']) {
  const config = JSON.parse(fs.readFileSync(configPath, 'utf8'));
  config.version = internalVersion;
  fs.writeFileSync(configPath, `${JSON.stringify(config, null, 2)}\n`);
}

for (const cargoPath of ['src-tauri/Cargo.toml', 'updater/src-tauri/Cargo.toml', 'linux-updater/Cargo.toml']) {
  let cargo = fs.readFileSync(cargoPath, 'utf8');
  cargo = cargo.replace(/^version\s*=\s*"[^"]+"/m, `version = "${internalVersion}"`);
  fs.writeFileSync(cargoPath, cargo);
}


const mainPath = 'src/main.js';
let main = fs.readFileSync(mainPath, 'utf8');
main = main.replace(/(?:OXIDE EDITOR|RIVET) · B\d+\.\d+\.\d+(?: · BUILD \d+)?/g, `RIVET · ${displayVersion} · BUILD ${buildNumber}`);
main = main.replace(/(?:OXIDE|RIVET) B\d+\.\d+\.\d+(?: · BUILD \d+)?/g, `RIVET ${displayVersion} · BUILD ${buildNumber}`);
main = main.replace(/Beta B\d+\.\d+\.\d+(?: · Build \d+)?/g, `Beta ${displayVersion} · Build ${buildNumber}`);
main = main.replace(/CURRENT B\d+\.\d+\.\d+/g, `CURRENT ${displayVersion}`);
fs.writeFileSync(mainPath, main);

console.log(`Rivet release version updated: internal ${internalVersion}, display ${displayVersion}, build ${buildNumber}`);
