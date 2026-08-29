export function createAdaptiveLayoutController({
  state,
  root,
  shell,
  workspaceBar,
  settingsSelect,
  settingsDescription,
  persistLayout,
  ensurePanel,
  refreshEditor,
}) {
  const allowedPanes = new Set(['project', 'editor', 'cargo', 'tutorial']);

  function description(mode = state.layout) {
    return mode === 'mobile'
      ? 'Mobile Layout uses a single primary workspace with Files / Editor / Cargo switching and touch-friendly chrome.'
      : 'Desktop Layout keeps Rivet’s full multi-panel workbench visible at the same time.';
  }

  function setPane(pane = 'editor') {
    const next = allowedPanes.has(pane) ? pane : 'editor';
    state.mobilePane = next;
    shell.dataset.mobilePane = next;
    workspaceBar?.querySelectorAll('[data-mobile-pane]').forEach((button) => {
      const active = button.dataset.mobilePane === next;
      button.classList.toggle('active', active);
      button.setAttribute('aria-pressed', active ? 'true' : 'false');
    });

    if (state.layout !== 'mobile') return;
    if (next === 'project' && !state.view.project) ensurePanel('project');
    if (next === 'cargo' && !state.view.cargo) ensurePanel('cargo');
    if (next === 'tutorial' && !state.tutorial.active) setPane('editor');
  }

  function apply(mode, { persist = true } = {}) {
    const next = mode === 'mobile' ? 'mobile' : 'desktop';
    state.layout = next;
    root.dataset.layout = next;
    shell.dataset.layout = next;
    if (persist) persistLayout(next);
    if (settingsSelect) settingsSelect.value = next;
    if (settingsDescription) settingsDescription.textContent = description(next);

    if (next === 'mobile') {
      if (!allowedPanes.has(state.mobilePane)) state.mobilePane = 'editor';
      setPane(state.tutorial.active ? 'tutorial' : state.mobilePane);
    }
    requestAnimationFrame(refreshEditor);
  }

  function bind() {
    workspaceBar?.querySelectorAll('[data-mobile-pane]').forEach((button) => {
      button.addEventListener('click', () => setPane(button.dataset.mobilePane));
    });
  }

  return { description, setPane, apply, bind };
}
