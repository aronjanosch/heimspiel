# Tauri app with a Rust core

heimspiel is one Rust core library used by a CLI (`heimspiel sync`, `heimspiel launch`, called by ES-DE
and scripts) and a Tauri app (setup wizard). Rust plus Tauri gives one small binary per OS with an
installer UI included, and keeps all logic in the core so the CLI works without the app.

## Consequences

- On NVIDIA under Wayland the Tauri window does not start at all unless `__NV_DISABLE_EXPLICIT_SYNC=1`
  (fallback `WEBKIT_DISABLE_DMABUF_RENDERER=1`) is set before GTK initialises. The app sets it at the
  start of `main` when it detects that combination and the user has not set either variable.
- The app also is the loading window above ES-DE (`heimspiel-app progress`): the CLI starts it for long
  steps and sends it JSON lines (`heimspiel_core::progress::Message`) on standard input. ES-DE shows only
  black while its `game-start` hook or a launch command runs
  (docs/research/2026-10-25-multi-file-and-loading.md).
