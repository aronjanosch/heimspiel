# Research: PS5 homebrew landscape vs. heimspiel for PS5 (as of 2026-10-24)

Question: what already exists on a jailbroken PS5 that overlaps with an all-in-one hub that
auto-installs emulators, integrates RomM, provisions ports and offers a controller frontend?

Method: `gh` API calls (repo metadata, releases, READMEs) on 2026-10-24, plus web search where
GitHub search returned nothing (its search quota ran out mid-session). Dates are GitHub
`published_at` / `pushed_at` values. "Unverified" means only read in a README or search snippet,
not run on hardware. Star counts are as of 2026-10-24. Piracy storefronts are excluded; their
existence is noted only where a source mentions them.

Target console: firmware 12.70, Relapse exploit, kstuff-lite 1.11, ShadowMountPlus 1.7beta7.

## Summary

- The closest existing thing to the heimspiel vision is **RommPS** (RomM client, emulator profiles,
  save sync) plus **ProsperoStore** (installs from a signed catalog). Neither is a hub that starts
  emulators with a ROM.
- **No PS5 port of ES-DE found.** The RetroArch fork by mihawk-99 bundles ES-DE with a theme
  (inside RetroArch's own launcher), so ES-DE on PS5 exists in that form only.
- **No PS5 frontend found that launches an emulator with a ROM path from another app.** Every
  project found uses a file-drop model: the ROM is placed in the emulator's own folder.
  **Correction (2026-10-24):** the PS5 Forwarder Format
  (https://github.com/heydemoura/ps5-forwarder-format, SPEC.md) defines home-screen forwarders that
  call `sceSystemServiceLaunchApp(target, argv)` with `--rom <path>`; see
  `2026-10-24-ps5-feasibility.md`. The "none documented" statements below predate this finding.
- **Canonical emulator base:** the RetroArch fork (mihawk-99/PS5_RetroArch) with 33 native cores is
  the most complete. It is the default preset in RommPS. Standalone emulators exist for PS2, PS1,
  GameCube/Wii and N64 (partly).
- **Ports:** oot64-ps5 (Ship of Harkinian port) takes `oot.o2r` placed in the title folder.
  RecompStore is a store for ports that sets up games from the user's disc image.
- **Catalog:** ProsperoStore's signed JSON store API is documented and usable by an external
  tool for discovery and download. Installing onto the console still needs ShadowMountPlus and an
  ELF loader on TCP 9021.
- Gap: a hub that auto-provisions originals, maps a RomM library to emulators, and launches them
  from one controller UI. The pieces exist separately (RommPS, PS5SX2, RetroArch fork, RecompStore).
  Nothing ties them together.

## 1. Frontends and launchers

| Project | Repo / source | Last release (date) | Stars | What it launches | How it launches |
|---|---|---|---|---|---|
| RetroPapa PS5 (Beta) | [zx0CF1/RetroPapaPS5-Beta](https://github.com/zx0CF1/RetroPapaPS5-Beta) (GPL not stated, license null) | v1.1.44b, pre-release, 2026-08-27 | 8 | Built-in cores only: Mednafen (NES, Genesis, PSX), LakeSnes (SNES), FBNeo (arcade) | In-process cores. Resident ELF, autostarted through PayloadManager. README says no external emulator handoff. Source: [README](https://github.com/zx0CF1/RetroPapaPS5-Beta) |
| Nativehbl | [Rufidj/Nativehbl](https://github.com/Rufidj/Nativehbl) (GPL-3.0) | 0.1, 2026-06-02 | 24 | Homebrew only, not emulators | SDL2 UI writes launch requests; a bootstrap ELF consumes them and starts the homebrew as a bigapp. Needs the Homebrew Launcher PKG installed. Source: [README](https://github.com/Rufidj/Nativehbl) |
| omega-dashboard | [TheCriicom/omega-dashboard](https://github.com/TheCriicom/omega-dashboard) (GPL-3.0), replaces the Home screen | last push 2026-10-16 | 48 | Not verified | Repo description: open-source Home replacement with store, music, phone remote; "no game catalogues, no connection with Overdrive". Needs websrv. Launch mechanism unverified. Source: [GitHub API](https://github.com/TheCriicom/omega-dashboard) and [web search](https://gbatemp.net/posts/10917874/) |
| AnchorPS5 | [Kuroge/AnchorPS5](https://github.com/Kuroge/AnchorPS5) (homebrew manager) | catalog last push 2026-10-05 | catalog 0 | Homebrew apps | Catalog repo [TheCriicom/AnchorPS5-catalog](https://github.com/TheCriicom/AnchorPS5-catalog): `catalog.json` lists apps; the client takes downloads from each app's GitHub releases with SHA-256. Client launch model not checked |
| RetroLoader | none found | n/a | n/a | n/a | Not found on GitHub search or in web search. Treat as nonexistent until a URL is given |
| Retro Papa / RetroArch fork frontend picker | [mihawk-99/PS5_RetroArch](https://github.com/mihawk-99/PS5_RetroArch) | see section 2 | see section 2 | RetroArch or ES-DE | Picker chooses between RetroArch and EmulationStation (ES-DE, with Alekfull NX theme). Source: [README](https://github.com/mihawk-99/PS5_RetroArch) |

Notes:
- **ES-DE on PS5:** The RetroArch fork ships ES-DE as a second frontend inside its own title.
  ES-DE reads `content/<system>/` (folder names as ES-DE names them: `psx`, `ps2`, `snes`, `saturn`).
  Source: [mihawk-99/PS5_RetroArch README, "Your files" and credits](https://github.com/mihawk-99/PS5_RetroArch).
  No separate ES-DE port was found.
- **Porpoise** (GameCube/Wii) can add a home-screen tile in `/data/homebrew`. The tile "hands the
  launch to the jailbreak's payload loader". This is the only launch handoff found that is
  documented in a README. Source: [elripalda/Porpoise README](https://github.com/elripalda/Porpoise-Dolphin-Emulator-for-PS5).
- **No launch-with-arguments interface** (argv, URL scheme, IPC for ROM path) was documented in
  any of the frontends above. Nativehbl's request-file mechanism is the closest, but it is internal
  between its own UI and bootstrap ELF.

## 2. Emulators

### RetroArch fork (canonical base)

- Repo: [mihawk-99/PS5_RetroArch](https://github.com/mihawk-99/PS5_RetroArch), GPL-3.0, 392 stars,
  pushed 2026-10-19. Latest release: v4.0.0-beta-4, 2026-10-19 (per
  [releases](https://github.com/mihawk-99/PS5_RetroArch/releases)). Repo description says "35 cores &
  44 systems"; README says "33 native cores". The README count is used below.
- Install: copy the `PPSA99169` folder to `/data/homebrew/` (README step 1). WebUI on port 6769
  for uploads and game entries. Source: README "Get started".
- Core table (README, section "Supported systems"): NES (FCEUmm), GB/GBC/GBA, **SNES (Snes9x)**,
  N64 (Mupen64Plus-Next, Vulkan), GameCube/Wii (Dolphin, Vulkan), NDS (DeSmuME), 3DS (Azahar, Vulkan),
  Virtual Boy, Pokémon Mini, PSX (Beetle PSX HW, Vulkan), PS2 (**PS5SX2**, Vulkan), PSP (PPSSPP,
  Vulkan), Genesis/Master System/Game Gear/Sega CD, 32X (PicoDrive), Saturn (Beetle Saturn),
  Dreamcast/NAOMI, FinalBurn Neo (arcade, Neo Geo), MAME, Neo Geo Pocket, NeoCD, PC Engine,
  SuperGrafx, PC-FX, Atari 2600/5200/7800/Lynx/Jaguar, C64 (VICE), Amiga (PUAE), MS-DOS
  (DOSBox Pure), ScummVM, 3DO (Opera), WonderSwan (Beetle Cygne). Source: README section
  "Supported systems".
- ROM handoff from another app: **none documented** (no argv, no URL scheme). Content is added
  through the WebUI or FTP into `content/<system>/`. Source: README "Your files" and "Add your games".
- Relevance to heimspiel: a file-drop target. An external tool could place ROMs into
  `content/<system>/` over FTP (documented port 1337 is not verified; FTP is mentioned in README).

### Standalone PS5 emulators

| Emulator | Repo | Last release | Stars | Systems | ROM location / handoff |
|---|---|---|---|---|---|
| PS5SX2 (PCSX2 port) | [Swordpdf/PS5SX2](https://github.com/Swordpdf/PS5SX2), GPL-3.0 | 2.15, tag `vk-285-177`, 2026-10-19 | 1526 | PS2 | Own cover-flow shelf. Games in `/data/PCSX2/games` (or USB `games/`, `PCSX2/games/`, `PS5SX2/games/`), BIOS in `/data/PCSX2/bios`. Accepts `.iso`, `.chd`, `.cso`, `.zso`. Helper payload loaded with kstuff. ShadowMountPlus mounts it from `/data/homebrew/PPSA99203/`. No external launch argument documented. Source: [README](https://github.com/Swordpdf/PS5SX2) |
| PSXS5 | [SynoPiia/PSXS5](https://github.com/SynoPiia/PSXS5), GPL-3.0 | v2.2.0, 2026-10-09 | 93 | PS1 (PCSX-ReARMed based) | Own shelf. Accepts `.cue/.bin`, `.chd`, `.pbp`, `.iso`, `.img`, `.mdf`, `.ccd`, `.m3u`. Logs in `/data/PSXS5/logs/`. Source: [README](https://github.com/SynoPiia/PSXS5) |
| SwanStationPS5 | [darkxex/SwanStationPS5](https://github.com/darkxex/SwanStationPS5), GPL-3.0 | 1.4.8, 2026-10-24 | 78 | PS1 | Plays PSXS5 games, keeps its own saves in `/data/SwanStationPS5` (per RommPS README). Source: [RommPS README](https://github.com/s0liton/RommPS) |
| Porpoise | [elripalda/Porpoise-Dolphin-Emulator-for-PS5](https://github.com/elripalda/Porpoise-Dolphin-Emulator-for-PS5), GPL-3.0 | v4.0, 2026-10-21 | 204 | GameCube, Wii | Scans usual folders and USB/extended drives. Formats `.iso`, `.gcm`, `.rvz`, `.ciso`, `.gcz`, `.wbfs`, `.wia`. Home-screen tile hands launch to payload loader. Source: [README](https://github.com/elripalda/Porpoise-Dolphin-Emulator-for-PS5) |
| KB-RPCS3 | [kamb205/KB-RPCS3](https://github.com/kamb205/KB-RPCS3), license NOASSERTION | Alpha 0.1.0 source-only, 2026-10-09 | 89 | PS3 | Unofficial, not endorsed by RPCS3 or Sony. Source release only; web search reported the combined binary has not passed final PS5 testing. Source: [GitHub API](https://github.com/kamb205/KB-RPCS3), [web search](https://github.com/kamb205/KB-RPCS3/releases) (unverified claim about binary status) |
| ProsperoEden (Switch) | [Swordpdf/ProsperoEden](https://github.com/Swordpdf/ProsperoEden) (RommPS lists it as `blackbearreloaded/ProsperoEden`, redirect), GPL-3.0 | v1.005.000, 2026-10-18 | 876 | Switch | RommPS: manual support only. Source: [RommPS README](https://github.com/s0liton/RommPS) |
| PS5N64 | `Globaleliteee/PS5N64` | n/a | n/a | N64 | Listed by RommPS; GitHub API returns 404. Unverified |
| Snes9x standalone | none found | n/a | n/a | SNES | Web search found no PS5 port. Snes9x is a core inside the RetroArch fork (see above) |

Not searched: Mednafen standalone, PPSSPP standalone, DuckStation, Dolphin standalone. Coverage is
not exhaustive. GitHub search for "retroarch ps5", "ps5 mednafen" and "ps5 snes9x" hit the
search rate limit during the session; the first two queries were not re-run.

## 3. RomM integration

- **RommPS** ([s0liton/RommPS](https://github.com/s0liton/RommPS), GPL-3.0, 91 stars, created 2026-10-02,
  latest release v1.3.0 on 2026-10-22). Described as "a RomM server client for jailbroken PS5s".
  - Requires RomM 5.2.0 or newer and etaHEN or onionHEN with kstuff. Source: README "Requirements".
  - Background payload `romm-sync.elf`, autostarted by the HEN. Web UI at `http://<ps5-ip>:8780`.
    Manual payload send over TCP 9021 with `nc`. Source: README "Manual Install" and "How it runs".
  - Save sync (background monitoring, upload, conflict handling), activity log, signed releases.
    Source: README feature list.
  - **Handoff to emulators:** no launching. Games and BIOS are downloaded into each emulator's folder
    through emulator profiles (presets for RetroArch, PS5SX2, PSXS5, SwanStationPS5, Porpoise, PS5N64,
    PS5NES). Hardlinks let one game serve several emulators on the same platform. Source: README
    "Emulators" and "Emulator Profiles". RommPS states it relies on the emulator not being sandboxed.
  - Install channel: ProsperoStore (PPSA76677 per README). Source: README "Install".
  - Not handled: PS4/PS5 save sync (RomM's FPKGi is noted as a library source only). Source: README "PS4/PS5 Games".
- Other RomM clients for PS5: none found. GitHub search for "ps5 romm" returned no results. Not exhaustive.

## 4. Ports and recompilations

| Project | Repo | Last release | Stars | Original expected at | Notes |
|---|---|---|---|---|---|
| oot64-ps5 (Ship of Harkinian) | [mshivam019/oot64-ps5](https://github.com/mshivam019/oot64-ps5), GPL-3.0 | v1.2.0, 2026-10-05 | 29 | `oot.o2r` in the installed title's `assets/` folder | Ships no game data. The user converts their own OoT ROM to `oot.o2r` with desktop Ship of Harkinian 9.2.3 (extracts the ROM), then copies it over USB, FTP or file manager. Installs as PPSA99620. Source: [README](https://github.com/mshivam019/oot64-ps5) |
| Ship of Harkinian (desktop) | [HarbourMasters/Shipwright](https://github.com/HarbourMasters/Shipwright) release 9.2.3 | 9.2.3 | n/a | ROM input | Used by oot64-ps5 for extraction. Source: oot64-ps5 README link |
| RecompStore | [elripalda/RecompStore](https://github.com/elripalda/RecompStore), GPL-3.0 | v2.7.0, 2026-10-23 | 125 | Set up from the user's own disc image, Xbox Live Arcade file, TU/DLC, or ROM, copied to where the port expects it | Store app (PPSA92000) for recomps, emulators and tools. Installs from GitHub release `.zip`/`.7z`, checks SHA-256 against GitHub. Has a copier payload that places the user's files. Port list not extracted from the README. Source: [README](https://github.com/elripalda/RecompStore) |
| 2 Ship 2 Harkinian, Zelda64Recomp, others on PS5 | not searched successfully | n/a | n/a | n/a | "2ship2harkinian ps5" returned no results. Zelda64Recomp on PS5 not checked. Mark unverified |

## 5. Stores and catalogs

### ProsperoStore (store app)

- Repo: [darkxex/ProsperoStore](https://github.com/darkxex/ProsperoStore), GPL-3.0, C++, 396 stars,
  latest release v1.000.105 on 2026-10-21. Title PPSA99000.
- Requirements: ShadowMountPlus 1.7beta4 or newer, and a local ELF loader on TCP 9021. It includes an
  exact-title one-shot helper built from [PS5-Lapy-JB-Daemon](https://github.com/mpereiraesaa/PS5-Lapy-JB-Daemon).
  Installs, updates and uninstalls verified on firmware 6.02 and 12.70 per README. Source: README "Install".
- Install path: copies the app folder into a ShadowMountPlus-scanned folder, e.g. `/data/homebrew/`.
  Source: README "Install".

### Catalog repo and store API (open)

- Catalog: [darkxex/ps5-homebrew-catalog](https://github.com/darkxex/ps5-homebrew-catalog), GPL-3.0,
  55 stars, pushed 2026-10-24. One JSON record per app in `apps/<TITLEID>.json`. Native PS5 apps only:
  "emulator ROMs and web pages aren't listed". Source: [README](https://github.com/darkxex/ps5-homebrew-catalog).
- Store API (documented in [docs/api.md](https://github.com/darkxex/ps5-homebrew-catalog/blob/main/docs/api.md),
  base `https://prosperostore.net/api/v1/`):
  - `versions.json`: current version of every app.
  - `index.json`: one compact entry per app.
  - `apps/<ID>.json`: full record. Schema 3. Fields include `titleid`, `version`, `source_repo`,
    `artifact_url` (developer's GitHub release), `sha256`, `format` (`zip` or `ffpfsc`), `size`,
    `safety` (sandbox, routes, helpers, network).
  - `manifest.json` and `manifest.sig`: hashes of all JSON files and a signature. Clients use it to
    verify the catalog before installing.
  - `news.json` and `news.sig`, icons under `icons/`.
  Source: [docs/api.md](https://github.com/darkxex/ps5-homebrew-catalog/blob/main/docs/api.md).
- Programmatic use: discovery and download are feasible from the public JSON. An external tool can
  read `index.json`, verify `manifest.sig`, fetch `artifact_url`, check `sha256`. **Putting the app on
  the console** still needs ShadowMountPlus and an ELF loader on the PS5. This is an inference from
  the README, not a tested path. The signature scheme and key are not documented in the parts read.
- Mirrors and alternates: [Kruppes/ps5-store-feed](https://github.com/Kruppes/ps5-store-feed)
  (static mirror, schema 3, 0 stars, 2026-10-12); [heydemoura/homebrew-catalog-base](https://github.com/heydemoura/homebrew-catalog-base)
  (template, 2026-10-20); [quatrixone/p5-marketplace](https://github.com/quatrixone/p5-marketplace)
  (PS4/PS5 catalog for P5 Manager, 2026-10-14); [AnchorPS5-catalog](https://github.com/TheCriicom/AnchorPS5-catalog)
  (2026-10-05).
- Desktop sender: [Pharaoh2k/PlayStation-Payload-Center](https://github.com/Pharaoh2k/PlayStation-Payload-Center),
  Avalonia/.NET 10, 15 stars, pushed 2026-10-16. Discovers, downloads, SHA-256-verifies and sends
  homebrew payloads to a PS4/PS5 over the network. Relevant as an existing desktop-side sender. Source: [GitHub API](https://github.com/Pharaoh2k/PlayStation-Payload-Center).

### RecompStore catalog

- The store reads a catalog that the site admin maintains (`site/public_html/admin/`). Format and
  public access not checked. Source: [RecompStore README](https://github.com/elripalda/RecompStore).

### homebrew.page

- Not checked. Mark as unverified; no source read in this session.

## 6. Gap analysis

| heimspiel need | Exists on PS5 | Gap |
|---|---|---|
| Install emulators | ProsperoStore / AnchorPS5 / RecompStore install apps from GitHub releases with SHA-256 checks | No single hub that installs and configures the emulators in one flow. Each is a separate app |
| Controller frontend | RetroArch fork's ES-DE picker; Retro Papa; Nativehbl (homebrew only); omega-dashboard (Home replacement) | No frontend shows a unified library across emulators and ports. ES-DE is inside RetroArch only, not standalone |
| RomM library | RommPS (download into emulator folders, save sync, profiles, web UI) | No launch step. Downloads and saves only. Requires RomM 5.2.0+ |
| Emulators with ROMs | PS5SX2 (PS2), PSXS5 and SwanStation (PS1), Porpoise (GC/Wii), RetroArch fork (33 cores) | Each reads its own folder. No documented external ROM handoff (no argv, URL scheme or IPC) |
| Ports with originals | oot64-ps5 (`oot.o2r`), RecompStore (disc image setup) | Originals are placed by the user or by RecompStore's copier. No cross-app matching by hash, no RomM link |
| Stores and catalogs | ProsperoStore signed JSON API and catalog | Installs need ShadowMountPlus and an ELF loader on the console. External tool path untested |
| Provisioning BIOS | RommPS downloads BIOS files to emulator folders (per README) | Not checked per emulator |

Overall: the building blocks exist, but each is an independent app with a file-drop model. The
missing piece is a launch and provisioning layer on the console side, fed by RomM and the store
API. No project found implements that layer; the closest are RommPS (data side) and
ProsperoStore's install helper (app side).

## Unverified or not found

- RetroLoader: not found (GitHub search and web search).
- Omega dashboard launch mechanism: unverified.
- AnchorPS5 client launch model: not checked.
- PS5N64: repo 404.
- KB-RPCS3 binary status: from a web snippet, not from the repo.
- 2 Ship 2 Harkinian and Zelda64Recomp on PS5: not found in the search that ran.
- homebrew.page: not checked.
- RetroArch fork core count: 33 in README vs 35 in repo description.
- Standalone Mednafen, PPSSPP, DuckStation, Dolphin: not searched.
