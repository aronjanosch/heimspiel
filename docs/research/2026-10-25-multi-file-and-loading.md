# Games of several files, resumable downloads and the loading window (2026-10-25)

Facts behind fetching RomM games made of several files, continuing cut-short downloads, and showing
progress while ES-DE waits. Sources: ES-DE 3.5.0 (tag `v3.5.0`, gitlab.com/es-de/emulationstation-de),
the RomM 5.4.1 server's `/openapi.json`, and tests against the user's RomM server.

## RomM 5.4.1

- **Every file of a game can be downloaded on its own:** `GET /api/roms/{file_id}/files/content/{file_name}`
  ("Get Romfile Content", the id is the file's, not the game's). It answers `Range: bytes=<from>-` with
  `206` and `content-range: bytes <from>-<last>/<size>` (tested with Metal Slug 6's `track01.iso` and Tekken 3's
  459 MB `.chd`). `/api/roms/{id}/content/{name}` also honours ranges but zips games of several files;
  `?file_ids=` picks files.
- `/api/roms/{id}` lists `files[]` with `id`, `file_name`, `file_path`, `file_size_bytes` and hashes
  (`md5_hash`, `sha1_hash`, `crc_hash`). A game of one file has exactly one entry. The `/api/roms` list
  sends `files: []`, so a sync needs one detail request per game with `has_multiple_files`.
- `file_path` is the server folder of the file: `roms/dc/Metal Slug 6` for a file of the folder game
  `Metal Slug 6` with `fs_path` `roms/dc`; deeper for subfolders (Cave Story: `roms/cavestory/data/Stage`).
  For a game of one file it equals `fs_path`.
- `fs_extension` is taken from the folder name: `svm` for `Day Of The Tentacle (CD Dos).svm`, `daphne`
  for `ace.daphne`, empty for `Metal Slug 6`.
- What the user's server holds as games of several files (68 platforms checked): a Dreamcast GDI with
  tracks (Metal Slug 6), SNES MSU-1 folders (`.sfc`, `.msu`, many `.pcm`), ScummVM `.svm` folders, Daphne
  `.daphne` folders, a NAOMI folder of two `.chd`, and artwork folders RomM took for games (`boxart`,
  `marquee`, `snap`, `miximages`, `videos`). No multi-disc PS1 game; PS1 games are single `.chd`/`.PBP`.

## ES-DE 3.5.0

- **Directories interpreted as files** (`USERGUIDE.md`, section of that name; `SystemData.cpp:668-700`):
  a folder whose name ends in an extension the system accepts is one game. If it holds a file or link with
  exactly the folder's name, that file is started (`FileData.cpp:1067-1082`), so
  `~/ROMs/dreamcast/Metal Slug 6.gdi/Metal Slug 6.gdi` is passed to the emulator; otherwise the folder
  itself (ScummVM, PS3).
- The `game-start` event receives that same path, the inner file (`FileData.cpp:2174`).
- **ES-DE shows black while `game-start` runs:** before firing the event it swaps buffers "to turn the
  screen black" unless the system runs in the background (`FileData.cpp:2167-2172`). Its own launch screen
  (`GuiLaunchScreen`) is therefore no progress display for heimspiel.
- `dreamcast`, `psx`, `ps2` and `saturn` all accept `.m3u` (`resources/systems/linux/es_systems.xml`);
  `dreamcast` also `.gdi`, `.cdi`, `.chd`, `.cue`.
- Hidden files are skipped (`SystemData.cpp:653`), so a marker `.heimspiel` inside a game folder does not
  show up.

## Tests (heimspiel, test ES-DE home in `~/.cache/heimspiel-tests`)

- Sync with `dc`, `psx`, `scummvm`: Metal Slug 6 became `dreamcast/Metal Slug 6.gdi/` with a Placeholder
  of the same name, the ScummVM games `.svm` folders, the artwork folder `miximages` was left out.
- Hook on `Metal Slug 6.gdi/Metal Slug 6.gdi`: 177 MB in 2.4 s; the folder then holds links to `disc.gdi`
  and the four tracks, its inner `Metal Slug 6.gdi` points at `disc.gdi`; all five MD5 sums match RomM's.
  The GDI names its tracks relative to itself (`1 0 4 2048 track01.iso 0`), and they resolve next to the
  link as well as next to the cached file. Not started in a real emulator (none installed).
- Tekken 3 (459 MB) stopped after 1.5 s with 345 MB in `Tekken 3 (USA).chd.part`; the next start fetched the
  rest with a range request in 0.6 s, MD5 matches.
- Two processes writing the same `.part` (an earlier, wrongly stopped test) produced a file 13 MB too
  large; heimspiel's size check refused it and dropped the part. ES-DE runs one `game-start` at a time.
- Cache limit 1 GiB with Day of the Tentacle, Tekken 3, Metal Slug 6 and Twisted Metal 2 cached (1.1 GiB):
  starting Tomb Raider dropped the two least recently played games, their links dangled, and starting
  Day of the Tentacle again fetched it anew and filled its folder.
- ES-DE skips links that point nowhere when it loads (`es_log.txt`: `File ".../Tekken 3 (USA).chd" does
  not exist, skipping entry`), so a game the cache dropped vanished from ES-DE. heimspiel now puts its
  Placeholder back: the hook after every download, `sync` on every run. Folders of games of several
  files stay, and ES-DE passes their inner link even when it dangles (`FileData.cpp:1071-1072` accepts
  any symlink).

## Loading window on Hyprland 0.56.2 (Omarchy, NVIDIA)

- `heimspiel-app progress` (Tauri, WebKitGTK) opens fullscreen (`fullscreen = 2`, 3840x2160) and renders the
  cover, title and progress; it closes when its input ends. After a failure it stays and closes on a key
  press (tested with `wtype -k Return`).
- Launched from a fullscreen test ES-DE through the `game-start` hook (Alien Trilogy, 274 MB): the window
  came up above ES-DE, ES-DE lost fullscreen meanwhile (`fullscreen = 0`; Omarchy sets
  `misc:on_focus_under_fullscreen = 1`), the fake emulator then received the downloaded game, and ES-DE was
  fullscreen again afterwards through the user's handler.
- Not tested: KDE Plasma and GNOME, whether a controller button dismisses a failure (WebKitGTK's
  gamepad support depends on its build), and `contrib/hyprland/es-de.lua` itself, which replaces the
  handler's "ES-DE alone on its workspace" with "every window opened on its workspace since has closed".
