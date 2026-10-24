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
- For apps in Quiver's own catalog the CLI takes the version from quiverlauncher.com, skips GitHub and
  then finds no download (`StartupVersionResolver.cs`; this was the SpartanPort failure). heimspiel
  therefore pins the newest GitHub release as `preferredVersion` before installing, and turns Quiver's
  auto update off. Updating Ports is heimspiel's job.
