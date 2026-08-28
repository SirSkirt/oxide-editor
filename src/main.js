import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import './styles.css';
import rivetLogo from './assets/rivet-logo.png';
import {
  THEME_COMPONENTS,
  createCustomTheme,
  loadCustomThemes,
  loadStoredTheme,
  normalizeThemeRecipe,
  persistTheme,
  resolveTheme,
  saveCustomThemes,
} from './theme-engine.js';

const app = document.querySelector('#app');

let customThemes = loadCustomThemes();
const initialTheme = loadStoredTheme(customThemes);
const initialThemeDefinition = resolveTheme(initialTheme, customThemes);

// Build 4 routes every theme — built-in or custom — through the same component
// recipe. The old data-theme blocks remain as a compatibility fallback, while
// these component attributes are the authoritative presentation contract.
document.documentElement.dataset.theme = 'composed';
document.documentElement.dataset.themeId = initialThemeDefinition.id;
document.documentElement.dataset.themeMaterial = initialThemeDefinition.recipe.material;
document.documentElement.dataset.themePalette = initialThemeDefinition.recipe.palette;
document.documentElement.dataset.themeControls = initialThemeDefinition.recipe.controls;
document.documentElement.dataset.themeSemantic = initialThemeDefinition.recipe.semantic;

const state = {
  theme: initialTheme,
  themeRecipe: normalizeThemeRecipe(initialThemeDefinition.recipe),
  themeStudioEditingId: '',
  themeStudioPreviewing: false,
  customThemes,
  projectPath: '',
  platform: { os: 'unknown', arch: 'unknown', pathCaseSensitive: false, automaticUpdates: false, updateMode: 'unknown' },
  tabs: [],
  activeTabPath: '',
  currentFile: '',
  dirty: false,
  release: false,
  buildRunning: false,
  outputMode: 'friendly',
  rawLines: [],
  friendlyLines: [],
  manifest: null,
  browserMode: 'open',
  browserPath: '',
  browserParent: null,
  browserSelectedPath: '',
  browserSelectedKind: '',
  browserRoots: [],
  view: { project: true, cargo: true, build: true },
  consoleView: 'build',
  diagnostics: [],
  liveCheck: true,
  analysisTimer: null,
  analysisRunning: false,
  analysisQueued: false,
  analysisGeneration: 0,
  completer: {
    enabled: true,
    available: false,
    visible: false,
    items: [],
    selected: 0,
    requestToken: 0,
    timer: null,
    prefixStart: 0,
    signatureVisible: false,
    dismissedThroughWord: false,
  },
  semanticReadability: {
    tokens: [],
    timer: null,
    requestToken: 0,
    active: false,
  },
  intelligence: {
    references: [],
    codeActions: [],
    pendingRename: null,
  },
  debugger: {
    available: false,
    adapter: '',
    message: '',
    running: false,
    stopped: false,
    threadId: null,
    threads: [],
    selectedFrameId: null,
    frames: [],
    variables: [],
    expandedVariables: new Map(),
    watches: [],
    watchResults: [],
    breakpoints: new Map(),
    editingBreakpoint: null,
    selectedTarget: null,
    executionPath: '',
    executionLine: 0,
    output: [],
    consoleHistory: [],
  },
  terminalRunning: false,
  terminalEnded: false,
  terminalVisible: false,
  terminalPositioned: false,
  updater: {
    pending: null,
    checking: false,
    installing: false,
    downloaded: 0,
    contentLength: 0,
  },
  tutorialEvalTimer: null,
  tutorial: {
    active: false,
    catalog: null,
    progress: { lessons: {} },
    lesson: null,
    stepIndex: 0,
    checkpoint: '',
    runOutput: '',
    runSuccess: null,
    advancing: false,
    stepComplete: false,
    lessonComplete: false,
    previousCargoView: true,
    previousLiveCheck: true,
  },
};

app.innerHTML = `
  <main class="oxide-shell welcome-mode">
    <header class="brandbar">
      <div class="brand-mark"><img src="${rivetLogo}" alt="Rivet logo" /></div>
      <div class="brand-copy"><strong>RIVET</strong><span>Rust Development Environment</span></div>
      <div class="toolchain-lamps" title="Detected Rust toolchain">
        <span class="lamp" id="cargo-lamp"></span><span id="cargo-version">Cargo: unknown</span>
        <span class="lamp" id="rustc-lamp"></span><span id="rustc-version">rustc: unknown</span>
      </div>
    </header>

    <nav class="menu-bar" aria-label="Rivet menu bar">
      <div class="menu-cluster">
        <div class="menu-host">
          <button class="menu-trigger" data-menu="file">File</button>
          <div class="menu-popup" data-popup="file" role="menu">
            <button role="menuitem" data-menu-action="new-project"><span>New Project…</span><kbd>Ctrl+N</kbd></button>
            <button role="menuitem" data-menu-action="open-project"><span>Open Project…</span><kbd>Ctrl+O</kbd></button>
            <div class="menu-separator"></div>
            <button role="menuitem" data-menu-action="save-file"><span>Save File</span><kbd>Ctrl+S</kbd></button>
            <button role="menuitem" data-menu-action="close-file"><span>Close File</span><kbd>Ctrl+W</kbd></button>
            <button role="menuitem" data-menu-action="save-project-as"><span>Save Project As…</span><kbd>Ctrl+Shift+S</kbd></button>
            <div class="menu-separator"></div>
            <button role="menuitem" data-menu-action="close-project"><span>Close Project</span></button>
            <button role="menuitem" data-menu-action="exit"><span>Exit Rivet</span></button>
          </div>
        </div>
        <div class="menu-host">
          <button class="menu-trigger" data-menu="edit">Edit</button>
          <div class="menu-popup" data-popup="edit" role="menu">
            <button role="menuitem" data-menu-action="undo"><span>Undo</span><kbd>Ctrl+Z</kbd></button>
            <button role="menuitem" data-menu-action="redo"><span>Redo</span><kbd>Ctrl+Y</kbd></button>
            <div class="menu-separator"></div>
            <button role="menuitem" data-menu-action="cut"><span>Cut</span><kbd>Ctrl+X</kbd></button>
            <button role="menuitem" data-menu-action="copy"><span>Copy</span><kbd>Ctrl+C</kbd></button>
            <button role="menuitem" data-menu-action="paste"><span>Paste</span><kbd>Ctrl+V</kbd></button>
            <button role="menuitem" data-menu-action="select-all"><span>Select All</span><kbd>Ctrl+A</kbd></button>
          </div>
        </div>
        <div class="menu-host">
          <button class="menu-trigger" data-menu="tools">Tools</button>
          <div class="menu-popup" data-popup="tools" role="menu">
            <button role="menuitem" data-menu-action="check"><span>Cargo Check</span><kbd>F6</kbd></button>
            <button role="menuitem" data-menu-action="build"><span>Cargo Build</span><kbd>F7</kbd></button>
            <button role="menuitem" data-menu-action="run"><span>Run…</span><kbd>F5</kbd></button>
            <button role="menuitem" data-menu-action="test"><span>Cargo Test</span><kbd>F8</kbd></button>
            <button role="menuitem" data-menu-action="clean"><span>Cargo Clean</span></button>
            <div class="menu-separator"></div>
            <button role="menuitem" data-menu-action="toggle-live-check"><span><i class="menu-check" data-check="live-check">✓</i> Live Rust Check</span></button>
            <button role="menuitem" data-menu-action="analyze-now"><span>Analyze Project Now</span><kbd>Ctrl+F6</kbd></button>
            <button role="menuitem" data-menu-action="toggle-completer"><span><i class="menu-check" data-check="completer">✓</i> Rust Code Analyzer/Completer</span></button>
            <button role="menuitem" data-menu-action="trigger-completion"><span>Trigger Code Completion</span><kbd>Ctrl+Space</kbd></button>
            <div class="menu-separator"></div>
            <button role="menuitem" data-menu-action="go-definition"><span>Go to Definition</span><kbd>F12</kbd></button>
            <button role="menuitem" data-menu-action="find-references"><span>Find References</span><kbd>Shift+F12</kbd></button>
            <button role="menuitem" data-menu-action="rename-symbol"><span>Rename Symbol…</span><kbd>F2</kbd></button>
            <button role="menuitem" data-menu-action="code-actions"><span>Code Actions / Quick Fixes…</span><kbd>Ctrl+.</kbd></button>
            <div class="menu-separator"></div>
            <button role="menuitem" data-menu-action="tutorial"><span>Interactive Rust Tutorial…</span><kbd>Ctrl+Alt+T</kbd></button>
            <div class="menu-separator"></div>
            <button role="menuitem" data-menu-action="add-dependency"><span>Add Dependency…</span></button>
            <button role="menuitem" data-menu-action="refresh-toolchain"><span>Refresh Toolchain</span></button>
          </div>
        </div>
        <div class="menu-host">
          <button class="menu-trigger" data-menu="debug">Debug</button>
          <div class="menu-popup" data-popup="debug" role="menu">
            <button role="menuitem" data-menu-action="debug-start"><span>Start Debugging</span><kbd>F9</kbd></button>
            <button role="menuitem" data-menu-action="debug-continue"><span>Continue</span><kbd>Ctrl+F10</kbd></button>
            <button role="menuitem" data-menu-action="debug-pause"><span>Pause</span></button>
            <button role="menuitem" data-menu-action="debug-restart"><span>Restart</span><kbd>Ctrl+Shift+F9</kbd></button>
            <div class="menu-separator"></div>
            <button role="menuitem" data-menu-action="debug-next"><span>Step Over</span><kbd>F10</kbd></button>
            <button role="menuitem" data-menu-action="debug-step-in"><span>Step Into</span><kbd>F11</kbd></button>
            <button role="menuitem" data-menu-action="debug-step-out"><span>Step Out</span><kbd>Shift+F11</kbd></button>
            <div class="menu-separator"></div>
            <button role="menuitem" data-menu-action="debug-stop"><span>Stop Debugging</span><kbd>Ctrl+F9</kbd></button>
            <button role="menuitem" data-menu-action="show-debug"><span>Show Debug Panel</span></button>
          </div>
        </div>
        <div class="menu-host">
          <button class="menu-trigger" data-menu="view">View</button>
          <div class="menu-popup" data-popup="view" role="menu">
            <button role="menuitem" data-menu-action="toggle-project"><span><i class="menu-check" data-check="project">✓</i> Project Panel</span></button>
            <button role="menuitem" data-menu-action="toggle-cargo"><span><i class="menu-check" data-check="cargo">✓</i> Cargo Inspector</span></button>
            <button role="menuitem" data-menu-action="toggle-build"><span><i class="menu-check" data-check="build">✓</i> Build Bay</span><kbd>Ctrl+&#96;</kbd></button>
            <div class="menu-separator"></div>
            <button role="menuitem" data-menu-action="show-build"><span>Build Output</span></button>
            <button role="menuitem" data-menu-action="show-terminal"><span>Run Terminal Window</span></button>
            <button role="menuitem" data-menu-action="show-problems"><span>Problems</span></button>
            <div class="menu-separator"></div>
            <div class="menu-section-label">THEME <span id="theme-menu-current">OXIDE</span></div>
            <button role="menuitem" data-menu-action="theme-oxide"><span><i class="menu-check" data-theme-check="oxide">✓</i> Oxide</span></button>
            <button role="menuitem" data-menu-action="theme-metallic"><span><i class="menu-check" data-theme-check="metallic"></i> Metallic</span></button>
            <button role="menuitem" data-menu-action="theme-rust"><span><i class="menu-check" data-theme-check="rust"></i> Rust</span></button>
            <button role="menuitem" data-menu-action="theme-modern-light"><span><i class="menu-check" data-theme-check="modern-light"></i> Modern (Light)</span></button>
            <button role="menuitem" data-menu-action="theme-modern-dark"><span><i class="menu-check" data-theme-check="modern-dark"></i> Modern (Dark)</span></button>
            <div id="custom-theme-menu-section" hidden>
              <div class="menu-separator"></div>
              <div class="menu-section-label">CUSTOM THEMES</div>
              <div id="custom-theme-menu-items"></div>
            </div>
            <div class="menu-separator"></div>
            <button role="menuitem" data-menu-action="theme-customize"><span>Theme Workshop…</span></button>
            <div class="menu-separator"></div>
            <button role="menuitem" data-menu-action="reset-layout"><span>Reset Layout</span></button>
          </div>
        </div>
        <div class="menu-host">
          <button class="menu-trigger" data-menu="help">Help</button>
          <div class="menu-popup" data-popup="help" role="menu">
            <button role="menuitem" data-menu-action="shortcuts"><span>Keyboard Shortcuts</span></button>
            <button role="menuitem" data-menu-action="check-updates"><span>Check for Updates…</span></button>
            <div class="menu-separator"></div>
            <button role="menuitem" data-menu-action="about"><span>About Rivet</span></button>
          </div>
        </div>
      </div>
      <div class="menu-project-readout" id="menu-project-readout">NO PROJECT</div>
      <div class="profile-switch" role="group" aria-label="Build profile">
        <button class="profile active" data-profile="debug">DEBUG</button>
        <button class="profile" data-profile="release">RELEASE</button>
      </div>
    </nav>

    <section class="command-deck" aria-label="Cargo commands">
      <button class="command-button" data-action="check"><span>✓</span>CHECK</button>
      <button class="command-button build" data-action="build"><span>◆</span>BUILD</button>
      <button class="command-button run" data-action="run"><span>▶</span>RUN</button>
      <button class="command-button debug-command" data-debug-action="start"><span>◈</span>DEBUG</button>
      <button class="command-button" data-action="test"><span>▣</span>TEST</button>
      <button class="command-button danger-subtle" data-action="clean"><span>⌫</span>CLEAN</button>
      <div class="command-readout" id="command-readout">SELECT A PROJECT TO BEGIN</div>
    </section>

    <section id="welcome-screen" class="welcome-screen">
      <div class="welcome-plate">
        <div class="welcome-eyebrow">RIVET · B1.3.6 · BUILD 7</div>
        <h1>Welcome to Rivet</h1>
        <p>To get started, select one of the options.</p>
        <div class="welcome-actions">
          <button type="button" id="welcome-new" class="welcome-action">
            <span class="welcome-action-icon">＋</span>
            <span><strong>NEW PROJECT</strong><small>Create a new Cargo project and start with Hello World.</small></span>
          </button>
          <button type="button" id="welcome-open" class="welcome-action">
            <span class="welcome-action-icon">▣</span>
            <span><strong>OPEN PROJECT</strong><small>Open an existing folder containing Cargo.toml.</small></span>
          </button>
          <button type="button" id="welcome-tutorial" class="welcome-action tutorial-welcome-action">
            <span class="welcome-action-icon">◆</span>
            <span><strong>TUTORIAL</strong><small>Learn Rust by writing, breaking, fixing, building, and running real code.</small></span>
          </button>
        </div>
        <div class="welcome-status">
          <span class="lamp" id="welcome-cargo-lamp"></span><span id="welcome-cargo-status">Checking Cargo…</span>
          <span class="divider">/</span>
          <span class="lamp" id="welcome-rustc-lamp"></span><span id="welcome-rustc-status">Checking rustc…</span>
        </div>
      </div>
    </section>

    <section id="workspace" class="workspace" hidden>
      <aside class="project-panel panel" data-view-panel="project">
        <div class="panel-head"><span>PROJECT</span><small id="project-name">—</small></div>
        <div id="project-tree" class="project-tree empty-state">File → Open Project…</div>
      </aside>
      <section class="editor-stack">
        <div class="tab-rail">
          <div id="file-tabs" class="file-tabs" role="tablist" aria-label="Open files"><div class="tab-empty">NO FILE OPEN</div></div>
          <button id="save-file" class="save-button" disabled>SAVE</button>
        </div>
        <button type="button" id="diagnostic-banner" class="diagnostic-banner" hidden>
          <span id="diagnostic-banner-level" class="diagnostic-level">CHECK</span>
          <span id="diagnostic-banner-text">No Rust problems detected.</span>
          <span class="diagnostic-banner-more">VIEW PROBLEMS →</span>
        </button>
        <div class="editor-wrap">
          <div id="line-numbers" class="line-numbers" title="Click a Rust line number to toggle a breakpoint"><span class="line-number" data-line="1">1</span></div>
          <pre id="syntax-layer" class="syntax-layer" aria-hidden="true"><code id="syntax-code"></code></pre>
          <div id="debug-line-highlight" class="debug-line-highlight" hidden></div>
          <span id="bracket-match-a" class="bracket-match-marker" hidden aria-hidden="true"></span>
          <span id="bracket-match-b" class="bracket-match-marker" hidden aria-hidden="true"></span>
          <textarea id="editor" class="code-editor" spellcheck="false" aria-label="Code editor" placeholder="Open a .rs, .toml, or text file from the project tree."></textarea>
          <div id="code-completer" class="code-completer" hidden role="listbox" aria-label="Rust Code Analyzer/Completer suggestions">
            <div id="completion-list" class="completion-list"></div>
            <aside class="completion-detail">
              <div class="completion-detail-head"><span id="completion-detail-kind">SYMBOL</span><strong id="completion-detail-label"></strong></div>
              <code id="completion-detail-signature"></code>
              <div id="completion-detail-docs" class="completion-detail-docs"></div>
            </aside>
          </div>
          <div id="signature-help" class="signature-help" hidden>
            <div id="signature-help-label" class="signature-help-label"></div>
            <div id="signature-help-docs" class="signature-help-docs"></div>
          </div>
        </div>
      </section>
      <aside id="tutorial-panel" class="tutorial-panel panel" hidden>
        <div class="panel-head"><span>TUTORIAL</span><small id="tutorial-course-label">BEGINNER</small></div>
        <div class="tutorial-panel-body">
          <div class="tutorial-lesson-meta"><span id="tutorial-lesson-title">Lesson</span><small id="tutorial-step-counter">STEP 1 / 1</small></div>
          <div class="tutorial-step-title" id="tutorial-step-title">Activity</div>
          <p class="tutorial-explanation" id="tutorial-explanation"></p>
          <div id="tutorial-example" class="tutorial-example" hidden>
            <span class="tutorial-example-label">EXAMPLE</span>
            <pre><code id="tutorial-example-code"></code></pre>
            <div id="tutorial-example-parts" class="tutorial-example-parts"></div>
          </div>
          <div class="tutorial-objective-block"><span>NOW YOU TRY</span><strong id="tutorial-objective"></strong></div>
          <div id="tutorial-feedback" class="tutorial-feedback">Rivet is watching the real project for this objective.</div>
          <div id="tutorial-experiment-note" class="tutorial-experiment-note" hidden>You’ve moved away from the lesson objective. That’s okay — experiment as much as you want.</div>
          <button type="button" id="tutorial-learn-more" class="tutorial-small-button">LEARN MORE</button>
          <div id="tutorial-learn-more-text" class="tutorial-learn-more-text" hidden></div>
          <div class="tutorial-panel-actions">
            <button type="button" id="tutorial-next" class="metal-button primary tutorial-next-button" hidden>NEXT STEP →</button>
            <button type="button" id="tutorial-return" class="metal-button">RETURN TO LESSON</button>
            <button type="button" id="tutorial-home-button" class="metal-button">TUTORIAL HOME</button>
            <button type="button" id="tutorial-exit" class="metal-button">EXIT TUTORIAL</button>
          </div>
        </div>
      </aside>
      <aside class="cargo-panel panel" data-view-panel="cargo">
        <div class="panel-head"><span>CARGO</span><small>MANIFEST</small></div>
        <div id="cargo-inspector" class="cargo-inspector empty-state">Cargo.toml information will appear here.</div>
      </aside>
    </section>

    <section id="build-console" class="build-console panel" data-view-panel="build" hidden>
      <div id="build-resizer" class="build-resizer" role="separator" aria-label="Resize Build Bay" aria-orientation="horizontal" aria-valuemin="96" aria-valuemax="500" tabindex="0" title="Drag to resize Build Bay · Double-click to reset">
        <span class="build-resizer-grip" aria-hidden="true"></span>
      </div>
      <div class="console-head enhanced-console-head">
        <div><span id="console-title">BUILD BAY</span><small id="build-status">IDLE</small></div>
        <div class="console-controls">
          <div class="console-view-tabs" role="tablist" aria-label="Bottom panel">
            <button class="console-tab console-view active" data-console-view="build">BUILD</button>
            <button class="console-tab console-view" data-console-view="problems">PROBLEMS <b id="problem-count">0</b></button>
            <button class="console-tab console-view" data-console-view="debug">DEBUG</button>
          </div>
          <div id="build-mode-tabs" class="console-tabs" role="tablist">
            <button class="console-tab active" data-mode="friendly">FRIENDLY</button>
            <button class="console-tab" data-mode="raw">RAW CARGO</button>
            <button id="clear-output" class="console-tab">CLEAR</button>
          </div>
        </div>
      </div>
      <div id="build-output" class="build-output console-pane" aria-live="polite"><div class="output-line muted">Project output will appear here.</div></div>
      <div id="problems-pane" class="problems-pane console-pane" hidden>
        <div id="problems-list" class="problems-list"><div class="problems-empty">No Rust problems detected.</div></div>
      </div>
      <div id="debug-pane" class="debug-pane console-pane" hidden>
        <div class="debug-toolbar">
          <span class="debug-toolbar-label"><b>DEBUGGER</b><small id="debugger-detail">CHECKING LLDB…</small></span>
          <button type="button" class="debug-tool primary" data-debug-action="start">START</button>
          <button type="button" class="debug-tool" data-debug-action="continue" disabled>CONTINUE</button>
          <button type="button" class="debug-tool" data-debug-action="pause" disabled>PAUSE</button>
          <button type="button" class="debug-tool" data-debug-action="restart" disabled>RESTART</button>
          <button type="button" class="debug-tool" data-debug-action="next" disabled>OVER</button>
          <button type="button" class="debug-tool" data-debug-action="step-in" disabled>INTO</button>
          <button type="button" class="debug-tool" data-debug-action="step-out" disabled>OUT</button>
          <button type="button" class="debug-tool stop" data-debug-action="stop" disabled>STOP</button>
          <label class="debug-thread-picker" title="Active debugger thread"><span>THREAD</span><select id="debug-thread-select" disabled><option value="">—</option></select></label>
        </div>
        <div class="debug-grid">
          <section class="debug-column debug-navigation-column">
            <div class="debug-section-head">BREAKPOINTS <small>RIGHT-CLICK GUTTER TO EDIT</small></div>
            <div id="debug-breakpoints" class="debug-list debug-breakpoint-list"><div class="debug-empty">Click a Rust line number to add a breakpoint.</div></div>
            <div class="debug-section-head">CALL STACK</div>
            <div id="debug-call-stack" class="debug-list debug-stack-list"><div class="debug-empty">Start debugging to inspect stack frames.</div></div>
          </section>
          <section class="debug-column variables-column"><div class="debug-section-head">LOCALS / VARIABLES <small>EXPAND VALUES WITH ▸</small></div><div id="debug-variables" class="debug-list"><div class="debug-empty">Variables appear when execution is paused.</div></div></section>
          <section class="debug-column"><div class="debug-section-head">WATCH</div><form id="debug-watch-form" class="debug-watch-form"><input id="debug-watch-input" spellcheck="false" placeholder="Expression, e.g. name"/><button type="submit">ADD</button></form><div id="debug-watch-list" class="debug-list"><div class="debug-empty">Add expressions to watch while paused.</div></div></section>
          <section class="debug-column output-column"><div class="debug-section-head">DEBUG CONSOLE <small>EXPRESSIONS + LLDB COMMANDS</small></div><div id="debug-output" class="debug-output"><div class="debug-empty">Debugger output will appear here.</div></div><form id="debug-console-form" class="debug-console-form"><span>›</span><input id="debug-console-input" spellcheck="false" autocomplete="off" placeholder="Expression or LLDB command…" disabled/><button type="submit" disabled>RUN</button></form></section>
        </div>
      </div>
    </section>

    <footer class="status-rail">
      <span id="file-status">NO FILE</span>
      <span id="analysis-status">RUST CHECK: IDLE</span>
      <span id="analyzer-status">ANALYZER: CHECKING</span>
      <span id="debugger-status">DEBUGGER: CHECKING</span>
      <span id="profile-status">PROFILE: DEBUG</span>
      <span>RIVET B1.3.6 · BUILD 7</span>
    </footer>
  </main>

  <dialog id="file-browser-dialog" class="oxide-dialog browser-dialog">
    <div class="dialog-head browser-dialog-head"><div><span id="browser-title">OPEN PROJECT</span><small>RIVET FILE BROWSER</small></div><button type="button" id="browser-close" class="dialog-close" aria-label="Close">×</button></div>
    <div class="browser-location browser-location-expanded">
      <button type="button" id="browser-up" class="metal-button browser-nav">↑ UP</button>
      <input id="browser-path" spellcheck="false" aria-label="Current folder" />
      <button type="button" id="browser-go" class="metal-button browser-nav">GO</button>
      <button type="button" id="browser-new-folder" class="metal-button browser-nav new-folder-button">+ FOLDER</button>
    </div>
    <div id="browser-new-folder-row" class="browser-inline-row" hidden>
      <label for="browser-new-folder-name">NEW FOLDER</label>
      <input id="browser-new-folder-name" spellcheck="false" placeholder="Folder name" />
      <button type="button" id="browser-create-folder" class="metal-button primary">CREATE</button>
      <button type="button" id="browser-cancel-folder" class="metal-button">CANCEL</button>
    </div>
    <div class="browser-workspace">
      <aside class="browser-roots-panel"><div class="browser-section-label">LOCATIONS</div><div id="browser-roots" class="browser-roots"></div></aside>
      <section class="browser-files-panel"><div class="browser-list-head"><span>NAME</span><span>TYPE</span></div><div id="browser-list" class="browser-list"></div></section>
    </div>
    <div id="browser-save-row" class="browser-save-row" hidden><label for="browser-project-name">PROJECT COPY NAME</label><input id="browser-project-name" spellcheck="false" /></div>
    <div id="browser-new-project-row" class="new-project-row" hidden>
      <label>PROJECT NAME<input id="new-project-name" spellcheck="false" placeholder="my-rust-project" /></label>
      <label>VERSION<input id="new-project-version" spellcheck="false" value="0.0.1" /></label>
      <div class="new-project-destination"><span>DESTINATION</span><strong id="new-project-destination">—</strong></div>
    </div>
    <div class="browser-footer">
      <div class="browser-status"><span class="lamp" id="browser-cargo-lamp"></span><span id="browser-status">Select a project folder.</span></div>
      <div class="dialog-actions compact"><button type="button" id="browser-cancel" class="metal-button">CANCEL</button><button type="button" id="browser-confirm" class="metal-button primary">OPEN PROJECT</button></div>
    </div>
  </dialog>

  <dialog id="dependency-dialog" class="oxide-dialog dependency-dialog">
    <form method="dialog" id="dependency-form">
      <div class="dialog-head">ADD DEPENDENCY</div>
      <label>CRATE NAME<input id="dep-name" required placeholder="serde" /></label>
      <label>VERSION<input id="dep-version" value="*" required /></label>
      <label>FEATURES <small>(comma separated)</small><input id="dep-features" placeholder="derive, std" /></label>
      <div class="dialog-actions"><button value="cancel" class="metal-button">CANCEL</button><button value="default" class="metal-button primary">ADD</button></div>
    </form>
  </dialog>

  <dialog id="run-dialog" class="oxide-dialog run-dialog">
    <div class="dialog-head"><span>RUN PROJECT</span><button type="button" id="run-close" class="dialog-close">×</button></div>
    <div class="run-body">
      <div class="run-question">How should Rivet run <strong id="run-project-name">this project</strong>?</div>
      <div id="run-detection" class="run-detection"></div>
      <button type="button" class="run-choice" data-run-mode="terminal"><span class="run-choice-icon">›_</span><span><strong>RUN IN RIVET TERMINAL</strong><small>For command-line programs. Supports stdin, prompts, and interactive text input.</small></span></button>
      <button type="button" class="run-choice" data-run-mode="gui"><span class="run-choice-icon">▣</span><span><strong>RUN AS GUI / NATIVE WINDOW</strong><small>For Tauri, egui, iced, winit, and other projects that create their own window.</small></span></button>
    </div>
    <div class="dialog-actions"><button type="button" id="run-cancel" class="metal-button">CANCEL</button></div>
  </dialog>

  <dialog id="tutorial-dialog" class="oxide-dialog tutorial-dialog">
    <div class="dialog-head"><div><span>INTERACTIVE RUST TUTORIAL</span><small>LEARN BY BUILDING</small></div><button type="button" id="tutorial-dialog-close" class="dialog-close">×</button></div>
    <div class="tutorial-home">
      <div class="tutorial-home-intro">
        <strong>Write code first. Read only when it helps.</strong>
        <p>Lessons use Rivet's real editor, Cargo projects, rustc diagnostics, Build Bay, and Run Terminal. There is no tutorial-only compiler hiding behind the curtain. Challenge steps accept multiple valid solutions when they demonstrate the requested concept and result.</p>
      </div>
      <div class="tutorial-course-grid">
        <section class="tutorial-course-card">
          <div class="tutorial-course-head"><span>BEGINNER</span><small id="tutorial-beginner-meta">NEW TO RUST</small></div>
          <p>Short explanations followed by real coding, deliberate mistakes, fixes, runs, and challenges.</p>
          <div id="tutorial-beginner-lessons" class="tutorial-lesson-list"></div>
        </section>
        <section class="tutorial-course-card advanced">
          <div class="tutorial-course-head"><span>ADVANCED</span><small>ROADMAP</small></div>
          <p>The advanced course shell is in place. These topics will become activity-driven lessons as the tutorial grows.</p>
          <div id="tutorial-advanced-topics" class="tutorial-topic-list"></div>
        </section>
      </div>
      <div id="tutorial-capability-summary" class="tutorial-capability-summary"></div>
    </div>
    <div class="dialog-actions tutorial-dialog-actions"><button type="button" id="tutorial-dialog-done" class="metal-button primary">CLOSE</button></div>
  </dialog>

  <dialog id="debug-target-dialog" class="oxide-dialog debug-target-dialog">
    <div class="dialog-head"><div><span>DEBUG TARGET</span><small>MULTI-BINARY CARGO PROJECT</small></div><button type="button" id="debug-target-close" class="dialog-close">×</button></div>
    <div class="debug-target-body"><p>Choose the Cargo binary Rivet should debug.</p><div id="debug-target-list" class="debug-target-list"></div></div>
    <div class="dialog-actions"><button type="button" id="debug-target-cancel" class="metal-button">CANCEL</button></div>
  </dialog>

  <dialog id="breakpoint-dialog" class="oxide-dialog breakpoint-dialog">
    <div class="dialog-head"><div><span>BREAKPOINT OPTIONS</span><small id="breakpoint-location">RUST SOURCE</small></div><button type="button" id="breakpoint-close" class="dialog-close">×</button></div>
    <form id="breakpoint-form" class="breakpoint-form">
      <label><span>CONDITION</span><input id="breakpoint-condition" spellcheck="false" placeholder="Example: score > 100"/></label>
      <label><span>HIT CONDITION</span><input id="breakpoint-hit-condition" spellcheck="false" placeholder="Example: 5"/></label>
      <label><span>LOG MESSAGE</span><input id="breakpoint-log-message" spellcheck="false" placeholder="Example: score = {score}"/></label>
      <p>A log message makes this breakpoint a logpoint: LLDB prints the message and continues instead of stopping.</p>
      <div class="dialog-actions breakpoint-actions"><button type="button" id="breakpoint-remove" class="metal-button danger">REMOVE</button><button type="button" id="breakpoint-cancel" class="metal-button">CANCEL</button><button type="submit" class="metal-button primary">SAVE</button></div>
    </form>
  </dialog>

  <dialog id="references-dialog" class="oxide-dialog intelligence-dialog">
    <div class="dialog-head"><div><span>FIND REFERENCES</span><small id="references-summary">RUST SYMBOL REFERENCES</small></div><button type="button" id="references-close" class="dialog-close">×</button></div>
    <div id="references-list" class="intelligence-list"><div class="intelligence-empty">No references loaded.</div></div>
    <div class="dialog-actions intelligence-actions"><button type="button" id="references-done" class="metal-button primary">CLOSE</button></div>
  </dialog>

  <dialog id="rename-dialog" class="oxide-dialog rename-dialog">
    <form id="rename-form">
      <div class="dialog-head"><div><span>SEMANTIC RENAME</span><small id="rename-symbol-label">RUST SYMBOL</small></div><button type="button" id="rename-close" class="dialog-close">×</button></div>
      <label><span>NEW NAME</span><input id="rename-input" spellcheck="false" autocomplete="off" /></label>
      <p class="intelligence-note">rust-analyzer will rename this symbol by meaning and scope, not by blind text replacement.</p>
      <div class="dialog-actions"><button type="button" id="rename-cancel" class="metal-button">CANCEL</button><button type="submit" class="metal-button primary">RENAME</button></div>
    </form>
  </dialog>

  <dialog id="code-actions-dialog" class="oxide-dialog intelligence-dialog">
    <div class="dialog-head"><div><span>CODE ACTIONS / QUICK FIXES</span><small>RUST-ANALYZER</small></div><button type="button" id="code-actions-close" class="dialog-close">×</button></div>
    <div id="code-actions-list" class="intelligence-list"><div class="intelligence-empty">No code actions loaded.</div></div>
    <div class="dialog-actions intelligence-actions"><button type="button" id="code-actions-done" class="metal-button">CLOSE</button></div>
  </dialog>

  <dialog id="theme-studio-dialog" class="oxide-dialog theme-studio-dialog">
    <form id="theme-studio-form">
      <div class="dialog-head"><div><span>THEME WORKSHOP</span><small>COMPOSABLE PRESENTATION</small></div><button type="button" id="theme-studio-close" class="dialog-close">×</button></div>
      <div class="theme-studio-body">
        <label class="theme-name-field"><span>Custom Theme Name</span><input id="theme-studio-name" maxlength="64" autocomplete="off" value="Custom Theme" /></label>
        <div class="theme-component-grid">
          <label class="theme-component-card"><span>MATERIAL</span><select id="theme-studio-material"></select><small id="theme-material-description"></small></label>
          <label class="theme-component-card"><span>COLOR PALETTE</span><select id="theme-studio-palette"></select><small id="theme-palette-description"></small></label>
          <label class="theme-component-card"><span>CONTROL TREATMENT</span><select id="theme-studio-controls"></select><small id="theme-controls-description"></small></label>
          <label class="theme-component-card"><span>SEMANTIC READABILITY</span><select id="theme-studio-semantic"></select><small id="theme-semantic-description"></small></label>
        </div>
        <div class="theme-recipe-readout">
          <strong>CURRENT RECIPE</strong>
          <code id="theme-recipe-readout">Oxide Iron · Oxide · Oxide Industrial · Oxide Readability</code>
          <p>Theme recipes change presentation only. Rivet's layout, panels, commands, and functionality stay identical.</p>
        </div>
        <div class="theme-readability-preview">
          <strong>SEMANTIC READABILITY PREVIEW</strong>
          <pre><span class="syntax-keyword">fn</span> <span class="syntax-function">main</span><span class="syntax-operator">() {</span>
    <span class="syntax-keyword">let</span> <span class="syntax-ident">player_name</span><span class="syntax-operator">:</span> <span class="syntax-type">String</span> <span class="syntax-operator">=</span> <span class="syntax-string">&quot;Quinn&quot;</span><span class="syntax-operator">;</span>
    <span class="syntax-keyword">let</span> <span class="syntax-ident">score</span> <span class="syntax-operator">=</span> <span class="syntax-number">42</span><span class="syntax-operator">;</span>
    <span class="syntax-macro">println!</span><span class="syntax-operator">(</span><span class="syntax-string">&quot;{}: {}&quot;</span><span class="syntax-operator">,</span> <span class="syntax-ident">player_name</span><span class="syntax-operator">,</span> <span class="syntax-ident">score</span><span class="syntax-operator">);</span> <span class="syntax-comment">// readable?</span>
<span class="syntax-operator">}</span></pre>
          <div id="theme-readability-warning" class="theme-readability-warning">Semantic palette and editor surface are compatible.</div>
        </div>
        <div id="theme-saved-wrap" class="theme-saved-wrap" hidden>
          <div class="theme-saved-head"><span>SAVED CUSTOM THEMES</span><small>SELECT ONE TO EDIT</small></div>
          <div id="theme-saved-list" class="theme-saved-list"></div>
        </div>
      </div>
      <div class="dialog-actions theme-studio-actions">
        <button type="button" id="theme-studio-delete" class="metal-button danger" hidden>DELETE</button>
        <span class="theme-action-spacer"></span>
        <button type="button" id="theme-studio-cancel" class="metal-button">CANCEL</button>
        <button type="button" id="theme-studio-preview" class="metal-button">PREVIEW</button>
        <button type="submit" class="metal-button primary">SAVE & APPLY</button>
      </div>
    </form>
  </dialog>

  <section id="terminal-window" class="terminal-window" hidden aria-label="Rivet Run Terminal">
    <header id="terminal-drag-handle" class="terminal-window-head">
      <div><span>RIVET RUN TERMINAL</span><small id="terminal-window-project">NO PROJECT</small></div>
      <div class="terminal-window-actions">
        <button type="button" id="terminal-window-stop" class="terminal-head-button stop" disabled>STOP</button>
        <button type="button" id="terminal-window-clear" class="terminal-head-button">CLEAR</button>
        <button type="button" id="terminal-window-close" class="terminal-head-button close" aria-label="Hide terminal">×</button>
      </div>
    </header>
    <div id="terminal-screen" class="terminal-screen" tabindex="0" aria-live="polite"><span class="terminal-muted">Rivet Run Terminal ready.</span></div>
    <form id="terminal-form" class="terminal-input-row">
      <span class="terminal-prompt">›</span>
      <input id="terminal-input" autocomplete="off" spellcheck="false" placeholder="Program input…" disabled />
      <button class="terminal-send" type="submit" disabled>SEND</button>
    </form>
  </section>

  <dialog id="message-dialog" class="oxide-dialog message-dialog">
    <div class="dialog-head" id="message-title">RIVET</div><div class="message-body" id="message-body"></div>
    <div class="dialog-actions message-actions"><button type="button" id="message-cancel" class="metal-button">CANCEL</button><button type="button" id="message-confirm" class="metal-button primary">OK</button></div>
  </dialog>

  <dialog id="update-dialog" class="oxide-dialog update-dialog">
    <div class="dialog-head"><div><span>RIVET UPDATE</span><small>RIVET PACKAGE UPDATE SERVICE</small></div><button type="button" id="update-close" class="dialog-close" aria-label="Close">×</button></div>
    <div class="update-body">
      <div class="update-version-row"><span id="update-current-version">CURRENT 1.3.0</span><b>→</b><strong id="update-new-version">NEW VERSION</strong></div>
      <div id="update-release-date" class="update-release-date"></div>
      <div class="update-notes-title">WHAT CHANGED</div>
      <div id="update-notes" class="update-notes">Release notes are unavailable.</div>
      <div id="update-progress-wrap" class="update-progress-wrap" hidden>
        <div class="update-progress-track"><div id="update-progress-bar" class="update-progress-bar"></div></div>
        <div id="update-progress-text" class="update-progress-text">Preparing update…</div>
      </div>
      <div id="update-error" class="update-error" hidden></div>
    </div>
    <div class="dialog-actions update-actions">
      <button type="button" id="update-later" class="metal-button">LATER</button>
      <button type="button" id="update-install" class="metal-button primary">DOWNLOAD & UPDATE</button>
    </div>
  </dialog>

  <dialog id="info-dialog" class="oxide-dialog info-dialog">
    <div class="dialog-head"><span id="info-title">RIVET</span><button type="button" id="info-close-x" class="dialog-close">×</button></div>
    <div class="info-body" id="info-body"></div><div class="dialog-actions message-actions"><button type="button" id="info-close" class="metal-button primary">CLOSE</button></div>
  </dialog>
`;

