# Recherche: Landschaft und Bausteine (Stand 2026-10-23)

Ergebnis einer Recherche-Session mit Code-Gutachten der beteiligten Projekte.
Belege als Datei:Zeile beziehen sich auf den jeweiligen Upstream-Stand vom 2026-10-23.
"Unbestätigt" heißt: nur aus Doku oder Code gelesen, nicht ausgeführt.

## Zielbild

Eine Bibliothek für alles: Steam, Heroic, manuelle PC-Spiele, RomM (oder lokale Ordner),
Recomps und Ports. Ausgaben: ES-DE (Controller) und eine Desktop-Ansicht in der eigenen
Tauri-App. Die App ist außerdem Einrichtungs-Assistent (wie EmuDeck). Linux zuerst, Windows später.
Später weitere Ausgaben: Steam (`shortcuts.vdf`), Playnite.

## Entscheidungen

| Thema | Entscheidung | Grund |
|---|---|---|
| Frontend | ES-DE 3.5, kein Fork | von außen gut befüllbar (Note 4/5) |
| Desktop-Ansicht | eigene, schlanke Ansicht in der Tauri-App | kein fertiges Desktop-Frontend liest ES-DE-Daten (Pegasus seit 2024 ohne Release, Playnite nur Windows, LaunchBox Linux frühestens 2027) |
| Technik | Tauri + Rust-Kern | eine Binary pro OS, Installer-UI inklusive |
| Emulatoren | kein RetroDECK; Flatpak (Linux) bzw. winget/Download in `Emulators\` (Windows); Configs von EmuDeck zur Laufzeit laden | RetroDECK nur Linux, Sandbox erschwert Host-Starts; EmuDeck-Configs gibt es für Linux (dragoonDorise/EmuDeck) und Windows (EmuDeck/emudeck-we), GPL-3.0, deshalb nicht ins Repo kopieren |
| Recomps installieren | Quiver Launcher als Abhängigkeit, kein Fork | MIT, 2858 Sterne, CLI vorhanden, Proton/Wine-Runner |
| RomM | eigener Teil; `romm-api`-Crate als Client; RomM-ES und romm-esde-bridge als Vorlage | RomM-ES: Konzept gut, Code schwach |
| Lizenz | MIT; GPL-Tools nur als externe Prozesse aufrufen | |

## ES-DE 3.5.0 (MIT)

- Game Importer (seit 3.3) nur per GUI, kein CLI-Flag (`es-app/src/guis/GuiGameImporter.cpp`). Er kopiert nur
  `.desktop`-Dateien mit passendem `Exec` (`steam://rungameid`, `heroic://launch`) nach `<ROMs>/<system>/`.
  Regeln: `resources/systems/linux/es_import_rules.xml`. Liest keine Steam-Manifeste. → Wir legen die Dateien selbst ab.
- Systeme `steam`/`epic` starten immer mit `%RUNINBACKGROUND%`; `desktop` hat "Suspend ES-DE" und "Keep ES-DE running".
- `ports`: Endungen .AppImage .desktop .exe .sh u. a.
- CLI-Optionen: `main.cpp:400-435` (u. a. `--home`, `--create-system-dirs`).
- Custom Event Scripts: `~/ES-DE/scripts/<event>/`, ausführbar. `game-start` läuft blockierend ohne Timeout
  (`Scripting.cpp:112`), Argumente: ROM-Pfad, Name, System, Systemname (`FileData.cpp:2174`). Kann weder abbrechen
  noch `%ROM%` ändern (bereits ersetzt, `FileData.cpp:1956`) → Platzhalter per Dateisystem ersetzen (Symlink).
  `startup` läuft vor dem Laden der Systeme (`main.cpp:758`).
- Custom Systems: `~/ES-DE/custom_systems/es_systems.xml`, Pflicht: name, fullname, path, extension, command;
  ROM-Ordner muss existieren. Gleichnamige Bundle-Systeme werden überschrieben. Theme ohne eigene Assets ok (Linear).
- `gamelist.xml`: `~/ES-DE/gamelists/<system>/`, Felder siehe `MetaData.cpp:23-48`, keine freien Felder.
  Wird beim Start gelesen; ES-DE schreibt geänderte Einträge zurück → nur bei geschlossenem ES-DE schreiben.
- Medien: `<MediaDirectory>/<system>/<typ>/<Dateistamm>.<ext>` (`FileData.cpp:347`). Typen: covers, screenshots,
  titlescreens, marquees, 3dboxes, backcovers, fanart, physicalmedia, videos, manuals.
