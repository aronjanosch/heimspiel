# Research: landscape and building blocks (as of 2026-10-23)

Result of a research session with code reviews of the projects involved.
File:line references point to each upstream's state on 2026-10-23.
"Unconfirmed" means: only read in docs or code, not executed.

## Goal

One library for everything: Steam, Heroic, manual PC games, RomM (or local folders),
recomps and ports. Outputs: ES-DE (controller) and a desktop view in our own
Tauri app. The app is also a setup wizard (like EmuDeck). Linux first, Windows later.
Further outputs later: Steam (`shortcuts.vdf`), Playnite.

## Decisions

| Topic | Decision | Reason |
|---|---|---|
| Frontend | ES-DE 3.5, no fork | easy to populate from outside (score 4/5) |
| Desktop view | own lean view in the Tauri app | no existing desktop frontend reads ES-DE data (Pegasus without a release since 2024, Playnite Windows only, LaunchBox on Linux 2027 at the earliest) |
| Technology | Tauri + Rust core | one binary per OS, installer UI included |
| Emulators | no RetroDECK; Flatpak (Linux) or winget/download into `Emulators\` (Windows); load EmuDeck configs at runtime | RetroDECK is Linux only, its sandbox makes host launches hard; EmuDeck configs exist for Linux (dragoonDorise/EmuDeck) and Windows (EmuDeck/emudeck-we), GPL-3.0, so not copied into the repo |
| Installing recomps | Quiver Launcher as a dependency, no fork | MIT, 2858 stars, CLI available, Proton/Wine runner |
| RomM | own component; `romm-api` crate as client; RomM-ES and romm-esde-bridge as templates | RomM-ES: good concept, weak code |
| License | MIT; call GPL tools only as external processes | |

> **Update 2026-10-23:** The desktop view is no longer built in heimspiel. It becomes its own project,
> Spieli (github.com/aronjanosch/spieli), which heimspiel installs and links from the setup wizard.
> See the project item "heimspiel: Spieli als Desktop-Ansicht mitliefern" and spieli `docs/concept.md`.

## ES-DE 3.5.0 (MIT)

- Game Importer (since 3.3) only via GUI, no CLI flag (`es-app/src/guis/GuiGameImporter.cpp`). It only copies
  `.desktop` files with a matching `Exec` (`steam://rungameid`, `heroic://launch`) to `<ROMs>/<system>/`.
  Rules: `resources/systems/linux/es_import_rules.xml`. Does not read Steam manifests. → We place the files ourselves.
- Systems `steam`/`epic` always launch with `%RUNINBACKGROUND%`; `desktop` has "Suspend ES-DE" and "Keep ES-DE running".
- `ports`: extensions .AppImage .desktop .exe .sh and others.
- CLI options: `main.cpp:400-435` (including `--home`, `--create-system-dirs`).
- Custom event scripts: `~/ES-DE/scripts/<event>/`, executable. `game-start` runs blocking without a timeout
  (`Scripting.cpp:112`), arguments: ROM path, name, system, system name (`FileData.cpp:2174`). It can neither abort
  nor change `%ROM%` (already substituted, `FileData.cpp:1956`) → replace the placeholder via the file system (symlink).
  `startup` runs before the systems are loaded (`main.cpp:758`).
- Custom systems: `~/ES-DE/custom_systems/es_systems.xml`, required: name, fullname, path, extension, command;
  the ROM folder must exist. Bundled systems with the same name are overridden. A theme without its own assets is fine (Linear).
- `gamelist.xml`: `~/ES-DE/gamelists/<system>/`, fields see `MetaData.cpp:23-48`, no custom fields.
  Read at startup; ES-DE writes changed entries back → only write while ES-DE is closed.
- Media: `<MediaDirectory>/<system>/<type>/<file stem>.<ext>` (`FileData.cpp:347`). Types: covers, screenshots,
  titlescreens, marquees, 3dboxes, backcovers, fanart, physicalmedia, videos, manuals.
- The scraper searches using `<name>` from the gamelist (on by default) → enter a recomp under the original title.
- Custom collections: `~/ES-DE/collections/custom-<Name>.cfg`, across systems.
- Emulator lookup: `es_find_rules.xml` (systempath, staticpath incl. Flatpak exports and `~/Applications`, corepath).
  Windows: `%ESPATH%\Emulators\…`, default paths, registry.