const $ = (selector) => document.querySelector(selector);
const els = {
  shell: $('.oxide-shell'),
  welcome: $('#welcome-screen'),
  workspace: $('#workspace'),
  buildConsole: $('#build-console'),
  buildResizer: $('#build-resizer'),
  tree: $('#project-tree'),
  projectName: $('#project-name'),
  editor: $('#editor'),
  syntaxLayer: $('#syntax-layer'),
  syntaxCode: $('#syntax-code'),
  debugLineHighlight: $('#debug-line-highlight'),
  bracketMatchA: $('#bracket-match-a'),
  bracketMatchB: $('#bracket-match-b'),
  lines: $('#line-numbers'),
  fileTabs: $('#file-tabs'),
  save: $('#save-file'),
  cargoInspector: $('#cargo-inspector'),
  output: $('#build-output'),
  buildStatus: $('#build-status'),
  consoleTitle: $('#console-title'),
  commandReadout: $('#command-readout'),
  menuProjectReadout: $('#menu-project-readout'),
  fileStatus: $('#file-status'),
  analysisStatus: $('#analysis-status'),
  analyzerStatus: $('#analyzer-status'),
  debuggerStatus: $('#debugger-status'),
  debuggerDetail: $('#debugger-detail'),
  debugPane: $('#debug-pane'),
  debugCallStack: $('#debug-call-stack'),
  debugBreakpoints: $('#debug-breakpoints'),
  debugVariables: $('#debug-variables'),
  debugThreadSelect: $('#debug-thread-select'),
  debugWatchForm: $('#debug-watch-form'),
  debugWatchInput: $('#debug-watch-input'),
  debugWatchList: $('#debug-watch-list'),
  debugOutput: $('#debug-output'),
  debugConsoleForm: $('#debug-console-form'),
  debugConsoleInput: $('#debug-console-input'),
  debugTargetDialog: $('#debug-target-dialog'),
  debugTargetList: $('#debug-target-list'),
  breakpointDialog: $('#breakpoint-dialog'),
  breakpointLocation: $('#breakpoint-location'),
  breakpointForm: $('#breakpoint-form'),
  breakpointCondition: $('#breakpoint-condition'),
  breakpointHitCondition: $('#breakpoint-hit-condition'),
  breakpointLogMessage: $('#breakpoint-log-message'),
  breakpointRemove: $('#breakpoint-remove'),
  referencesDialog: $('#references-dialog'),
  referencesSummary: $('#references-summary'),
  referencesList: $('#references-list'),
  renameDialog: $('#rename-dialog'),
  renameForm: $('#rename-form'),
  renameInput: $('#rename-input'),
  renameSymbolLabel: $('#rename-symbol-label'),
  codeActionsDialog: $('#code-actions-dialog'),
  codeActionsList: $('#code-actions-list'),
  themeStudioDialog: $('#theme-studio-dialog'),
  themeStudioForm: $('#theme-studio-form'),
  themeStudioName: $('#theme-studio-name'),
  themeStudioMaterial: $('#theme-studio-material'),
  themeStudioPalette: $('#theme-studio-palette'),
  themeStudioControls: $('#theme-studio-controls'),
  themeStudioSemantic: $('#theme-studio-semantic'),
  themeMaterialDescription: $('#theme-material-description'),
  themePaletteDescription: $('#theme-palette-description'),
  themeControlsDescription: $('#theme-controls-description'),
  themeSemanticDescription: $('#theme-semantic-description'),
  themeRecipeReadout: $('#theme-recipe-readout'),
  themeReadabilityWarning: $('#theme-readability-warning'),
  themeSavedWrap: $('#theme-saved-wrap'),
  themeSavedList: $('#theme-saved-list'),
  themeStudioDelete: $('#theme-studio-delete'),
  themeStudioPreview: $('#theme-studio-preview'),
  codeCompleter: $('#code-completer'),
  completionList: $('#completion-list'),
  completionDetailKind: $('#completion-detail-kind'),
  completionDetailLabel: $('#completion-detail-label'),
  completionDetailSignature: $('#completion-detail-signature'),
  completionDetailDocs: $('#completion-detail-docs'),
  signatureHelp: $('#signature-help'),
  signatureHelpLabel: $('#signature-help-label'),
  signatureHelpDocs: $('#signature-help-docs'),
  profileStatus: $('#profile-status'),
  cargoLamp: $('#cargo-lamp'),
  rustcLamp: $('#rustc-lamp'),
  cargoVersion: $('#cargo-version'),
  rustcVersion: $('#rustc-version'),
  welcomeCargoLamp: $('#welcome-cargo-lamp'),
  welcomeRustcLamp: $('#welcome-rustc-lamp'),
  welcomeCargoStatus: $('#welcome-cargo-status'),
  welcomeRustcStatus: $('#welcome-rustc-status'),
  dependencyDialog: $('#dependency-dialog'),
  dependencyForm: $('#dependency-form'),
  browserDialog: $('#file-browser-dialog'),
  browserTitle: $('#browser-title'),
  browserPath: $('#browser-path'),
  browserUp: $('#browser-up'),
  browserRoots: $('#browser-roots'),
  browserList: $('#browser-list'),
  browserSaveRow: $('#browser-save-row'),
  browserProjectName: $('#browser-project-name'),
  browserNewProjectRow: $('#browser-new-project-row'),
  newProjectName: $('#new-project-name'),
  newProjectVersion: $('#new-project-version'),
  newProjectDestination: $('#new-project-destination'),
  browserNewFolderRow: $('#browser-new-folder-row'),
  browserNewFolderName: $('#browser-new-folder-name'),
  browserStatus: $('#browser-status'),
  browserCargoLamp: $('#browser-cargo-lamp'),
  browserConfirm: $('#browser-confirm'),
  messageDialog: $('#message-dialog'),
  messageTitle: $('#message-title'),
  messageBody: $('#message-body'),
  messageCancel: $('#message-cancel'),
  messageConfirm: $('#message-confirm'),
  updateDialog: $('#update-dialog'),
  updateClose: $('#update-close'),
  updateCurrentVersion: $('#update-current-version'),
  updateNewVersion: $('#update-new-version'),
  updateReleaseDate: $('#update-release-date'),
  updateNotes: $('#update-notes'),
  updateProgressWrap: $('#update-progress-wrap'),
  updateProgressBar: $('#update-progress-bar'),
  updateProgressText: $('#update-progress-text'),
  updateError: $('#update-error'),
  updateLater: $('#update-later'),
  updateInstall: $('#update-install'),
  infoDialog: $('#info-dialog'),
  infoTitle: $('#info-title'),
  infoBody: $('#info-body'),
  diagnosticBanner: $('#diagnostic-banner'),
  diagnosticBannerLevel: $('#diagnostic-banner-level'),
  diagnosticBannerText: $('#diagnostic-banner-text'),
  problemsList: $('#problems-list'),
  problemCount: $('#problem-count'),
  buildModeTabs: $('#build-mode-tabs'),
  terminalWindow: $('#terminal-window'),
  terminalWindowProject: $('#terminal-window-project'),
  terminalDragHandle: $('#terminal-drag-handle'),
  terminalScreen: $('#terminal-screen'),
  terminalForm: $('#terminal-form'),
  terminalInput: $('#terminal-input'),
  stopTerminal: $('#terminal-window-stop'),
  runDialog: $('#run-dialog'),
  runProjectName: $('#run-project-name'),
  runDetection: $('#run-detection'),
  tutorialDialog: $('#tutorial-dialog'),
  tutorialBeginnerLessons: $('#tutorial-beginner-lessons'),
  tutorialBeginnerMeta: $('#tutorial-beginner-meta'),
  tutorialAdvancedTopics: $('#tutorial-advanced-topics'),
  tutorialCapabilitySummary: $('#tutorial-capability-summary'),
  tutorialPanel: $('#tutorial-panel'),
  tutorialCourseLabel: $('#tutorial-course-label'),
  tutorialLessonTitle: $('#tutorial-lesson-title'),
  tutorialStepCounter: $('#tutorial-step-counter'),
  tutorialStepTitle: $('#tutorial-step-title'),
  tutorialExplanation: $('#tutorial-explanation'),
  tutorialExample: $('#tutorial-example'),
  tutorialExampleCode: $('#tutorial-example-code'),
  tutorialExampleParts: $('#tutorial-example-parts'),
  tutorialObjective: $('#tutorial-objective'),
  tutorialFeedback: $('#tutorial-feedback'),
  tutorialExperimentNote: $('#tutorial-experiment-note'),
  tutorialLearnMore: $('#tutorial-learn-more'),
  tutorialLearnMoreText: $('#tutorial-learn-more-text'),
  tutorialNext: $('#tutorial-next'),
  tutorialReturn: $('#tutorial-return'),
  tutorialHomeButton: $('#tutorial-home-button'),
};

function escapeHtml(value = '') {
  return String(value)
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&#039;');
}

function pathBase(path = '') {
  return path.split(/[\\/]/).filter(Boolean).at(-1) || path;
}

function pathParent(path = '') {
  const trimmed = path.replace(/[\\/]+$/, '');
  const index = Math.max(trimmed.lastIndexOf('\\'), trimmed.lastIndexOf('/'));
  if (index < 0) return trimmed;
  if (/^[A-Za-z]:$/.test(trimmed.slice(0, index))) return `${trimmed.slice(0, index)}\\`;
  return trimmed.slice(0, index) || '/';
}

function normalizePath(path = '') {
  const normalized = path.replaceAll('\\', '/').replace(/\/$/, '');
  return state.platform.pathCaseSensitive ? normalized : normalized.toLowerCase();
}

async function detectPlatform() {
  try {
    const info = await invoke('platform_info');
    state.platform = info;
    document.documentElement.dataset.oxideOs = info.os || 'unknown';
  } catch (error) {
    console.warn('Could not detect Rivet platform information:', error);
  }
}

function setLamp(element, ok) {
  element.classList.toggle('ok', Boolean(ok));
  element.classList.toggle('bad', !ok);
}

function closeMenus() {
  document.querySelectorAll('.menu-popup.open').forEach((popup) => popup.classList.remove('open'));
  document.querySelectorAll('.menu-trigger.active').forEach((button) => button.classList.remove('active'));
}

const THEME_PALETTE_OVERRIDE_MAP = Object.freeze({
  text: '--t-text',
  muted: '--t-muted',
  faint: '--t-faint',
  border: '--t-border',
  accent: '--t-accent',
  accentHot: '--t-accent-hot',
  panel: '--t-panel',
  editor: '--t-editor',
  input: '--t-input',
  caret: '--t-caret',
});
const THEME_SEMANTIC_OVERRIDE_MAP = Object.freeze({
  default: '--syntax-default',
  keyword: '--syntax-keyword',
  identifier: '--syntax-ident',
  string: '--syntax-string',
  number: '--syntax-number',
  type: '--syntax-type',
  macro: '--syntax-macro',
  function: '--syntax-function',
  comment: '--syntax-comment',
  operator: '--syntax-operator',
});
const THEME_OVERRIDE_VARIABLES = Object.freeze([
  ...Object.values(THEME_PALETTE_OVERRIDE_MAP),
  ...Object.values(THEME_SEMANTIC_OVERRIDE_MAP),
]);

function applyThemeOverrides(definition) {
  const rootStyle = document.documentElement.style;
  THEME_OVERRIDE_VARIABLES.forEach((property) => rootStyle.removeProperty(property));
  const palette = definition?.overrides?.palette || {};
  const semantic = definition?.overrides?.semantic || {};
  Object.entries(THEME_PALETTE_OVERRIDE_MAP).forEach(([key, property]) => {
    if (palette[key]) rootStyle.setProperty(property, String(palette[key]));
  });
  Object.entries(THEME_SEMANTIC_OVERRIDE_MAP).forEach(([key, property]) => {
    if (semantic[key]) rootStyle.setProperty(property, String(semantic[key]));
  });
}

function themeDefinition(themeId = state.theme) {
  return resolveTheme(themeId, state.customThemes);
}

function themeDisplayLabel(themeId = state.theme) {
  const definition = themeDefinition(themeId);
  return definition?.menuLabel || definition?.name?.toUpperCase() || 'OXIDE';
}

function applyTheme(themeId, { persist = true } = {}) {
  const definition = themeDefinition(themeId);
  const recipe = normalizeThemeRecipe(definition.recipe);
  state.theme = definition.id;
  state.themeRecipe = recipe;

  // data-theme="composed" deliberately keeps every built-in and custom theme
  // on the same presentation path. Component attributes are the stable Theme
  // Engine API; no theme is allowed to change workspace geometry or behavior.
  const root = document.documentElement;
  root.dataset.theme = 'composed';
  root.dataset.themeId = definition.id;
  root.dataset.themeMaterial = recipe.material;
  root.dataset.themePalette = recipe.palette;
  root.dataset.themeControls = recipe.controls;
  root.dataset.themeSemantic = recipe.semantic;
  applyThemeOverrides(definition);

  const current = document.querySelector('#theme-menu-current');
  if (current) current.textContent = themeDisplayLabel(definition.id);

  document.querySelectorAll('[data-theme-check]').forEach((check) => {
    check.textContent = check.dataset.themeCheck === definition.id ? '✓' : '';
  });

  if (persist) persistTheme(definition.id);

  // Semantic tokens represent meaning, not color. The CSS palette bound to
  // the active recipe changes instantly without asking rust-analyzer to
  // recompute token classifications. Presentation refresh must remain
  // non-fatal: a theme/highlighter problem should never prevent the rest of
  // Rivet's startup handlers from being registered.
  try {
    updateSyntaxHighlight();
  } catch (error) {
    console.warn('Could not refresh syntax presentation after theme change:', error);
  }
}

function componentOptions(group, selected) {
  return Object.entries(THEME_COMPONENTS[group]).map(([id, definition]) =>
    `<option value="${escapeHtml(id)}" ${id === selected ? 'selected' : ''}>${escapeHtml(definition.label)}</option>`
  ).join('');
}

function recipeFromThemeStudio() {
  return normalizeThemeRecipe({
    material: els.themeStudioMaterial.value,
    palette: els.themeStudioPalette.value,
    controls: els.themeStudioControls.value,
    semantic: els.themeStudioSemantic.value,
  });
}

function updateThemeStudioSummary() {
  const recipe = recipeFromThemeStudio();
  const material = THEME_COMPONENTS.materials[recipe.material];
  const palette = THEME_COMPONENTS.palettes[recipe.palette];
  const controls = THEME_COMPONENTS.controls[recipe.controls];
  const semantic = THEME_COMPONENTS.semantic[recipe.semantic];
  els.themeMaterialDescription.textContent = material.description;
  els.themePaletteDescription.textContent = palette.description;
  els.themeControlsDescription.textContent = controls.description;
  els.themeSemanticDescription.textContent = semantic.description;
  els.themeRecipeReadout.textContent = `${material.label} · ${palette.label} · ${controls.label} · ${semantic.label}`;
  const paletteTone = palette.tone || 'dark';
  const semanticTone = semantic.tone || 'dark';
  const compatible = paletteTone === semanticTone;
  if (els.themeReadabilityWarning) {
    els.themeReadabilityWarning.classList.toggle('warn', !compatible);
    els.themeReadabilityWarning.textContent = compatible
      ? 'Semantic palette and editor surface are contrast-matched.'
      : `READABILITY WARNING: ${semantic.label} is designed for ${semanticTone} editor surfaces, while ${palette.label} uses a ${paletteTone} editor surface. Preview before saving.`;
  }
}

function renderSavedThemeList() {
  const themes = state.customThemes;
  els.themeSavedWrap.hidden = themes.length === 0;
  if (!themes.length) {
    els.themeSavedList.innerHTML = '';
    return;
  }
  els.themeSavedList.innerHTML = themes.map((theme) => {
    const recipe = normalizeThemeRecipe(theme.recipe);
    return `<button type="button" class="theme-saved-item ${theme.id === state.themeStudioEditingId ? 'active' : ''}" data-edit-theme="${escapeHtml(theme.id)}">
      <strong>${escapeHtml(theme.name)}</strong>
      <small>${escapeHtml(THEME_COMPONENTS.materials[recipe.material].label)} · ${escapeHtml(THEME_COMPONENTS.palettes[recipe.palette].label)} · ${escapeHtml(THEME_COMPONENTS.semantic[recipe.semantic].label)}</small>
    </button>`;
  }).join('');
  els.themeSavedList.querySelectorAll('[data-edit-theme]').forEach((button) => {
    button.addEventListener('click', () => fillThemeStudio(button.dataset.editTheme));
  });
}

function renderCustomThemeMenu() {
  const section = document.querySelector('#custom-theme-menu-section');
  const host = document.querySelector('#custom-theme-menu-items');
  if (!section || !host) return;
  section.hidden = state.customThemes.length === 0;
  host.innerHTML = state.customThemes.map((theme) =>
    `<button role="menuitem" data-menu-action="theme-${escapeHtml(theme.id)}"><span><i class="menu-check" data-theme-check="${escapeHtml(theme.id)}"></i> ${escapeHtml(theme.name)}</span></button>`
  ).join('');
  document.querySelectorAll('[data-theme-check]').forEach((check) => {
    check.textContent = check.dataset.themeCheck === state.theme ? '✓' : '';
  });
}

function fillThemeStudio(themeId = '') {
  const editing = state.customThemes.find((theme) => theme.id === themeId) || null;
  const source = editing || themeDefinition(state.theme);
  const recipe = normalizeThemeRecipe(source.recipe);
  state.themeStudioEditingId = editing?.id || '';
  els.themeStudioName.value = editing?.name || 'Custom Theme';
  els.themeStudioMaterial.innerHTML = componentOptions('materials', recipe.material);
  els.themeStudioPalette.innerHTML = componentOptions('palettes', recipe.palette);
  els.themeStudioControls.innerHTML = componentOptions('controls', recipe.controls);
  els.themeStudioSemantic.innerHTML = componentOptions('semantic', recipe.semantic);
  els.themeStudioDelete.hidden = !editing;
  updateThemeStudioSummary();
  renderSavedThemeList();
}

function previewThemeStudio() {
  const recipe = recipeFromThemeStudio();
  const editing = state.customThemes.find((theme) => theme.id === state.themeStudioEditingId);
  const root = document.documentElement;
  root.dataset.theme = 'composed';
  root.dataset.themeId = 'preview';
  root.dataset.themeMaterial = recipe.material;
  root.dataset.themePalette = recipe.palette;
  root.dataset.themeControls = recipe.controls;
  root.dataset.themeSemantic = recipe.semantic;
  applyThemeOverrides(editing || { overrides: {} });
  state.themeStudioPreviewing = true;
  try { updateSyntaxHighlight(); } catch (error) { console.warn('Could not preview theme recipe:', error); }
}

