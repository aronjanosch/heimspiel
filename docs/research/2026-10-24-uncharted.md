# Uncharted: The Nathan Drake Collection under Proton (2026-10-24)

First PS4 Port tested end to end by hand, before heimspiel's `dump` kind was written. Same system as
[2026-10-23-tests.md](2026-10-23-tests.md). Test files live in `~/.cache/heimspiel-tests/work/uncharted`.

## Original: a self-made decrypted PKG

- The user's own PKG `Uncharted The Nathan Drake Collection CUSA02320 v1.00.pkg`, made decrypted on their
  console (not a retail PKG), extracts completely with `ps4-pkg-tool <pkg> <output>` into
  `CUSA02320/` (59 GB): `eboot.bin`, `big2-ps4_Shipping.elf`, `big3-ps4_Shipping.elf`, `sce_module/`,
  `sce_sys/`, `shareddata/`, `u1data/`, `u2data/`, `u3data/`. This complements test 6 of
  2026-10-23-tests.md, where a retail PKG crashed the tool: the tool handles decrypted PKGs only.
- The update PKG v1.02 was not extracted.
- `sce_sys/param.sfo` (`\0PSF`, version 0x101, 22 entries; text entries have format 0x0204, numbers 0x0404):
  `TITLE_ID` CUSA02320, `TITLE` "Uncharted: The Nathan Drake Collection™", `APP_VER` 01.00,
  `VERSION` 01.00, `CONTENT_ID` UP9000-CUSA02320_00-UNCHARTEDTRILOGY, `CATEGORY` gd.
- `sce_sys/icon0.png` is the PS4 menu icon, usable as cover.

## Port: Supermedo/uncharted_pc

- GPL-2.0, about 1900 stars. Releases carry one asset, `uncharted-windows.zip` (v0.3.8: 123 MB, SHA-256
  2110dc9b…, matching GitHub). Windows only. Not in Quiver's catalog, so the catalog bug of ADR 0003 does not
  apply.
- README: needs "your own decrypted dump of the collection (CUSA02343, CUSA02320, ...)" with `eboot.bin`,
  `big2-ps4_Shipping.elf`, `big3-ps4_Shipping.elf`, `sce_sys`, `u1data`, `u2data`, `u3data`. Versions 1.00 and
  1.02 are mentioned as supported.
- `Uncharted.exe` is the launcher (PyInstaller, Python 3.14, Tk). It keeps its options in
  `%APPDATA%\uncharted-launcher\settings.json`: `game_dir` (the Dump as Windows path), `start` (`u1`, `u2`,
  `u3`), `check_updates`, `fullscreen` and about 60 more keys. Saves and shader caches go to `user\` next to
  the executable unless `user_dir` is set; `out\` holds the prepared game image.
- `Play Uncharted.exe` (124 KB) starts the game with the saved settings without showing the launcher. Its
  strings: `%ls\Uncharted.exe`, `"%ls" --play`, "Uncharted.exe was not found next to this file."
  `Uncharted.exe --play` does the same.
- The launcher checks for updates at start (`check_updates`) and installs them over its folder.

## Runs under Proton Experimental

Started by hand with `proton waitforexitandrun` and a fresh prefix:

- `Uncharted.exe` (launcher) runs and shows "No game folder yet" until `game_dir` is set.
- Starting the game through `run.py`'s console path ended with exit code 120: Python could not flush stdout
  without a console. Running it through `cmd.exe` with a redirect works and the game (Drake's Fortune menu)
  renders correctly, but a black conhost window stays open next to it.
- **`Uncharted.exe --play`**, with `game_dir` and `start` in `settings.json` and `PROTON_USE_XALIA=0`:
  the game starts directly, without the launcher and **without a black window** (confirmed by the user).
  Proton blocked until the window was closed and returned 0; `user\last_run.log` ends with "Window closed
  (close button, Alt+F4 or system quit request)".
- Proton's Xalia crashed once on the launcher window ("Unhandled exception in Xalia: Invalid window handle"),
  hence `PROTON_USE_XALIA=0` in the later runs.
- The log's "not found" lines are optional effect files; the game runs regardless. Vulkan reports a few
  unavailable AMD extensions on NVIDIA; DLSS is detected.

## Quiver with a Windows Port

From the Quiver 3.5.1 source (`~/.cache/heimspiel-tests/dl/quiver-src`):

- `linuxRunner: "proton"` runs `proton waitforexitandrun <exe>` with **no further arguments**
  (`Services/WindowsRunnerService.cs:290-335`), so `--play` cannot be passed; `Play Uncharted.exe` makes it
  unnecessary. A `custom` runner command could add arguments but would duplicate Proton's environment.
- `linuxPrefixPath` is used as `STEAM_COMPAT_DATA_PATH`; without it the prefix lands in
  `Apps/<folder>/.steam-compat-data`. `SteamGameId`/`SteamAppId` are a stable id derived from the executable.
- A release with several programs: `--run` refuses ("Multiple executables found", opens the GUI) unless
  `Apps/<folder>/selected_executable.txt` holds the absolute path of one (`Services/CLIHandler.cs:381-420`,
  `Models/GameInfo.cs:1342-1375`). Any existing file with an extension counts as valid on Linux
  (`GameInstallationService.cs:1504-1513`).
- Quiver's extraction leaves `run.py` (a script with `#!/usr/bin/env python3` and CRLF line ends)
  executable. On Linux, `FindExecutableCandidates` lists `.exe` files only when no file looks like a native
  program, and the launch uses the first candidate when the noted one is not among them
  (`lib/QuiverLauncher.Core/Services/GameInstallationService.cs`, `Services/GameLaunchService.cs:36-77`). The
  second launch from ES-DE therefore ran `run.py` and failed with "env: ‘python3\r’: No such file or
  directory" although `selected_executable.txt` named `Play Uncharted.exe` (`launch-debug.log`:
  `chosenExecutable=…/run.py`). heimspiel clears the executable bit of the other top-level files of a
  Windows Port.
