# Rivet B1.3.6 · Build 7

B1.3.6 Build 7 strengthens Rivet RDE's industrial identity by making the **Metallic** and **Rust** themes behave like physical materials instead of mostly color variations. Layout, functionality, editor geometry, and workflows are unchanged.

## Metallic = forged Iron

The Metallic theme is now treated as a dark forged-iron workstation material. Build 7 adds:

- visible but restrained directional brushed-metal grain
- faint cross-scratches and uneven metal mottle
- colder exposed upper-edge highlights
- darker recessed/lower edges and seams
- stronger inset depth on panel chrome and the Build Bay
- finer brushed texture on buttons, tabs, and interactive controls
- a subtle iron texture in the line-number gutter

The theme is still named **Metallic** for compatibility, while its material component is labeled **Forged Iron** in Theme Workshop.

## Rust = Rusty Iron

Rust now explicitly builds on the same forged-iron geometry and adds weathering rather than acting as a brown recolor. It adds:

- irregular rust/oxidation patches
- patina concentrated near seams, corners, and recessed edges
- darker exposed-iron variation between oxidized areas
- rougher directional grain than clean Iron
- weathered control surfaces and warmer worn edges
- stronger rust accumulation around panel/chrome boundaries

The Theme Workshop material component is labeled **Rusty Iron**.

## Readability guardrail

Material texture is intentionally strongest on Rivet's chassis, toolbars, panel headers, inspectors, controls, menus, dialogs, Build Bay framing, and other chrome. The actual Rust source backdrop remains low-noise and uses a solid editor surface so Semantic Readability Colors stay dominant. The line-number gutter receives only a very faint material cue.

Modern Dark and Modern Light remain intentionally clean and flat so Rivet keeps a meaningful distinction between its manufactured industrial themes and conventional IDE presentation.

## Existing fixes retained

Build 7 retains Build 6's syntax-overlay visibility fix, Build 5's contrast-first Semantic Readability palettes, Build 4's composable Theme Workshop/custom-theme architecture, Build 3's startup and Debian migration fixes, the version+build updater, LLDB/DAP debugger, Rust Code Analyzer/Completer, resizable Build Bay, and the rest of B1.3.6.

## Version

- Release version: `1.3.6`
- Display version: **B1.3.6**
- Build: **7**
- Full identity: **Rivet B1.3.6 · Build 7**