function restoreThemeAfterStudioPreview() {
  if (!state.themeStudioPreviewing) return;
  state.themeStudioPreviewing = false;
  applyTheme(state.theme, { persist: false });
}

function openThemeStudio() {
  const editingId = state.theme.startsWith('custom:') ? state.theme : '';
  state.themeStudioPreviewing = false;
  fillThemeStudio(editingId);
  els.themeStudioDialog.showModal();
  requestAnimationFrame(() => els.themeStudioName.focus());
}

function saveThemeStudio() {
  const name = els.themeStudioName.value.trim() || 'Custom Theme';
  const recipe = recipeFromThemeStudio();
  const existingIndex = state.customThemes.findIndex((theme) => theme.id === state.themeStudioEditingId);
  let saved;
  if (existingIndex >= 0) {
    saved = {
      ...state.customThemes[existingIndex],
      name,
      recipe,
    };
    state.customThemes.splice(existingIndex, 1, saved);
  } else {
    saved = createCustomTheme({ name, recipe });
    state.customThemes.push(saved);
  }
  state.customThemes = saveCustomThemes(state.customThemes);
  customThemes = state.customThemes;
  state.themeStudioEditingId = saved.id;
  state.themeStudioPreviewing = false;
  renderCustomThemeMenu();
  applyTheme(saved.id);
  els.themeStudioDialog.close();
}

function deleteThemeStudioTheme() {
  const id = state.themeStudioEditingId;
  if (!id) return;
  state.customThemes = saveCustomThemes(state.customThemes.filter((theme) => theme.id !== id));
  customThemes = state.customThemes;
  state.themeStudioEditingId = '';
  renderCustomThemeMenu();
  if (state.theme === id) applyTheme('oxide');
  fillThemeStudio('');
}

const BUILD_BAY_HEIGHT_KEY = 'oxide.layout.buildBayHeight';
const BUILD_BAY_MIN_HEIGHT = 96;

function buildBayHeightLimits() {
  const viewport = Math.max(520, window.innerHeight || 0);
  const max = Math.max(BUILD_BAY_MIN_HEIGHT, Math.min(Math.round(viewport * 0.58), viewport - 350));
  return { min: BUILD_BAY_MIN_HEIGHT, max };
}

function currentBuildBayHeight() {
  const measured = els.buildConsole.getBoundingClientRect().height;
  if (measured > 0) return measured;
  const cssValue = Number.parseFloat(getComputedStyle(els.shell).getPropertyValue('--build-height'));
  return Number.isFinite(cssValue) ? cssValue : 180;
}

function updateBuildBayResizeAria(height = currentBuildBayHeight()) {
  const { min, max } = buildBayHeightLimits();
  els.buildResizer.setAttribute('aria-valuemin', String(min));
  els.buildResizer.setAttribute('aria-valuemax', String(max));
  els.buildResizer.setAttribute('aria-valuenow', String(Math.round(Math.min(max, Math.max(min, height)))));
}

function setBuildBayHeight(height, { persist = true } = {}) {
  const { min, max } = buildBayHeightLimits();
  const clamped = Math.round(Math.min(max, Math.max(min, Number(height) || min)));
  els.shell.style.setProperty('--build-height-user', `${clamped}px`);
  updateBuildBayResizeAria(clamped);
  if (persist) localStorage.setItem(BUILD_BAY_HEIGHT_KEY, String(clamped));
  return clamped;
}

function restoreBuildBayHeight() {
  const saved = Number(localStorage.getItem(BUILD_BAY_HEIGHT_KEY));
  if (Number.isFinite(saved) && saved >= BUILD_BAY_MIN_HEIGHT) {
    setBuildBayHeight(saved, { persist: false });
  } else {
    els.shell.style.removeProperty('--build-height-user');
    requestAnimationFrame(() => updateBuildBayResizeAria());
  }
}

function resetBuildBayHeight() {
  localStorage.removeItem(BUILD_BAY_HEIGHT_KEY);
  els.shell.style.removeProperty('--build-height-user');
  requestAnimationFrame(() => updateBuildBayResizeAria());
}

function setupBuildBayResize() {
  let drag = null;

  els.buildResizer.addEventListener('pointerdown', (event) => {
    if (event.button !== 0) return;
    const startHeight = currentBuildBayHeight();
    drag = { startY: event.clientY, startHeight, lastHeight: startHeight };
    els.buildResizer.setPointerCapture?.(event.pointerId);
    els.shell.classList.add('build-bay-resizing');
    document.body.classList.add('build-bay-resizing');
    event.preventDefault();
  });

  els.buildResizer.addEventListener('pointermove', (event) => {
    if (!drag) return;
    drag.lastHeight = setBuildBayHeight(drag.startHeight + (drag.startY - event.clientY), { persist: false });
  });

  const finishDrag = (event) => {
    if (!drag) return;
    const finalHeight = drag.lastHeight;
    drag = null;
    try { els.buildResizer.releasePointerCapture?.(event.pointerId); } catch { /* pointer may already be released */ }
    els.shell.classList.remove('build-bay-resizing');
    document.body.classList.remove('build-bay-resizing');
    setBuildBayHeight(finalHeight, { persist: true });
  };
  els.buildResizer.addEventListener('pointerup', finishDrag);
  els.buildResizer.addEventListener('pointercancel', finishDrag);

  els.buildResizer.addEventListener('dblclick', resetBuildBayHeight);
  els.buildResizer.addEventListener('keydown', (event) => {
    const step = event.shiftKey ? 40 : 16;
    if (event.key === 'ArrowUp') {
      setBuildBayHeight(currentBuildBayHeight() + step);
      event.preventDefault();
    } else if (event.key === 'ArrowDown') {
      setBuildBayHeight(currentBuildBayHeight() - step);
      event.preventDefault();
    } else if (event.key === 'Home') {
      resetBuildBayHeight();
      event.preventDefault();
    }
  });

  window.addEventListener('resize', () => {
    if (!els.shell.style.getPropertyValue('--build-height-user')) {
      updateBuildBayResizeAria();
      return;
    }
    setBuildBayHeight(currentBuildBayHeight(), { persist: false });
  });
}

function setProjectUiState() {
  const loaded = Boolean(state.projectPath);
  els.shell.classList.toggle('welcome-mode', !loaded);
  els.welcome.hidden = loaded;
  els.workspace.hidden = !loaded;
  els.buildConsole.hidden = !loaded || !state.view.build;
  if (!loaded) {
    els.tutorialPanel.hidden = true;
    els.shell.classList.remove('tutorial-active');
  }
  if (!loaded) {
    els.commandReadout.textContent = 'SELECT A PROJECT TO BEGIN';
    els.menuProjectReadout.textContent = 'NO PROJECT';
    els.fileStatus.textContent = 'NO FILE';
    els.analysisStatus.textContent = 'RUST CHECK: IDLE';
  }
  updateMenuAvailability();
}

function updateMenuAvailability() {
  const projectLoaded = Boolean(state.projectPath);
  const fileLoaded = Boolean(state.currentFile);
  const cargoBusy = state.buildRunning || state.terminalRunning || state.debugger.running;

  document.querySelectorAll('[data-menu-action="save-file"], [data-menu-action="close-file"]').forEach((button) => {
    button.disabled = !fileLoaded;
  });
  document.querySelectorAll('[data-menu-action="save-project-as"], [data-menu-action="close-project"]').forEach((button) => {
    button.disabled = !projectLoaded;
  });
  document.querySelectorAll('[data-menu-action="check"], [data-menu-action="build"], [data-menu-action="run"], [data-menu-action="test"], [data-menu-action="clean"], [data-menu-action="analyze-now"]').forEach((button) => {
    button.disabled = !projectLoaded || cargoBusy;
  });
  document.querySelectorAll('[data-menu-action="show-terminal"]').forEach((button) => {
    button.disabled = !projectLoaded;
  });
  document.querySelectorAll('[data-menu-action="add-dependency"]').forEach((button) => {
    button.disabled = !projectLoaded || cargoBusy;
  });
  document.querySelectorAll('.command-button').forEach((button) => {
    button.disabled = !projectLoaded || cargoBusy;
  });
  document.querySelectorAll('[data-check="live-check"]').forEach((check) => {
    check.textContent = state.liveCheck ? '✓' : '';
  });
  document.querySelectorAll('[data-check="completer"]').forEach((check) => {
    check.textContent = state.completer.enabled ? '✓' : '';
  });
  document.querySelectorAll('[data-theme-check]').forEach((check) => {
    check.textContent = check.dataset.themeCheck === state.theme ? '✓' : '';
  });
  const themeReadout = document.querySelector('#theme-menu-current');
  if (themeReadout) themeReadout.textContent = themeDisplayLabel(state.theme);
  document.querySelectorAll('[data-menu-action="trigger-completion"]').forEach((button) => {
    button.disabled = !projectLoaded || !fileLoaded || !state.completer.available || !state.completer.enabled;
  });
  document.querySelectorAll('[data-menu-action="go-definition"], [data-menu-action="find-references"], [data-menu-action="rename-symbol"], [data-menu-action="code-actions"]').forEach((button) => {
    button.disabled = !projectLoaded || !fileLoaded || !state.completer.available || !state.currentFile?.toLowerCase().endsWith('.rs');
  });
  document.querySelectorAll('[data-menu-action="debug-start"], [data-debug-action="start"]').forEach((button) => {
    button.disabled = !projectLoaded || state.buildRunning || state.terminalRunning || state.debugger.running;
  });
  document.querySelectorAll('[data-menu-action="debug-continue"], [data-debug-action="continue"]').forEach((button) => {
    button.disabled = !state.debugger.running || !state.debugger.stopped;
  });
  document.querySelectorAll('[data-menu-action="debug-pause"], [data-debug-action="pause"]').forEach((button) => {
    button.disabled = !state.debugger.running || state.debugger.stopped;
  });
  document.querySelectorAll('[data-menu-action="debug-restart"], [data-debug-action="restart"]').forEach((button) => {
    button.disabled = !state.debugger.running;
  });
  document.querySelectorAll('[data-menu-action="debug-next"], [data-menu-action="debug-step-in"], [data-menu-action="debug-step-out"], [data-debug-action="next"], [data-debug-action="step-in"], [data-debug-action="step-out"]').forEach((button) => {
    button.disabled = !state.debugger.running || !state.debugger.stopped;
  });
  document.querySelectorAll('[data-menu-action="debug-stop"], [data-debug-action="stop"]').forEach((button) => {
    button.disabled = !state.debugger.running;
  });
  document.querySelectorAll('[data-menu-action="show-debug"]').forEach((button) => { button.disabled = !projectLoaded; });
  if (els.debugConsoleInput) els.debugConsoleInput.disabled = !state.debugger.running || !state.debugger.stopped;
  if (els.debugConsoleForm) els.debugConsoleForm.querySelector('button').disabled = !state.debugger.running || !state.debugger.stopped;
  renderDebugThreads();
}

async function detectToolchain() {
  try {
    const info = await invoke('toolchain_info');
    els.cargoVersion.textContent = info.cargo || 'Cargo: not found';
    els.rustcVersion.textContent = info.rustc || 'rustc: not found';
    els.welcomeCargoStatus.textContent = info.cargo || 'Cargo not found';
    els.welcomeRustcStatus.textContent = info.rustc || 'rustc not found';
    setLamp(els.cargoLamp, info.cargo_found);
    setLamp(els.rustcLamp, info.rustc_found);
    setLamp(els.welcomeCargoLamp, info.cargo_found);
    setLamp(els.welcomeRustcLamp, info.rustc_found);
    try {
      const analyzer = await invoke('rust_analyzer_status');
      state.completer.available = Boolean(analyzer.available);
      els.analyzerStatus.textContent = analyzer.available ? 'ANALYZER: READY' : 'ANALYZER: NOT FOUND';
      els.analyzerStatus.title = analyzer.available ? analyzer.version : analyzer.message;
    } catch (analyzerError) {
      state.completer.available = false;
      els.analyzerStatus.textContent = 'ANALYZER: ERROR';
      els.analyzerStatus.title = String(analyzerError);
    }
    try {
      const debuggerInfo = await invoke('debugger_status');
      state.debugger.available = Boolean(debuggerInfo.available);
      state.debugger.adapter = debuggerInfo.adapter || '';
      state.debugger.message = debuggerInfo.message || '';
      els.debuggerStatus.textContent = debuggerInfo.available ? 'DEBUGGER: READY' : 'DEBUGGER: NOT FOUND';
      els.debuggerStatus.title = debuggerInfo.available ? `${debuggerInfo.version} · ${debuggerInfo.path}` : debuggerInfo.message;
      els.debuggerDetail.textContent = debuggerInfo.available ? `${debuggerInfo.adapter} READY` : 'LLDB DAP NOT FOUND';
      els.debuggerDetail.title = debuggerInfo.message || '';
    } catch (debuggerError) {
      state.debugger.available = false;
      state.debugger.message = String(debuggerError);
      els.debuggerStatus.textContent = 'DEBUGGER: ERROR';
      els.debuggerStatus.title = String(debuggerError);
      els.debuggerDetail.textContent = 'DEBUGGER CHECK FAILED';
    }
    updateMenuAvailability();
  } catch (error) {
    setLamp(els.cargoLamp, false);
    setLamp(els.rustcLamp, false);
    setLamp(els.welcomeCargoLamp, false);
    setLamp(els.welcomeRustcLamp, false);
    els.welcomeCargoStatus.textContent = 'Toolchain check failed';
    els.welcomeRustcStatus.textContent = String(error);
    appendFriendly('error', `Toolchain check failed: ${error}`);
  }
}

function fileBadge(name) {
  if (name.endsWith('.rs')) return '<b class="badge rs">Rs</b>';
  if (name.endsWith('.toml')) return '<b class="badge toml">Tm</b>';
  if (name.endsWith('.lock')) return '<b class="badge lock">Lk</b>';
  return '<b class="badge">•</b>';
}

async function warmRustAnalyzer(projectPath = state.projectPath) {
  if (!projectPath || !state.completer.enabled || !state.completer.available) return;
  els.analyzerStatus.textContent = 'ANALYZER: STARTING';
  try {
    await invoke('rust_analyzer_warmup', { projectPath });
    if (state.projectPath === projectPath) {
      els.analyzerStatus.textContent = 'ANALYZER: READY';
      els.analyzerStatus.title = 'Rust Code Analyzer/Completer is connected. Semantic Readability Colors use rust-analyzer when available.';
      scheduleSemanticReadability(40);
    }
  } catch (error) {
    els.analyzerStatus.textContent = 'ANALYZER: RETRY';
    els.analyzerStatus.title = String(error);
    updateMenuAvailability();
  }
}

function renderTree(entries) {
  if (!entries.length) {
    els.tree.innerHTML = '<div class="empty-state">Project is empty.</div>';
    return;
  }
  els.tree.innerHTML = entries.map((entry) => `
    <button class="${entry.kind === 'file' ? 'tree-entry file' : 'tree-entry folder'}" data-path="${escapeHtml(entry.path)}" style="--depth:${entry.depth}">
      <span class="tree-icon">${entry.kind === 'file' ? fileBadge(entry.name) : '▾'}</span><span>${escapeHtml(entry.name)}</span>
    </button>`).join('');
  els.tree.querySelectorAll('.tree-entry.file').forEach((button) => button.addEventListener('click', () => loadFile(button.dataset.path)));
  updateTreeFileStates();
}

function updateTreeFileStates() {
  const openPaths = new Set(state.tabs.map((tab) => normalizePath(tab.path)));
  els.tree.querySelectorAll('.tree-entry.file').forEach((button) => {
    const normalized = normalizePath(button.dataset.path);
    button.classList.toggle('open-file', openPaths.has(normalized));
    button.classList.toggle('active-file', normalized === normalizePath(state.activeTabPath));
  });
}

async function openProjectPath(projectPath, { keepBrowserOpen = false, created = false, preserveTutorial = false } = {}) {
  if (!projectPath) return false;
  if (state.buildRunning) {
    showInfo('CARGO IS BUSY', '<p class="info-error">Wait for the current Cargo operation to finish before switching projects.</p>');
    return false;
  }
  if (hasDirtyTabs() && !await oxideConfirm('UNSAVED FILES', `Discard unsaved changes in ${dirtyTabCount()} open file${dirtyTabCount() === 1 ? '' : 's'} before opening another project?`, 'DISCARD')) return false;
  if (state.terminalRunning) {
    if (!await oxideConfirm('PROGRAM RUNNING', 'A program is still running in the Rivet Run Terminal. Stop it before switching projects?', 'STOP & OPEN')) return false;
    try { await invoke('terminal_stop'); } catch { /* process may have just exited */ }
    hideTerminalWindow();
  }
  if (state.debugger.running) {
    if (!await oxideConfirm('DEBUG SESSION ACTIVE', 'A program is currently being debugged. Stop debugging before switching projects?', 'STOP & OPEN')) return false;
    await stopDebugging();
  }
  if (state.tutorial.active && !preserveTutorial) exitTutorialMode();

  els.commandReadout.textContent = 'LOADING PROJECT…';
  try {
    const [entries, manifest] = await Promise.all([
      invoke('list_project_files', { projectPath }),
      invoke('manifest_view', { projectPath }),
    ]);

    state.projectPath = projectPath;
    state.manifest = manifest;
    state.diagnostics = [];
    state.debugger.breakpoints.clear();
    state.debugger.selectedTarget = null;
    state.debugger.threads = [];
    state.debugger.expandedVariables.clear();
    renderBreakpoints();
    renderDebugThreads();
    clearSemanticReadability();
    state.analysisGeneration += 1;
    clearTabs();
    renderTree(entries);
    renderManifest(manifest);
    els.projectName.textContent = pathBase(projectPath).toUpperCase();
    els.menuProjectReadout.textContent = manifest.package_name.toUpperCase();
    els.menuProjectReadout.title = projectPath;
    els.commandReadout.textContent = `READY · ${manifest.package_name}`;
    els.buildStatus.textContent = 'PROJECT READY';

    clearOutput();
    appendFriendly('success', `${created ? '✓ Project created' : '✓ Project opened'} · ${manifest.package_name}`);
    appendFriendly('stage', `Cargo manifest ready · v${manifest.version} · edition ${manifest.edition}`);
    appendFriendly('muted', projectPath);

    const main = entries.find((entry) => entry.kind === 'file' && /[\\/]src[\\/]main\.rs$/i.test(entry.path));
    const lib = entries.find((entry) => entry.kind === 'file' && /[\\/]src[\\/]lib\.rs$/i.test(entry.path));
    if (main || lib) await loadFile((main || lib).path);

    renderProblems();
    setProjectUiState();
    warmRustAnalyzer(projectPath);
    if (!keepBrowserOpen && els.browserDialog.open) els.browserDialog.close();
    if (state.liveCheck) scheduleAnalysis(350);
    return true;
  } catch (error) {
    els.commandReadout.textContent = 'PROJECT LOAD FAILED';
    if (els.browserDialog.open) showBrowserError(String(error));
    else appendFriendly('error', String(error));
    return false;
  }
}

function activeTab() {
  return state.tabs.find((tab) => normalizePath(tab.path) === normalizePath(state.activeTabPath)) || null;
}

function dirtyTabCount() {
  syncActiveTabFromEditor();
  return state.tabs.filter((tab) => tab.dirty).length;
}

function hasDirtyTabs() {
  return dirtyTabCount() > 0;
}

function tabBadge(name) {
  if (name.toLowerCase().endsWith('.rs')) return 'Rs';
  if (name.toLowerCase().endsWith('.toml')) return 'Tm';
  if (name.toLowerCase().endsWith('.lock')) return 'Lk';
  return 'Tx';
}

function syncActiveTabFromEditor() {
  const tab = activeTab();
  if (!tab) return;
  tab.content = els.editor.value;
  tab.scrollTop = els.editor.scrollTop;
  tab.selectionStart = els.editor.selectionStart;
  tab.selectionEnd = els.editor.selectionEnd;
  tab.dirty = state.dirty;
}

function renderTabs() {
  if (!state.tabs.length) {
    els.fileTabs.innerHTML = '<div class="tab-empty">NO FILE OPEN</div>';
    updateTreeFileStates();
    return;
  }

  els.fileTabs.innerHTML = state.tabs.map((tab) => `
    <div class="file-tab ${normalizePath(tab.path) === normalizePath(state.activeTabPath) ? 'active' : ''} ${tab.dirty ? 'dirty' : ''}" role="tab" tabindex="0" aria-selected="${normalizePath(tab.path) === normalizePath(state.activeTabPath)}" data-tab-path="${escapeHtml(tab.path)}" title="${escapeHtml(tab.path)}">
      <span class="tab-type">${tabBadge(tab.name)}</span>
      <span class="tab-label">${escapeHtml(tab.name)}</span>
      <span class="tab-dirty" aria-label="${tab.dirty ? 'Unsaved changes' : ''}">${tab.dirty ? '●' : ''}</span>
      <button type="button" class="tab-close" aria-label="Close ${escapeHtml(tab.name)}" title="Close ${escapeHtml(tab.name)}">×</button>
    </div>`).join('');

  els.fileTabs.querySelectorAll('.file-tab').forEach((tabElement) => {
    const path = tabElement.dataset.tabPath;
    tabElement.addEventListener('click', (event) => {
      if (!event.target.closest('.tab-close')) activateTab(path);
    });
    tabElement.addEventListener('keydown', (event) => {
      if (event.key === 'Enter' || event.key === ' ') {
        event.preventDefault();
        activateTab(path);
      }
    });
    tabElement.addEventListener('auxclick', (event) => {
      if (event.button === 1) closeTab(path);
    });
    tabElement.querySelector('.tab-close').addEventListener('click', (event) => {
      event.stopPropagation();
      closeTab(path);
    });
  });

  updateTreeFileStates();
  requestAnimationFrame(() => {
    els.fileTabs.querySelector('.file-tab.active')?.scrollIntoView({ block: 'nearest', inline: 'nearest' });
  });
}

function updateActiveTabVisual() {
  const tab = activeTab();
  els.fileTabs.querySelectorAll('.file-tab').forEach((element) => {
    if (normalizePath(element.dataset.tabPath) !== normalizePath(state.activeTabPath)) return;
    element.classList.toggle('dirty', Boolean(tab?.dirty));
    const indicator = element.querySelector('.tab-dirty');
    if (indicator) {
      indicator.textContent = tab?.dirty ? '●' : '';
      indicator.setAttribute('aria-label', tab?.dirty ? 'Unsaved changes' : '');
    }
  });
}

function setEditorFromTab(tab) {
  closeCompletionUi();
  state.completer.dismissedThroughWord = false;
  if (!tab) {
    state.activeTabPath = '';
    state.currentFile = '';
    state.dirty = false;
    els.editor.value = '';
    clearSemanticReadability();
    updateSyntaxHighlight();
    els.editor.readOnly = true;
    els.editor.placeholder = 'Open a .rs, .toml, or text file from the project tree.';
    els.fileStatus.textContent = state.projectPath ? 'NO FILE OPEN' : 'NO FILE';
    els.save.disabled = true;
    renderTabs();
    updateLineNumbers();
    updateDiagnosticBanner();
    updateBracketMatch();
    updateMenuAvailability();
    return;
  }

  state.activeTabPath = tab.path;
  state.currentFile = tab.path;
  state.dirty = tab.dirty;
  els.editor.readOnly = false;
  els.editor.value = tab.content;
  clearSemanticReadability();
  updateSyntaxHighlight();
  scheduleSemanticReadability(80);
  els.fileStatus.textContent = tab.path;
  els.save.disabled = false;
  renderTabs();
  updateLineNumbers();
  updateDiagnosticBanner();
  requestAnimationFrame(() => {
    els.editor.scrollTop = tab.scrollTop || 0;
    const max = els.editor.value.length;
    els.editor.setSelectionRange(Math.min(tab.selectionStart || 0, max), Math.min(tab.selectionEnd || 0, max));
    els.lines.scrollTop = els.editor.scrollTop;
    updateBracketMatch();
  });
  updateMenuAvailability();
}

function activateTab(path) {
  if (!path || normalizePath(path) === normalizePath(state.activeTabPath)) return;
  syncActiveTabFromEditor();
  const tab = state.tabs.find((item) => normalizePath(item.path) === normalizePath(path));
  if (tab) setEditorFromTab(tab);
}

async function loadFile(path) {
  const existing = state.tabs.find((tab) => normalizePath(tab.path) === normalizePath(path));
  if (existing) {
    activateTab(existing.path);
    return;
  }

  try {
    syncActiveTabFromEditor();
    const content = await invoke('read_text_file', { path });
    const tab = {
      path,
      name: pathBase(path),
      content,
      dirty: false,
      scrollTop: 0,
      selectionStart: 0,
      selectionEnd: 0,
    };
    state.tabs.push(tab);
    setEditorFromTab(tab);
  } catch (error) {
    appendFriendly('error', `Could not open file: ${error}`);
  }
}

async function reloadTabFromDisk(path) {
  const tab = state.tabs.find((item) => normalizePath(item.path) === normalizePath(path));
  if (!tab) return;
  try {
    tab.content = await invoke('read_text_file', { path });
    tab.dirty = false;
    if (normalizePath(state.activeTabPath) === normalizePath(path)) setEditorFromTab(tab);
    else renderTabs();
  } catch (error) {
    appendFriendly('error', `Could not refresh ${pathBase(path)}: ${error}`);
  }
}

async function saveCurrentFile({ announce = true } = {}) {
  const tab = activeTab();
  if (!tab) return false;
  syncActiveTabFromEditor();
  try {
    await invoke('write_text_file', { path: tab.path, content: tab.content });
    tab.dirty = false;
    state.dirty = false;
    updateActiveTabVisual();
    if (announce) appendFriendly('success', `Saved ${tab.name}`);
    if (tab.name.toLowerCase() === 'cargo.toml') await refreshManifest();
    return true;
  } catch (error) {
    appendFriendly('error', `Save failed: ${error}`);
    return false;
  }
}

async function saveAllDirtyTabs({ announce = false } = {}) {
  syncActiveTabFromEditor();
  const dirtyTabs = state.tabs.filter((tab) => tab.dirty);
  if (!dirtyTabs.length) return true;

  let manifestChanged = false;
  for (const tab of dirtyTabs) {
    try {
      await invoke('write_text_file', { path: tab.path, content: tab.content });
      tab.dirty = false;
      if (tab.name.toLowerCase() === 'cargo.toml') manifestChanged = true;
      if (announce) appendFriendly('success', `Saved ${tab.name}`);
    } catch (error) {
      appendFriendly('error', `Could not save ${tab.name}: ${error}`);
      renderTabs();
      return false;
    }
  }

  state.dirty = Boolean(activeTab()?.dirty);
  renderTabs();
  if (manifestChanged) await refreshManifest();
  return true;
}

function updateDirty() {
  const tab = activeTab();
  if (tab) tab.dirty = state.dirty;
  updateActiveTabVisual();
}

async function closeTab(path = state.activeTabPath) {
  if (!path) return;
  syncActiveTabFromEditor();
  const index = state.tabs.findIndex((tab) => normalizePath(tab.path) === normalizePath(path));
  if (index < 0) return;
  const tab = state.tabs[index];
  if (tab.dirty && !await oxideConfirm('UNSAVED FILE', `Discard unsaved changes in ${tab.name}?`, 'DISCARD')) return;

  state.tabs.splice(index, 1);
  if (normalizePath(state.activeTabPath) !== normalizePath(path)) {
    renderTabs();
    return;
  }
  const next = state.tabs[index] || state.tabs[index - 1] || null;
  setEditorFromTab(next);
}

function clearTabs() {
  state.tabs = [];
  setEditorFromTab(null);
}

async function closeProject() {
  if (!state.projectPath) return;
  if (hasDirtyTabs() && !await oxideConfirm('CLOSE PROJECT', `Discard unsaved changes in ${dirtyTabCount()} file${dirtyTabCount() === 1 ? '' : 's'} and close this project?`, 'CLOSE PROJECT')) return;
  if (state.terminalRunning) {
    if (!await oxideConfirm('PROGRAM RUNNING', 'A program is still running in the Rivet Terminal. Stop it and close the project?', 'STOP & CLOSE')) return;
    try { await invoke('terminal_stop'); } catch { /* terminal may already have exited */ }
  }
  if (state.debugger.running) {
    if (!await oxideConfirm('DEBUG SESSION ACTIVE', 'Stop the active debugger session and close the project?', 'STOP & CLOSE')) return;
    await stopDebugging();
  }

  if (state.tutorial.active) exitTutorialMode();
  hideTerminalWindow();

  try { await invoke('rust_analyzer_stop'); } catch { /* analyzer may not have started */ }
  closeCompletionUi();
  state.projectPath = '';
  state.manifest = null;
  state.diagnostics = [];
  state.debugger.breakpoints.clear();
  state.debugger.output = [];
  state.debugger.watches = [];
  state.debugger.watchResults = [];
  state.debugger.threads = [];
  state.debugger.selectedTarget = null;
  state.debugger.expandedVariables.clear();
  state.intelligence.references = [];
  state.intelligence.codeActions = [];
  state.intelligence.pendingRename = null;
  state.completer.dismissedThroughWord = false;
  renderBreakpoints();
  renderDebugThreads();
  clearSemanticReadability();
  state.analysisGeneration += 1;
  if (state.analysisTimer) clearTimeout(state.analysisTimer);
  clearTabs();
  els.tree.innerHTML = '<div class="empty-state">File → Open Project…</div>';
  els.projectName.textContent = '—';
  els.cargoInspector.innerHTML = '<div class="empty-state">Cargo.toml information will appear here.</div>';
  els.commandReadout.textContent = 'SELECT A PROJECT TO BEGIN';
  els.buildStatus.textContent = 'IDLE';
  clearOutput();
  renderProblems();
  setProjectUiState();
}

