/* Rivet B1.3.6 Build 4 — Composable Theme Engine
 *
 * Themes are presentation recipes, never workspace/layout definitions.
 * Built-in and user themes use the same recipe shape so future custom theme
 * tooling can grow without creating a second theme system.
 */

export const THEME_STORAGE_KEY = 'oxide.appearance.theme';
export const CUSTOM_THEMES_STORAGE_KEY = 'oxide.appearance.customThemes';
export const CUSTOM_THEME_SCHEMA_VERSION = 1;

export const THEME_COMPONENTS = Object.freeze({
  materials: Object.freeze({
    oxide: { label: 'Oxide Iron', description: 'Original Rivet industrial surfaces with restrained depth.' },
    metallic: { label: 'Forged Iron', description: 'Dark forged iron with visible brushed grain, recessed seams, edge highlights, and physical depth.' },
    rust: { label: 'Rusty Iron', description: 'The forged-iron material aged with oxidation, patina, worn seams, exposed dark metal, and rougher surface variation.' },
    modern: { label: 'Modern Flat', description: 'Clean conventional IDE surfaces with minimal material texture.' },
  }),
  palettes: Object.freeze({
    oxide: { label: 'Oxide', tone: 'dark', description: 'Charcoal iron with Rivet rust-orange accents.' },
    metallic: { label: 'Metallic', tone: 'dark', description: 'Cool gunmetal neutrals with restrained copper accents.' },
    rust: { label: 'Rust', tone: 'dark', description: 'Aged iron, warm brown-black surfaces, and oxidized accents.' },
    'modern-dark': { label: 'Modern Dark', tone: 'dark', description: 'Neutral contemporary dark IDE colors.' },
    'modern-light': { label: 'Modern Light', tone: 'light', description: 'Neutral contemporary light IDE colors.' },
  }),
  controls: Object.freeze({
    oxide: { label: 'Oxide Industrial', description: 'Compact squared controls with the original Rivet treatment.' },
    metallic: { label: 'Forged', description: 'Raised/beveled controls with machined highlights and inset pressed states.' },
    rust: { label: 'Weathered', description: 'Forged controls with rougher oxidized edges and aged depth.' },
    modern: { label: 'Modern', description: 'Simple flat controls with subtle borders and small corner radius.' },
  }),
  semantic: Object.freeze({
    oxide: { label: 'Oxide Readability', tone: 'dark', description: 'Original Semantic Readability palette.' },
    metallic: { label: 'Metallic Readability', tone: 'dark', description: 'High-contrast steel, sage, amber, copper, and cream roles for dark gunmetal editors.' },
    rust: { label: 'Rust Readability', tone: 'dark', description: 'Warm but contrast-forward roles for dark aged-iron editors, retaining cool steel identifiers.' },
    'modern-dark': { label: 'Modern Dark Readability', tone: 'dark', description: 'Clean high-contrast semantic roles for conventional dark editors.' },
    'modern-light': { label: 'Modern Light Readability', tone: 'light', description: 'Purpose-built dark-on-light semantic roles with a dark neutral fallback so code cannot disappear on white.' },
  }),
});

export const BUILT_IN_THEMES = Object.freeze({
  oxide: Object.freeze({
    id: 'oxide', label: 'Oxide', menuLabel: 'OXIDE', builtIn: true,
    recipe: Object.freeze({ material: 'oxide', palette: 'oxide', controls: 'oxide', semantic: 'oxide' }),
  }),
  metallic: Object.freeze({
    id: 'metallic', label: 'Metallic', menuLabel: 'METALLIC', builtIn: true,
    recipe: Object.freeze({ material: 'metallic', palette: 'metallic', controls: 'metallic', semantic: 'metallic' }),
  }),
  rust: Object.freeze({
    id: 'rust', label: 'Rust', menuLabel: 'RUST', builtIn: true,
    recipe: Object.freeze({ material: 'rust', palette: 'rust', controls: 'rust', semantic: 'rust' }),
  }),
  'modern-light': Object.freeze({
    id: 'modern-light', label: 'Modern (Light)', menuLabel: 'MODERN LIGHT', builtIn: true,
    recipe: Object.freeze({ material: 'modern', palette: 'modern-light', controls: 'modern', semantic: 'modern-light' }),
  }),
  'modern-dark': Object.freeze({
    id: 'modern-dark', label: 'Modern (Dark)', menuLabel: 'MODERN DARK', builtIn: true,
    recipe: Object.freeze({ material: 'modern', palette: 'modern-dark', controls: 'modern', semantic: 'modern-dark' }),
  }),
});