## Heroic 2.22.4 (GPL-3.0)

- One `.desktop` per game in `~/.local/share/applications` and on the desktop (`src/backend/shortcuts/shortcuts/shortcuts.ts:34-78`),
  `Exec=xdg-open heroic://launch?appName=…&runner=…`.
- Created automatically on install if `addDesktopShortcuts`/`addStartMenuShortcuts` is set (`src/backend/config.ts:331`),
  stored in the configStore (path unconfirmed, probably `~/.config/heroic/store/configStore.json`).
- Afterwards only via GUI. Alternative: read `installed.json` (legendary, gog_store, nile) and launch
  `heroic --no-gui heroic://launch…`.
- `xdg-open` returns immediately; the ES-DE system `epic` runs in background mode, so this is not a blocker.

## Steam

- ES-DE needs the `.desktop` files that Steam creates per game; if they are missing, create them ourselves
  (`steam steam://rungameid/<id>`).

## RomM integration

- **RomM-ES** (WirlyWirly/RomM-ES, MIT, ~950 lines of Python): placeholder `RomM-ES:<id>` in `~/ROMs/<system>`,
  covers in `downloaded_media`, `gamelist.xml` via string replacement, `game-start` hook downloads via curl.
  Shortcomings: token in plain-text log (`GameStart.py:150`), quoting bug in `GameStart.sh:12`, no cache limit,
  no multi-disc, deleted games remain, no timeouts/tests.
- **ferry** (pelluch/ferry, GPL-3.0): full sync instead of on demand, no gamelist/covers; good reference for API and idempotency.
- **romm-esde-bridge** (Mrzhao2018, MIT, alpha): ES-DE + on-demand cache, template.
- **romm-cli / `romm-api`** (patricksmill/romm-cli, MIT, Rust, CI): client crate with streaming downloads → our basis.
- Others: RomP (RompEmu, GPL-3.0, standalone client with emulators), romm-bifrost (symlinks), romm-fuse (FUSE),
  romm-steam-sync, launchbox-romm-plugin, official Playnite plugin (Windows only).
- RomM API: bearer token (`rmm_…`), endpoints include `/api/platforms`, `/api/roms`, `/api/roms/{id}/content/…`, `/api/firmware`.

## Recomps and ports

### Installation: Quiver Launcher (tgeorgiadis/quiver-launcher, MIT, C#/.NET 10, v3.5.1)

- Downloads release assets from GitHub/GitLab/Codeberg, updates, catalog (quiverlauncher.com). Core as its own library
  (`lib/QuiverLauncher.Core`). No build logic: prebuilt builds only.
- CLI (`Services/CLIHandler.cs`): `--list`, `--run <Name>`, `--download <Name>`, `--update`, `--uninstall`,
  `--add-steam-shortcut`, `--update-launcher`. Adding new catalog apps only via GUI.
- Windows builds on Linux via Proton or Wine (`Services/WindowsRunnerService.cs`), one prefix per game.
- The library lives next to the AppImage (`apps.json`, `Apps/`), fallback `~/.local/share/QuiverLauncher/`.
- Open: does `--run` block until the game exits? Upstream PR idea: `--list --json` / ES-DE export.

### Rejected

- Nyaldee/Ports-Launcher (GPL-3.0): GUI only, install path next to the binary, only 2 of 449 entries with an explicit
  Linux source, remote catalog without signature.
- RetroPortingToolKit/Retro-Launcher (MIT): catalog only self-compiling PS1/SNES recomps (106 titles), own UI;
  but RomM hash matching and catalog schema (`docs/CATALOG.md`: `rom_identity`, `bios_identity`) as a template.
- Decomp Buddy (GPL-3.0): Electron, Linux untested. omarchy-games-menu (MIT, Rust): own QuickShell overlay,
  but CLI patterns (`catalog.toml`, release checks, SteamGridDB) as a template.
- PortMaster: ARM handhelds.

### Catalogs

- **awesome-decomp-games** (SubmitGame, CC0): `data/games.json`, 2312 entries, updated daily.
  Fields include repo, release, release_date, type, platform, game_key, needs_assets, ratings, screenshot.
  No hashes, no asset info.