function diagnosticsForFile(path) {
  const normalized = normalizePath(path);
  return state.diagnostics.filter((diagnostic) => normalizePath(diagnostic.file_path) === normalized);
}

const RUST_KEYWORDS = new Set([
  'as', 'async', 'await', 'break', 'const', 'continue', 'crate', 'dyn', 'else', 'enum',
  'extern', 'false', 'fn', 'for', 'if', 'impl', 'in', 'let', 'loop', 'match', 'mod',
  'move', 'mut', 'pub', 'ref', 'return', 'self', 'Self', 'static', 'struct', 'super',
  'trait', 'true', 'type', 'unsafe', 'use', 'where', 'while', 'yield', 'abstract',
  'become', 'box', 'do', 'final', 'macro', 'override', 'priv', 'typeof', 'unsized', 'virtual', 'try'
]);

const RUST_PRIMITIVE_TYPES = new Set([
  'bool', 'char', 'str', 'i8', 'i16', 'i32', 'i64', 'i128', 'isize',
  'u8', 'u16', 'u32', 'u64', 'u128', 'usize', 'f32', 'f64'
]);

function rustDeclaredFunctions(source) {
  return new Set([...source.matchAll(/\bfn\s+([A-Za-z_][A-Za-z0-9_]*)/g)].map((match) => match[1]));
}