- Proton creates a prefix around files that already exist: `copy_pfx` only creates missing directories and
  files (`proton`, `setup_prefix` and `copy_pfx`). heimspiel can therefore write `settings.json` into
  `pfx/drive_c/users/steamuser/AppData/Roaming/` before the first start.

**Catalog apps install only the version the catalog lists.** The first launch from ES-DE ended with "No
eligible download was found." `Supermedo/uncharted_pc` is in Quiver's catalog (slug
`uncharted-the-nathan-drake-collection-uncharted-pc`, verified `v0.3.6`), which Quiver caches in
`Cache/catalog-release-status.json` and `Cache/catalog-release-links.json` on its first run. For a catalog app
the release comes from quiverlauncher.com, and a pin the site does not list selects nothing
(`Services/CatalogReleases.cs:212-219`). `--download` with one `apps.json` entry:

| Quiver folder | `preferredVersion` | Result |
|---|---|---|
| fresh (no catalog cache yet) | none | `Selected: uncharted-windows.zip` |
| fresh | `v0.3.8`, `0.3.8` | No eligible download |
| with catalog cache | none | No eligible download |
| with catalog cache, catalog files removed | none | `Selected: uncharted-windows.zip` |
| with catalog cache | `v0.3.6` (verified) | `Selected: uncharted-windows.zip` |
| with catalog cache | `v0.3.7` | No eligible download |

The message is the CLI's fallback when no release was selected (`CLIHandler.cs:626-628`). Windows assets are
eligible on Linux (`DownloadAssetSelection.cs`, `Select`). Ship of Harkinian worked pinned to 9.3.1 because
that is its verified catalog version; the SpartanPort failure of test 6 fits the same rule. Why the unpinned
catalog case fails was not pinned down.

## Design conclusions

- Manifest kind `dump`: heimspiel reads `param.sfo`, checks title ID and the files the Port needs, writes
  `game_dir`, `start` and `check_updates = false` into the launcher's `settings.json` in the Port's prefix,
  keeping the user's other settings, and runs `Play Uncharted.exe` through Quiver.
- heimspiel pins the version Quiver's catalog verified, the newest GitHub release only for apps outside the
  catalog, and retries with the catalog's version once Quiver has fetched the catalog.
- One ES-DE entry per game (U1, U2, U3); the entry passes the game as `--game`.
- heimspiel sets `linuxRunner` and `linuxPrefixPath` itself, so it knows where the prefix is.
- Open: whether Quiver's launch (without `PROTON_USE_XALIA=0`, with Quiver's own `SteamGameId`) also shows
  no black window; saves live in the Port's folder (`user\`) and must survive Port updates.