- **recomp.fyi** `.recomp.json` (schema `recomp.fyi/schema/v1.json`, CC BY 4.0, draft): game, wikidata, system, type,
  `original {region, revision, serial, sha1}`, targets, toolchain. No fields for target path/extraction.
  Board: ~2583 projects.
- Package sources: AUR has zelda64recomp-bin, soh-bin, banjorecomp-bin, bm64recomp-bin, starship-sf64, sm64coopdx, among others.
  Flathub and AM: none. soarpkgs: 2ship2harkinian, banjorecomp.

### How ports get their original

| Family | Mechanism | Automatable |
|---|---|---|
| N64Recomp (Zelda64, Banjo …) | dialog → hash check → stores `config_path/<game_id>.z64` (`librecomp/src/recomp.cpp` `select_rom()` from line 360, `stored_filename()` line 63) | probably: place .z64 (big-endian) beforehand, untested |
| HarbourMasters (SoH, 2S2H, Starship) | ROM → `.o2r`; `RunExtract(argc, argv)` (`soh/soh/OTRGlobals.cpp:411`), extractor `soh-torch --src --dest` | yes, via arguments or extractor (untested); hashes in `docs/supportedHashes.json` |
| sm64coopdx | `baserom.<ver>.z64` in the user path, md5 in `src/pc/rom_checker.cpp` | yes, fixed location |
| UnleashedRecomp | installer, container or folder, hash tables `install/hashes/` | partially |
| snesrev/zelda3 | `zelda3.sfc` + `extract_assets` | yes, self-build required |
| PS4 ports (Uncharted NDC, GoW3 in `gow3_pc/`) | dump folder `CUSA02344` via dialog; Windows build only | provide dump; repo without license |
| PS2Recomp (God of War) | extracted disc files, self-build (MSVC) | self-build |

### Extracting originals

| Format | Tool |
|---|---|
| PS2 ISO | `7z` / `bsdtar` |
| PS3 ISO (encrypted) | `ps3dec` (Redrrx/ps3dec, needs disc key), `libray` |
| X360 XISO | `extract-xiso`, `xdvdfs` |
| PS4 PKG | `ps4-pkg-tool` (xXJSONDeruloXx/ps4-pkg-tools, GPL-2.0, CLI, Linux; retail/update untested); shadPS4 has no CLI for this |

## Concept: provisioning manifest per port

```toml
[port.zelda64recomp]
quiver   = "Zelda64Recomp/Zelda64Recomp"
original = { system = "n64", sha1 = "…" }
provide  = { kind = "librecomp", file = "<game_id>.z64" }

[port.soh]
quiver   = "HarbourMasters/Shipwright"
original = { system = "n64", sha1 = ["…"] }
provide  = { kind = "args" }
```

Kinds: `librecomp` (place in the runtime folder), `args` (path at launch), `fixed` (fixed folder),
`dump` (extracted dump, kept locally), `dialog` (fallback).
Matching: RomM SHA1 against recomp.fyi `original.sha1`/`serial`, repo hash lists, otherwise title/`game_key`.

## Emulators on Windows

- winget: Libretro.RetroArch, DolphinEmulator.Dolphin, PCSX2Team.PCSX2, Stenzek.DuckStation, Cemu.Cemu,
  shadps4-emu.shadPS4, xemu-project.xemu. RPCS3, PPSSPP, Ryujinx, Xenia unclear → GitHub release.
- RetroBat rejected: Batocera ES fork, content partly CC BY-NC-SA.

## Open tests

1. `quiver --run`: does it block until the game exits? Clean Proton launch?
2. ES-DE on Hyprland: return and focus after Steam/Heroic games.
3. RomM-ES against our own RomM server (placeholder, hook, cover).
4. librecomp: is a pre-placed `.z64` accepted without a dialog?
5. SoH: ROM via argument or `soh-torch` without a dialog.
6. `ps4-pkg-tool` with a PKG.
7. Tauri/WebKitGTK on NVIDIA (possibly `WEBKIT_DISABLE_DMABUF_RENDERER=1`).

Results: [2026-10-23-tests.md](2026-10-23-tests.md).