function syntaxTokenClass(identifier, followingText, functions) {
  if (RUST_KEYWORDS.has(identifier)) return 'syntax-keyword';
  const next = followingText.match(/^\s*([!(])/ )?.[1] || '';
  if (next === '!') return 'syntax-macro';
  if (RUST_PRIMITIVE_TYPES.has(identifier) || /^[A-Z][A-Za-z0-9_]*$/.test(identifier)) return 'syntax-type';
  if (functions.has(identifier) || next === '(') return 'syntax-function';
  // The lexical fallback intentionally makes every ordinary identifier steel blue.
  // rust-analyzer can then refine variables, parameters, fields, functions, and types semantically.
  return 'syntax-ident';
}

function renderRustSyntaxLexical(source) {
  const functions = rustDeclaredFunctions(source);
  let html = '';
  let i = 0;
  const push = (text, cls = '') => {
    const escaped = escapeHtml(text);
    html += cls ? `<span class="${cls}">${escaped}</span>` : escaped;
  };

  while (i < source.length) {
    const ch = source[i];
    const next = source[i + 1] || '';

    // Line comments.
    if (ch === '/' && next === '/') {
      const end = source.indexOf('\n', i);
      const stop = end === -1 ? source.length : end;
      push(source.slice(i, stop), 'syntax-comment');
      i = stop;
      continue;
    }

    // Nested Rust block comments.
    if (ch === '/' && next === '*') {
      let depth = 1;
      let j = i + 2;
      while (j < source.length && depth > 0) {
        if (source[j] === '/' && source[j + 1] === '*') { depth += 1; j += 2; continue; }
        if (source[j] === '*' && source[j + 1] === '/') { depth -= 1; j += 2; continue; }
        j += 1;
      }
      push(source.slice(i, j), 'syntax-comment');
      i = j;
      continue;
    }

    // Raw strings: r"...", r#"..."#, r##"..."##, etc.
    if (ch === 'r') {
      const raw = source.slice(i).match(/^r(#{0,16})"/);
      if (raw) {
        const hashes = raw[1];
        const opener = raw[0];
        const closer = `"${hashes}`;
        const end = source.indexOf(closer, i + opener.length);
        const stop = end === -1 ? source.length : end + closer.length;
        push(source.slice(i, stop), 'syntax-string');
        i = stop;
        continue;
      }
    }

    // Normal strings and byte strings.
    if (ch === '"' || (ch === 'b' && next === '"')) {
      const start = i;
      let j = ch === 'b' ? i + 2 : i + 1;
      let escaped = false;
      while (j < source.length) {
        const current = source[j];
        if (!escaped && current === '"') { j += 1; break; }
        if (!escaped && current === '\\') escaped = true;
        else escaped = false;
        j += 1;
      }
      push(source.slice(start, j), 'syntax-string');
      i = j;
      continue;
    }

    // Character literals. Lifetimes such as 'a stay as identifiers unless semantic tokens refine them.
    if (ch === "'" && source[i + 1] && source[i + 2] === "'") {
      push(source.slice(i, i + 3), 'syntax-character');
      i += 3;
      continue;
    }
    if (ch === "'" && source[i + 1] === '\\') {
      let j = i + 2;
      while (j < source.length && source[j] !== "'") j += 1;
      if (j < source.length) j += 1;
      push(source.slice(i, j), 'syntax-character');
      i = j;
      continue;
    }

    // Numbers, including common Rust suffixes.
    if (/\d/.test(ch)) {
      const number = source.slice(i).match(/^(?:0[xob][0-9A-Fa-f_]+|\d[\d_]*(?:\.\d[\d_]*)?(?:[eE][+-]?\d[\d_]*)?)(?:[iu](?:8|16|32|64|128|size)|f(?:32|64))?/i)?.[0];
      if (number) {
        push(number, 'syntax-number');
        i += number.length;
        continue;
      }
    }

    // Identifiers, functions, types, and macros.
    if (/[A-Za-z_]/.test(ch)) {
      const ident = source.slice(i).match(/^[A-Za-z_][A-Za-z0-9_]*/)?.[0];
      if (ident) {
        push(ident, syntaxTokenClass(ident, source.slice(i + ident.length, i + ident.length + 24), functions));
        i += ident.length;
        continue;
      }
    }

    push(ch);
    i += 1;
  }
  return html;
}

function semanticSyntaxClass(token) {
  const type = String(token?.tokenType || '');
  const map = {
    variable: 'syntax-variable', parameter: 'syntax-parameter', property: 'syntax-property', constParameter: 'syntax-parameter',
    function: 'syntax-function', method: 'syntax-method', macro: 'syntax-macro', derive: 'syntax-derive', attribute: 'syntax-attribute', decorator: 'syntax-attribute', builtinAttribute: 'syntax-attribute', deriveHelper: 'syntax-attribute',
    type: 'syntax-type', builtinType: 'syntax-type', class: 'syntax-type', interface: 'syntax-trait', struct: 'syntax-struct', enum: 'syntax-enum', trait: 'syntax-trait', typeAlias: 'syntax-type-alias', typeParameter: 'syntax-type-parameter', union: 'syntax-type',
    string: 'syntax-string', character: 'syntax-character', escapeSequence: 'syntax-string', formatSpecifier: 'syntax-string',
    number: 'syntax-number', boolean: 'syntax-boolean', comment: 'syntax-comment',
    keyword: 'syntax-keyword', selfKeyword: 'syntax-keyword', selfTypeKeyword: 'syntax-keyword',
    namespace: 'syntax-namespace', toolModule: 'syntax-namespace', enumMember: 'syntax-enum-member', constant: 'syntax-constant', lifetime: 'syntax-lifetime', label: 'syntax-lifetime', generic: 'syntax-ident',
    operator: 'syntax-operator', arithmetic: 'syntax-operator', bitwise: 'syntax-operator', comparison: 'syntax-operator', logical: 'syntax-operator',
    punctuation: 'syntax-punctuation', attributeBracket: 'syntax-punctuation', angle: 'syntax-punctuation', brace: 'syntax-punctuation', bracket: 'syntax-punctuation', parenthesis: 'syntax-punctuation', colon: 'syntax-punctuation', comma: 'syntax-punctuation', dot: 'syntax-punctuation', semi: 'syntax-punctuation', macroBang: 'syntax-macro',
    unresolvedReference: 'syntax-ident',
  };
  return map[type] || '';
}

function semanticLineStarts(source) {
  const starts = [0];
  for (let index = 0; index < source.length; index += 1) if (source[index] === '\n') starts.push(index + 1);
  return starts;
}

function renderRustSyntax(source, semanticTokens = []) {
  if (!semanticTokens.length) return renderRustSyntaxLexical(source);
  const starts = semanticLineStarts(source);
  const ranges = semanticTokens.map((token) => {
    const cls = semanticSyntaxClass(token);
    const lineStart = starts[token.line];
    if (!cls || lineStart == null) return null;
    const start = Math.min(source.length, lineStart + Number(token.startCharacter || 0));
    const end = Math.min(source.length, start + Number(token.length || 0));
    return end > start ? { start, end, cls } : null;
  }).filter(Boolean).sort((left, right) => left.start - right.start || left.end - right.end);

  if (!ranges.length) return renderRustSyntaxLexical(source);
  let html = '';
  let cursor = 0;
  for (const range of ranges) {
    if (range.end <= cursor) continue;
    const start = Math.max(cursor, range.start);
    if (start > cursor) html += renderRustSyntaxLexical(source.slice(cursor, start));
    html += `<span class="${range.cls}">${escapeHtml(source.slice(start, range.end))}</span>`;
    cursor = range.end;
  }
  if (cursor < source.length) html += renderRustSyntaxLexical(source.slice(cursor));
  return html;
}

function syntaxHighlightEnabled() {
  return Boolean(state.currentFile && state.currentFile.toLowerCase().endsWith('.rs'));
}

function syncSyntaxScroll() {
  if (!els.syntaxCode) return;
  els.syntaxCode.style.transform = `translate(${-els.editor.scrollLeft}px, ${-els.editor.scrollTop}px)`;
}

function updateSyntaxHighlight() {
  if (!els.syntaxLayer || !els.syntaxCode) return;
  const enabled = syntaxHighlightEnabled();
  els.editor.classList.toggle('syntax-active', enabled);
  els.syntaxLayer.hidden = !enabled;
  if (!enabled) {
    els.syntaxCode.textContent = '';
    return;
  }
  const semanticTokens = state.semanticReadability.active ? state.semanticReadability.tokens : [];
  // A trailing newline keeps the backdrop's final empty line aligned with textarea rendering.
  els.syntaxCode.innerHTML = renderRustSyntax(els.editor.value, semanticTokens) + (els.editor.value.endsWith('\n') ? '\n' : '');
  syncSyntaxScroll();
}

function clearSemanticReadability() {
  state.semanticReadability.requestToken += 1;
  if (state.semanticReadability.timer) clearTimeout(state.semanticReadability.timer);
  state.semanticReadability.timer = null;
  state.semanticReadability.tokens = [];
  state.semanticReadability.active = false;
}

function scheduleSemanticReadability(delay = 260) {
  if (!syntaxHighlightEnabled() || !state.projectPath || !state.completer.available) {
    clearSemanticReadability();
    updateSyntaxHighlight();
    return;
  }
  if (state.semanticReadability.timer) clearTimeout(state.semanticReadability.timer);
  const requestToken = ++state.semanticReadability.requestToken;
  const projectPath = state.projectPath;
  const path = state.currentFile;
  const content = els.editor.value;
  state.semanticReadability.timer = setTimeout(async () => {
    state.semanticReadability.timer = null;
    try {
      const tokens = await invoke('rust_semantic_tokens', { projectPath, path, content });
      if (requestToken !== state.semanticReadability.requestToken || normalizePath(path) !== normalizePath(state.currentFile) || content !== els.editor.value) return;
      state.semanticReadability.tokens = Array.isArray(tokens) ? tokens : [];
      state.semanticReadability.active = state.semanticReadability.tokens.length > 0;
      updateSyntaxHighlight();
    } catch {
      if (requestToken !== state.semanticReadability.requestToken) return;
      // Lexical colors remain active if rust-analyzer is warming up or unavailable.
      state.semanticReadability.tokens = [];
      state.semanticReadability.active = false;
      updateSyntaxHighlight();
    }
  }, delay);
}

function positionDebugLineHighlight() {
  if (!els.debugLineHighlight) return;
  const visible = state.debugger.stopped && state.currentFile && normalizePath(state.debugger.executionPath) === normalizePath(state.currentFile) && state.debugger.executionLine > 0;
  els.debugLineHighlight.hidden = !visible;
  if (!visible) return;
  els.debugLineHighlight.style.top = `${12 + ((state.debugger.executionLine - 1) * 20.15) - els.editor.scrollTop}px`;
}

function updateLineNumbers() {
  const count = Math.max(1, els.editor.value.split('\n').length);
  const byLine = new Map();
  if (state.currentFile) {
    for (const diagnostic of diagnosticsForFile(state.currentFile)) {
      const existing = byLine.get(diagnostic.line);
      if (!existing || (diagnostic.level === 'error' && existing.level !== 'error')) byLine.set(diagnostic.line, diagnostic);
    }
  }

  els.lines.innerHTML = Array.from({ length: count }, (_, index) => {
    const line = index + 1;
    const diagnostic = byLine.get(line);
    const hasBreakpoint = breakpointLinesForFile(state.currentFile).has(line);
    const isExecution = state.debugger.stopped && normalizePath(state.debugger.executionPath) === normalizePath(state.currentFile) && state.debugger.executionLine === line;
    const kinds = [diagnostic ? `diagnostic-${diagnostic.level}` : '', hasBreakpoint ? 'breakpoint' : '', isExecution ? 'debug-execution' : ''].filter(Boolean).join(' ');
    const titleParts = [];
    if (diagnostic) titleParts.push(diagnostic.message);
    if (hasBreakpoint) titleParts.push('Breakpoint');
    if (isExecution) titleParts.push('Current execution line');
    const title = titleParts.length ? ` title="${escapeHtml(titleParts.join(' · '))}"` : '';
    return `<span class="line-number${kinds ? ` ${kinds}` : ''}" data-line="${line}"${title}>${line}</span>`;
  }).join('');
  els.lines.scrollTop = els.editor.scrollTop;
  positionDebugLineHighlight();
}

function friendlyDiagnosticHint(diagnostic) {
  const message = `${diagnostic.message || ''} ${diagnostic.label || ''}`.toLowerCase();
  if (diagnostic.code === 'E0384' || message.includes('immutable')) {
    return 'This variable is immutable. If you intend to change it, add mut after let.';
  }
  if (diagnostic.code === 'E0382' || message.includes('moved value')) {
    return 'Rust moved ownership of this value. Borrow it with a reference when the next operation should not take ownership.';
  }
  if (diagnostic.code === 'E0425' || message.includes('cannot find value')) {
    return 'Rust cannot find that name in the current scope. Check the spelling and make sure the variable was created before it is used.';
  }
  if (diagnostic.code === 'E0308' || message.includes('mismatched types')) {
    return 'The value here has a different type than Rust expected. Check what the expression produces and what the receiving code requires.';
  }
  if (message.includes('expected `;`') || message.includes('semicolon')) {
    return 'Almost! Rust expects this statement to end with a semicolon. Try adding ; at the end of the statement.';
  }
  return diagnostic.suggestions?.[0] || '';
}

function updateDiagnosticBanner() {
  if (!state.currentFile || !state.currentFile.toLowerCase().endsWith('.rs')) {
    els.diagnosticBanner.hidden = true;
    return;
  }
  const fileProblems = diagnosticsForFile(state.currentFile);
  if (!fileProblems.length) {
    els.diagnosticBanner.hidden = true;
    return;
  }

  const diagnostic = fileProblems.find((item) => item.level === 'error') || fileProblems[0];
  els.diagnosticBanner.hidden = false;
  els.diagnosticBanner.classList.toggle('warning', diagnostic.level === 'warning');
  els.diagnosticBannerLevel.textContent = diagnostic.level.toUpperCase();
  const code = diagnostic.code ? ` ${diagnostic.code}` : '';
  const friendly = friendlyDiagnosticHint(diagnostic);
  const hint = friendly ? ` · Hint: ${friendly}` : '';
  els.diagnosticBannerText.textContent = `Line ${diagnostic.line}:${diagnostic.column} ·${code} ${diagnostic.message}${hint}`;
}

function renderProblems() {
  const diagnostics = state.diagnostics;
  els.problemCount.textContent = String(diagnostics.length);
  els.problemCount.classList.toggle('has-problems', diagnostics.length > 0);
  if (!diagnostics.length) {
    els.problemsList.innerHTML = '<div class="problems-empty">✓ No Rust problems detected.</div>';
    updateLineNumbers();
    updateDiagnosticBanner();
    return;
  }

  els.problemsList.innerHTML = diagnostics.map((diagnostic, index) => {
    const code = diagnostic.code ? `<span class="problem-code">${escapeHtml(diagnostic.code)}</span>` : '';
    const label = diagnostic.label ? `<div class="problem-label">${escapeHtml(diagnostic.label)}</div>` : '';
    const friendly = friendlyDiagnosticHint(diagnostic);
    const hintItems = [...new Set([friendly, ...(diagnostic.suggestions || [])].filter(Boolean))];
    const hints = hintItems.length
      ? `<div class="problem-hints">${hintItems.map((hint, hintIndex) => `<div><b>${hintIndex === 0 && friendly ? 'FRIENDLY' : 'HINT'}</b> ${escapeHtml(hint)}</div>`).join('')}</div>`
      : '';
    return `
      <button type="button" class="problem-item ${diagnostic.level}" data-problem-index="${index}">
        <span class="problem-icon">${diagnostic.level === 'error' ? '×' : '!'}</span>
        <span class="problem-main"><span class="problem-title">${escapeHtml(diagnostic.message)} ${code}</span>${label}${hints}</span>
        <span class="problem-location">${escapeHtml(diagnostic.file_name)}:${diagnostic.line}:${diagnostic.column}</span>
      </button>`;
  }).join('');

  els.problemsList.querySelectorAll('.problem-item').forEach((button) => {
    button.addEventListener('click', () => jumpToDiagnostic(state.diagnostics[Number(button.dataset.problemIndex)]));
  });
  updateLineNumbers();
  updateDiagnosticBanner();
}

function offsetForLineColumn(text, line, column) {
  const lines = text.split('\n');
  let offset = 0;
  for (let index = 0; index < Math.max(0, line - 1) && index < lines.length; index += 1) offset += lines[index].length + 1;
  return Math.min(text.length, offset + Math.max(0, column - 1));
}

async function jumpToDiagnostic(diagnostic) {
  if (!diagnostic) return;
  await loadFile(diagnostic.file_path);
  const tab = activeTab();
  if (!tab) return;
  const start = offsetForLineColumn(els.editor.value, diagnostic.line, diagnostic.column);
  const end = offsetForLineColumn(els.editor.value, diagnostic.end_line || diagnostic.line, diagnostic.end_column || diagnostic.column + 1);
  els.editor.focus();
  els.editor.setSelectionRange(start, Math.max(start, end));
  const lineHeight = 20.15;
  els.editor.scrollTop = Math.max(0, (diagnostic.line - 4) * lineHeight);
  els.lines.scrollTop = els.editor.scrollTop;
}

function scheduleAnalysis(delay = 900) {
  if (!state.liveCheck || !state.projectPath || state.buildRunning || state.terminalRunning || state.debugger.running) return;
  if (!activeTab()?.name.toLowerCase().endsWith('.rs')) return;
  if (state.analysisTimer) clearTimeout(state.analysisTimer);
  state.analysisTimer = setTimeout(() => {
    state.analysisTimer = null;
    runDiagnostics({ silent: true });
  }, delay);
}

async function runDiagnostics({ silent = false, force = false } = {}) {
  if (!state.projectPath || state.buildRunning || state.terminalRunning || state.debugger.running) return;
  if (state.analysisRunning) {
    state.analysisQueued = true;
    return;
  }
  if (!force && !state.liveCheck) return;

  const generation = ++state.analysisGeneration;
  state.analysisRunning = true;
  state.analysisQueued = false;
  els.analysisStatus.textContent = 'RUST CHECK: RUNNING';
  els.analysisStatus.classList.add('hot');

  if (!await saveAllDirtyTabs()) {
    state.analysisRunning = false;
    els.analysisStatus.textContent = 'RUST CHECK: SAVE FAILED';
    els.analysisStatus.classList.remove('hot');
    return;
  }

  try {
    const result = await invoke('cargo_diagnostics', { projectPath: state.projectPath, release: state.release });
    if (generation !== state.analysisGeneration) return;
    state.diagnostics = result.diagnostics || [];
    renderProblems();
    scheduleTutorialEvaluation(80);
    const errors = state.diagnostics.filter((item) => item.level === 'error').length;
    const warnings = state.diagnostics.filter((item) => item.level === 'warning').length;
    els.analysisStatus.textContent = errors ? `RUST CHECK: ${errors} ERROR${errors === 1 ? '' : 'S'}` : warnings ? `RUST CHECK: ${warnings} WARNING${warnings === 1 ? '' : 'S'}` : 'RUST CHECK: CLEAN';
    if (!silent) {
      setConsoleView('problems');
      if (!state.view.build) setViewPanel('build', true);
    }
  } catch (error) {
    if (generation === state.analysisGeneration) {
      els.analysisStatus.textContent = 'RUST CHECK: FAILED';
      if (!silent) appendFriendly('error', `Rust analysis failed: ${error}`);
    }
  } finally {
    state.analysisRunning = false;
    els.analysisStatus.classList.remove('hot');
    if (state.analysisQueued && state.liveCheck) {
      state.analysisQueued = false;
      scheduleAnalysis(150);
    }
  }
}

function renderManifest(manifest) {
  const deps = manifest.dependencies || [];
  els.cargoInspector.innerHTML = `
    <section class="manifest-block">
      <div class="manifest-label">PACKAGE</div>
      <strong>${escapeHtml(manifest.package_name)}</strong>
      <dl><div><dt>Version</dt><dd>${escapeHtml(manifest.version)}</dd></div><div><dt>Edition</dt><dd>${escapeHtml(manifest.edition)}</dd></div></dl>
    </section>
    <section class="manifest-block dependency-block">
      <div class="manifest-row"><div class="manifest-label">DEPENDENCIES</div><button id="add-dependency" class="tiny-button">+ ADD</button></div>
      <div class="dependency-list">
        ${deps.length ? deps.map((dep) => `
          <div class="dependency-item"><div><strong>${escapeHtml(dep.name)}</strong><small>${escapeHtml(dep.display)}</small></div><button class="remove-dependency" data-name="${escapeHtml(dep.name)}" title="Remove ${escapeHtml(dep.name)}">×</button></div>`).join('') : '<div class="empty-deps">No dependencies.</div>'}
      </div>
    </section>
    <button id="open-manifest" class="metal-button full">OPEN CARGO.TOML</button>`;

  $('#add-dependency')?.addEventListener('click', openDependencyDialog);
  $('#open-manifest')?.addEventListener('click', () => loadFile(joinProjectPath('Cargo.toml')));
  els.cargoInspector.querySelectorAll('.remove-dependency').forEach((button) => button.addEventListener('click', () => removeDependency(button.dataset.name)));
}

function joinProjectPath(file) {
  const separator = state.projectPath.includes('\\') ? '\\' : '/';
  return `${state.projectPath.replace(/[\\/]$/, '')}${separator}${file}`;
}

async function refreshManifest() {
  if (!state.projectPath) return;
  try {
    const manifest = await invoke('manifest_view', { projectPath: state.projectPath });
    state.manifest = manifest;
    renderManifest(manifest);
  } catch (error) {
    appendFriendly('error', `Cargo.toml refresh failed: ${error}`);
  }
}

function openDependencyDialog() {
  if (!state.projectPath) return;
  $('#dep-name').value = '';
  $('#dep-version').value = '*';
  $('#dep-features').value = '';
  els.dependencyDialog.showModal();
}

async function removeDependency(name) {
  if (!await oxideConfirm('REMOVE DEPENDENCY', `Remove dependency '${name}' from Cargo.toml?`, 'REMOVE')) return;
  try {
    await invoke('remove_dependency', { projectPath: state.projectPath, name });
    await refreshManifest();
    const manifestTab = state.tabs.find((tab) => tab.name.toLowerCase() === 'cargo.toml');
    if (manifestTab && !manifestTab.dirty) await reloadTabFromDisk(manifestTab.path);
    appendFriendly('success', `Removed dependency ${name}`);
    if (state.liveCheck) scheduleAnalysis(250);
    scheduleTutorialEvaluation(120);
  } catch (error) {
    appendFriendly('error', `Could not remove dependency: ${error}`);
  }
}

async function addDependencyFromDialog(event) {
  event.preventDefault();
  const name = $('#dep-name').value.trim();
  const version = $('#dep-version').value.trim();
  const features = $('#dep-features').value.split(',').map((item) => item.trim()).filter(Boolean);
  if (!name || !version) return;

  try {
    await invoke('add_dependency', { projectPath: state.projectPath, name, version, features });
    els.dependencyDialog.close();
    await refreshManifest();
    const manifestTab = state.tabs.find((tab) => tab.name.toLowerCase() === 'cargo.toml');
    if (manifestTab && !manifestTab.dirty) await reloadTabFromDisk(manifestTab.path);
    appendFriendly('success', `Added dependency ${name}`);
    if (state.liveCheck) scheduleAnalysis(250);
  } catch (error) {
    appendFriendly('error', `Could not add dependency: ${error}`);
  }
}

function clearOutput() {
  state.rawLines = [];
  state.friendlyLines = [];
  renderOutput();
}

function appendRaw(stream, line) {
  state.rawLines.push({ kind: stream === 'stderr' ? 'raw-stderr' : 'raw', text: line });
  if (state.outputMode === 'raw' && state.consoleView === 'build') renderOutput();
}

function appendFriendly(kind, text) {
  state.friendlyLines.push({ kind, text });
  if (state.outputMode === 'friendly' && state.consoleView === 'build') renderOutput();
}

function interpretCargoLine(line) {
  const clean = line.replace(/\x1b\[[0-9;]*m/g, '').trim();
  if (!clean) return;
  const compile = clean.match(/^Compiling\s+(.+)/i);
  const checking = clean.match(/^Checking\s+(.+)/i);
  const finished = clean.match(/^Finished\s+(.+)/i);
  if (compile) appendFriendly('working', `Compiling ${compile[1]}`);
  else if (checking) appendFriendly('working', `Checking ${checking[1]}`);
  else if (finished) appendFriendly('success', clean);
  else if (/^warning(?::|\[)/i.test(clean)) appendFriendly('warning', clean);
  else if (/^error(?::|\[)/i.test(clean)) appendFriendly('error', clean);
  else if (/^Running\s+/i.test(clean)) appendFriendly('runline', clean);
}

function renderOutput() {
  const source = state.outputMode === 'friendly' ? state.friendlyLines : state.rawLines;
  els.output.innerHTML = source.length
    ? source.map((item) => `<div class="output-line ${item.kind}">${escapeHtml(item.text)}</div>`).join('')
    : '<div class="output-line muted">No output.</div>';
  els.output.scrollTop = els.output.scrollHeight;
}

function setBuildRunning(running, action = '') {
  state.buildRunning = running;
  els.buildStatus.textContent = running ? `${action.toUpperCase()} IN PROGRESS` : (state.projectPath ? 'PROJECT READY' : 'IDLE');
  els.buildStatus.classList.toggle('hot', running);
  updateMenuAvailability();
}

async function cargoAction(action) {
  if (!state.projectPath || state.buildRunning || state.terminalRunning || state.debugger.running) return;
  if (!await saveAllDirtyTabs()) return;
  if (!state.view.build) setViewPanel('build', true);
  setConsoleView('build');
  clearOutput();
  setBuildRunning(true, action);
  els.commandReadout.textContent = `${action.toUpperCase()} · CARGO ACTIVE`;
  appendFriendly('stage', `${action.toUpperCase()} · ${state.release ? 'RELEASE' : 'DEBUG'} profile`);

  try {
    const result = await invoke('cargo_action', { action, projectPath: state.projectPath, release: state.release });
    if (result.success) {
      appendFriendly('success', `✓ Cargo ${action} completed successfully.`);
      els.commandReadout.textContent = `${action.toUpperCase()} COMPLETE`;
    } else {
      appendFriendly('error', `✕ Cargo ${action} failed with exit code ${result.exit_code ?? 'unknown'}.`);
      els.commandReadout.textContent = `${action.toUpperCase()} FAILED`;
    }
  } catch (error) {
    appendFriendly('error', `Cargo could not start: ${error}`);
    els.commandReadout.textContent = 'CARGO ERROR';
  } finally {
    setBuildRunning(false);
    if (action !== 'clean' && state.liveCheck) scheduleAnalysis(200);
  }
}

function projectLikelyGui() {
  const names = (state.manifest?.dependencies || []).map((dependency) => dependency.name.toLowerCase());
  const guiCrates = ['tauri', 'eframe', 'egui', 'iced', 'slint', 'winit', 'gtk', 'gtk4', 'bevy'];
  return names.some((name) => guiCrates.includes(name));
}

function requestRun() {
  if (!state.projectPath || state.buildRunning || state.terminalRunning || state.debugger.running) return;
  els.runProjectName.textContent = state.manifest?.package_name || pathBase(state.projectPath);
  if (state.tutorial.active && currentTutorialStep()?.run_required) {
    els.runDetection.innerHTML = '<span class="lamp ok"></span><span>This tutorial activity expects the Rivet Run Terminal so it can verify the real program output.</span>';
  } else {
    els.runDetection.innerHTML = projectLikelyGui()
      ? '<span class="lamp ok"></span><span>Rivet found a GUI-oriented dependency. GUI / Native Window may be the right choice.</span>'
      : '<span class="lamp"></span><span>Choose Terminal for console programs or GUI / Native Window if your program creates its own window.</span>';
  }
  els.runDialog.showModal();
}


function breakpointEntryForFile(path, create = false) {
  if (!path) return null;
  const key = normalizePath(path);
  if (!state.debugger.breakpoints.has(key) && create) state.debugger.breakpoints.set(key, { path, points: new Map() });
  return state.debugger.breakpoints.get(key) || null;
}

function breakpointLinesForFile(path) {
  return new Set(breakpointEntryForFile(path)?.points.keys() || []);
}

function debuggerBreakpointSets() {
  return [...state.debugger.breakpoints.values()]
    .filter((item) => item.points.size)
    .map((item) => ({
      path: item.path,
      breakpoints: [...item.points.values()].sort((left, right) => left.line - right.line),
      lines: [],
    }));
}

async function syncBreakpointSet(path) {
  if (!state.debugger.running || !path) return;
  const entry = breakpointEntryForFile(path);
  try {
    await invoke('debugger_set_breakpoints', {
      breakpointSet: {
        path,
        breakpoints: entry ? [...entry.points.values()].sort((left, right) => left.line - right.line) : [],
        lines: [],
      },
    });
  } catch (error) {
    appendDebugOutput(`Could not update breakpoint: ${error}`, 'error');
  }
}

async function toggleBreakpoint(path, line) {
  if (!path || !path.toLowerCase().endsWith('.rs') || !line) return;
  const key = normalizePath(path);
  const entry = breakpointEntryForFile(path, true);
  if (entry.points.has(line)) entry.points.delete(line);
  else entry.points.set(line, { line, condition: '', hitCondition: '', logMessage: '' });
  if (!entry.points.size) state.debugger.breakpoints.delete(key);
  updateLineNumbers();
  renderBreakpoints();
  await syncBreakpointSet(path);
}

async function jumpToBreakpoint(path, line) {
  try {
    await loadFile(path);
    const offset = offsetForLineColumn(els.editor.value, line, 1);
    els.editor.focus();
    els.editor.setSelectionRange(offset, offset);
    els.editor.scrollTop = Math.max(0, (line - 5) * 20.15);
    els.lines.scrollTop = els.editor.scrollTop;
  } catch (error) {
    appendDebugOutput(`Could not open breakpoint source: ${error}`, 'error');
  }
}

function renderBreakpoints() {
  const points = [...state.debugger.breakpoints.values()].flatMap((entry) => [...entry.points.values()].map((point) => ({ ...point, path: entry.path })));
  points.sort((left, right) => normalizePath(left.path).localeCompare(normalizePath(right.path)) || left.line - right.line);
  if (!points.length) {
    els.debugBreakpoints.innerHTML = '<div class="debug-empty">Click a Rust line number to add a breakpoint.</div>';
    return;
  }
  els.debugBreakpoints.innerHTML = points.map((point, index) => {
    const detail = point.logMessage
      ? `LOG · ${point.logMessage}`
      : [point.condition ? `IF ${point.condition}` : '', point.hitCondition ? `HIT ${point.hitCondition}` : ''].filter(Boolean).join(' · ') || 'BREAK ALWAYS';
    return `<div class="debug-breakpoint-row ${point.logMessage ? 'logpoint' : ''}" data-breakpoint-index="${index}"><span class="debug-breakpoint-dot"></span><button type="button" class="debug-breakpoint-main"><b>${escapeHtml(pathBase(point.path))}:${point.line}</b><small>${escapeHtml(detail)}</small></button><button type="button" class="debug-breakpoint-edit" title="Breakpoint options">⋮</button></div>`;
  }).join('');
  els.debugBreakpoints.querySelectorAll('.debug-breakpoint-row').forEach((row) => {
    const point = points[Number(row.dataset.breakpointIndex)];
    row.querySelector('.debug-breakpoint-main').addEventListener('click', () => jumpToBreakpoint(point.path, point.line));
    row.querySelector('.debug-breakpoint-edit').addEventListener('click', () => openBreakpointEditor(point.path, point.line));
  });
}

function openBreakpointEditor(path, line) {
  if (!path || !line) return;
  const entry = breakpointEntryForFile(path, true);
  const created = !entry.points.has(line);
  if (created) entry.points.set(line, { line, condition: '', hitCondition: '', logMessage: '' });
  if (created && state.debugger.running) void syncBreakpointSet(path);
  const point = entry.points.get(line);
  state.debugger.editingBreakpoint = { path, line };
  els.breakpointLocation.textContent = `${pathBase(path)} : LINE ${line}`;
  els.breakpointCondition.value = point.condition || '';
  els.breakpointHitCondition.value = point.hitCondition || '';
  els.breakpointLogMessage.value = point.logMessage || '';
  if (!els.breakpointDialog.open) els.breakpointDialog.showModal();
  requestAnimationFrame(() => els.breakpointCondition.focus());
}

function closeBreakpointEditor() {
  state.debugger.editingBreakpoint = null;
  if (els.breakpointDialog.open) els.breakpointDialog.close();
}

async function saveBreakpointOptions(event) {
  event.preventDefault();
  const editing = state.debugger.editingBreakpoint;
  if (!editing) return;
  const entry = breakpointEntryForFile(editing.path, true);
  entry.points.set(editing.line, {
    line: editing.line,
    condition: els.breakpointCondition.value.trim(),
    hitCondition: els.breakpointHitCondition.value.trim(),
    logMessage: els.breakpointLogMessage.value.trim(),
  });
  closeBreakpointEditor();
  updateLineNumbers();
  renderBreakpoints();
  await syncBreakpointSet(editing.path);
}

async function removeEditedBreakpoint() {
  const editing = state.debugger.editingBreakpoint;
  if (!editing) return;
  const key = normalizePath(editing.path);
  const entry = breakpointEntryForFile(editing.path);
  entry?.points.delete(editing.line);
  if (entry && !entry.points.size) state.debugger.breakpoints.delete(key);
  closeBreakpointEditor();
  updateLineNumbers();
  renderBreakpoints();
  await syncBreakpointSet(editing.path);
}

let debugTargetResolver = null;
function finishDebugTargetChoice(target) {
  if (els.debugTargetDialog.open) els.debugTargetDialog.close();
  if (!debugTargetResolver) return;
  const resolve = debugTargetResolver;
  debugTargetResolver = null;
  resolve(target);
}

function chooseDebugTarget(targets) {
  if (debugTargetResolver) debugTargetResolver(null);
  els.debugTargetList.innerHTML = targets.map((target, index) => `<button type="button" class="debug-target-choice" data-target-index="${index}"><strong>${escapeHtml(target.name)}</strong><small>${escapeHtml(target.package)}</small></button>`).join('');
  els.debugTargetList.querySelectorAll('.debug-target-choice').forEach((button) => button.addEventListener('click', () => {
    finishDebugTargetChoice(targets[Number(button.dataset.targetIndex)] || null);
  }));
  if (!els.debugTargetDialog.open) els.debugTargetDialog.showModal();
  return new Promise((resolve) => { debugTargetResolver = resolve; });
}

function appendDebugOutput(text, kind = 'normal') {
  if (!text) return;
  state.debugger.output.push({ text: String(text), kind });
  if (state.debugger.output.length > 700) state.debugger.output.splice(0, state.debugger.output.length - 700);
  els.debugOutput.innerHTML = state.debugger.output.length
    ? state.debugger.output.map((item) => `<div class="debug-output-line ${item.kind}">${escapeHtml(item.text)}</div>`).join('')
    : '<div class="debug-empty">Debugger output will appear here.</div>';
  els.debugOutput.scrollTop = els.debugOutput.scrollHeight;
}

function renderDebugThreads() {
  const threads = state.debugger.threads || [];
  els.debugThreadSelect.innerHTML = threads.length
    ? threads.map((thread) => `<option value="${thread.id}" ${thread.id === state.debugger.threadId ? 'selected' : ''}>${escapeHtml(thread.name || `Thread ${thread.id}`)}</option>`).join('')
    : '<option value="">—</option>';
  els.debugThreadSelect.disabled = !state.debugger.stopped || !threads.length;
}

function resetDebugInspection() {
  state.debugger.frames = [];
  state.debugger.variables = [];
  state.debugger.expandedVariables.clear();
  state.debugger.selectedFrameId = null;
  state.debugger.executionPath = '';
  state.debugger.executionLine = 0;
  els.debugCallStack.innerHTML = '<div class="debug-empty">Execution is running.</div>';
  els.debugVariables.innerHTML = '<div class="debug-empty">Pause at a breakpoint to inspect variables.</div>';
  renderDebugThreads();
  renderWatches();
  updateLineNumbers();
}

function renderDebugStack() {
  els.debugCallStack.innerHTML = state.debugger.frames.length
    ? state.debugger.frames.map((frame, index) => `<button type="button" class="debug-frame ${frame.id === state.debugger.selectedFrameId ? 'active' : ''}" data-frame-index="${index}"><b>${escapeHtml(frame.name)}</b><span>${escapeHtml(pathBase(frame.path) || 'unknown')}:${frame.line}</span></button>`).join('')
    : '<div class="debug-empty">No stack frames reported.</div>';
  els.debugCallStack.querySelectorAll('.debug-frame').forEach((button) => button.addEventListener('click', async () => {
    const frame = state.debugger.frames[Number(button.dataset.frameIndex)];
    if (frame) await selectDebugFrame(frame);
  }));
}

function debugVariableHtml(item, depth = 0) {
  const reference = Number(item.variablesReference || 0);
  const expansion = reference > 0 ? state.debugger.expandedVariables.get(reference) : null;
  const open = Boolean(expansion?.expanded);
  const children = expansion?.children || [];
  const toggle = reference > 0 ? `<button type="button" class="debug-variable-toggle" data-variable-reference="${reference}" title="${open ? 'Collapse' : 'Expand'}">${open ? '▾' : '▸'}</button>` : '<button type="button" class="debug-variable-toggle" disabled>·</button>';
  const row = `<div class="debug-variable debug-variable-child" style="--debug-depth:${depth}">${toggle}<span class="debug-variable-scope">${escapeHtml(item.scope || '')}</span><b>${escapeHtml(item.name)}</b><code>${escapeHtml(item.value)}</code><small>${escapeHtml(item.typeName || '')}</small>${open ? `<div class="debug-variable-children">${children.length ? children.map((child) => debugVariableHtml(child, depth + 1)).join('') : '<div class="debug-empty">No child values.</div>'}</div>` : ''}</div>`;
  return row;
}

function renderDebugVariables() {
  els.debugVariables.innerHTML = state.debugger.variables.length
    ? state.debugger.variables.map((item) => debugVariableHtml(item)).join('')
    : '<div class="debug-empty">No local variables reported for this frame.</div>';
  els.debugVariables.querySelectorAll('.debug-variable-toggle:not(:disabled)').forEach((button) => button.addEventListener('click', async () => {
    const reference = Number(button.dataset.variableReference);
    let expansion = state.debugger.expandedVariables.get(reference);
    if (!expansion) {
      expansion = { expanded: false, children: null };
      state.debugger.expandedVariables.set(reference, expansion);
    }
    if (expansion.expanded) {
      expansion.expanded = false;
      renderDebugVariables();
      return;
    }
    if (!expansion.children) {
      try {
        expansion.children = await invoke('debugger_variables', { variablesReference: reference });
      } catch (error) {
        appendDebugOutput(`Could not expand variable: ${error}`, 'error');
        expansion.children = [];
      }
    }
    expansion.expanded = true;
    renderDebugVariables();
  }));
}

function renderWatches() {
  if (!state.debugger.watches.length) {
    els.debugWatchList.innerHTML = '<div class="debug-empty">Add expressions to watch while paused.</div>';
    return;
  }
  els.debugWatchList.innerHTML = state.debugger.watches.map((expression, index) => {
    const result = state.debugger.watchResults[index];
    const value = result?.error ? `<code class="watch-error">${escapeHtml(result.error)}</code>` : `<code>${escapeHtml(result?.result ?? (state.debugger.stopped ? '…' : 'paused only'))}</code>`;
    return `<div class="debug-watch"><button type="button" class="debug-watch-remove" data-watch-index="${index}" title="Remove watch">×</button><b>${escapeHtml(expression)}</b>${value}<small>${escapeHtml(result?.typeName || '')}</small></div>`;
  }).join('');
  els.debugWatchList.querySelectorAll('.debug-watch-remove').forEach((button) => button.addEventListener('click', () => {
    state.debugger.watches.splice(Number(button.dataset.watchIndex), 1);
    state.debugger.watchResults = [];
    renderWatches();
    if (state.debugger.stopped) refreshWatches();
  }));
}

async function refreshWatches() {
  if (!state.debugger.stopped || !state.debugger.selectedFrameId) { renderWatches(); return; }
  state.debugger.watchResults = await Promise.all(state.debugger.watches.map(async (expression) => {
    try {
      return await invoke('debugger_evaluate', { expression, frameId: state.debugger.selectedFrameId });
    } catch (error) {
      return { error: String(error) };
    }
  }));
  renderWatches();
}

async function selectDebugFrame(frame) {
  state.debugger.selectedFrameId = frame.id;
  state.debugger.executionPath = frame.path || '';
  state.debugger.executionLine = frame.line || 0;
  state.debugger.expandedVariables.clear();
  renderDebugStack();
  const projectRoot = normalizePath(state.projectPath);
  const framePath = normalizePath(frame.path || '');
  if (frame.path && (framePath === projectRoot || framePath.startsWith(`${projectRoot}/`))) {
    try {
      await loadFile(frame.path);
      const offset = offsetForLineColumn(els.editor.value, frame.line || 1, frame.column || 1);
      els.editor.setSelectionRange(offset, offset);
      els.editor.scrollTop = Math.max(0, ((frame.line || 1) - 5) * 20.15);
      els.lines.scrollTop = els.editor.scrollTop;
    } catch { /* stack frames can point into stdlib or generated sources */ }
  }
  updateLineNumbers();
  try {
    const scopes = await invoke('debugger_scopes', { frameId: frame.id });
    const groups = await Promise.all(scopes.filter((scope) => scope.variablesReference > 0).map(async (scope) => {
      try {
        const variables = await invoke('debugger_variables', { variablesReference: scope.variablesReference });
        return variables.map((variable) => ({ ...variable, scope: scope.name }));
      } catch {
        return [];
      }
    }));
    state.debugger.variables = groups.flat();
  } catch (error) {
    state.debugger.variables = [];
    appendDebugOutput(`Could not read variables: ${error}`, 'error');
  }
  renderDebugVariables();
  await refreshWatches();
}

async function refreshDebugInspection(threadId) {
  if (!threadId) return;
  try {
    state.debugger.frames = await invoke('debugger_stack_trace', { threadId });
    state.debugger.selectedFrameId = state.debugger.frames[0]?.id ?? null;
    renderDebugStack();
    if (state.debugger.frames[0]) await selectDebugFrame(state.debugger.frames[0]);
  } catch (error) {
    appendDebugOutput(`Could not read call stack: ${error}`, 'error');
  }
}

async function refreshDebugThreads(preferredThreadId = null) {
  try {
    state.debugger.threads = await invoke('debugger_threads');
    const preferred = Number(preferredThreadId || state.debugger.threadId || 0);
    const selected = state.debugger.threads.find((thread) => thread.id === preferred) || state.debugger.threads[0] || null;
    state.debugger.threadId = selected?.id ?? null;
    renderDebugThreads();
    if (state.debugger.threadId) await refreshDebugInspection(state.debugger.threadId);
  } catch (error) {
    state.debugger.threads = [];
    renderDebugThreads();
    appendDebugOutput(`Could not read debugger threads: ${error}`, 'error');
  }
}

async function startDebugging() {
  if (!state.projectPath || state.debugger.running || state.buildRunning || state.terminalRunning) return;
  if (!state.debugger.available) {
    showInfo('DEBUGGER NOT FOUND', `<p>${escapeHtml(state.debugger.message || 'Rivet could not find lldb-dap.')}</p><p>Rivet uses LLDB's Debug Adapter Protocol for structured Rust debugging.</p>`);
    return;
  }
  if (!await saveAllDirtyTabs()) return;
  const targets = await invoke('debugger_targets', { projectPath: state.projectPath }).catch((error) => {
    appendDebugOutput(`Could not inspect debug targets: ${error}`, 'error');
    return [];
  });
  let target = null;
  if (Array.isArray(targets) && targets.length > 1) {
    target = await chooseDebugTarget(targets);
    if (!target) return;
  } else if (Array.isArray(targets) && targets.length === 1) {
    target = targets[0];
  }
  state.debugger.selectedTarget = target;
  setViewPanel('build', true);
  setConsoleView('debug');
  state.debugger.output = [];
  appendDebugOutput(`Building ${target ? `${target.package} :: ${target.name}` : 'project'} with debug information…`, 'stage');
  state.debugger.running = true;
  state.debugger.stopped = false;
  state.debugger.threadId = null;
  state.debugger.threads = [];
  resetDebugInspection();
  els.debuggerStatus.textContent = 'DEBUGGER: STARTING';
  els.debuggerDetail.textContent = 'BUILDING DEBUG TARGET';
  els.commandReadout.textContent = 'DEBUG · BUILDING';
  updateMenuAvailability();
  try {
    const result = await invoke('debugger_start', { projectPath: state.projectPath, breakpoints: debuggerBreakpointSets(), target });
    appendDebugOutput(`Debugger attached to ${result.executable}`, 'success');
    appendDebugOutput(`Adapter: ${result.adapter}`, 'muted');
  } catch (error) {
    state.debugger.running = false;
    state.debugger.stopped = false;
    state.debugger.threads = [];
    renderDebugThreads();
    els.debuggerStatus.textContent = 'DEBUGGER: READY';
    els.debuggerDetail.textContent = 'START FAILED';
    els.commandReadout.textContent = 'DEBUG START FAILED';
    appendDebugOutput(String(error), 'error');
    updateMenuAvailability();
  }
}

async function debuggerCommand(action) {
  if (!state.debugger.running) return;
  const map = { continue: 'debugger_continue', pause: 'debugger_pause', next: 'debugger_next', 'step-in': 'debugger_step_in', 'step-out': 'debugger_step_out' };
  const command = map[action];
  if (!command) return;
  try {
    await invoke(command, { threadId: state.debugger.threadId });
    if (action !== 'pause') {
      state.debugger.stopped = false;
      resetDebugInspection();
      els.debuggerStatus.textContent = 'DEBUGGER: RUNNING';
      els.debuggerDetail.textContent = action === 'continue' ? 'CONTINUING' : `STEP ${action.toUpperCase()}`;
    }
    updateMenuAvailability();
  } catch (error) {
    appendDebugOutput(`${action}: ${error}`, 'error');
  }
}

async function restartDebugging() {
  if (!state.debugger.running) return;
  try {
    appendDebugOutput('Restarting debuggee…', 'stage');
    await invoke('debugger_restart');
    state.debugger.stopped = false;
    state.debugger.threadId = null;
    state.debugger.threads = [];
    resetDebugInspection();
    els.debuggerStatus.textContent = 'DEBUGGER: RUNNING';
    els.debuggerDetail.textContent = 'RESTARTING PROGRAM';
    els.commandReadout.textContent = 'DEBUG · RESTARTING';
    updateMenuAvailability();
  } catch (error) {
    appendDebugOutput(`Restart failed: ${error}`, 'error');
  }
}

async function runDebugConsole(event) {
  event.preventDefault();
  const expression = els.debugConsoleInput.value.trim();
  if (!expression || !state.debugger.running || !state.debugger.stopped) return;
  els.debugConsoleInput.value = '';
  state.debugger.consoleHistory.push(expression);
  appendDebugOutput(`› ${expression}`, 'debug-console-command');
  try {
    const result = await invoke('debugger_repl', { expression, frameId: state.debugger.selectedFrameId });
    appendDebugOutput(result.typeName ? `${result.result}  ·  ${result.typeName}` : result.result, 'debug-console-result');
    await refreshWatches();
  } catch (error) {
    appendDebugOutput(String(error), 'error');
  }
}

async function stopDebugging() {
  if (!state.debugger.running) return;
  try { await invoke('debugger_stop'); } catch (error) { appendDebugOutput(`Stop debugger: ${error}`, 'error'); }
  state.debugger.running = false;
  state.debugger.stopped = false;
  state.debugger.threadId = null;
  state.debugger.threads = [];
  resetDebugInspection();
  els.debuggerStatus.textContent = state.debugger.available ? 'DEBUGGER: READY' : 'DEBUGGER: NOT FOUND';
  els.debuggerDetail.textContent = state.debugger.available ? `${state.debugger.adapter} READY` : 'LLDB DAP NOT FOUND';
  els.commandReadout.textContent = state.projectPath ? 'PROJECT READY' : 'SELECT A PROJECT TO BEGIN';
  updateMenuAvailability();
}

async function handleDebugAction(action) {
  if (action === 'start') return startDebugging();
  if (action === 'stop') return stopDebugging();
  if (action === 'restart') return restartDebugging();
  return debuggerCommand(action);
}

function setConsoleView(view) {
  if (!['build', 'problems', 'debug'].includes(view)) view = 'build';
  state.consoleView = view;
  document.querySelectorAll('.console-view').forEach((button) => button.classList.toggle('active', button.dataset.consoleView === view));
  els.output.hidden = view !== 'build';
  $('#problems-pane').hidden = view !== 'problems';
  els.debugPane.hidden = view !== 'debug';
  els.buildModeTabs.hidden = view !== 'build';
  els.consoleTitle.textContent = view === 'build' ? 'BUILD BAY' : view === 'problems' ? 'RUST PROBLEMS' : 'DEBUG WORKBENCH';
}

function appendTerminalChunk(stream, data) {
  const span = document.createElement('span');
  span.className = stream === 'stderr' ? 'terminal-stderr' : stream === 'input' ? 'terminal-input-echo' : stream === 'system' ? 'terminal-system' : 'terminal-stdout';
  span.textContent = data;
  els.terminalScreen.appendChild(span);
  els.terminalScreen.scrollTop = els.terminalScreen.scrollHeight;
  if (state.tutorial.active && stream === 'stdout') {
    state.tutorial.runOutput += data;
  }
}

function clearTerminal() {
  els.terminalScreen.innerHTML = '';
  if (state.tutorial.active) state.tutorial.runOutput = '';
}

function positionTerminalWindow() {
  if (state.terminalPositioned) return;
  const width = Math.min(780, Math.max(560, window.innerWidth - 120));
  const left = Math.max(20, Math.round((window.innerWidth - width) / 2));
  const top = Math.max(70, Math.round((window.innerHeight - 430) / 2));
  els.terminalWindow.style.width = `${width}px`;
  els.terminalWindow.style.left = `${left}px`;
  els.terminalWindow.style.top = `${top}px`;
  state.terminalPositioned = true;
}

function showTerminalWindow({ focus = true } = {}) {
  if (!state.projectPath) return;
  positionTerminalWindow();
  els.terminalWindow.hidden = false;
  state.terminalVisible = true;
  els.terminalWindowProject.textContent = pathBase(state.projectPath).toUpperCase();
  els.terminalWindowProject.title = state.projectPath;
  if (focus) requestAnimationFrame(() => state.terminalRunning ? els.terminalInput.focus() : els.terminalScreen.focus());
}

function hideTerminalWindow() {
  els.terminalWindow.hidden = true;
  state.terminalVisible = false;
  if (state.currentFile) requestAnimationFrame(() => els.editor.focus());
}

function setupTerminalDragging() {
  let drag = null;
  els.terminalDragHandle.addEventListener('pointerdown', (event) => {
    if (event.target.closest('button')) return;
    const rect = els.terminalWindow.getBoundingClientRect();
    drag = { x: event.clientX, y: event.clientY, left: rect.left, top: rect.top };
    els.terminalDragHandle.setPointerCapture(event.pointerId);
  });
  els.terminalDragHandle.addEventListener('pointermove', (event) => {
    if (!drag) return;
    const nextLeft = Math.max(0, Math.min(window.innerWidth - 220, drag.left + event.clientX - drag.x));
    const nextTop = Math.max(0, Math.min(window.innerHeight - 80, drag.top + event.clientY - drag.y));
    els.terminalWindow.style.left = `${nextLeft}px`;
    els.terminalWindow.style.top = `${nextTop}px`;
  });
  const finish = () => { drag = null; };
  els.terminalDragHandle.addEventListener('pointerup', finish);
  els.terminalDragHandle.addEventListener('pointercancel', finish);
}

async function startTerminalRun() {
  if (!state.projectPath || state.terminalRunning || state.buildRunning || state.debugger.running) return;
  if (!await saveAllDirtyTabs()) return;
  showTerminalWindow({ focus: false });
  clearTerminal();
  clearOutput();
  setConsoleView('build');
  appendFriendly('stage', `Preparing ${pathBase(state.projectPath)} to run…`);
  state.terminalRunning = true;
  state.terminalEnded = false;
  state.tutorial.runSuccess = null;
  els.stopTerminal.disabled = true;
  els.terminalInput.disabled = true;
  $('.terminal-send').disabled = true;
  els.buildStatus.textContent = 'BUILDING FOR RUN';
  els.commandReadout.textContent = 'RUN · PREPARING EXECUTABLE';
  updateMenuAvailability();

  try {
    await invoke('terminal_start', { projectPath: state.projectPath, release: state.release });
  } catch (error) {
    state.terminalRunning = false;
    els.stopTerminal.disabled = true;
    els.terminalInput.disabled = true;
    $('.terminal-send').disabled = true;
    appendTerminalChunk('stderr', `Rivet could not prepare the program: ${error}\n`);
    els.buildStatus.textContent = 'PROJECT READY';
    updateMenuAvailability();
  }
}

async function sendTerminalInput(event) {
  event.preventDefault();
  if (!state.terminalRunning) return;
  const value = els.terminalInput.value;
  els.terminalInput.value = '';
  appendTerminalChunk('input', `› ${value}\n`);
  try {
    await invoke('terminal_write', { data: `${value}\n` });
  } catch (error) {
    appendTerminalChunk('stderr', `Rivet input error: ${error}\n`);
  }
}

async function stopTerminal() {
  if (!state.terminalRunning) return;
  try {
    await invoke('terminal_stop');
  } catch (error) {
    appendTerminalChunk('stderr', `\nCould not stop program: ${error}\n`);
  }
}

function dismissFinishedTerminal() {
  if (!state.terminalEnded) return;
  state.terminalEnded = false;
  hideTerminalWindow();
}

async function loadBrowserRoots() {
  try {
    state.browserRoots = await invoke('filesystem_roots');
    els.browserRoots.innerHTML = state.browserRoots.map((root) => `<button type="button" class="browser-root" data-path="${escapeHtml(root.path)}"><span class="root-icon">▣</span><span>${escapeHtml(root.label)}</span></button>`).join('');
    els.browserRoots.querySelectorAll('.browser-root').forEach((button) => button.addEventListener('click', () => browseTo(button.dataset.path)));
  } catch (error) {
    showBrowserError(`Could not list filesystem locations: ${error}`);
  }
}

async function openFileBrowser(mode) {
  closeMenus();
  state.browserMode = mode;
  state.browserSelectedPath = '';
  state.browserSelectedKind = '';
  els.browserNewFolderRow.hidden = true;
  els.browserSaveRow.hidden = mode !== 'save-as';
  els.browserNewProjectRow.hidden = mode !== 'new-project';

  if (mode === 'save-as') {
    if (!state.projectPath) return;
    els.browserTitle.textContent = 'SAVE PROJECT AS';
    els.browserConfirm.textContent = 'SAVE COPY';
    els.browserProjectName.value = `${pathBase(state.projectPath)}-copy`;
  } else if (mode === 'new-project') {
    els.browserTitle.textContent = 'NEW RUST PROJECT';
    els.browserConfirm.textContent = 'CREATE PROJECT';
    els.newProjectName.value = '';
    els.newProjectVersion.value = '0.0.1';
  } else {
    els.browserTitle.textContent = 'OPEN PROJECT';
    els.browserConfirm.textContent = 'OPEN PROJECT';
  }

  els.browserDialog.showModal();
  await loadBrowserRoots();
  let startingPath = state.projectPath || '';
  if ((mode === 'save-as' || mode === 'new-project') && state.projectPath) startingPath = pathParent(state.projectPath);
  if (!startingPath) {
    try { startingPath = await invoke('default_browse_path'); } catch { startingPath = ''; }
  }
  await browseTo(startingPath);
  if (mode === 'new-project') requestAnimationFrame(() => els.newProjectName.focus());
}

async function browseTo(path) {
  if (!path) return;
  state.browserSelectedPath = '';
  state.browserSelectedKind = '';
  els.browserStatus.classList.remove('error');
  els.browserStatus.textContent = 'Reading folder…';
  try {
    const listing = await invoke('browse_directory', { path });
    state.browserPath = listing.current_path;
    state.browserParent = listing.parent_path;
    els.browserPath.value = listing.current_path;
    els.browserUp.disabled = !listing.parent_path;
    setLamp(els.browserCargoLamp, listing.is_cargo_project);
    els.newProjectDestination.textContent = listing.current_path;

    if (state.browserMode === 'open') {
      els.browserStatus.textContent = listing.is_cargo_project ? 'Cargo.toml found — this folder can be opened as a project.' : 'Choose a folder containing Cargo.toml.';
    } else if (state.browserMode === 'new-project') {
      els.browserStatus.textContent = 'Choose the parent folder where Rivet should create the new project.';
    } else {
      els.browserStatus.textContent = 'Choose where Rivet should create the project copy.';
    }
    renderBrowserEntries(listing.entries);
  } catch (error) {
    showBrowserError(String(error));
  }
}

function renderBrowserEntries(entries) {
  if (!entries.length) {
    els.browserList.innerHTML = '<div class="browser-empty">This folder is empty.</div>';
    return;
  }

  els.browserList.innerHTML = entries.map((entry) => `
    <button type="button" class="browser-entry ${entry.kind}" data-path="${escapeHtml(entry.path)}" data-kind="${entry.kind}">
      <span class="browser-entry-name"><i>${entry.kind === 'folder' ? '▸' : '·'}</i>${escapeHtml(entry.name)}</span>
      <span class="browser-entry-type">${entry.kind === 'folder' ? 'FOLDER' : fileType(entry.name)}</span>
    </button>`).join('');

  els.browserList.querySelectorAll('.browser-entry').forEach((button) => {
    button.addEventListener('click', () => {
      els.browserList.querySelectorAll('.browser-entry.selected').forEach((row) => row.classList.remove('selected'));
      button.classList.add('selected');
      state.browserSelectedPath = button.dataset.path;
      state.browserSelectedKind = button.dataset.kind;
      if (button.dataset.kind === 'folder') {
        els.browserStatus.classList.remove('error');
        if (state.browserMode === 'open') els.browserStatus.textContent = `Selected folder: ${pathBase(button.dataset.path)}`;
        else if (state.browserMode === 'new-project') {
          els.browserStatus.textContent = `New project parent: ${pathBase(button.dataset.path)}`;
          els.newProjectDestination.textContent = button.dataset.path;
        } else els.browserStatus.textContent = `Destination folder: ${pathBase(button.dataset.path)}`;
      }
    });
    button.addEventListener('dblclick', () => {
      if (button.dataset.kind === 'folder') browseTo(button.dataset.path);
    });
  });
}

function fileType(name) {
  return (name.includes('.') ? name.split('.').at(-1).toUpperCase() : 'FILE').slice(0, 8) || 'FILE';
}

function showBrowserError(message) {
  els.browserStatus.classList.add('error');
  els.browserStatus.textContent = message;
  setLamp(els.browserCargoLamp, false);
}

async function confirmBrowserSelection() {
  const chosenDirectory = state.browserSelectedKind === 'folder' ? state.browserSelectedPath : state.browserPath;
  if (!chosenDirectory) return;

  if (state.browserMode === 'open') {
    await openProjectPath(chosenDirectory);
    return;
  }

  if (state.browserMode === 'new-project') {
    const projectName = els.newProjectName.value.trim();
    const version = els.newProjectVersion.value.trim() || '0.0.1';
    if (!projectName) {
      showBrowserError('Enter a project name.');
      return;
    }
    els.browserConfirm.disabled = true;
    els.browserStatus.classList.remove('error');
    els.browserStatus.textContent = 'Forging new Cargo project…';
    try {
      const newPath = await invoke('create_project', { destinationParent: chosenDirectory, projectName, version });
      const opened = await openProjectPath(newPath, { keepBrowserOpen: true, created: true });
      if (opened) {
        els.browserDialog.close();
        appendFriendly('success', `Hello World project created at ${newPath}`);
      }
    } catch (error) {
      showBrowserError(String(error));
    } finally {
      els.browserConfirm.disabled = false;
    }
    return;
  }

  if (!state.projectPath) return;
  const name = els.browserProjectName.value.trim();
  if (!name) {
    showBrowserError('Enter a name for the copied project.');
    return;
  }
  if (!await saveAllDirtyTabs()) return;
  els.browserConfirm.disabled = true;
  els.browserStatus.classList.remove('error');
  els.browserStatus.textContent = 'Copying project…';
  try {
    const newPath = await invoke('save_project_as', { projectPath: state.projectPath, destinationParent: chosenDirectory, projectName: name });
    const opened = await openProjectPath(newPath, { keepBrowserOpen: true });
    if (opened) {
      els.browserDialog.close();
      appendFriendly('success', `Project copy created: ${newPath}`);
    }
  } catch (error) {
    showBrowserError(String(error));
  } finally {
    els.browserConfirm.disabled = false;
  }
}

async function createBrowserFolder() {
  const folderName = els.browserNewFolderName.value.trim();
  if (!folderName || !state.browserPath) return;
  try {
    const newPath = await invoke('create_directory', { parentPath: state.browserPath, folderName });
    els.browserNewFolderRow.hidden = true;
    els.browserNewFolderName.value = '';
    await browseTo(newPath);
  } catch (error) {
    showBrowserError(String(error));
  }
}

async function loadTutorialData() {
  if (!state.tutorial.catalog) state.tutorial.catalog = await invoke('tutorial_catalog');
  state.tutorial.progress = await invoke('tutorial_progress');
}

function lessonProgress(lessonId) {
  return state.tutorial.progress?.lessons?.[lessonId] || { completed_steps: 0, completed: false, checkpoint_source: '' };
}

function tutorialCapabilitySummary() {
  const completed = (state.tutorial.catalog?.beginner || []).filter((lesson) => lessonProgress(lesson.id).completed);
  if (!completed.length) return 'Start anywhere. Rivet tracks capabilities, not grades.';
  const skills = completed.map((lesson) => lesson.skill);
  return `Comfortable so far: ${skills.join(', ')}.`;
}

function renderTutorialHome() {
  const beginner = state.tutorial.catalog?.beginner || [];
  els.tutorialBeginnerMeta.textContent = `${beginner.length} LESSONS · NEW TO RUST`;
  els.tutorialBeginnerLessons.innerHTML = beginner.map((lesson, index) => {
    const progress = lessonProgress(lesson.id);
    const status = progress.completed ? 'COMFORTABLE' : progress.completed_steps > 0 ? 'LEARNING' : 'NOT STARTED';
    const progressText = progress.completed ? `${lesson.steps.length}/${lesson.steps.length}` : `${Math.min(progress.completed_steps, lesson.steps.length)}/${lesson.steps.length}`;
    return `<button type="button" class="tutorial-lesson-card" data-lesson-id="${escapeHtml(lesson.id)}"><span class="tutorial-lesson-number">${String(index + 1).padStart(2, '0')}</span><span class="tutorial-lesson-copy"><strong>${escapeHtml(lesson.title)}</strong><small>${escapeHtml(lesson.summary)}</small></span><span class="tutorial-lesson-state ${progress.completed ? 'comfortable' : progress.completed_steps ? 'learning' : ''}">${status}<b>${progressText}</b></span></button>`;
  }).join('');
  els.tutorialBeginnerLessons.querySelectorAll('[data-lesson-id]').forEach((button) => button.addEventListener('click', () => startTutorialLesson(button.dataset.lessonId)));
  els.tutorialAdvancedTopics.innerHTML = (state.tutorial.catalog?.advanced_topics || []).map((topic) => `<div><span>◆</span>${escapeHtml(topic)}</div>`).join('');
  els.tutorialCapabilitySummary.textContent = tutorialCapabilitySummary();
}

async function openTutorialHome() {
  closeMenus();
  try {
    await loadTutorialData();
    renderTutorialHome();
    els.tutorialDialog.showModal();
  } catch (error) {
    showInfo('TUTORIAL ERROR', `<p class="info-error">Rivet could not load the tutorial: ${escapeHtml(String(error))}</p>`);
  }
}

function currentTutorialStep() {
  return state.tutorial.lesson?.steps?.[state.tutorial.stepIndex] || null;
}

function renderTutorialPanel() {
  const lesson = state.tutorial.lesson;
  const step = currentTutorialStep();
  if (!state.tutorial.active || !lesson || !step) return;
  els.tutorialPanel.hidden = false;
  els.shell.classList.add('tutorial-active');
  els.tutorialCourseLabel.textContent = lesson.course.toUpperCase();
  els.tutorialLessonTitle.textContent = lesson.title;
  els.tutorialStepCounter.textContent = `STEP ${state.tutorial.stepIndex + 1} / ${lesson.steps.length}`;
  els.tutorialStepTitle.textContent = step.title;
  els.tutorialExplanation.textContent = step.explanation;
  els.tutorialObjective.textContent = step.objective;
  const exampleParts = Array.isArray(step.example_parts) ? step.example_parts : [];
  const hasExample = Boolean(step.example_code);
  els.tutorialExample.hidden = !hasExample;
  els.tutorialExampleCode.textContent = step.example_code || '';
  els.tutorialExampleParts.innerHTML = hasExample
    ? exampleParts.map((part) => `<div><code>${escapeHtml(part.token)}</code><span>${escapeHtml(part.meaning)}</span></div>`).join('')
    : '';
  els.tutorialLearnMoreText.textContent = step.learn_more_text;
  state.tutorial.stepComplete = false;
  state.tutorial.lessonComplete = false;
  els.tutorialNext.hidden = true;
  els.tutorialNext.textContent = 'NEXT STEP →';
  els.tutorialReturn.hidden = false;
  els.tutorialHomeButton.hidden = false;
  els.tutorialLearnMore.hidden = false;
  els.tutorialLearnMoreText.hidden = true;
  els.tutorialLearnMore.textContent = 'LEARN MORE';
  const flexibleChallenge = step.id.toLowerCase().includes('challenge');
  els.tutorialFeedback.textContent = flexibleChallenge
    ? '◆ Multiple solutions accepted. Rivet checks the concept and result, not your exact names or layout.'
    : step.run_required
      ? 'When the code is ready, use Rivet’s normal Run button.'
      : 'Write directly in the real editor. Rivet will recognize the objective.';
  els.tutorialFeedback.classList.remove('success');
  els.tutorialExperimentNote.hidden = true;
}

async function startTutorialLesson(lessonId, { reset = false } = {}) {
  try {
    await loadTutorialData();
    const lesson = state.tutorial.catalog.beginner.find((item) => item.id === lessonId);
    if (!lesson) throw new Error('Unknown tutorial lesson.');
    const progress = lessonProgress(lessonId);
    const path = await invoke('tutorial_prepare_lesson', { lessonId, reset });
    const tutorialWasAlreadyActive = state.tutorial.active;
    if (!tutorialWasAlreadyActive) {
      state.tutorial.previousCargoView = state.view.cargo;
      state.tutorial.previousLiveCheck = state.liveCheck;
    }
    state.tutorial.active = true;
    state.liveCheck = true;
    updateMenuAvailability();
    state.tutorial.lesson = lesson;
    state.tutorial.stepIndex = reset ? 0 : Math.min(progress.completed_steps || 0, Math.max(0, lesson.steps.length - 1));
    state.tutorial.checkpoint = reset ? '' : (progress.checkpoint_source || '');
    state.tutorial.runOutput = '';
    state.tutorial.runSuccess = null;
    state.tutorial.stepComplete = false;
    state.tutorial.lessonComplete = false;
    state.tutorial.advancing = false;
    els.tutorialDialog.close();
    if (state.view.cargo) setViewPanel('cargo', false);
    const opened = await openProjectPath(path, { created: false, preserveTutorial: true });
    if (!opened) {
      state.tutorial.active = false;
      state.liveCheck = state.tutorial.previousLiveCheck;
      updateMenuAvailability();
      if (state.tutorial.previousCargoView && !state.view.cargo) setViewPanel('cargo', true);
      return;
    }
    state.tutorial.checkpoint = state.tutorial.checkpoint || els.editor.value;
    if (progress.completed && !reset) {
      renderTutorialCompletion();
    } else {
      renderTutorialPanel();
      scheduleTutorialEvaluation(200);
    }
  } catch (error) {
    showInfo('TUTORIAL ERROR', `<p class="info-error">${escapeHtml(String(error))}</p>`);
  }
}

async function persistTutorialProgress(completed = false) {
  if (!state.tutorial.lesson) return;
  const completedSteps = completed ? state.tutorial.lesson.steps.length : state.tutorial.stepIndex;
  await invoke('tutorial_set_progress', {
    lessonId: state.tutorial.lesson.id,
    completedSteps,
    completed,
    checkpointSource: state.tutorial.checkpoint || els.editor.value,
  });
  state.tutorial.progress = await invoke('tutorial_progress');
}

function scheduleTutorialEvaluation(delay = 320) {
  if (!state.tutorial.active || state.tutorial.advancing || state.tutorial.stepComplete || state.tutorial.lessonComplete) return;
  clearTimeout(state.tutorialEvalTimer);
  state.tutorialEvalTimer = setTimeout(() => evaluateTutorialStep(), delay);
}

async function evaluateTutorialStep() {
  if (!state.tutorial.active || state.tutorial.advancing || state.tutorial.stepComplete || state.tutorial.lessonComplete || !state.tutorial.lesson || !currentTutorialStep()) return;
  const diagnosticCodes = state.diagnostics.map((item) => item.code).filter(Boolean);
  const diagnosticMessages = state.diagnostics.map((item) => `${item.message || ''} ${item.label || ''}`);
  const diagnosticLevels = state.diagnostics.map((item) => item.level || '').filter(Boolean);
  try {
    const result = await invoke('tutorial_evaluate', {
      request: {
        lesson_id: state.tutorial.lesson.id,
        step_index: state.tutorial.stepIndex,
        source: els.editor.value,
        run_output: state.tutorial.runOutput,
        run_success: state.tutorial.runSuccess,
        diagnostic_codes: diagnosticCodes,
        diagnostic_messages: diagnosticMessages,
        diagnostic_levels: diagnosticLevels,
      },
    });
    if (result.complete) {
      const lesson = state.tutorial.lesson;
      const isFinalStep = state.tutorial.stepIndex >= lesson.steps.length - 1;
      state.tutorial.stepComplete = true;
      els.tutorialFeedback.textContent = isFinalStep
        ? '✓ Objective complete. Re-read anything you want, then finish the lesson when you are ready.'
        : '✓ Objective complete. Re-read anything you want, then continue when you are ready.';
      els.tutorialFeedback.classList.add('success');
      els.tutorialExperimentNote.hidden = true;
      els.tutorialNext.hidden = false;
      els.tutorialNext.textContent = isFinalStep ? 'COMPLETE LESSON →' : 'NEXT STEP →';
      return;
    }
    els.tutorialFeedback.classList.remove('success');
    const step = currentTutorialStep();
    const flexibleChallenge = step.id.toLowerCase().includes('challenge');
    els.tutorialFeedback.textContent = step.run_required && state.tutorial.runSuccess == null
      ? 'Code is yours to experiment with. Run it when you want Rivet to verify the result.'
      : flexibleChallenge
        ? 'Not quite there yet. Multiple solutions are valid — focus on the requested behavior and concept rather than matching Rivet’s example exactly.'
        : result.feedback;
    els.tutorialExperimentNote.hidden = els.editor.value === state.tutorial.checkpoint;
  } catch (error) {
    els.tutorialFeedback.textContent = `Tutorial tracker could not check this step: ${error}`;
  }
}

function nextTutorialLesson() {
  const lessons = state.tutorial.catalog?.beginner || [];
  const currentId = state.tutorial.lesson?.id;
  const index = lessons.findIndex((lesson) => lesson.id === currentId);
  return index >= 0 && index + 1 < lessons.length ? lessons[index + 1] : null;
}

function renderTutorialCompletion() {
  const lesson = state.tutorial.lesson;
  if (!lesson) return;
  const next = nextTutorialLesson();
  state.tutorial.stepComplete = false;
  state.tutorial.lessonComplete = true;
  state.tutorial.advancing = false;
  els.tutorialPanel.hidden = false;
  els.shell.classList.add('tutorial-active');
  els.tutorialCourseLabel.textContent = lesson.course.toUpperCase();
  els.tutorialLessonTitle.textContent = lesson.title;
  els.tutorialStepCounter.textContent = 'LESSON COMPLETE';
  els.tutorialStepTitle.textContent = `${lesson.skill}: Comfortable`;
  els.tutorialExplanation.textContent = 'You completed this lesson by writing and running real Rust code. You can keep experimenting in this project, continue immediately, or return to the tutorial home.';
  els.tutorialExample.hidden = true;
  els.tutorialExampleCode.textContent = '';
  els.tutorialExampleParts.innerHTML = '';
  els.tutorialObjective.textContent = next
    ? `Ready for the next lesson: ${next.title}.`
    : 'You reached the end of the currently available Beginner lessons.';
  els.tutorialFeedback.textContent = '✓ Capability updated: Comfortable';
  els.tutorialFeedback.classList.add('success');
  els.tutorialExperimentNote.hidden = true;
  els.tutorialLearnMore.hidden = true;
  els.tutorialLearnMoreText.hidden = true;
  els.tutorialReturn.hidden = true;
  els.tutorialNext.hidden = false;
  els.tutorialNext.textContent = next ? `NEXT LESSON → ${next.title.toUpperCase()}` : 'RETURN TO TUTORIAL HOME';
  els.tutorialHomeButton.hidden = next == null;
}

async function handleTutorialNext() {
  if (state.tutorial.stepComplete && !state.tutorial.lessonComplete) {
    await advanceTutorialStep();
    return;
  }
  await goToNextTutorialLesson();
}

async function goToNextTutorialLesson() {
  const next = nextTutorialLesson();
  if (next) {
    await startTutorialLesson(next.id);
    return;
  }
  await openTutorialHome();
}

async function returnToTutorialHome() {
  await openTutorialHome();
}

async function advanceTutorialStep() {
  const lesson = state.tutorial.lesson;
  if (!lesson) return;
  state.tutorial.advancing = true;
  if (!await saveCurrentFile({ announce: false })) {
    state.tutorial.advancing = false;
    els.tutorialFeedback.textContent = 'Rivet could not save the lesson checkpoint. Fix the save problem before continuing.';
    return;
  }
  state.tutorial.checkpoint = els.editor.value;
  state.tutorial.runOutput = '';
  state.tutorial.runSuccess = null;
  state.tutorial.stepComplete = false;
  const nextIndex = state.tutorial.stepIndex + 1;
  if (nextIndex >= lesson.steps.length) {
    await invoke('tutorial_set_progress', {
      lessonId: lesson.id,
      completedSteps: lesson.steps.length,
      completed: true,
      checkpointSource: state.tutorial.checkpoint,
    });
    state.tutorial.progress = await invoke('tutorial_progress');
    state.tutorial.advancing = false;
    renderTutorialCompletion();
    return;
  }
  state.tutorial.stepIndex = nextIndex;
  await invoke('tutorial_set_progress', {
    lessonId: lesson.id,
    completedSteps: nextIndex,
    completed: false,
    checkpointSource: state.tutorial.checkpoint,
  });
  state.tutorial.progress = await invoke('tutorial_progress');
  state.tutorial.advancing = false;
  renderTutorialPanel();
  scheduleTutorialEvaluation(150);
}

async function returnToTutorialCheckpoint() {
  if (!state.tutorial.active || !state.currentFile || !state.tutorial.checkpoint) return;
  const tab = activeTab();
  if (!tab) return;
  tab.content = state.tutorial.checkpoint;
  tab.dirty = true;
  setEditorFromTab(tab);
  await saveCurrentFile({ announce: false });
  state.tutorial.runOutput = '';
  state.tutorial.runSuccess = null;
  state.tutorial.stepComplete = false;
  state.tutorial.lessonComplete = false;
  els.tutorialNext.hidden = true;
  els.tutorialExperimentNote.hidden = true;
  els.tutorialFeedback.textContent = 'Checkpoint restored. Continue the current activity from here.';
  await runDiagnostics({ silent: true, force: true });
  scheduleTutorialEvaluation(150);
}

function exitTutorialMode() {
  state.tutorial.active = false;
  state.tutorial.lesson = null;
  state.tutorial.runOutput = '';
  state.tutorial.runSuccess = null;
  state.tutorial.stepComplete = false;
  state.tutorial.lessonComplete = false;
  clearTimeout(state.tutorialEvalTimer);
  els.tutorialPanel.hidden = true;
  els.shell.classList.remove('tutorial-active');
  state.liveCheck = state.tutorial.previousLiveCheck;
  updateMenuAvailability();
  if (state.tutorial.previousCargoView && !state.view.cargo) setViewPanel('cargo', true);
}

function setViewPanel(panel, visible) {
  state.view[panel] = visible;
  els.shell.classList.toggle(`hide-${panel}`, !visible);
  document.querySelectorAll(`[data-check="${panel}"]`).forEach((check) => { check.textContent = visible ? '✓' : ''; });
  if (panel === 'build') els.buildConsole.hidden = !state.projectPath || !visible;
}

function resetLayout({ resetBuildBay = true } = {}) {
  setViewPanel('project', true);
  setViewPanel('cargo', true);
  setViewPanel('build', true);
  if (resetBuildBay) resetBuildBayHeight();
}

async function runEditCommand(action) {
  const editor = els.editor;
  editor.focus();
  if (action === 'select-all') {
    editor.setSelectionRange(0, editor.value.length);
    return;
  }
  if (action === 'paste') {
    try {
      const text = await navigator.clipboard.readText();
      editor.setRangeText(text, editor.selectionStart, editor.selectionEnd, 'end');
      markEditorChanged();
      return;
    } catch {
      document.execCommand('paste');
      return;
    }
  }
  const command = { undo: 'undo', redo: 'redo', cut: 'cut', copy: 'copy' }[action];
  if (command) document.execCommand(command);
  if (['cut', 'undo', 'redo'].includes(action)) requestAnimationFrame(markEditorChanged);
}

function isRustEditorContext() {
  return Boolean(
    state.projectPath &&
    state.currentFile &&
    state.currentFile.toLowerCase().endsWith('.rs') &&
    !els.editor.readOnly
  );
}

function lspPositionAt(text, offset) {
  const before = text.slice(0, Math.max(0, offset));
  const line = (before.match(/\n/g) || []).length;
  const lastBreak = before.lastIndexOf('\n');
  return { line, character: before.length - lastBreak - 1 };
}

function offsetFromLspPosition(text, position) {
  const targetLine = Math.max(0, Number(position?.line || 0));
  const targetCharacter = Math.max(0, Number(position?.character || 0));
  let line = 0;
  let offset = 0;
  while (line < targetLine && offset < text.length) {
    const next = text.indexOf('\n', offset);
    if (next === -1) return text.length;
    offset = next + 1;
    line += 1;
  }
  const end = text.indexOf('\n', offset);
  const lineEnd = end === -1 ? text.length : end;
  return Math.min(offset + targetCharacter, lineEnd);
}

function completionPrefixAt(text, offset) {
  const before = text.slice(0, offset);
  const match = before.match(/[A-Za-z_][A-Za-z0-9_]*$/);
  const prefix = match?.[0] || '';
  const start = offset - prefix.length;
  const previous = start > 0 ? text[start - 1] : '';
  const memberAccess = previous === '.' || before.endsWith('::') || /[.:]$/.test(before);
  return { prefix, start, memberAccess };
}

function closeCompletionUi({ dismissWord = false } = {}) {
  state.completer.requestToken += 1;
  state.completer.visible = false;
  state.completer.items = [];
  state.completer.selected = 0;
  els.codeCompleter.hidden = true;
  els.signatureHelp.hidden = true;
  state.completer.signatureVisible = false;
  if (dismissWord) state.completer.dismissedThroughWord = true;
}

function caretPopupPoint() {
  const text = els.editor.value;
  const position = els.editor.selectionStart;
  const before = text.slice(0, position);
  const line = (before.match(/\n/g) || []).length;
  const lastBreak = before.lastIndexOf('\n');
  const column = before.length - lastBreak - 1;
  const style = getComputedStyle(els.editor);
  const fontSize = parseFloat(style.fontSize) || 13;
  const lineHeight = parseFloat(style.lineHeight) || fontSize * 1.55;
  const canvas = caretPopupPoint.canvas || (caretPopupPoint.canvas = document.createElement('canvas'));
  const context = canvas.getContext('2d');
  context.font = style.font;
  const charWidth = context.measureText('M').width || fontSize * 0.62;
  const left = els.editor.offsetLeft + (parseFloat(style.paddingLeft) || 14) + column * charWidth - els.editor.scrollLeft;
  const top = els.editor.offsetTop + (parseFloat(style.paddingTop) || 12) + (line + 1) * lineHeight - els.editor.scrollTop;
  return { left, top };
}

function positionCompletionUi() {
  const point = caretPopupPoint();
  const wrap = els.editor.parentElement;
  const margin = 8;
  const gutter = Math.min(52, Math.max(0, wrap.clientWidth - 280));
  const usableWidth = Math.max(180, wrap.clientWidth - gutter - (margin * 2));
  const popupWidth = Math.min(650, usableWidth, Math.max(320, wrap.clientWidth * 0.52));

  // Completer positioning rule: never flip above or cover the line being typed.
  // If the caret is near the bottom, the editor simply clips/shrinks the popup instead.
  const top = Math.max(margin, point.top + 4);
  const below = Math.max(0, wrap.clientHeight - top - margin);
  const popupHeight = Math.max(44, Math.min(250, below || 44));

  let left = point.left;
  if (left + popupWidth > wrap.clientWidth - margin) left = wrap.clientWidth - popupWidth - margin;
  left = Math.max(margin, left);

  els.codeCompleter.classList.toggle('compact', popupWidth < 500);
  els.codeCompleter.style.left = `${left}px`;
  els.codeCompleter.style.top = `${top}px`;
  els.codeCompleter.style.width = `${popupWidth}px`;
  els.codeCompleter.style.height = `${popupHeight}px`;
  els.codeCompleter.style.maxHeight = `${Math.max(44, below)}px`;

  const signatureWidth = Math.max(180, Math.min(720, wrap.clientWidth - left - margin));
  const signatureTop = Math.max(margin, Math.min(point.top - 72, wrap.clientHeight - 64));
  els.signatureHelp.style.left = `${left}px`;
  els.signatureHelp.style.top = `${signatureTop}px`;
  els.signatureHelp.style.maxWidth = `${signatureWidth}px`;
}

function completionKindGlyph(kind) {
  const glyphs = {
    Method: 'M', Function: 'ƒ', Field: 'F', Variable: 'V', Module: '▣',
    Keyword: 'K', Struct: 'S', Enum: 'E', Constant: 'C', Property: 'P',
    Constructor: 'N', Reference: 'R', Trait: 'T', 'Enum Member': 'E', 'Type Parameter': 'T', Symbol: '•',
  };
  return glyphs[kind] || '•';
}

function updateCompletionDetail() {
  const item = state.completer.items[state.completer.selected];
  if (!item) return;
  els.completionDetailKind.textContent = item.kind || 'SYMBOL';
  els.completionDetailLabel.textContent = item.label || '';
  els.completionDetailSignature.textContent = item.detail || item.insertText || '';
  els.completionDetailDocs.textContent = item.documentation || 'rust-analyzer did not provide additional documentation for this item.';
  els.completionList.querySelectorAll('.completion-item').forEach((element, index) => {
    element.classList.toggle('selected', index === state.completer.selected);
    element.setAttribute('aria-selected', index === state.completer.selected ? 'true' : 'false');
  });
  els.completionList.querySelector('.completion-item.selected')?.scrollIntoView({ block: 'nearest' });
}

function renderCompletionItems(items) {
  state.completer.items = items;
  state.completer.selected = Math.min(state.completer.selected, Math.max(0, items.length - 1));
  if (!items.length) {
    els.codeCompleter.hidden = true;
    state.completer.visible = false;
    return;
  }
  els.completionList.innerHTML = items.map((item, index) => `
    <button type="button" class="completion-item ${index === state.completer.selected ? 'selected' : ''}" data-completion-index="${index}" role="option" aria-selected="${index === state.completer.selected}">
      <span class="completion-kind completion-kind-${escapeHtml((item.kind || 'symbol').toLowerCase().replaceAll(' ', '-'))}">${escapeHtml(completionKindGlyph(item.kind))}</span>
      <span class="completion-label">${escapeHtml(item.label)}</span>
      <span class="completion-type">${escapeHtml(item.kind || 'Symbol')}</span>
    </button>`).join('');
  els.completionList.querySelectorAll('.completion-item').forEach((button) => {
    button.addEventListener('pointerdown', (event) => {
      event.preventDefault();
      state.completer.selected = Number(button.dataset.completionIndex || 0);
      acceptCompletion();
    });
  });
  els.codeCompleter.hidden = false;
  state.completer.visible = true;
  positionCompletionUi();
  updateCompletionDetail();
}

function normalizeCompletionInsert(text = '') {
  return String(text)
    .replace(/\$\{\d+:([^}]*)\}/g, '$1')
    .replace(/\$\{\d+\}/g, '')
    .replace(/\$\d+/g, '');
}

function applyTextEdits(original, edits) {
  const normalized = edits.map((edit) => ({
    start: offsetFromLspPosition(original, edit.range.start),
    end: offsetFromLspPosition(original, edit.range.end),
    text: normalizeCompletionInsert(edit.newText),
    primary: Boolean(edit.primary),
  })).sort((a, b) => b.start - a.start || b.end - a.end);

  const primary = normalized.find((edit) => edit.primary);
  let caret = primary ? primary.start + primary.text.length : els.editor.selectionStart;
  if (primary) {
    for (const edit of normalized) {
      if (edit === primary) continue;
      if (edit.start <= primary.start) caret += edit.text.length - (edit.end - edit.start);
    }
  }

  let value = original;
  for (const edit of normalized) value = value.slice(0, edit.start) + edit.text + value.slice(edit.end);
  return { value, caret: Math.max(0, caret) };
}

function acceptCompletion() {
  const item = state.completer.items[state.completer.selected];
  if (!item) return false;
  const original = els.editor.value;
  const cursor = els.editor.selectionStart;
  const edits = [];
  for (const edit of item.additionalTextEdits || []) edits.push({ ...edit, primary: false });
  if (item.textEdit) {
    edits.push({ ...item.textEdit, primary: true });
    const applied = applyTextEdits(original, edits);
    els.editor.value = applied.value;
    els.editor.setSelectionRange(Math.min(applied.caret, applied.value.length), Math.min(applied.caret, applied.value.length));
  } else {
    const start = state.completer.prefixStart;
    const insert = normalizeCompletionInsert(item.insertText || item.label);
    els.editor.setRangeText(insert, start, cursor, 'end');
  }
  closeCompletionUi();
  markEditorChanged();
  els.editor.focus();
  const inserted = item.insertText || item.label || '';
  if (inserted.includes('(')) requestSignatureHelp();
  return true;
}

function handleCompleterKey(event) {
  if ((event.ctrlKey || event.metaKey) && event.code === 'Space') {
    event.preventDefault();
    state.completer.dismissedThroughWord = false;
    requestCodeCompletion({ manual: true });
    return true;
  }
  if (!state.completer.visible) {
    if (event.key === 'Escape' && (state.completer.signatureVisible || isRustEditorContext())) {
      event.preventDefault();
      closeCompletionUi({ dismissWord: true });
      return true;
    }
    return false;
  }
  if (event.key === 'ArrowDown') {
    event.preventDefault();
    state.completer.selected = (state.completer.selected + 1) % state.completer.items.length;
    updateCompletionDetail();
    return true;
  }
  if (event.key === 'ArrowUp') {
    event.preventDefault();
    state.completer.selected = (state.completer.selected - 1 + state.completer.items.length) % state.completer.items.length;
    updateCompletionDetail();
    return true;
  }
  if (event.key === 'Enter' || event.key === 'Tab') {
    event.preventDefault();
    acceptCompletion();
    return true;
  }
  if (event.key === 'Escape') {
    event.preventDefault();
    closeCompletionUi({ dismissWord: true });
    return true;
  }
  return false;
}

async function requestCodeCompletion({ manual = false } = {}) {
  if (manual) state.completer.dismissedThroughWord = false;
  if (!manual && state.completer.dismissedThroughWord) return;
  if (!state.completer.enabled || !state.completer.available || !isRustEditorContext()) {
    if (manual && !state.completer.available) {
      showInfo('RUST CODE ANALYZER/COMPLETER', '<p>rust-analyzer was not found. Install it with <code>rustup component add rust-analyzer</code>, then choose Tools → Refresh Toolchain.</p>');
    }
    return;
  }
  const cursor = els.editor.selectionStart;
  if (cursor !== els.editor.selectionEnd) return;
  const token = completionPrefixAt(els.editor.value, cursor);
  if (!manual && !token.memberAccess && token.prefix.length < 1) {
    els.codeCompleter.hidden = true;
    state.completer.visible = false;
    return;
  }
  state.completer.prefixStart = token.start;
  const requestToken = ++state.completer.requestToken;
  const position = lspPositionAt(els.editor.value, cursor);
  try {
    const response = await invoke('rust_completions', {
      projectPath: state.projectPath,
      path: state.currentFile,
      content: els.editor.value,
      line: position.line,
      character: position.character,
    });
    if (requestToken !== state.completer.requestToken) return;
    const current = completionPrefixAt(els.editor.value, els.editor.selectionStart);
    const needle = current.prefix.toLowerCase();
    const ranked = (response || [])
      .map((item) => {
        const key = String(item.filterText || item.label || '').toLowerCase();
        const label = String(item.label || '').toLowerCase();
        let rank = 3;
        if (!needle) rank = 0;
        else if (label.startsWith(needle)) rank = 0;
        else if (key.startsWith(needle)) rank = 1;
        else if (label.includes(needle) || key.includes(needle)) rank = 2;
        return { item, rank };
      })
      .filter(({ rank }) => rank < 3)
      .sort((a, b) => a.rank - b.rank || String(a.item.sortText || a.item.label).localeCompare(String(b.item.sortText || b.item.label)))
      .slice(0, 60)
      .map(({ item }) => item);
    state.completer.selected = 0;
    state.completer.prefixStart = current.start;
    renderCompletionItems(ranked);
  } catch (error) {
    if (requestToken !== state.completer.requestToken) return;
    els.analyzerStatus.textContent = 'ANALYZER: RETRY';
    els.analyzerStatus.title = String(error);
    closeCompletionUi();
  }
}

function scheduleCodeCompletion(event) {
  state.completer.requestToken += 1;
  if (state.completer.timer) clearTimeout(state.completer.timer);
  if (!isRustEditorContext() || !state.completer.enabled || !state.completer.available) return;
  const data = event?.data || '';

  if (state.completer.dismissedThroughWord) {
    const continuesWord = /^[A-Za-z0-9_]+$/.test(data);
    if (!data) {
      const current = completionPrefixAt(els.editor.value, els.editor.selectionStart);
      if (!current.prefix && !current.memberAccess) state.completer.dismissedThroughWord = false;
      els.codeCompleter.hidden = true;
      state.completer.visible = false;
      return;
    }
    if (continuesWord) {
      els.codeCompleter.hidden = true;
      state.completer.visible = false;
      return;
    }
    // A boundary ended the dismissed word. Do not reopen on the boundary itself;
    // the first character of the next word can trigger completion normally.
    state.completer.dismissedThroughWord = false;
    els.codeCompleter.hidden = true;
    state.completer.visible = false;
    return;
  }

  if (data === ')' || data === ';' || data === '\n' || data === ' ') {
    els.codeCompleter.hidden = true;
    state.completer.visible = false;
  }
  if (data === '(' || data === ',') {
    requestSignatureHelp();
  } else if (data === ')') {
    els.signatureHelp.hidden = true;
    state.completer.signatureVisible = false;
  }
  state.completer.timer = setTimeout(() => requestCodeCompletion(), 90);
}

async function requestSignatureHelp() {
  if (!state.completer.enabled || !state.completer.available || !isRustEditorContext()) return;
  const position = lspPositionAt(els.editor.value, els.editor.selectionStart);
  try {
    const help = await invoke('rust_signature_help', {
      projectPath: state.projectPath,
      path: state.currentFile,
      content: els.editor.value,
      line: position.line,
      character: position.character,
    });
    if (!help?.label) {
      els.signatureHelp.hidden = true;
      state.completer.signatureVisible = false;
      return;
    }
    const active = help.parameters?.[help.activeParameter] || '';
    els.signatureHelpLabel.innerHTML = active
      ? `${escapeHtml(help.label)}<span class="signature-active-parameter">ACTIVE: ${escapeHtml(active)}</span>`
      : escapeHtml(help.label);
    els.signatureHelpDocs.textContent = help.documentation || '';
    els.signatureHelp.hidden = false;
    state.completer.signatureVisible = true;
    positionCompletionUi();
  } catch {
    els.signatureHelp.hidden = true;
    state.completer.signatureVisible = false;
  }
}


function sourcePathLabel(path = '') {
  const normalized = normalizePath(path);
  const root = normalizePath(state.projectPath);
  if (root && (normalized === root || normalized.startsWith(`${root}/`))) {
    return path.replaceAll('\\', '/').slice(state.projectPath.replaceAll('\\', '/').replace(/\/$/, '').length + 1) || pathBase(path);
  }
  return path;
}

function analyzerPositionPayload() {
  const position = lspPositionAt(els.editor.value, els.editor.selectionStart);
  return {
    projectPath: state.projectPath,
    path: state.currentFile,
    content: els.editor.value,
    line: position.line,
    character: position.character,
  };
}

function requireRustIntelligence() {
  if (!isRustEditorContext()) {
    showInfo('RUST INTELLIGENCE', '<p>Open a Rust source file to use this command.</p>');
    return false;
  }
  if (!state.completer.available) {
    showInfo('RUST ANALYZER NOT FOUND', '<p>Install rust-analyzer with <code>rustup component add rust-analyzer</code>, then choose Tools → Refresh Toolchain.</p>');
    return false;
  }
  return true;
}

async function navigateToRustLocation(location) {
  if (!location?.path || !location?.range) return;
  await loadFile(location.path);
  if (normalizePath(state.currentFile) !== normalizePath(location.path)) return;
  const start = offsetFromLspPosition(els.editor.value, location.range.start);
  const end = offsetFromLspPosition(els.editor.value, location.range.end);
  els.editor.setSelectionRange(start, Math.max(start, end));
  const style = getComputedStyle(els.editor);
  const lineHeight = parseFloat(style.lineHeight) || 20;
  const targetTop = Number(location.range.start?.line || 0) * lineHeight;
  els.editor.scrollTop = Math.max(0, targetTop - Math.max(30, els.editor.clientHeight * 0.32));
  els.lines.scrollTop = els.editor.scrollTop;
  syncSyntaxScroll();
  updateBracketMatch();
  els.editor.focus();
}

async function goToDefinition() {
  if (!requireRustIntelligence()) return;
  closeCompletionUi();
  try {
    const locations = await invoke('rust_definition', analyzerPositionPayload());
    if (!locations?.length) {
      showInfo('GO TO DEFINITION', '<p>rust-analyzer did not find a definition at the current caret position.</p>');
      return;
    }
    await navigateToRustLocation(locations[0]);
  } catch (error) {
    showInfo('GO TO DEFINITION FAILED', `<p class="info-error">${escapeHtml(String(error))}</p>`);
  }
}

function renderReferences() {
  const refs = state.intelligence.references || [];
  els.referencesSummary.textContent = `${refs.length} REFERENCE${refs.length === 1 ? '' : 'S'}`;
  if (!refs.length) {
    els.referencesList.innerHTML = '<div class="intelligence-empty">No references found for this symbol.</div>';
    return;
  }
  els.referencesList.innerHTML = refs.map((location, index) => {
    const line = Number(location.range?.start?.line || 0) + 1;
    const column = Number(location.range?.start?.character || 0) + 1;
    return `<button type="button" class="intelligence-result" data-reference-index="${index}"><span class="intelligence-result-icon">R</span><span><b>${escapeHtml(pathBase(location.path))}:${line}</b><small>${escapeHtml(sourcePathLabel(location.path))} · column ${column}</small></span><span class="intelligence-jump">OPEN →</span></button>`;
  }).join('');
  els.referencesList.querySelectorAll('[data-reference-index]').forEach((button) => button.addEventListener('click', async () => {
    const location = refs[Number(button.dataset.referenceIndex || 0)];
    if (els.referencesDialog.open) els.referencesDialog.close();
    await navigateToRustLocation(location);
  }));
}

async function findReferences() {
  if (!requireRustIntelligence()) return;
  closeCompletionUi();
  try {
    state.intelligence.references = await invoke('rust_references', analyzerPositionPayload());
    renderReferences();
    if (!els.referencesDialog.open) els.referencesDialog.showModal();
  } catch (error) {
    showInfo('FIND REFERENCES FAILED', `<p class="info-error">${escapeHtml(String(error))}</p>`);
  }
}

function applyEditsToText(text, edits = []) {
  const normalized = edits.map((edit) => ({
    start: offsetFromLspPosition(text, edit.range?.start),
    end: offsetFromLspPosition(text, edit.range?.end),
    text: String(edit.newText ?? ''),
  })).sort((a, b) => b.start - a.start || b.end - a.end);
  let output = text;
  for (const edit of normalized) output = output.slice(0, edit.start) + edit.text + output.slice(edit.end);
  return output;
}

async function refreshProjectTree() {
  if (!state.projectPath) return;
  try {
    renderTree(await invoke('list_project_files', { projectPath: state.projectPath }));
  } catch (error) {
    appendFriendly('warning', `Project tree refresh failed: ${error}`);
  }
}

async function applyWorkspaceEdit(edit, label = 'Rust edit') {
  if (!edit?.editCount) return false;
  if (edit.unsupportedOperations) {
    showInfo('ACTION NOT APPLIED', `<p>This action includes ${edit.unsupportedOperations} file create/rename/delete operation${edit.unsupportedOperations === 1 ? '' : 's'} that Rivet does not apply automatically yet.</p>`);
    return false;
  }
  const affected = [];
  try {
    for (const file of edit.files || []) {
      const original = await invoke('read_text_file', { path: file.path });
      const updated = applyEditsToText(original, file.edits || []);
      if (updated === original) continue;
      await invoke('write_text_file', { path: file.path, content: updated });
      affected.push(file.path);
    }
    for (const path of affected) {
      if (state.tabs.some((tab) => normalizePath(tab.path) === normalizePath(path))) await reloadTabFromDisk(path);
    }
    await refreshProjectTree();
    if (state.liveCheck) scheduleAnalysis(120);
    appendFriendly('success', `${label} applied · ${edit.editCount} edit${edit.editCount === 1 ? '' : 's'} across ${affected.length} file${affected.length === 1 ? '' : 's'}.`);
    return true;
  } catch (error) {
    showInfo('RUST EDIT FAILED', `<p class="info-error">${escapeHtml(String(error))}</p>`);
    return false;
  }
}

async function startSemanticRename() {
  if (!requireRustIntelligence()) return;
  closeCompletionUi();
  if (!await saveAllDirtyTabs({ announce: false })) return;
  const payload = analyzerPositionPayload();
  try {
    const prepared = await invoke('rust_prepare_rename', payload);
    if (!prepared?.range) {
      showInfo('SEMANTIC RENAME', '<p>The symbol at the caret cannot be renamed.</p>');
      return;
    }
    const start = offsetFromLspPosition(els.editor.value, prepared.range.start);
    const end = offsetFromLspPosition(els.editor.value, prepared.range.end);
    const currentName = prepared.placeholder || els.editor.value.slice(start, end);
    state.intelligence.pendingRename = { ...payload, currentName };
    els.renameSymbolLabel.textContent = currentName ? `RENAME ${currentName}` : 'RUST SYMBOL';
    els.renameInput.value = currentName;
    if (!els.renameDialog.open) els.renameDialog.showModal();
    requestAnimationFrame(() => { els.renameInput.focus(); els.renameInput.select(); });
  } catch (error) {
    showInfo('SEMANTIC RENAME FAILED', `<p class="info-error">${escapeHtml(String(error))}</p>`);
  }
}

async function submitSemanticRename(event) {
  event.preventDefault();
  const pending = state.intelligence.pendingRename;
  const newName = els.renameInput.value.trim();
  if (!pending || !newName || newName === pending.currentName) {
    if (els.renameDialog.open) els.renameDialog.close();
    return;
  }
  try {
    const edit = await invoke('rust_rename', {
      projectPath: pending.projectPath,
      path: pending.path,
      content: pending.content,
      line: pending.line,
      character: pending.character,
      newName,
    });
    if (els.renameDialog.open) els.renameDialog.close();
    if (!edit?.editCount) {
      showInfo('SEMANTIC RENAME', '<p>rust-analyzer returned no rename edits.</p>');
      return;
    }
    const approved = await oxideConfirm('SEMANTIC RENAME', `Rename ${pending.currentName || 'symbol'} to ${newName}? rust-analyzer will apply ${edit.editCount} edit${edit.editCount === 1 ? '' : 's'} across ${edit.files?.length || 0} file${edit.files?.length === 1 ? '' : 's'}.`, 'RENAME');
    if (approved) await applyWorkspaceEdit(edit, `Renamed ${pending.currentName || 'symbol'} → ${newName}`);
  } catch (error) {
    showInfo('SEMANTIC RENAME FAILED', `<p class="info-error">${escapeHtml(String(error))}</p>`);
  } finally {
    state.intelligence.pendingRename = null;
  }
}

function renderCodeActions() {
  const actions = state.intelligence.codeActions || [];
  if (!actions.length) {
    els.codeActionsList.innerHTML = '<div class="intelligence-empty">No rust-analyzer actions are available at the caret.</div>';
    return;
  }
  els.codeActionsList.innerHTML = actions.map((action, index) => {
    const disabled = Boolean(action.disabledReason || !action.edit);
    return `<button type="button" class="intelligence-result code-action-result ${action.preferred ? 'preferred' : ''}" data-code-action-index="${index}" ${disabled ? 'disabled' : ''}><span class="intelligence-result-icon">${action.preferred ? '★' : '⚙'}</span><span><b>${escapeHtml(action.title)}</b><small>${escapeHtml(action.disabledReason || action.kind || 'rust-analyzer action')}</small></span><span class="intelligence-jump">${disabled ? 'UNAVAILABLE' : 'APPLY →'}</span></button>`;
  }).join('');
  els.codeActionsList.querySelectorAll('[data-code-action-index]:not(:disabled)').forEach((button) => button.addEventListener('click', async () => {
    const action = actions[Number(button.dataset.codeActionIndex || 0)];
    if (!action?.edit) return;
    if (els.codeActionsDialog.open) els.codeActionsDialog.close();
    await applyWorkspaceEdit(action.edit, action.title);
  }));
}

async function showCodeActions() {
  if (!requireRustIntelligence()) return;
  closeCompletionUi();
  if (!await saveAllDirtyTabs({ announce: false })) return;
  try {
    state.intelligence.codeActions = await invoke('rust_code_actions', analyzerPositionPayload());
    renderCodeActions();
    if (!els.codeActionsDialog.open) els.codeActionsDialog.showModal();
  } catch (error) {
    showInfo('CODE ACTIONS FAILED', `<p class="info-error">${escapeHtml(String(error))}</p>`);
  }
}

const AUTO_CLOSE_PAIRS = { '(': ')', '[': ']', '{': '}', '"': '"' };
const AUTO_CLOSE_ENDINGS = new Set(Object.values(AUTO_CLOSE_PAIRS));

function handleAutoClosePairs(event) {
  if (event.ctrlKey || event.metaKey || event.altKey || event.isComposing) return false;
  const key = event.key;
  const start = els.editor.selectionStart;
  const end = els.editor.selectionEnd;
  const next = els.editor.value[start] || '';

  if (AUTO_CLOSE_ENDINGS.has(key) && start === end && next === key) {
    event.preventDefault();
    els.editor.setSelectionRange(start + 1, start + 1);
    updateBracketMatch();
    return true;
  }

  const closing = AUTO_CLOSE_PAIRS[key];
  if (!closing) return false;
  state.completer.dismissedThroughWord = false;
  event.preventDefault();
  closeCompletionUi();
  if (start !== end) {
    const selected = els.editor.value.slice(start, end);
    els.editor.setRangeText(`${key}${selected}${closing}`, start, end, 'select');
    els.editor.setSelectionRange(start + 1, end + 1);
  } else {
    els.editor.setRangeText(`${key}${closing}`, start, end, 'end');
    els.editor.setSelectionRange(start + 1, start + 1);
  }
  markEditorChanged();
  updateBracketMatch();
  if (key === '(') requestAnimationFrame(requestSignatureHelp);
  return true;
}

function matchingBracketOffsets(text, caret) {
  const pairs = { '(': ')', '[': ']', '{': '}', ')': '(', ']': '[', '}': '{' };
  let index = caret > 0 && pairs[text[caret - 1]] ? caret - 1 : (pairs[text[caret]] ? caret : -1);
  if (index < 0) return null;
  const bracket = text[index];
  const forward = '([{'.includes(bracket);
  const match = pairs[bracket];
  let depth = 0;
  for (let cursor = index; forward ? cursor < text.length : cursor >= 0; cursor += forward ? 1 : -1) {
    const char = text[cursor];
    if (char === bracket) depth += 1;
    else if (char === match) {
      depth -= 1;
      if (depth === 0) return [index, cursor];
    }
  }
  return null;
}

function editorPointForOffset(offset) {
  const before = els.editor.value.slice(0, Math.max(0, offset));
  const line = (before.match(/\n/g) || []).length;
  const lastBreak = before.lastIndexOf('\n');
  const columnText = before.slice(lastBreak + 1);
  const style = getComputedStyle(els.editor);
  const fontSize = parseFloat(style.fontSize) || 13;
  const lineHeight = parseFloat(style.lineHeight) || fontSize * 1.55;
  const canvas = editorPointForOffset.canvas || (editorPointForOffset.canvas = document.createElement('canvas'));
  const context = canvas.getContext('2d');
  context.font = style.font;
  const width = context.measureText(columnText).width;
  const charWidth = context.measureText('M').width || fontSize * 0.62;
  return {
    left: els.editor.offsetLeft + (parseFloat(style.paddingLeft) || 14) + width - els.editor.scrollLeft,
    top: els.editor.offsetTop + (parseFloat(style.paddingTop) || 12) + line * lineHeight - els.editor.scrollTop,
    width: Math.max(6, charWidth),
    height: lineHeight,
  };
}

function updateBracketMatch() {
  const markers = [els.bracketMatchA, els.bracketMatchB];
  const matches = isRustEditorContext() && els.editor.selectionStart === els.editor.selectionEnd
    ? matchingBracketOffsets(els.editor.value, els.editor.selectionStart)
    : null;
  if (!matches) {
    markers.forEach((marker) => { marker.hidden = true; });
    return;
  }
  matches.forEach((offset, index) => {
    const point = editorPointForOffset(offset);
    const marker = markers[index];
    marker.hidden = false;
    marker.style.left = `${point.left}px`;
    marker.style.top = `${point.top}px`;
    marker.style.width = `${point.width}px`;
    marker.style.height = `${point.height}px`;
  });
}

function markEditorChanged() {
  const tab = activeTab();
  if (!tab) return;
  state.dirty = true;
  tab.dirty = true;
  tab.content = els.editor.value;
  tab.selectionStart = els.editor.selectionStart;
  tab.selectionEnd = els.editor.selectionEnd;
  updateDirty();
  updateLineNumbers();
  updateSyntaxHighlight();
  scheduleSemanticReadability();
  scheduleAnalysis();
  scheduleTutorialEvaluation();
  updateBracketMatch();
}

function cycleTab(direction = 1) {
  if (state.tabs.length < 2) return;
  syncActiveTabFromEditor();
  const current = Math.max(0, state.tabs.findIndex((tab) => normalizePath(tab.path) === normalizePath(state.activeTabPath)));
  const next = (current + direction + state.tabs.length) % state.tabs.length;
  setEditorFromTab(state.tabs[next]);
}

function showInfo(title, html) {
  els.infoTitle.textContent = title;
  els.infoBody.innerHTML = html;
  els.infoDialog.showModal();
}

function resetUpdateDialog() {
  state.updater.downloaded = 0;
  state.updater.contentLength = 0;
  els.updateProgressWrap.hidden = true;
  els.updateProgressBar.style.width = '0%';
  els.updateProgressBar.classList.remove('indeterminate');
  els.updateProgressText.textContent = 'Preparing update…';
  els.updateError.hidden = true;
  els.updateError.textContent = '';
  els.updateLater.disabled = false;
  els.updateInstall.disabled = false;
  els.updateInstall.textContent = 'DOWNLOAD & UPDATE';
}

function showUpdatePrompt(update) {
  state.updater.pending = update;
  resetUpdateDialog();
  els.updateCurrentVersion.textContent = `CURRENT B1.3.6 · BUILD ${update.currentBuildNumber || 1}`;
  els.updateNewVersion.textContent = `${update.displayVersion || `B${update.version}`} · BUILD ${update.buildNumber || 1}`;
  els.updateReleaseDate.textContent = update.date ? `Published ${update.date}` : 'A newer Rivet package is available.';
  els.updateNotes.textContent = update.body?.trim() || 'This release does not include update notes.';

  if (update.installSupported === false) {
    els.updateInstall.disabled = true;
    els.updateInstall.textContent = 'APPIMAGE AUTO-UPDATE ONLY';
    els.updateError.hidden = false;
    els.updateError.textContent = update.installHint || 'Automatic installation is not available for this Rivet package type.';
  }

  if (!els.updateDialog.open) els.updateDialog.showModal();
}

async function checkForRivetUpdates({ manual = false } = {}) {
  if (state.updater.checking || state.updater.installing) return;
  if (state.updater.pending) {
    showUpdatePrompt(state.updater.pending);
    return;
  }
  state.updater.checking = true;
  try {
    const update = await invoke('oxide_update_check');
    if (update) {
      showUpdatePrompt(update);
    } else if (manual) {
      showInfo('RIVET UPDATE', '<div class="update-status-message"><strong>Rivet is up to date.</strong><p>No newer signed Rivet package is available for this installation.</p></div>');
    }
  } catch (error) {
    const message = String(error);
    console.warn('Rivet package update check failed:', message);
    if (manual) {
      const setupHint = /pubkey|public key|signature|not configured/i.test(message)
        ? '<p>The updater signing key has not been configured for this build yet. Run <code>scripts/setup-updater.ps1</code> once before publishing package updates.</p>'
        : '<p>Rivet could not reach or validate the GitHub package feed. Your editor can continue normally.</p>';
      showInfo('UPDATE CHECK FAILED', `<div class="update-status-message"><strong>Could not check for updates.</strong>${setupHint}<p class="update-error-detail">${escapeHtml(message)}</p></div>`);
    }
  } finally {
    state.updater.checking = false;
  }
}

function updateDownloadProgress(payload = {}) {
  if (!state.updater.installing) return;
  if (payload.event === 'progress') {
    state.updater.downloaded = Number(payload.downloaded || 0);
    state.updater.contentLength = Number(payload.contentLength || 0);
    if (state.updater.contentLength > 0) {
      const percent = Math.min(100, (state.updater.downloaded / state.updater.contentLength) * 100);
      els.updateProgressBar.classList.remove('indeterminate');
      els.updateProgressBar.style.width = `${percent.toFixed(1)}%`;
      els.updateProgressText.textContent = `Downloading signed package ${formatBytes(state.updater.downloaded)} / ${formatBytes(state.updater.contentLength)} · ${Math.floor(percent)}%`;
    } else {
      els.updateProgressBar.classList.add('indeterminate');
      els.updateProgressText.textContent = `Downloading signed package · ${formatBytes(state.updater.downloaded)}`;
    }
  } else if (payload.event === 'finished') {
    els.updateProgressBar.classList.remove('indeterminate');
    els.updateProgressBar.style.width = '100%';
    els.updateProgressText.textContent = 'Download complete. Verifying package signature…';
  }
}

async function installPendingUpdate() {
  const update = state.updater.pending;
  if (!update || state.updater.installing) return;

  state.updater.installing = true;
  els.updateLater.disabled = true;
  els.updateInstall.disabled = true;
  els.updateInstall.textContent = 'DOWNLOADING…';
  els.updateProgressWrap.hidden = false;
  els.updateError.hidden = true;
  els.updateProgressBar.classList.add('indeterminate');
  state.updater.downloaded = 0;
  state.updater.contentLength = 0;
  els.updateProgressText.textContent = 'Contacting the Rivet package feed…';

  try {
    const result = await invoke('oxide_update_prepare', { version: update.version, buildNumber: update.buildNumber || 1 });
    if (!result?.helperStarted) throw new Error('Rivet Update Service did not start.');
    els.updateProgressBar.classList.remove('indeterminate');
    els.updateProgressBar.style.width = '100%';
    els.updateProgressText.textContent = 'Package signature verified. Rivet will close and apply the update…';
    els.updateInstall.textContent = 'STARTING UPDATER…';
    await new Promise((resolve) => setTimeout(resolve, 450));
    await invoke('quit_app');
  } catch (error) {
    state.updater.installing = false;
    els.updateLater.disabled = false;
    els.updateInstall.disabled = false;
    els.updateInstall.textContent = 'TRY AGAIN';
    els.updateProgressBar.classList.remove('indeterminate');
    els.updateError.hidden = false;
    els.updateError.textContent = `Update failed: ${error}`;
  }
}

function formatBytes(value) {
  const bytes = Math.max(0, Number(value) || 0);
  if (bytes < 1024) return `${bytes} B`;
  const units = ['KB', 'MB', 'GB'];
  let amount = bytes / 1024;
  let index = 0;
  while (amount >= 1024 && index < units.length - 1) {
    amount /= 1024;
    index += 1;
  }
  return `${amount.toFixed(amount >= 10 ? 1 : 2)} ${units[index]}`;
}

function showAbout() {
  showInfo('ABOUT RIVET', `<img class="about-mark about-logo" src="${rivetLogo}" alt="Rivet logo" /><div class="about-copy"><strong>Rivet</strong><span>Rust Development Environment · Beta B1.3.6 · Build 7</span><p>A cross-platform Rust-first IDE for Windows and Linux, with Cargo project management, compiler diagnostics, rust-analyzer code intelligence, LLDB/DAP debugging, signed Rivet package updates, a floating interactive Run Terminal, a 26-lesson hands-on Rust tutorial, five built-in material themes, theme-aware Semantic Readability Colors, and the composable Theme Workshop for user-created presentation recipes.</p></div>`);
}

function showShortcuts() {
  showInfo('KEYBOARD SHORTCUTS', `<div class="shortcut-grid"><span>New Project</span><kbd>Ctrl+N</kbd><span>Open Project</span><kbd>Ctrl+O</kbd><span>Save File</span><kbd>Ctrl+S</kbd><span>Close File</span><kbd>Ctrl+W</kbd><span>Switch Tab</span><kbd>Ctrl+Tab</kbd><span>Save Project As</span><kbd>Ctrl+Shift+S</kbd><span>Run</span><kbd>F5</kbd><span>Start / Continue Debugging</span><kbd>F9</kbd><span>Step Over</span><kbd>F10</kbd><span>Step Into</span><kbd>F11</kbd><span>Step Out</span><kbd>Shift+F11</kbd><span>Stop Debugging</span><kbd>Ctrl+F9</kbd><span>Check</span><kbd>F6</kbd><span>Build</span><kbd>F7</kbd><span>Test</span><kbd>F8</kbd><span>Analyze Now</span><kbd>Ctrl+F6</kbd><span>Code Completion</span><kbd>Ctrl+Space</kbd><span>Go to Definition</span><kbd>F12</kbd><span>Find References</span><kbd>Shift+F12</kbd><span>Semantic Rename</span><kbd>F2</kbd><span>Code Actions / Quick Fixes</span><kbd>Ctrl+.</kbd><span>Interactive Tutorial</span><kbd>Ctrl+Alt+T</kbd><span>Toggle Build Bay</span><kbd>Ctrl+&#96;</kbd></div>`);
}

let messageResolver = null;
function oxideConfirm(title, message, confirmLabel = 'OK') {
  if (messageResolver) messageResolver(false);
  els.messageTitle.textContent = title;
  els.messageBody.textContent = message;
  els.messageConfirm.textContent = confirmLabel;
  els.messageDialog.showModal();
  return new Promise((resolve) => { messageResolver = resolve; });
}

function finishMessage(result) {
  if (els.messageDialog.open) els.messageDialog.close();
  if (messageResolver) {
    const resolve = messageResolver;
    messageResolver = null;
    resolve(result);
  }
}

async function handleMenuAction(action) {
  closeMenus();
  if (['undo', 'redo', 'cut', 'copy', 'paste', 'select-all'].includes(action)) return runEditCommand(action);
  if (['check', 'build', 'test', 'clean'].includes(action)) return cargoAction(action);
  if (action === 'run') return requestRun();

  if (action === 'debug-start') await startDebugging();
  else if (action === 'debug-continue') await debuggerCommand('continue');
  else if (action === 'debug-pause') await debuggerCommand('pause');
  else if (action === 'debug-restart') await restartDebugging();
  else if (action === 'debug-next') await debuggerCommand('next');
  else if (action === 'debug-step-in') await debuggerCommand('step-in');
  else if (action === 'debug-step-out') await debuggerCommand('step-out');
  else if (action === 'debug-stop') await stopDebugging();
  else if (action === 'show-debug') { setViewPanel('build', true); setConsoleView('debug'); }
  else if (action === 'new-project') await openFileBrowser('new-project');
  else if (action === 'open-project') await openFileBrowser('open');
  else if (action === 'save-file') await saveCurrentFile();
  else if (action === 'close-file') await closeTab();
  else if (action === 'save-project-as') await openFileBrowser('save-as');
  else if (action === 'close-project') await closeProject();
  else if (action === 'exit') await invoke('quit_app');
  else if (action === 'add-dependency') openDependencyDialog();
  else if (action === 'refresh-toolchain') { await detectToolchain(); if (state.projectPath) warmRustAnalyzer(); }
  else if (action === 'toggle-live-check') {
    state.liveCheck = !state.liveCheck;
    updateMenuAvailability();
    els.analysisStatus.textContent = state.liveCheck ? 'RUST CHECK: LIVE' : 'RUST CHECK: OFF';
    if (state.liveCheck) scheduleAnalysis(100);
  }
  else if (action === 'analyze-now') await runDiagnostics({ silent: false, force: true });
  else if (action === 'toggle-completer') {
    state.completer.enabled = !state.completer.enabled;
    if (!state.completer.enabled) closeCompletionUi();
    updateMenuAvailability();
  }
  else if (action === 'trigger-completion') await requestCodeCompletion({ manual: true });
  else if (action === 'go-definition') await goToDefinition();
  else if (action === 'find-references') await findReferences();
  else if (action === 'rename-symbol') await startSemanticRename();
  else if (action === 'code-actions') await showCodeActions();
  else if (action === 'tutorial') await openTutorialHome();
  else if (action === 'theme-customize') openThemeStudio();
  else if (action.startsWith('theme-')) applyTheme(action.slice('theme-'.length));
  else if (action === 'toggle-project') setViewPanel('project', !state.view.project);
  else if (action === 'toggle-cargo') setViewPanel('cargo', !state.view.cargo);
  else if (action === 'toggle-build') setViewPanel('build', !state.view.build);
  else if (action === 'show-build') { setViewPanel('build', true); setConsoleView('build'); }
  else if (action === 'show-terminal') showTerminalWindow();
  else if (action === 'show-problems') { setViewPanel('build', true); setConsoleView('problems'); }
  else if (action === 'reset-layout') resetLayout();
  else if (action === 'shortcuts') showShortcuts();
  else if (action === 'check-updates') await checkForRivetUpdates({ manual: true });
  else if (action === 'about') showAbout();
}

function setupMenus() {
  document.querySelectorAll('.menu-trigger').forEach((trigger) => trigger.addEventListener('click', (event) => {
    event.stopPropagation();
    const popup = document.querySelector(`[data-popup="${trigger.dataset.menu}"]`);
    const wasOpen = popup.classList.contains('open');
    closeMenus();
    if (!wasOpen) {
      popup.classList.add('open');
      trigger.classList.add('active');
    }
  }));
  document.querySelector('.menu-cluster')?.addEventListener('click', (event) => {
    const button = event.target.closest('[data-menu-action]');
    if (button && !button.disabled) handleMenuAction(button.dataset.menuAction);
  });
  document.addEventListener('pointerdown', (event) => {
    if (!event.target.closest('.menu-host')) closeMenus();
  });
}

renderCustomThemeMenu();
setupMenus();
applyTheme(state.theme, { persist: false });

$('#theme-studio-close').addEventListener('click', () => els.themeStudioDialog.close());
$('#theme-studio-cancel').addEventListener('click', () => els.themeStudioDialog.close());
els.themeStudioPreview.addEventListener('click', previewThemeStudio);
els.themeStudioDialog.addEventListener('close', restoreThemeAfterStudioPreview);
['change', 'input'].forEach((eventName) => {
  [els.themeStudioMaterial, els.themeStudioPalette, els.themeStudioControls, els.themeStudioSemantic].forEach((control) => {
    control.addEventListener(eventName, updateThemeStudioSummary);
  });
});
els.themeStudioForm.addEventListener('submit', (event) => {
  event.preventDefault();
  saveThemeStudio();
});
els.themeStudioDelete.addEventListener('click', async () => {
  const theme = state.customThemes.find((item) => item.id === state.themeStudioEditingId);
  if (!theme) return;
  if (await oxideConfirm('DELETE CUSTOM THEME', `Delete custom theme “${theme.name}”?`, 'DELETE')) deleteThemeStudioTheme();
});

$('#welcome-new').addEventListener('click', () => openFileBrowser('new-project'));
$('#welcome-open').addEventListener('click', () => openFileBrowser('open'));
$('#welcome-tutorial').addEventListener('click', openTutorialHome);
$('#tutorial-dialog-close').addEventListener('click', () => els.tutorialDialog.close());
$('#tutorial-dialog-done').addEventListener('click', () => els.tutorialDialog.close());
$('#tutorial-return').addEventListener('click', returnToTutorialCheckpoint);
$('#tutorial-next').addEventListener('click', handleTutorialNext);
$('#tutorial-home-button').addEventListener('click', returnToTutorialHome);
$('#tutorial-exit').addEventListener('click', exitTutorialMode);
els.tutorialLearnMore.addEventListener('click', () => {
  const willShow = els.tutorialLearnMoreText.hidden;
  els.tutorialLearnMoreText.hidden = !willShow;
  els.tutorialLearnMore.textContent = willShow ? 'HIDE DETAILS' : 'LEARN MORE';
});
els.save.addEventListener('click', () => saveCurrentFile());
els.editor.addEventListener('input', (event) => { markEditorChanged(); scheduleCodeCompletion(event); updateBracketMatch(); });
els.editor.addEventListener('scroll', () => { els.lines.scrollTop = els.editor.scrollTop; syncSyntaxScroll(); positionDebugLineHighlight(); updateBracketMatch(); if (state.completer.visible || state.completer.signatureVisible) positionCompletionUi(); });
els.editor.addEventListener('click', updateBracketMatch);
els.editor.addEventListener('keyup', updateBracketMatch);
els.editor.addEventListener('select', updateBracketMatch);
const RIVET_INDENT = '    ';

function currentLineContext(text, position) {
  const lineStart = text.lastIndexOf('\n', Math.max(0, position - 1)) + 1;
  const nextBreak = text.indexOf('\n', position);
  const lineEnd = nextBreak === -1 ? text.length : nextBreak;
  return {
    lineStart,
    lineEnd,
    beforeCursor: text.slice(lineStart, position),
    afterCursor: text.slice(position, lineEnd),
  };
}

function handleSmartEnter(event) {
  if (event.key !== 'Enter' || event.ctrlKey || event.metaKey || event.altKey || event.shiftKey) return false;

  event.preventDefault();
  const start = els.editor.selectionStart;
  const end = els.editor.selectionEnd;
  const text = els.editor.value;
  const context = currentLineContext(text, start);
  const baseIndent = context.beforeCursor.match(/^\s*/)?.[0] || '';
  const codeBeforeCursor = context.beforeCursor.trimEnd();
  const opensBlock = codeBeforeCursor.endsWith('{');
  const closesBlockNext = context.afterCursor.trimStart().startsWith('}');

  if (opensBlock && closesBlockNext && start === end) {
    const insert = `\n${baseIndent}${RIVET_INDENT}\n${baseIndent}`;
    els.editor.setRangeText(insert, start, end, 'end');
    const caret = start + 1 + baseIndent.length + RIVET_INDENT.length;
    els.editor.setSelectionRange(caret, caret);
  } else {
    const indent = opensBlock ? `${baseIndent}${RIVET_INDENT}` : baseIndent;
    els.editor.setRangeText(`\n${indent}`, start, end, 'end');
  }

  markEditorChanged();
  return true;
}

function handleClosingBraceIndent(event) {
  if (event.key !== '}' || event.ctrlKey || event.metaKey || event.altKey) return false;
  if (els.editor.selectionStart !== els.editor.selectionEnd) return false;

  const position = els.editor.selectionStart;
  const context = currentLineContext(els.editor.value, position);
  if (context.beforeCursor.trim().length !== 0) return false;
  if (context.beforeCursor.length < RIVET_INDENT.length) return false;

  event.preventDefault();
  const removeFrom = position - Math.min(RIVET_INDENT.length, context.beforeCursor.length);
  els.editor.setRangeText('}', removeFrom, position, 'end');
  markEditorChanged();
  return true;
}

els.editor.addEventListener('keydown', (event) => {
  if (handleCompleterKey(event)) return;
  if (handleAutoClosePairs(event)) return;
  if (handleSmartEnter(event)) return;
  if (handleClosingBraceIndent(event)) return;

  if (event.key === 'Tab' && !event.ctrlKey && !event.metaKey && !event.altKey) {
    event.preventDefault();
    els.editor.setRangeText(RIVET_INDENT, els.editor.selectionStart, els.editor.selectionEnd, 'end');
    markEditorChanged();
  }
});

els.lines.addEventListener('click', async (event) => {
  const lineElement = event.target.closest('.line-number');
  if (!lineElement || !state.currentFile?.toLowerCase().endsWith('.rs')) return;
  await toggleBreakpoint(state.currentFile, Number(lineElement.dataset.line));
});

els.lines.addEventListener('contextmenu', (event) => {
  const lineElement = event.target.closest('.line-number');
  if (!lineElement || !state.currentFile?.toLowerCase().endsWith('.rs')) return;
  event.preventDefault();
  openBreakpointEditor(state.currentFile, Number(lineElement.dataset.line));
  updateLineNumbers();
  renderBreakpoints();
});

els.debugWatchForm.addEventListener('submit', async (event) => {
  event.preventDefault();
  const expression = els.debugWatchInput.value.trim();
  if (!expression || state.debugger.watches.includes(expression)) return;
  state.debugger.watches.push(expression);
  els.debugWatchInput.value = '';
  renderWatches();
  if (state.debugger.stopped) await refreshWatches();
});

els.debugThreadSelect.addEventListener('change', async () => {
  const threadId = Number(els.debugThreadSelect.value || 0);
  if (!threadId || !state.debugger.stopped) return;
  state.debugger.threadId = threadId;
  await refreshDebugInspection(threadId);
});

els.debugConsoleForm.addEventListener('submit', runDebugConsole);
els.breakpointForm.addEventListener('submit', saveBreakpointOptions);
$('#breakpoint-remove').addEventListener('click', removeEditedBreakpoint);
$('#breakpoint-cancel').addEventListener('click', closeBreakpointEditor);
$('#breakpoint-close').addEventListener('click', closeBreakpointEditor);
$('#references-close').addEventListener('click', () => els.referencesDialog.close());
$('#references-done').addEventListener('click', () => els.referencesDialog.close());
$('#rename-close').addEventListener('click', () => { state.intelligence.pendingRename = null; els.renameDialog.close(); });
$('#rename-cancel').addEventListener('click', () => { state.intelligence.pendingRename = null; els.renameDialog.close(); });
els.renameForm.addEventListener('submit', submitSemanticRename);
$('#code-actions-close').addEventListener('click', () => els.codeActionsDialog.close());
$('#code-actions-done').addEventListener('click', () => els.codeActionsDialog.close());
$('#debug-target-cancel').addEventListener('click', () => finishDebugTargetChoice(null));
$('#debug-target-close').addEventListener('click', () => finishDebugTargetChoice(null));

els.diagnosticBanner.addEventListener('click', () => {
  setViewPanel('build', true);
  setConsoleView('problems');
});

document.querySelectorAll('.command-button[data-action]').forEach((button) => button.addEventListener('click', () => {
  if (button.dataset.action === 'run') requestRun();
  else cargoAction(button.dataset.action);
}));
document.querySelectorAll('[data-debug-action]').forEach((button) => button.addEventListener('click', () => handleDebugAction(button.dataset.debugAction)));

document.querySelectorAll('.profile').forEach((button) => button.addEventListener('click', () => {
  state.release = button.dataset.profile === 'release';
  document.querySelectorAll('.profile').forEach((item) => item.classList.toggle('active', item === button));
  els.profileStatus.textContent = `PROFILE: ${state.release ? 'RELEASE' : 'DEBUG'}`;
  if (state.liveCheck) scheduleAnalysis(100);
}));

document.querySelectorAll('.console-tab[data-mode]').forEach((button) => button.addEventListener('click', () => {
  state.outputMode = button.dataset.mode;
  document.querySelectorAll('.console-tab[data-mode]').forEach((item) => item.classList.toggle('active', item === button));
  renderOutput();
}));

document.querySelectorAll('.console-view').forEach((button) => button.addEventListener('click', () => setConsoleView(button.dataset.consoleView)));
$('#clear-output').addEventListener('click', clearOutput);
$('#terminal-window-clear').addEventListener('click', clearTerminal);
$('#terminal-window-close').addEventListener('click', hideTerminalWindow);
setupTerminalDragging();
els.stopTerminal.addEventListener('click', stopTerminal);
els.terminalForm.addEventListener('submit', sendTerminalInput);
els.terminalInput.addEventListener('keydown', (event) => {
  if (event.ctrlKey && event.key.toLowerCase() === 'c' && state.terminalRunning) {
    event.preventDefault();
    stopTerminal();
  }
});
els.terminalScreen.addEventListener('keydown', (event) => {
  if (state.terminalEnded) {
    event.preventDefault();
    event.stopPropagation();
    dismissFinishedTerminal();
  }
});

els.dependencyForm.addEventListener('submit', addDependencyFromDialog);
$('#browser-close').addEventListener('click', () => els.browserDialog.close());
$('#browser-cancel').addEventListener('click', () => els.browserDialog.close());
$('#browser-go').addEventListener('click', () => browseTo(els.browserPath.value.trim()));
els.browserPath.addEventListener('keydown', (event) => {
  if (event.key === 'Enter') {
    event.preventDefault();
    browseTo(els.browserPath.value.trim());
  }
});
els.browserUp.addEventListener('click', () => { if (state.browserParent) browseTo(state.browserParent); });
els.browserConfirm.addEventListener('click', confirmBrowserSelection);
$('#browser-new-folder').addEventListener('click', () => {
  els.browserNewFolderRow.hidden = !els.browserNewFolderRow.hidden;
  if (!els.browserNewFolderRow.hidden) requestAnimationFrame(() => els.browserNewFolderName.focus());
});
$('#browser-create-folder').addEventListener('click', createBrowserFolder);
$('#browser-cancel-folder').addEventListener('click', () => {
  els.browserNewFolderRow.hidden = true;
  els.browserNewFolderName.value = '';
});
els.browserNewFolderName.addEventListener('keydown', (event) => {
  if (event.key === 'Enter') {
    event.preventDefault();
    createBrowserFolder();
  }
});

els.messageCancel.addEventListener('click', () => finishMessage(false));
els.messageConfirm.addEventListener('click', () => finishMessage(true));
els.messageDialog.addEventListener('cancel', (event) => {
  event.preventDefault();
  finishMessage(false);
});
$('#info-close').addEventListener('click', () => els.infoDialog.close());
$('#info-close-x').addEventListener('click', () => els.infoDialog.close());

function dismissUpdatePrompt() {
  if (state.updater.installing) return;
  if (els.updateDialog.open) els.updateDialog.close();
}

// Update controls are application-level UI. Bind them once during startup so
// they work whether Rivet is on the welcome screen, editing a file, or has no
// project loaded at all.
els.updateClose.addEventListener('click', dismissUpdatePrompt);
els.updateLater.addEventListener('click', dismissUpdatePrompt);
els.updateInstall.addEventListener('click', async () => {
  try {
    await installPendingUpdate();
  } catch (error) {
    // installPendingUpdate normally handles its own errors, but keep the
    // button from ever failing silently if an unexpected frontend error leaks.
    state.updater.installing = false;
    els.updateLater.disabled = false;
    els.updateInstall.disabled = false;
    els.updateInstall.textContent = 'TRY AGAIN';
    els.updateError.hidden = false;
    els.updateError.textContent = `Update failed: ${error}`;
  }
});
els.updateDialog.addEventListener('cancel', (event) => {
  if (state.updater.installing) {
    event.preventDefault();
    return;
  }
  dismissUpdatePrompt();
});

$('#run-close').addEventListener('click', () => els.runDialog.close());
$('#run-cancel').addEventListener('click', () => els.runDialog.close());
document.querySelectorAll('[data-run-mode]').forEach((button) => button.addEventListener('click', async () => {
  const mode = button.dataset.runMode;
  els.runDialog.close();
  if (mode === 'terminal') await startTerminalRun();
  else await cargoAction('run');
}));

document.addEventListener('keydown', async (event) => {
  if (event.key === 'Escape') {
    closeMenus();
    return;
  }
  const ctrl = event.ctrlKey || event.metaKey;
  if (ctrl && event.key === 'Tab') {
    event.preventDefault();
    cycleTab(event.shiftKey ? -1 : 1);
  } else if (ctrl && event.key.toLowerCase() === 'w') {
    event.preventDefault();
    await closeTab();
  } else if (ctrl && event.key.toLowerCase() === 'n') {
    event.preventDefault();
    await openFileBrowser('new-project');
  } else if (ctrl && event.key.toLowerCase() === 'o') {
    event.preventDefault();
    await openFileBrowser('open');
  } else if (ctrl && event.shiftKey && event.key.toLowerCase() === 's') {
    event.preventDefault();
    if (state.projectPath) await openFileBrowser('save-as');
  } else if (ctrl && !event.shiftKey && event.key.toLowerCase() === 's') {
    event.preventDefault();
    await saveCurrentFile();
  } else if (ctrl && event.key === '`') {
    event.preventDefault();
    setViewPanel('build', !state.view.build);
  } else if (ctrl && event.altKey && event.key.toLowerCase() === 't') {
    event.preventDefault();
    await openTutorialHome();
  } else if (ctrl && event.key === 'F6') {
    event.preventDefault();
    await runDiagnostics({ silent: false, force: true });
  } else if (event.key === 'F12' && event.shiftKey) {
    event.preventDefault();
    await findReferences();
  } else if (event.key === 'F12') {
    event.preventDefault();
    await goToDefinition();
  } else if (event.key === 'F2') {
    event.preventDefault();
    await startSemanticRename();
  } else if (ctrl && event.key === '.') {
    event.preventDefault();
    await showCodeActions();
  } else if (event.ctrlKey && event.shiftKey && event.key === 'F9') {
    event.preventDefault();
    await restartDebugging();
  } else if (event.ctrlKey && event.key === 'F9') {
    event.preventDefault();
    await stopDebugging();
  } else if (event.key === 'F9') {
    event.preventDefault();
    if (state.debugger.running && state.debugger.stopped) await debuggerCommand('continue');
    else if (!state.debugger.running) await startDebugging();
  } else if (event.shiftKey && event.key === 'F11') {
    event.preventDefault();
    await debuggerCommand('step-out');
  } else if (event.key === 'F11') {
    event.preventDefault();
    await debuggerCommand('step-in');
  } else if (event.key === 'F10') {
    event.preventDefault();
    await debuggerCommand(event.ctrlKey ? 'continue' : 'next');
  } else if (event.key === 'F5') {
    event.preventDefault();
    requestRun();
  } else if (event.key === 'F6') {
    event.preventDefault();
    await cargoAction('check');
  } else if (event.key === 'F7') {
    event.preventDefault();
    await cargoAction('build');
  } else if (event.key === 'F8') {
    event.preventDefault();
    await cargoAction('test');
  }
});

listen('oxide-update-download', (event) => {
  updateDownloadProgress(event.payload);
});

listen('cargo-output', (event) => {
  const { stream, line } = event.payload;
  appendRaw(stream, line);
  interpretCargoLine(line);
});

listen('cargo-state', (event) => {
  const { state: cargoState, detail } = event.payload;
  if (cargoState === 'started' || cargoState === 'finished') appendFriendly('stage', detail);
});

listen('debugger-output', (event) => {
  const { output, category } = event.payload || {};
  appendDebugOutput(output || '', category === 'stderr' ? 'error' : category === 'adapter' ? 'muted' : 'normal');
});

listen('debugger-state', async (event) => {
  const { state: debuggerState, detail } = event.payload || {};
  if (detail) appendDebugOutput(detail, debuggerState === 'adapter-exited' ? 'error' : 'stage');
  if (debuggerState === 'building') {
    els.debuggerStatus.textContent = 'DEBUGGER: BUILDING';
    els.debuggerDetail.textContent = 'CARGO DEBUG BUILD';
  } else if (debuggerState === 'starting') {
    els.debuggerStatus.textContent = 'DEBUGGER: STARTING';
    els.debuggerDetail.textContent = 'STARTING LLDB DAP';
  } else if (debuggerState === 'running') {
    state.debugger.running = true;
    els.debuggerStatus.textContent = 'DEBUGGER: RUNNING';
    els.debuggerDetail.textContent = 'PROGRAM RUNNING';
    els.commandReadout.textContent = 'DEBUG · PROGRAM ACTIVE';
  } else if (debuggerState === 'adapter-exited' && state.debugger.running) {
    try { await invoke('debugger_stop'); } catch { /* adapter already exited */ }
    state.debugger.running = false;
    state.debugger.stopped = false;
    state.debugger.threadId = null;
    state.debugger.threads = [];
    resetDebugInspection();
    els.debuggerStatus.textContent = state.debugger.available ? 'DEBUGGER: READY' : 'DEBUGGER: NOT FOUND';
    els.debuggerDetail.textContent = 'SESSION ENDED';
  }
  updateMenuAvailability();
});

listen('debugger-event', async (event) => {
  const message = event.payload || {};
  const kind = message.event;
  const body = message.body || {};
  if (kind === 'output') {
    appendDebugOutput(body.output || '', body.category === 'stderr' ? 'error' : 'normal');
    return;
  }
  if (kind === 'stopped') {
    state.debugger.running = true;
    state.debugger.stopped = true;
    state.debugger.threadId = body.threadId || state.debugger.threadId;
    els.debuggerStatus.textContent = 'DEBUGGER: PAUSED';
    els.debuggerDetail.textContent = `${String(body.reason || 'stopped').toUpperCase()}${body.description ? ` · ${body.description}` : ''}`;
    els.commandReadout.textContent = `DEBUG · ${String(body.reason || 'PAUSED').toUpperCase()}`;
    setViewPanel('build', true);
    setConsoleView('debug');
    appendDebugOutput(`Paused: ${body.description || body.reason || 'debugger stop'}`, 'stage');
    updateMenuAvailability();
    await refreshDebugThreads(state.debugger.threadId);
    return;
  }
  if (kind === 'continued') {
    state.debugger.stopped = false;
    state.debugger.threads = [];
    resetDebugInspection();
    els.debuggerStatus.textContent = 'DEBUGGER: RUNNING';
    els.debuggerDetail.textContent = 'PROGRAM RUNNING';
    updateMenuAvailability();
    return;
  }
  if (kind === 'thread' && state.debugger.stopped) {
    await refreshDebugThreads(state.debugger.threadId);
    return;
  }
  if (kind === 'exited') {
    appendDebugOutput(`Program exited${body.exitCode == null ? '' : ` with code ${body.exitCode}`}.`, body.exitCode === 0 ? 'success' : 'error');
    return;
  }
  if (kind === 'terminated') {
    try { await invoke('debugger_stop'); } catch { /* adapter may already have exited */ }
    state.debugger.running = false;
    state.debugger.stopped = false;
    state.debugger.threadId = null;
    state.debugger.threads = [];
    resetDebugInspection();
    els.debuggerStatus.textContent = state.debugger.available ? 'DEBUGGER: READY' : 'DEBUGGER: NOT FOUND';
    els.debuggerDetail.textContent = 'SESSION COMPLETE';
    els.commandReadout.textContent = 'DEBUG COMPLETE';
    updateMenuAvailability();
  }
});

listen('terminal-output', (event) => {
  const { stream, data } = event.payload;
  appendTerminalChunk(stream, data);
});

listen('terminal-state', (event) => {
  const { state: terminalState, detail, exit_code: exitCode } = event.payload;
  if (terminalState === 'building') {
    state.terminalRunning = true;
    state.terminalEnded = false;
    els.stopTerminal.disabled = true;
    els.terminalInput.disabled = true;
    $('.terminal-send').disabled = true;
    els.buildStatus.textContent = 'BUILDING FOR RUN';
    els.commandReadout.textContent = 'RUN · BUILDING';
    updateMenuAvailability();
    return;
  }
  if (terminalState === 'build-failed') {
    state.terminalRunning = false;
    state.terminalEnded = false;
    state.tutorial.runSuccess = false;
    els.stopTerminal.disabled = true;
    els.terminalInput.disabled = true;
    $('.terminal-send').disabled = true;
    clearTerminal();
    appendTerminalChunk('system', 'Build failed. Open Build Bay or Problems for the compiler details.');
    els.buildStatus.textContent = 'BUILD FAILED';
    els.commandReadout.textContent = 'RUN · BUILD FAILED';
    updateMenuAvailability();
    if (state.liveCheck) scheduleAnalysis(120);
    scheduleTutorialEvaluation(120);
    return;
  }
  if (terminalState === 'started') {
    showTerminalWindow({ focus: false });
    clearTerminal();
    state.terminalRunning = true;
    state.terminalEnded = false;
    els.stopTerminal.disabled = false;
    els.terminalInput.disabled = false;
    $('.terminal-send').disabled = false;
    els.buildStatus.textContent = 'PROGRAM RUNNING';
    els.commandReadout.textContent = 'RUN · PROGRAM ACTIVE';
    updateMenuAvailability();
    requestAnimationFrame(() => els.terminalInput.focus());
    return;
  }
  if (terminalState === 'finished') {
    state.terminalRunning = false;
    state.terminalEnded = true;
    state.tutorial.runSuccess = exitCode === 0;
    els.stopTerminal.disabled = true;
    els.terminalInput.disabled = true;
    $('.terminal-send').disabled = true;
    const suffix = exitCode == null ? '' : ` (exit code ${exitCode})`;
    appendTerminalChunk(exitCode === 0 ? 'stdout' : 'stderr', `\n\n${detail}${suffix}\n`);
    appendTerminalChunk('system', '\nPress any key to exit...');
    els.buildStatus.textContent = 'PROJECT READY';
    els.commandReadout.textContent = 'RUN COMPLETE';
    updateMenuAvailability();
    requestAnimationFrame(() => els.terminalScreen.focus());
    if (state.liveCheck) scheduleAnalysis(250);
    scheduleTutorialEvaluation(120);
  }
});

els.editor.readOnly = true;
clearTerminal();
els.tutorialPanel.hidden = true;
renderProblems();
renderOutput();
renderWatches();
setupBuildBayResize();
resetLayout({ resetBuildBay: false });
restoreBuildBayHeight();
setProjectUiState();
detectPlatform().finally(() => detectToolchain());
window.setTimeout(() => checkForRivetUpdates({ manual: false }), 1400);
