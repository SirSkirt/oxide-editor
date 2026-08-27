import fs from 'node:fs';

const css = fs.readFileSync('src/styles.css', 'utf8');
const MIN_CONTRAST = 4.5;
const themes = ['metallic', 'rust', 'modern-dark', 'modern-light'];
const roles = [
  '--syntax-default',
  '--syntax-keyword',
  '--syntax-ident',
  '--syntax-string',
  '--syntax-number',
  '--syntax-type',
  '--syntax-macro',
  '--syntax-function',
  '--syntax-comment',
  '--syntax-operator',
];

function blockFor(attribute, value) {
  const start = `:root[${attribute}="${value}"]`;
  const at = css.indexOf(start);
  if (at < 0) throw new Error(`Missing ${start} theme block`);
  const open = css.indexOf('{', at);
  const close = css.indexOf('}', open);
  if (open < 0 || close < 0) throw new Error(`Malformed ${start} theme block`);
  return css.slice(open + 1, close);
}

function variable(block, name) {
  const escaped = name.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const match = block.match(new RegExp(`${escaped}\\s*:\\s*(#[0-9a-fA-F]{6})`));
  if (!match) throw new Error(`Missing ${name}`);
  return match[1];
}

function channel(value) {
  const c = value / 255;
  return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
}

function luminance(hex) {
  const value = hex.slice(1);
  const r = channel(parseInt(value.slice(0, 2), 16));
  const g = channel(parseInt(value.slice(2, 4), 16));
  const b = channel(parseInt(value.slice(4, 6), 16));
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

function contrast(a, b) {
  const l1 = luminance(a);
  const l2 = luminance(b);
  const lighter = Math.max(l1, l2);
  const darker = Math.min(l1, l2);
  return (lighter + 0.05) / (darker + 0.05);
}

let failed = false;
for (const theme of themes) {
  const palette = blockFor('data-theme-palette', theme);
  const semantic = blockFor('data-theme-semantic', theme);
  const editor = variable(palette, '--t-editor');
  console.log(`\n${theme} editor ${editor}`);
  for (const role of roles) {
    const color = variable(semantic, role);
    const ratio = contrast(editor, color);
    const pass = ratio >= MIN_CONTRAST;
    console.log(`${pass ? 'PASS' : 'FAIL'} ${role.padEnd(20)} ${color}  ${ratio.toFixed(2)}:1`);
    if (!pass) failed = true;
  }
}

if (failed) {
  console.error(`\nSemantic Readability validation failed. Repaired themes require at least ${MIN_CONTRAST}:1 contrast.`);
  process.exit(1);
}

console.log(`\nSemantic Readability validation passed (minimum ${MIN_CONTRAST}:1).`);
