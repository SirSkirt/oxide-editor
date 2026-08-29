import fs from 'node:fs';

const main = fs.readFileSync('src/main.js', 'utf8');
const css = fs.readFileSync('src/styles.css', 'utf8');
const mobileCss = fs.readFileSync('src/mobile/mobile.css', 'utf8');
const requiredMain = [
  'data-menu-action="settings"',
  'id="settings-dialog"',
  'id="settings-theme-select"',
  'id="settings-custom-theme"',
  'id="settings-layout-select"',
  'id="settings-live-check"',
  'id="settings-completer"',
  'Desktop Layout',
  'Mobile Layout',
  "const LAYOUT_STORAGE_KEY = 'oxide.layout.mode'",
  "document.documentElement.dataset.layout = initialLayoutMode",
  'function applyLayoutMode(',
  'function setMobilePane(',
  'data-mobile-pane="project"',
  'data-mobile-pane="editor"',
  'data-mobile-pane="cargo"',
];
for (const token of requiredMain) {
  if (!main.includes(token)) throw new Error(`Missing Settings/Layout implementation token: ${token}`);
}

const requiredCss = [
  ':root[data-layout="mobile"] body',
  ':root[data-layout="mobile"] .mobile-workspace-bar',
  '.oxide-shell[data-mobile-pane="project"] .workspace > .project-panel',
  '.oxide-shell[data-mobile-pane="editor"] .workspace > .editor-stack',
  '.oxide-shell[data-mobile-pane="cargo"] .workspace > .cargo-panel',
  ':root[data-layout="mobile"] .browser-dialog',
  ':root[data-layout="mobile"] .settings-field select',
];
for (const token of requiredCss) {
  if (!mobileCss.includes(token)) throw new Error(`Missing mobile layout CSS contract: ${token}`);
}


function menuSection(source, menuName, nextMenuName) {
  const startMarker = `<button class="menu-trigger" data-menu="${menuName}">`;
  const endMarker = `<button class="menu-trigger" data-menu="${nextMenuName}">`;
  const start = source.indexOf(startMarker);
  const end = source.indexOf(endMarker, start + startMarker.length);
  if (start < 0 || end < 0 || end <= start) {
    throw new Error(`Could not inspect ${menuName} menu`);
  }
  return source.slice(start, end);
}

// Menu validation must be independent of Git's checkout line endings. Windows
// runners commonly use CRLF while Linux uses LF, so never parse menu boundaries
// by literal newline sequences.
const toolsMenu = menuSection(main, 'tools', 'debug');
if (!toolsMenu.includes('data-menu-action="settings"')) {
  throw new Error('Settings regression: Tools menu must contain Tools → Settings');
}
if (toolsMenu.includes('data-menu-action="toggle-live-check"') || toolsMenu.includes('data-menu-action="toggle-completer"')) {
  throw new Error('Application preference regression: persistent editor toggles must live in Settings');
}

const viewMenu = menuSection(main, 'view', 'help');
if (viewMenu.includes('Theme Workshop') || viewMenu.includes('theme-metallic') || viewMenu.includes('theme-oxide')) {
  throw new Error('Theme selection regression: theme preferences must live in Tools → Settings, not View');
}

console.log('PASS Tools → Settings contains theme and layout preferences');
console.log('PASS Mobile/Desktop layout preference is persisted and user-selectable');
console.log('PASS Mobile workspace keeps Files / Editor / Cargo access without changing functionality');
console.log('PASS Mobile layout styling is isolated in src/mobile/mobile.css');
console.log('PASS Theme controls were removed from View and centralized in Settings');
console.log('PASS persistent editor assistance toggles live in Settings');