- Scraper sucht mit `<name>` aus der gamelist (Default an) → Recomp als Originaltitel eintragen.
- Custom Collections: `~/ES-DE/collections/custom-<Name>.cfg`, systemübergreifend.
- Emulator-Suche: `es_find_rules.xml` (systempath, staticpath inkl. Flatpak-Exports und `~/Applications`, corepath).
  Windows: `%ESPATH%\Emulators\…`, Standardpfade, Registry.

## Heroic 2.22.4 (GPL-3.0)

- `.desktop` pro Spiel in `~/.local/share/applications` und Desktop (`src/backend/shortcuts/shortcuts/shortcuts.ts:34-78`),
  `Exec=xdg-open heroic://launch?appName=…&runner=…`.
- Automatisch bei Installation, wenn `addDesktopShortcuts`/`addStartMenuShortcuts` gesetzt (`src/backend/config.ts:331`),
  gespeichert im configStore (Pfad unbestätigt, vermutlich `~/.config/heroic/store/configStore.json`).
- Nachträglich nur per GUI. Alternative: `installed.json` lesen (legendary, gog_store, nile) und
  `heroic --no-gui heroic://launch…` starten.
- `xdg-open` kehrt sofort zurück; ES-DE-System `epic` läuft im Hintergrund-Modus, daher kein Blocker.

## Steam

- ES-DE braucht die `.desktop`-Dateien, die Steam pro Spiel anlegt; fehlen sie, selbst erzeugen
  (`steam steam://rungameid/<id>`).

## RomM-Anbindung

- **RomM-ES** (WirlyWirly/RomM-ES, MIT, ~950 Zeilen Python): Platzhalter `RomM-ES:<id>` in `~/ROMs/<system>`,
  Cover in `downloaded_media`, `gamelist.xml` per String-Ersetzung, `game-start`-Hook lädt per curl.
  Mängel: Token im Klartext-Log (`GameStart.py:150`), Quoting-Fehler in `GameStart.sh:12`, kein Cache-Limit,
  kein Multi-Disc, gelöschte Spiele bleiben, keine Timeouts/Tests.
- **ferry** (pelluch/ferry, GPL-3.0): Voll-Sync statt On-Demand, keine gamelist/Cover; gute Referenz für API und Idempotenz.
- **romm-esde-bridge** (Mrzhao2018, MIT, Alpha): ES-DE + On-Demand-Cache, Vorlage.
- **romm-cli / `romm-api`** (patricksmill/romm-cli, MIT, Rust, CI): Client-Crate mit Streaming-Downloads → Basis für uns.
- Weitere: RomP (RompEmu, GPL-3.0, eigenständiger Client mit Emulatoren), romm-bifrost (Symlinks), romm-fuse (FUSE),
  romm-steam-sync, launchbox-romm-plugin, offizielles Playnite-Plugin (nur Windows).
- RomM-API: Bearer-Token (`rmm_…`), Endpunkte u. a. `/api/platforms`, `/api/roms`, `/api/roms/{id}/content/…`, `/api/firmware`.

## Recomps und Ports

### Installation: Quiver Launcher (tgeorgiadis/quiver-launcher, MIT, C#/.NET 10, v3.5.1)

- Lädt Release-Assets von GitHub/GitLab/Codeberg, Updates, Katalog (quiverlauncher.com). Kern als eigene Lib
  (`lib/QuiverLauncher.Core`). Keine Build-Logik: nur fertige Builds.
- CLI (`Services/CLIHandler.cs`): `--list`, `--run <Name>`, `--download <Name>`, `--update`, `--uninstall`,
  `--add-steam-shortcut`, `--update-launcher`. Aufnahme neuer Katalog-Apps nur per GUI.
- Windows-Builds unter Linux über Proton oder Wine (`Services/WindowsRunnerService.cs`), Prefix pro Spiel.
- Bibliothek liegt neben dem AppImage (`apps.json`, `Apps/`), Fallback `~/.local/share/QuiverLauncher/`.
- Offen: blockiert `--run` bis Spielende? Upstream-PR-Idee: `--list --json` / ES-DE-Export.

### Verworfen

- Nyaldee/Ports-Launcher (GPL-3.0): nur GUI, Install-Pfad neben Binary, nur 2 von 449 Einträgen mit expliziter
  Linux-Quelle, Remote-Katalog ohne Signatur.
- RetroPortingToolKit/Retro-Launcher (MIT): Katalog nur selbstkompilierende PS1/SNES-Recomps (106 Titel), eigene UI;
  aber RomM-Hash-Matching und Catalog-Schema (`docs/CATALOG.md`: `rom_identity`, `bios_identity`) als Vorlage.
- Decomp Buddy (GPL-3.0): Electron, Linux ungetestet. omarchy-games-menu (MIT, Rust): eigenes QuickShell-Overlay,
  aber CLI-Muster (`catalog.toml`, Release-Checks, SteamGridDB) als Vorlage.
