# Quiver installs Ports, with our own download as fallback

heimspiel installs, updates and launches Ports through Quiver Launcher (MIT) as an external dependency,
without forking it: Quiver already handles release assets from GitHub, GitLab and Codeberg and runs
Windows builds through Proton or Wine. heimspiel writes Quiver's `apps.json` itself (one entry per
repository, only while Quiver is closed, atomically) and launches with `quiver --run`, which blocks until
the game exits.

## Consequences

- `quiver --download` installs but never exits, so heimspiel calls it with a timeout and detects
  completion from `installed-build.txt`.
- Quiver cannot install every repository from its CLI (SpartanPort: "No eligible download"). For those
  heimspiel downloads the release asset itself with the same asset filter.
- heimspiel runs Quiver in its own folder, because Quiver keeps its library next to its AppImage.
- For apps in Quiver's own catalog the CLI installs only a version quiverlauncher.com lists; unpinned or
  pinned to another version it finds no download (`CatalogReleases.cs`; this was the SpartanPort and the
  first Uncharted failure, docs/research/2026-10-24-uncharted.md). heimspiel pins the catalog's verified
  version for those and the newest GitHub release for the rest, so a catalog app may get an older release
  than GitHub's newest. Quiver's auto update stays off; updating Ports is heimspiel's job.
- Windows Ports are launched by heimspiel with `proton waitforexitandrun`, not with `quiver --run`: Quiver
  starts any file in the release that begins with `#!/` natively instead of the chosen `.exe` (Uncharted's
  `run.py`), and passes no environment of heimspiel's. Quiver still installs them.
