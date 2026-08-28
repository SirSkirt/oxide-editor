# Rivet B1.3.6 · Build 6

B1.3.6 Build 6 is a focused **editor visibility repair** for the composable theme system. Projects and Rust source were loading correctly in Build 5, but the code could appear completely blank in the editor even though the file contents were present.

## Root cause

Rivet renders Rust highlighting using two stacked editor layers:

1. a syntax-highlight backdrop containing the visible colored source; and
2. the real textarea above it, which owns caret, selection, scrolling, editing, diagnostics, and keyboard behavior.

When syntax highlighting is active, the textarea intentionally makes its own glyphs transparent so the colored syntax layer can show through. Build 4's composable theme architecture changed every built-in/custom theme to `data-theme="composed"`. An older themed-editor CSS rule then applied an opaque `--t-editor` background to the textarea *after* the normal `.syntax-active` transparency rule.

The result was an opaque textarea with transparent glyphs sitting on top of valid highlighted source — effectively hiding every line of code while leaving the line numbers, project tree, Cargo inspector, and Build Bay working normally.

## Fix

Build 6 adds an explicit composable-theme overlay contract:

```css
:root[data-theme="composed"] .code-editor.syntax-active {
  color: transparent;
  background-color: transparent;
  -webkit-text-fill-color: transparent;
}
```

This higher-specificity rule is placed after the theme editor-surface binding, so all built-in themes and custom theme recipes keep their editor material/color while the syntax layer remains visible through the editing textarea.

The fix applies to **Oxide, Metallic, Rust, Modern Dark, Modern Light, and custom composed themes** without changing layout or editing functionality.

## Regression protection

`npm run validate:themes` now checks both:

- Semantic Readability contrast for the repaired theme palettes; and
- that the composed-theme syntax textarea retains a transparent background while highlighting is active and that this rule appears after the themed editor background binding.

This turns the exact Build 5 failure mode into a CI-detectable regression instead of relying only on visual testing.

## Existing features retained

Build 6 retains Build 5's repaired Semantic Readability palettes and neutral fallback source role, Build 4's composable Theme Workshop/custom theme architecture, Build 3's startup and Debian migration fixes, version+build updater behavior, LLDB/DAP debugger, Rust Code Analyzer/Completer, resizable Build Bay, and the rest of the B1.3.6 functionality.

## Version

- Release version: `1.3.6`
- Display version: **B1.3.6**
- Build: **6**
- Full identity: **Rivet B1.3.6 · Build 6**
