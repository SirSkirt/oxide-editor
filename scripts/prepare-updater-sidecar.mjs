import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

const isWindows = process.platform === 'win32';
const exe = isWindows ? '.exe' : '';

const rustInfo = execFileSync('rustc', ['-vV'], { encoding: 'utf8' });
const target = /^host:\s+(\S+)/m.exec(rustInfo)?.[1];
if (!target) {
  console.error('Could not determine the Rust host target triple.');
  process.exit(1);
}

console.log(`Preparing Oxide updater sidecar for ${target}...`);
execFileSync('cargo', [
  'build',
  '--release',
  '--manifest-path',
  'updater/src-tauri/Cargo.toml',
], { stdio: 'inherit' });

const source = path.join('updater', 'src-tauri', 'target', 'release', `oxide-updater${exe}`);
const destinationDir = path.join('src-tauri', 'binaries');
const destination = path.join(destinationDir, `oxide-updater-${target}${exe}`);

if (!fs.existsSync(source)) {
  console.error(`Updater build succeeded but ${source} was not found.`);
  process.exit(1);
}

fs.mkdirSync(destinationDir, { recursive: true });
fs.copyFileSync(source, destination);
console.log(`Updater sidecar staged at ${destination}`);
