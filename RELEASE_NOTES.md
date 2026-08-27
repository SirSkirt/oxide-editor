# Rivet B1.3.6 · Build 5

B1.3.6 Build 5 is the **Semantic Readability repair** for Rivet's expanded theme system. The Oxide readability palette remains unchanged; Metallic, Rust, Modern Dark, and especially Modern Light now prioritize source-code contrast and semantic separation instead of merely matching the surrounding UI palette.

## Modern Light visibility fix

Modern Light had a structural readability bug beyond its individual token colors: source text that did not receive a specific lexical/semantic class inherited the old dark-theme neutral gray. Against the white Modern Light editor this could make ordinary code appear extremely faint or effectively disappear.

Build 5 adds a first-class `--syntax-default` Semantic Readability role. The syntax layer now uses that theme-aware neutral source color for all unclassified/fallback text.

Modern Light therefore uses a purpose-built **dark-on-light** semantic palette, including a dark neutral fallback, dark steel-blue identifiers, dark sage strings, dark amber numbers/types, warm dark keywords/macros, and clearly readable comments. Red remains reserved for real errors/problems.

## Rebalanced dark-theme readability

The repaired dark themes also receive higher-contrast palettes:

- **Metallic** — brighter steel identifiers, sage strings, amber/gold numbers and types, copper/orange keywords/macros, cream functions, and more readable gray-green comments against gunmetal.
- **Rust** — preserves the warm aged-iron character while keeping identifiers deliberately cool/steel-colored and lifting comments and semantic roles away from the dark rust surface.
- **Modern Dark** — cleaner, brighter conventional dark-IDE semantic colors with stronger category separation.
- **Oxide** — unchanged, by design.

## Theme Workshop readability preview

Theme Workshop now shows an actual Rust **Semantic Readability Preview** while composing a custom theme. Palette and Semantic Readability presets carry an intended editor-surface tone (`dark` or `light`), and the Workshop warns when a user mixes a light editor surface with a dark semantic preset or vice versa. The combination is still allowed; Rivet simply makes the readability risk visible before saving it.

The custom-theme semantic override map also gains a `default`/neutral source role so future granular custom color controls can tune fallback source text explicitly.

## Contrast regression validation

A new `npm run validate:themes` check audits every repaired built-in semantic role against its matching editor background. Metallic, Rust, Modern Dark, and Modern Light must maintain at least **4.5:1** contrast for neutral text, keywords, identifiers, strings, numbers, types, macros, functions, comments, and operators. Build and release GitHub Actions now run this validation before publishing.

## Existing features retained

Build 5 retains Build 4's composable Theme Workshop, material/palette/control/semantic separation, the Build 3 startup and Debian migration fixes, version+build updater behavior, LLDB/DAP debugger, Rust Code Analyzer/Completer, resizable Build Bay, and the rest of the B1.3.6 functionality.

## Version

- Release version: `1.3.6`
- Display version: **B1.3.6**
- Build: **5**
- Full identity: **Rivet B1.3.6 · Build 5**
