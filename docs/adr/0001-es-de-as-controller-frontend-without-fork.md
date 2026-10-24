# ES-DE as the controller frontend, without a fork

heimspiel writes its Library into an unmodified ES-DE (3.5) instead of forking it or building its own
couch UI. ES-DE can be populated entirely from outside (ROM folders, `gamelist.xml`, media folders,
custom systems, `game-start` scripts), and the manual tests showed Steam, Heroic and Quiver launches
returning to it cleanly. Everything heimspiel needs therefore goes through those files, which ES-DE only
reads at startup, so heimspiel writes them only while ES-DE is closed.

## Considered Options

- **Forking ES-DE**: rejected; we would carry the maintenance of a large C++ frontend for features the
  file interface already covers.
- **RetroBat**: rejected; a Batocera ES fork, Windows only, content partly CC BY-NC-SA.

## Consequences

- No custom gamelist fields exist, so heimspiel state (which Entry came from which Source) lives in
  heimspiel, not in ES-DE.
- On Hyprland ES-DE comes back tiled after a fullscreen game; the fix is an opt-in compositor snippet
  offered by the setup wizard, not heimspiel code (see ADR 0008).
