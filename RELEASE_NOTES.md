# Rivet B1.3.6 · Build 4

B1.3.6 Build 4 is the **Composable Themes** build. It keeps Rivet's layout and functionality unchanged while replacing the first-generation monolithic theme selection with a component-based presentation engine designed for built-in and user-created themes.

## Composable theme recipes

Every theme now resolves through the same four-part recipe:

- **Material** — physical surface, texture, depth, seams, highlights, and weathering
- **Color Palette** — UI surfaces, text, borders, editor colors, and accents
- **Control Treatment** — button/tab edges, bevel/pressed behavior, and industrial-vs-modern control styling
- **Semantic Readability** — Rust semantic token colors

The five built-in themes are implemented as recipes using that same model. Theme components only affect presentation; they do not alter workspace geometry, panel placement, commands, or functionality.

## Material improvements

Build 4 also strengthens the intended distinction between Rivet's industrial themes:

- **Oxide** remains the relatively restrained/original industrial material.
- **Metallic** now leans further into forged/brushed gunmetal with subtle directional texture, brighter upper edges, darker inset edges, recessed depth, and more physical raised controls.
- **Rust** uses the forged-depth approach as a foundation but adds aged iron, irregular oxidation/patina, worn seams and edges, and rougher surface variation instead of simply tinting the interface brown.
- **Modern** material intentionally strips most of that physical treatment away for a conventional IDE presentation.

## Theme Workshop

**View → Theme → Theme Workshop…** now provides the first user-custom-theme workflow. Users can:

- choose Material independently
- choose the UI Color Palette independently
- choose Control Treatment independently
- choose the Semantic Readability palette independently
- preview an unsaved component combination without committing it
- name and save a custom theme
- edit or delete saved custom themes
- select saved custom themes directly from the Theme menu

Custom themes persist locally. The storage format is versioned and includes reserved palette/semantic override maps so future Rivet builds can add fine-grained custom colors, import/export, and additional theme components without introducing a second theme system.

## Existing fixes retained

Build 4 includes the Build 3 startup reliability fix and the Linux Oxide → Rivet Debian migration metadata (`Provides`, `Replaces`, and `Conflicts`). The signed updater continues to compare release version plus build number.

## Version

- Release version: `1.3.6`
- Display version: **B1.3.6**
- Build: **4**
- Full identity: **Rivet B1.3.6 · Build 4**
