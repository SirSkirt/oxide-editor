# Oxide Editor B1.3.5 · Build 3

## Resizable Build Bay

- Added a forged-metal resize grip along the Build Bay's top edge.
- Drag upward to give Build/Problems/Debug more room; drag downward to reclaim editor space.
- Added viewport-aware minimum and maximum heights so the Build Bay cannot become uselessly tiny or consume the entire editor.
- The chosen height is stored locally and restored between Oxide sessions.
- Double-click the grip or press Home while it is focused to restore the responsive default height.
- Keyboard resizing is available with Up/Down; holding Shift uses a larger step.
- View → Reset Layout also restores the default Build Bay height.
- The user-resized height stays separate from the hide/show layout state, so hiding the Build Bay still collapses it correctly.

## Build-aware Windows + Linux updates

- Hardened update ordering around Oxide's actual identity pair: **release version first, build number second**.
- B1.3.5 Build 1 now treats B1.3.5 Build 2/3 as newer even though all use the same `1.3.5` SemVer.
- Added updater comparison tests covering newer builds, equal/older builds, and public-version precedence.
- Added `Cache-Control: no-cache` and `Pragma: no-cache` to update-feed requests to reduce stale same-release feed reuse.
- Release workflow now publishes each build under a distinct GitHub Release tag such as `v1.3.5-b3` while keeping Oxide's public version at `1.3.5`.
- Windows and Linux release jobs validate that generated platform feeds contain the expected `release_version` and `build` before upload.
- New build releases stay in GitHub draft state until both Windows and Linux jobs finish, preventing `releases/latest` from pointing at a half-published update.
- Signed package manifests continue to be checked against the exact requested version + build by both updater helpers before installation.
- The legacy Windows `latest.json` bridge remains available for older updater-enabled Oxide builds.

## Forged-workbench UI polish

- Kept Oxide's existing layout and industrial identity; this is a refinement pass, not a redesign.
- Tightened panel/header hierarchy with cleaner forged edges and restrained copper markers.
- Improved command buttons, profile selector, tabs, project-tree rows, editor chrome, Cargo inspector cards, and dependency rows.
- Improved Build Bay tabs/output treatment so the bottom workbench reads as one intentional subsystem.
- Added more consistent dark workbench scrollbars across project, Cargo, debug, tutorial, browser, and completion surfaces.
- Preserved the compact desktop density, charcoal/iron surfaces, rust-orange accents, and existing Build Bay/project-tree structure.

# Oxide Editor B1.3.5 · Build 2

## Semantic Readability Colors

- Added Oxide's internally named **Semantic Readability Colors** system for Rust source readability.
- rust-analyzer semantic tokens now refine the real editor's color categories when the analyzer is ready.
- Oxide retains its lexical Rust highlighter as a fallback while rust-analyzer is starting or when semantic data is unavailable.
- Ordinary variables and identifiers consistently use steel blue (`#83A6B8`) instead of white/purple.
- Rust keywords use rust orange (`#D87941`).
- Strings use sage green (`#8FAF72`).
- Numbers/booleans use amber (`#D3A95F`).
- Types use brass/gold (`#C4A45F`).
- Macros use brighter copper/orange (`#E99A62`).
- Function/method names use warm cream (`#DDD0BF`).
- Comments use muted gray-green (`#70786E`).
- Operators and punctuation remain neutral light gray/off-white.
- Red is deliberately reserved for real compiler/errors/problems rather than normal source categories.

## Debugger expansion

- Added Cargo binary-target discovery and an Oxide target picker for projects with multiple runnable binaries.
- Expanded Linux LLDB adapter detection to find both modern `lldb-dap` and older/versioned `lldb-vscode` binaries used by Ubuntu/Pop!_OS packages.
- Added thread discovery and thread selection while paused.
- Added recursive/nested variable expansion for structured values.
- Added breakpoint options for conditions, hit conditions, and log messages/logpoints.
- Added a paused-state Debug Console/REPL backed by LLDB DAP expression evaluation.
- Added debugger Restart alongside Continue, Pause, Step Over, Step Into, Step Out, and Stop.
- Added a breakpoint list to the Debug Workbench with navigation and breakpoint editing.
- Improved stale/finished debugger-session cleanup so a dead LLDB adapter does not block the next run.
- Breakpoint markers use copper/gold rather than error red.

