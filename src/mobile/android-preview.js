export function isAndroidPlatform(state) {
  return state?.platform?.os === 'android';
}

export function applyAndroidEditorPreview({ state, els }) {
  if (!isAndroidPlatform(state)) return false;
  document.documentElement.dataset.androidPreview = 'true';

  const note = 'Android editor preview — local Rust toolchain backend is not installed yet.';
  [els.cargoLamp, els.rustcLamp, els.welcomeCargoLamp, els.welcomeRustcLamp].forEach((lamp) => {
    if (!lamp) return;
    lamp.classList.remove('ok', 'bad');
    lamp.classList.add('pending');
  });
  if (els.cargoVersion) els.cargoVersion.textContent = 'Cargo: backend pending';
  if (els.rustcVersion) els.rustcVersion.textContent = 'rustc: backend pending';
  if (els.welcomeCargoStatus) els.welcomeCargoStatus.textContent = 'Cargo backend planned';
  if (els.welcomeRustcStatus) els.welcomeRustcStatus.textContent = 'rustc backend planned';
  if (els.commandReadout) els.commandReadout.textContent = 'ANDROID EDITOR PREVIEW · TOOLCHAIN BACKEND PENDING';
  if (els.analysisStatus) {
    els.analysisStatus.textContent = 'RUST CHECK: UNAVAILABLE';
    els.analysisStatus.title = note;
  }
  if (els.analyzerStatus) {
    els.analyzerStatus.textContent = 'ANALYZER: PENDING';
    els.analyzerStatus.title = note;
  }
  if (els.debuggerStatus) {
    els.debuggerStatus.textContent = 'DEBUGGER: PENDING';
    els.debuggerStatus.title = note;
  }
  if (els.debuggerDetail) els.debuggerDetail.textContent = 'ANDROID BACKEND PENDING';
  if (els.settingsLiveCheck) {
    els.settingsLiveCheck.disabled = true;
    els.settingsLiveCheck.title = note;
  }
  if (els.settingsCompleter) {
    els.settingsCompleter.disabled = true;
    els.settingsCompleter.title = note;
  }
  const tutorial = document.querySelector('#welcome-tutorial');
  if (tutorial) {
    tutorial.disabled = true;
    tutorial.title = 'The interactive tutorial needs the Android Rust toolchain backend, planned for a later build.';
  }
  document.querySelectorAll('[data-menu-action="tutorial"]').forEach((button) => {
    button.disabled = true;
    button.title = 'The interactive tutorial needs the Android Rust toolchain backend.';
  });
  return true;
}
