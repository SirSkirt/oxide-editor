import fs from 'node:fs';

const packagePath = 'package.json';
const packageJson = JSON.parse(fs.readFileSync(packagePath, 'utf8'));
const current = Number(packageJson.buildNumber || 1);
packageJson.buildNumber = current + 1;
fs.writeFileSync(packagePath, `${JSON.stringify(packageJson, null, 2)}\n`);

const mainPath = 'src/main.js';
let main = fs.readFileSync(mainPath, 'utf8');
const display = packageJson.displayVersion;
main = main.replace(/OXIDE EDITOR · B\d+\.\d+\.\d+(?: · BUILD \d+)?/g, `OXIDE EDITOR · ${display} · BUILD ${packageJson.buildNumber}`);
main = main.replace(/OXIDE B\d+\.\d+\.\d+(?: · BUILD \d+)?/g, `OXIDE ${display} · BUILD ${packageJson.buildNumber}`);
main = main.replace(/Beta B\d+\.\d+\.\d+(?: · Build \d+)?/g, `Beta ${display} · Build ${packageJson.buildNumber}`);
main = main.replace(/CURRENT B\d+\.\d+\.\d+(?: · BUILD \$\{update\.currentBuildNumber \|\| 1\})?/g, `CURRENT ${display} · BUILD \${update.currentBuildNumber || 1}`);
fs.writeFileSync(mainPath, main);

console.log(`Oxide build number updated: ${current} -> ${packageJson.buildNumber}`);
console.log(`Updater ordering: ${packageJson.version} Build ${packageJson.buildNumber} (release version, then build number)`);
