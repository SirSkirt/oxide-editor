# Rivet B1.3.6 · Build 2

B1.3.6 Build 2 renames **Oxide Editor** to **Rivet**. The product tagline is **Rust Development Environment**. This build is a branding change only: the B1.3.6 theme engine, IDE layout, commands, debugger, tutorial, Build Bay, Cargo integration, and editor functionality remain the same.

## Branding

- Product name: **Rivet**
- Tagline: **Rust Development Environment**
- New Rivet application/logo mark across the main window and application icons.
- Window titles, About dialog, Run Terminal, file browser, updater UI, tutorial wording, and release presentation now use Rivet branding.
- The default visual theme is still named **Oxide**. Theme names are presentation choices and are independent of the product name.

## Themes

The five B1.3.6 themes remain unchanged:

- **Oxide** — default forged-workbench appearance.
- **Metallic** — machined/forged metal treatment.
- **Rust** — weathered rusted-iron treatment.
- **Modern (Light)** — conventional light IDE presentation.
- **Modern (Dark)** — conventional dark IDE presentation.

Semantic Readability Colors remain theme-aware and continue to preserve semantic category distinctions within every theme.

## Upgrade compatibility

Existing installations must be able to update into Rivet rather than becoming a separate product installation. For that reason, several legacy implementation identifiers intentionally remain unchanged in Build 2, including the `com.oxide.editor` application identifier, `oxide-editor` executable/package slug, `oxide-*` update feed/artifact names, updater command channels, and updater signing identity. These are compatibility details, not user-facing branding.

The existing application-data folder is also retained so tutorial progress and other persisted data are not orphaned by the rename.

## Version

- Release version: `1.3.6`
- Display version: **B1.3.6**
- Build: **2**
- Full identity: **Rivet B1.3.6 · Build 2**
