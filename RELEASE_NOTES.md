# Rivet B1.3.6 · Build 8

B1.3.6 Build 8 introduces **centralized Settings and user-selectable Desktop/Mobile layouts**.

## Tools → Settings

Application preferences now have a dedicated home under **Tools → Settings…** instead of theme choices living in the View menu.

The first Settings sections include:

- **Theme** — one dropdown for Oxide, Metallic, Rust, Modern (Light), Modern (Dark), and saved custom themes.
- **Custom Theme** — a compact button directly beneath the theme selector that opens Theme Workshop.
- **Layout Mode** — an explicit selector for **Desktop Layout** and **Mobile Layout**.
- **Live Rust Check** and **Rust Code Analyzer/Completer** — persistent editor-assistance preferences, moved out of the main Tools command list and into Settings.

Theme and layout choices are persisted independently. Future application preferences should be added to Settings instead of being scattered across unrelated menus.

## User-controlled layout mode

Layout is a user preference, not a device classification. Rivet may choose a sensible first-run default based on the initial viewport, but after the user chooses a layout it does not automatically switch it.

This allows workflows such as:

- Windows tablet → Mobile Layout;
- a future/supported mobile target with a dock or large external display → Desktop Layout; and
- desktop/laptop → either layout when the user prefers a different interaction density.

## Mobile Layout

Mobile Layout keeps Rivet's functionality while rearranging the workbench for narrow/touch use:

- the editor remains the primary workspace;
- a compact **Files / Editor / Cargo** switcher replaces permanently visible side panels;
- Tutorial joins that switcher while an interactive lesson is active;
- Cargo command buttons remain available in a horizontally scrollable touch strip;
- menus use larger touch targets and mobile-safe popup positioning;
- Build Bay remains available and resizable;
- tabs/status/Build Bay controls can scroll when they exceed the available width;
- Welcome reflows to a single-column launch surface; and
- dialogs, the internal file browser, Theme Workshop, and Settings adapt to the mobile arrangement.

Desktop Layout remains Rivet's existing multi-panel workbench. No feature is removed by changing layouts.

## Theme workflow

Theme implementation is unchanged: themes remain material/presentation recipes and Semantic Readability remains theme-aware. Build 8 only relocates theme selection to Settings and connects Theme Workshop through the **Custom Theme** button.

The Build 7 forged-iron and rusty-iron texture systems remain intact.

## Validation

A new `npm run validate:layout` check verifies the Settings/Adaptive Layout contract in CI alongside `npm run validate:themes`. It checks that Settings contains theme/layout controls, Mobile Layout exposes Files/Editor/Cargo switching, the layout preference is persisted, and theme selection is no longer duplicated in View.

## Version

- Release version: `1.3.6`
- Display version: **B1.3.6**
- Build: **8**
- Full identity: **Rivet B1.3.6 · Build 8**