# Oxide Editor B1.3.5 · Build 1

## Debugging foundation

- Added Oxide's first proper IDE debugger subsystem using LLDB's Debug Adapter Protocol (DAP).
- Added LLDB/lldb-dap detection with Windows and Pop!_OS/Ubuntu-oriented installation guidance when unavailable.
- Added clickable Rust editor gutter breakpoints and live breakpoint synchronization during an active debug session.
- Added Start, Continue, Pause, Step Over, Step Into, Step Out, and Stop debugger controls.
- Added current execution-line highlighting.
- Added a Debug Workbench in the bottom panel with call stack, locals/variables, watch expressions, and debugger output.
- Debug launches build the normal Cargo binary in debug profile and attach LLDB to that executable; Oxide does not introduce a custom project/build format.
- Build 1 supports one runnable binary target. Multi-target debug selection is intentionally deferred to a later B1.3.5 build.

# Oxide Editor B1.3.4 · Build 2

B1.3.4 continues **The Compatibility Update** with a proper automatic updater for Linux `.deb` installations and responsive fixes for shorter laptop displays.

## Linux automatic `.deb` updates

- Oxide now detects when the running Linux release came from a Debian package.
- The signed Linux update ZIP contains both the AppImage and `.deb` payloads.
- Oxide downloads and verifies the signed package before requesting any elevated access.
- `.deb` installs use Linux's normal **Polkit/pkexec** authorization flow; Oxide never asks for or receives the user's password itself.
- After authorization, the update helper installs the staged package with `dpkg --install` and reopens Oxide.
- AppImage updates continue to replace the user-owned AppImage directly without elevation.
- Unpackaged development builds do not attempt system-level self-installation.

## Smaller-screen layout repair

- The editor/workspace can no longer expand over the Build Bay when lesson content is tall.
- Project, Cargo, and Tutorial panels now obey the available workspace height.
- The Tutorial body scrolls vertically on shorter screens.
- Tutorial navigation/actions remain reachable with a sticky action area.
- Build Bay height scales down on short laptop displays before sacrificing editor space.
- Additional width scaling keeps Tutorial Mode usable on narrower laptop windows.

## Rust syntax highlighting

- Added Oxide-native Rust syntax highlighting in the real editor, including tutorial projects.
- Rust keywords and macros use Oxide rust/orange, strings use green, declared variables use copper-red, types use steel-blue, numbers use amber, and comments use a muted gray.
- Highlighting is layered behind the existing textarea so editing, selections, diagnostics, indentation, and Rust Code Analyzer/Completer behavior stay on the same editor path.

## Retained from B1.3.3

- Rust Code Analyzer/Completer powered by rust-analyzer.
- Node.js 24 CI/tooling baseline.
- Windows updater warning cleanup.
- Windows and Linux signed package update feeds.
- 26-lesson hands-on Rust tutorial.

- Fixed compact-screen editor layout where the inline Rust diagnostic banner could expand over most of the editor and push the code area toward the Build Bay.
- Rust Code Analyzer/Completer popups now clamp themselves to the available editor area and switch to a compact suggestions-only layout when space is tight.
- GitHub Actions maintained JavaScript actions now use Node.js 24-native major versions (`checkout@v7`, `setup-node@v7`, `upload-artifact@v7`).

## Internal build-number updates

- Added an internal build number separate from the public Oxide version.
- B1.3.4 Build 2 can be detected as newer than B1.3.4 Build 1.
- Update feeds now publish both `release_version` and `build`.
- Signed update package filenames include the build number.
- The update dialog and About/status UI show build identity where useful.
- Added `npm run release:build` to increment a rebuild without changing the public version.