const DEFAULT_RECIPE = BUILT_IN_THEMES.oxide.recipe;

function validComponent(group, value, fallback) {
  return Object.hasOwn(THEME_COMPONENTS[group], value) ? value : fallback;
}

export function normalizeThemeRecipe(recipe = {}) {
  return {
    material: validComponent('materials', recipe.material, DEFAULT_RECIPE.material),
    palette: validComponent('palettes', recipe.palette, DEFAULT_RECIPE.palette),
    controls: validComponent('controls', recipe.controls, DEFAULT_RECIPE.controls),
    semantic: validComponent('semantic', recipe.semantic, DEFAULT_RECIPE.semantic),
  };
}

function normalizeOverrides(overrides = {}) {
  const palette = overrides?.palette && typeof overrides.palette === 'object' ? { ...overrides.palette } : {};
  const semantic = overrides?.semantic && typeof overrides.semantic === 'object' ? { ...overrides.semantic } : {};
  return { palette, semantic };
}

export function normalizeCustomTheme(theme, index = 0) {
  if (!theme || typeof theme !== 'object') return null;
  const id = String(theme.id || `custom-${index + 1}`).replace(/[^a-zA-Z0-9:_-]/g, '-');
  const name = String(theme.name || `Custom Theme ${index + 1}`).trim().slice(0, 64) || `Custom Theme ${index + 1}`;
  return {
    id: id.startsWith('custom:') ? id : `custom:${id.replace(/^custom[-:]?/, '')}`,
    name,
    recipe: normalizeThemeRecipe(theme.recipe),
    overrides: normalizeOverrides(theme.overrides),
    builtIn: false,
  };
}

export function loadCustomThemes() {
  try {
    const raw = localStorage.getItem(CUSTOM_THEMES_STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    const source = Array.isArray(parsed) ? parsed : parsed?.themes;
    if (!Array.isArray(source)) return [];
    return source.map(normalizeCustomTheme).filter(Boolean);
  } catch (error) {
    console.warn('Could not load Rivet custom themes:', error);
    return [];
  }
}

export function saveCustomThemes(themes = []) {
  const normalized = themes.map(normalizeCustomTheme).filter(Boolean);
  try {
    localStorage.setItem(CUSTOM_THEMES_STORAGE_KEY, JSON.stringify({
      schema: CUSTOM_THEME_SCHEMA_VERSION,
      themes: normalized,
    }));
  } catch (error) {
    console.warn('Could not save Rivet custom themes:', error);
  }
  return normalized;
}

export function resolveTheme(themeId, customThemes = []) {
  if (Object.hasOwn(BUILT_IN_THEMES, themeId)) return BUILT_IN_THEMES[themeId];
  const custom = customThemes.find((theme) => theme.id === themeId);
  if (custom) {
    return {
      ...custom,
      menuLabel: custom.name.toUpperCase(),
    };
  }
  return BUILT_IN_THEMES.oxide;
}

export function loadStoredTheme(customThemes = []) {
  try {
    const stored = localStorage.getItem(THEME_STORAGE_KEY);
    return resolveTheme(stored, customThemes).id;
  } catch {
    return 'oxide';
  }
}

export function persistTheme(themeId) {
  try { localStorage.setItem(THEME_STORAGE_KEY, themeId); } catch { /* non-fatal in restricted webviews */ }
}

export function createCustomTheme({ name, recipe, overrides = {} } = {}) {
  const stamp = `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 7)}`;
  return normalizeCustomTheme({
    id: `custom:${stamp}`,
    name: name || 'Custom Theme',
    recipe,
    overrides,
  });
}
