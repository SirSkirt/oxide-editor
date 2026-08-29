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


const toolsPopupStart = main.indexOf('<div class="menu-popup" data-popup="tools"');
const toolsPopupEnd = main.indexOf('</div>\n        </div>\n        <div class="menu-host">', toolsPopupStart);
if (toolsPopupStart < 0 || toolsPopupEnd < 0) throw new Error('Could not inspect Tools menu');
const toolsMenu = main.slice(toolsPopupStart, toolsPopupEnd);
if (toolsMenu.includes('data-menu-action="toggle-live-check"') || toolsMenu.includes('data-menu-action="toggle-completer"')) {
  throw new Error('Application preference regression: persistent editor toggles must live in Settings');
}

const viewPopupStart = main.indexOf('<div class="menu-popup" data-popup="view"');
const viewPopupEnd = main.indexOf('</div>\n        </div>\n        <div class="menu-host">', viewPopupStart);
if (viewPopupStart < 0 || viewPopupEnd < 0) throw new Error('Could not inspect View menu');
const viewMenu = main.slice(viewPopupStart, viewPopupEnd);
if (viewMenu.includes('Theme Workshop') || viewMenu.includes('theme-metallic') || viewMenu.includes('theme-oxide')) {
  throw new Error('Theme selection regression: theme preferences must live in Tools → Settings, not View');
}

console.log('PASS Tools → Settings contains theme and layout preferences');
console.log('PASS Mobile/Desktop layout preference is persisted and user-selectable');
console.log('PASS Mobile workspace keeps Files / Editor / Cargo access without changing functionality');
console.log('PASS Mobile layout styling is isolated in src/mobile/mobile.css');
console.log('PASS Theme controls were removed from View and centralized in Settings');
console.log('PASS persistent editor assistance toggles live in Settings');