- PortMaster: ARM-Handhelds.

### Kataloge

- **awesome-decomp-games** (SubmitGame, CC0): `data/games.json`, 2312 Einträge, täglich aktualisiert.
  Felder u. a. repo, release, release_date, type, platform, game_key, needs_assets, ratings, screenshot.
  Keine Hashes, keine Asset-Infos.
- **recomp.fyi** `.recomp.json` (Schema `recomp.fyi/schema/v1.json`, CC BY 4.0, Draft): game, wikidata, system, type,
  `original {region, revision, serial, sha1}`, targets, toolchain. Keine Felder für Zielpfad/Extraktion.
  Board: ~2583 Projekte.
- Paketquellen: AUR hat u. a. zelda64recomp-bin, soh-bin, banjorecomp-bin, bm64recomp-bin, starship-sf64, sm64coopdx.
  Flathub und AM: keine. soarpkgs: 2ship2harkinian, banjorecomp.

### Wie Ports ihr Original bekommen

| Familie | Mechanismus | Automatisierbar |
|---|---|---|
| N64Recomp (Zelda64, Banjo …) | Dialog → Hash-Prüfung → speichert `config_path/<game_id>.z64` (`librecomp/src/recomp.cpp` `select_rom()` ab Z. 360, `stored_filename()` Z. 63) | vermutlich: .z64 (big-endian) vorab ablegen, ungetestet |
| HarbourMasters (SoH, 2S2H, Starship) | ROM → `.o2r`; `RunExtract(argc, argv)` (`soh/soh/OTRGlobals.cpp:411`), Extraktor `soh-torch --src --dest` | ja, über Argumente bzw. Extraktor (ungetestet); Hashes in `docs/supportedHashes.json` |
| sm64coopdx | `baserom.<ver>.z64` im User-Pfad, md5 in `src/pc/rom_checker.cpp` | ja, fester Ort |
| UnleashedRecomp | Installer, Container oder Ordner, Hash-Tabellen `install/hashes/` | teilweise |
| snesrev/zelda3 | `zelda3.sfc` + `extract_assets` | ja, Selbstbau nötig |
| PS4-Ports (Uncharted NDC, GoW3 in `gow3_pc/`) | Dump-Ordner `CUSA02344` per Dialog; nur Windows-Build | Dump bereitstellen; Repo ohne Lizenz |
| PS2Recomp (God of War) | entpackte Disc-Dateien, Selbstbau (MSVC) | Selbstbau |

### Originale entpacken

| Format | Tool |
|---|---|
| PS2-ISO | `7z` / `bsdtar` |
| PS3-ISO (verschlüsselt) | `ps3dec` (Redrrx/ps3dec, braucht Disc-Key), `libray` |
| X360 XISO | `extract-xiso`, `xdvdfs` |
| PS4 PKG | `ps4-pkg-tool` (xXJSONDeruloXx/ps4-pkg-tools, GPL-2.0, CLI, Linux; Retail/Update ungetestet); shadPS4 hat kein CLI dafür |

## Konzept: Provisioning-Manifest pro Port

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

Arten: `librecomp` (in Runtime-Ordner legen), `args` (Pfad beim Start), `fixed` (fester Ordner),
`dump` (entpackter Dump, dauerhaft lokal), `dialog` (Rückfall).
Matching: RomM-SHA1 gegen recomp.fyi `original.sha1`/`serial`, Repo-Hashlisten, sonst Titel/`game_key`.

## Emulatoren unter Windows

- winget: Libretro.RetroArch, DolphinEmulator.Dolphin, PCSX2Team.PCSX2, Stenzek.DuckStation, Cemu.Cemu,
  shadps4-emu.shadPS4, xemu-project.xemu. RPCS3, PPSSPP, Ryujinx, Xenia unklar → GitHub-Release.
- RetroBat verworfen: Batocera-ES-Fork, Inhalte teils CC BY-NC-SA.

## Offene Tests

1. `quiver --run`: blockiert bis Spielende? Proton-Start sauber?
2. ES-DE unter Hyprland: Rückkehr und Fokus nach Steam-/Heroic-Spielen.
3. RomM-ES gegen den eigenen RomM-Server (Platzhalter, Hook, Cover).
4. librecomp: vorab abgelegte `.z64` ohne Dialog akzeptiert?
5. SoH: ROM per Argument bzw. `soh-torch` ohne Dialog.
6. `ps4-pkg-tool` mit einem PKG.
7. Tauri/WebKitGTK auf NVIDIA (ggf. `WEBKIT_DISABLE_DMABUF_RENDERER=1`).
