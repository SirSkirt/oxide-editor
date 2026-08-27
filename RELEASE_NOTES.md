# Oxide Editor B1.3.5 · Build 4

## Rust Intelligence & Editing

Build 4 turns more of Oxide's existing rust-analyzer infrastructure into everyday IDE features.

### Rust Code Analyzer/Completer UX fix
- Completion suggestions are anchored **below the current typing line** and no longer flip upward over the code being edited.
- **Escape** dismisses completion for the rest of the current word/token.
- After a word boundary, completion can appear again when the next word begins.
- Escape also cancels pending completion requests so a popup cannot reappear immediately after being dismissed.
- Ctrl+Space still manually requests completion.

### Go to Definition
- **F12** or Tools → Go to Definition.
- Uses rust-analyzer's real `textDocument/definition` result.
- Opens the target file and selects/scrolls to the returned symbol range.

### Find References
- **Shift+F12** or Tools → Find References.
- Uses rust-analyzer reference data, including the declaration.
- Results appear in an Oxide-styled reference browser and are clickable.

### Semantic Rename
- **F2** or Tools → Rename Symbol.
- Uses rust-analyzer prepare-rename + semantic rename rather than blind text replacement.
- Oxide shows the number of edits/files before applying the rename.
- Multi-file text edits are applied through Oxide's normal file backend and open tabs are refreshed afterward.

### Code Actions / Quick Fixes
- **Ctrl+.** or Tools → Code Actions / Quick Fixes.
- Requests rust-analyzer code actions at the caret and resolves edits where rust-analyzer supports it.
- Preferred actions are visually identified.
- Build 4 applies text-edit based actions. File create/rename/delete resource operations and command-only actions are intentionally reported as unavailable rather than guessed at.

### Editing polish
- Auto-close pairs for `()`, `[]`, `{}`, and double quotes.
- Typing an existing closing pair advances over it instead of duplicating it.
- Selecting text and typing an opening pair wraps the selection.
- Matching `()`, `[]`, and `{}` are highlighted around the caret.
- Single quotes are deliberately not auto-paired in this pass so Rust lifetimes such as `'a` are not disrupted.

## Preserved from Build 3
- Vertically resizable Build Bay with persisted height and reset behavior.
- B1.3.6 visual-direction preview / forged-workbench polish without changing Oxide's core layout.
- Windows and Linux automatic updates compare `(release_version, build)` and publish each build under a distinct release tag.
- Semantic Readability Colors.
- LLDB/DAP debugger, breakpoint management, watches, variables, call stack, Debug Console, and target selection.

## Version
- Public release: `1.3.5` / `B1.3.5`
- Internal build: `4`
- Full identity: **B1.3.5 · Build 4**
