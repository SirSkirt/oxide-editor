import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

const isWindows = process.platform === 'win32';
const isLinux = process.platform === 'linux';
const exe = isWindows ? '.exe' : '';

if (!isWindows && !isLinux) {
  console.error(`Oxide updater sidecar preparation is not implemented for ${process.platform} yet.`);
  process.exit(1);
}

const rustInfo = execFileSync('rustc', ['-vV'], { encoding: 'utf8' });
const target = /^host:\s+(\S+)/m.exec(rustInfo)?.[1];
if (!target) {
  console.error('Could not determine the Rust host target triple.');
  process.exit(1);
}

const manifest = isWindows
  ? 'updater/src-tauri/Cargo.toml'
  : 'linux-updater/Cargo.toml';
const targetDir = isWindows
  ? path.join('updater', 'src-tauri', 'target')
  : path.join('linux-updater', 'target');

console.log(`Preparing Oxide updater sidecar for ${target} using ${manifest}...`);
execFileSync('cargo', [
  'build',
  '--release',
  '--manifest-path',
  manifest,
], { stdio: 'inherit' });

const source = path.join(targetDir, 'release', `oxide-updater${exe}`);
const destinationDir = path.join('src-tauri', 'binaries');
const destination = path.join(destinationDir, `oxide-updater-${target}${exe}`);

if (!fs.existsSync(source)) {
  console.error(`Updater build succeeded but ${source} was not found.`);
  process.exit(1);
}

fs.mkdirSync(destinationDir, { recursive: true });
fs.copyFileSync(source, destination);
if (!isWindows) fs.chmodSync(destination, 0o755);
console.log(`Updater sidecar staged at ${destination}`);
