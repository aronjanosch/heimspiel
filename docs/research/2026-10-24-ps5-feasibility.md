# Research: PS5 feasibility for heimspiel (as of 2026-10-24)

Question: can heimspiel reach a jailbroken PS5 (FW 12.70; tethered jailbreak with kstuff-lite 1.11,
ShadowMountPlus 1.7beta7, elfldr on 9021, FTP via ftpsrv on 2121), and which of three approaches is
realistic: (A) a PC-side "PS5 Output", (B) a native on-console hub, (C) an ES-DE port for the PS5.

"Unverified" means: read in docs or code only, not executed on hardware. Sources were read with
`gh api` / the GitHub and GitLab APIs on 2026-10-24. Upstream file paths are relative to the named repo.
Scratch copies of the fetched READMEs are in `/tmp/claude-1000/-home-aron-Work/babd7ef4-29ef-4793-a94d-b0351be887d9/scratchpad/ps5/readme/`.

## Summary

- **Launching a ROM from another app on the PS5 is already solved by a documented contract.** The
  PS5 Forwarder Format passes `--rom <path>` as argv to a target title through
  `sceSystemServiceLaunchApp`. PS5 RetroArch uses `sceSystemServiceLoadExec` with its own eboot and
  argv. Both are documented; neither needs a custom launcher built from scratch.
- **ES-DE already runs on the PS5**, inside the PS5 RetroArch title (GPL-3.0-or-later, v4.0.0-beta-4,
  pushed 2026-10-19). It is ES-DE 3.5.0 plus a 10-file, 419-line port patch, and it launches games only
  through RetroArch's game mode. Option C therefore means "reuse or extend an existing port", not "fork
  from zero".
- **A is the cheapest start.** The forwarder format says a tool "without C" can write a forwarder by
  copying files over FTP into `/data/homebrew/`. ROM push is plain FTP. The fragile parts are the
  tethered jailbreak, the beta ShadowMountPlus, and per-port originals.
- **B exists in several forms already** (Jewel Case, Forwarder Manager, PS5-ECAM, RommPS, PS5 RetroArch's
  picker). Building a new hub duplicates them.
- **Recommendation:** start with A, scoped to forwarders plus FTP ROM push. Spike C only as a
  consumer of the existing PS5 RetroArch ES-DE, not as a port of our own. Do not start B.

## Effort estimates (rough, my judgement, unverified)

| Approach | MVP | Full | Main risk |
|---|---|---|---|
| A. PC-side Output (forwarders over FTP, ROM push, ShadowMountPlus status) | 3 to 5 weeks | 2 to 3 months | beta ShadowMountPlus behaviour, per-port originals, tethered FTP |
| A + own catalog install from PC | +1 to 2 weeks | | duplicates ProsperoStore; signed-catalog verification |
| B. Own on-console hub | 4 to 8 weeks | 2 to 4 months | duplicates existing launchers; sandbox and launch rules |
| C. Own ES-DE port | 2 to 4 months (GL/SDL/FFmpeg/curl build chain) | more | multi-library build, GL stack, RetroArch-only launch |
| C'. Consume PS5 RetroArch's ES-DE | 1 to 2 weeks spike | | upstream title is one repo with RetroArch in it |

## A. PC-side "PS5 Output" in heimspiel

### What the PS5 side offers

- **FTP:** `ps5-payload-dev/ftpsrv` serves on port 2121, is GPLv3+, was pushed 2026-10-20, and must be
  launched through an ELF loader. Its README says "The FTP server accepts connection on port 2121" and
  lists SITE commands MTRW (remount `/system` writable) and SELF. https://github.com/ps5-payload-dev/ftpsrv
  (README). Because the jailbreak is tethered, ftpsrv must be sent again after every reboot (user
  statement, not verified in repo).
- **ELF loader:** elfldr on TCP 9021. `ps5-payload-dev/sdk` README: "ELF loaders known to work include
  ps5-payload-elfldr". https://github.com/ps5-payload-dev/sdk (README).
- **ShadowMountPlus** (`drakmor/ShadowMountPlus`, GPL-3.0, release `1.7beta7` published 2026-10-21 as
  non-prerelease). It scans `/data/homebrew` and mounts `.ffpkg`, `.exfat`, `.ffpfs`, `.ffpfsc` images.
  Its HTTP/JSON API (`api_bind_address` default `127.0.0.1`, `api_port` 10101) has **no authentication**
  (docs/api.md: "The API has no authentication. Bind it to a network-facing address only on a trusted
  network."). Routes relevant to A: `/api/v1/version`, `/api/v1/storage`, `/api/v1/games`,
  `/api/v1/games/info`, `/api/v1/scan`, `/api/v1/games/mount|unmount|copy|move|delete`,
  `/api/v1/manual/add|remove`, `/api/v1/games/fakelib`.
  **There is no install route**: homebrew install = place the folder, then scan.
  Sources: https://github.com/drakmor/ShadowMountPlus (README.md, docs/api.md, docs/openapi.yaml).
  Note: the README says 1.7 requires kstuff-lite v1.07+ and the README and catalog both say
  "warning: mounting images can cause shutdown problems and data corruption on internal drives".
- **Catalog and store:** `darkxex/ps5-homebrew-catalog` (GPL-3.0, pushed 2026-10-24) lists **native
  apps only**: "ELF payloads, PS4 packages, backports, emulator ROMs and web pages aren't listed"
  (README). Entries are one JSON per title ID with `artifact_url`, `sha256`, `version`, `kind`
  (`app` 74, `game` 49, `tool` 6 at the time of reading). Only `.zip` of the app folder is accepted
  (docs/artifact-formats.md). https://github.com/darkxex/ps5-homebrew-catalog
  The store API is at `https://prosperostore.net/api/v1/` with a signed `manifest.json` and `manifest.sig`
  (docs/api.md).
- **ProsperoStore** (`darkxex/ProsperoStore`, GPL-3.0, pushed 2026-10-22) installs on the console. Its
  README: "Installs, updates and uninstalls have run on firmware 6.02 and 12.70. Other firmware is
  experimental", and it needs ShadowMountPlus 1.7beta4 or newer plus elfldr on 9021.
  https://github.com/darkxex/ProsperoStore (README).

### What a PC-side Output can do

1. **Forwarders over FTP (no C required).** The forwarder format is a plain folder: copy
   `template/eboot.bin` and `template/sce_module/libc.prx` into a folder named after a free title ID
   (`PPSA99200` to `PPSA99899`), write `forwarder.json` and `sce_sys/param.json`, add a 512x512
   `sce_sys/icon0.png`, and place the folder in `/data/homebrew/`. Quote: "For tool authors without C ...
   place the folder in `/data/homebrew/`." https://github.com/heydemoura/ps5-forwarder-format (README,
   "For tool authors without C"; SPEC.md sections 2, 5). The template files are binaries from that repo
   and should be **fetched at runtime or from a release, not copied into heimspiel** (ADR 0005 pattern).
2. **ROM push.** Jewel Case reads `/data/roms/<system>/`, and `/mnt/usb0/roms` (its README). PS5 RetroArch
   reads `/mnt/usb0/roms/...` or `content/<system>/` under the title folder (its README, "Your files" and
   `--rom` section). PS5-ECAM uses `/data/ps5cemu/<emulator>/...`. The target folder differs per
   emulator; the Output must map Entries to each emulator's folder, the same way ADR 0001 maps files.
3. **Status and scan.** ShadowMountPlus `/api/v1/scan` and `/api/v1/games` give mount state after
   copying. Use `/api/v1/scan` after FTP upload, not a reboot.
4. **Catalog install from PC.** Possible (signed `manifest.json` + zip), but it duplicates ProsperoStore,
   which already does it on the console. Defer.

### Per-port originals (Provisioning)

- Ports often need the user's own data, and some convert on the console. Examples from the catalog:
  Ship of Harkinian (Vulkan) "converts your own ROM on the console" (PPSA99320); Link's Awakening
  "requires game files prepared from your own Switch version 1.0.0 base-game" (PPSA99670); Wind Waker HD
  "requires game files prepared from your own USA Wii U dump" (PPSA99641). Catalog descriptions, not
  verified on hardware.
- So Provisioning on PS5 is per-port. The GLOSSARY term fits, but a PC cannot always finish it. This
  matches ADR 0007's "decrypted Dump only" stance.

### Fragility

- **Tethered jailbreak**: every reboot needs the exploit, elfldr and ftpsrv re-sent. Unverified how the
  exploit is run; user says Relapse + kstuff-lite 1.11.
- **kstuff-lite 1.11-opt**: README says it "builds on the existing kstuff-lite crypto, FSELF, NPDRM, and
  loader work" (https://github.com/EchoStretch/kstuff-lite). The repo has no license field (GitHub API
  returns `license: null`). Firmware list not in README, so FW 12.70 support is the user's statement,
  unverified here.
- **ShadowMountPlus** is in beta at `1.7beta7`; `1.7beta6-fix1` and `fix2` were prereleases. The same
  README warns about data corruption on internal drives.
- **Security**: the HTTP API has no auth; default bind is loopback. Keep the default.
- **Stub-level fragility of other projects**: PS5-ECAM and RommPS warn that emulators must not be
  sandboxed to read `/data` (RommPS README: "RommPS relies on your emulator NOT being sandboxed").

## B. Native on-console hub

### Can one app launch another app with a ROM argument?

Yes, documented in two independent projects.

1. **Forwarder format (SPEC.md section 7).** A resident `launcher.elf` listens on 127.0.0.1:10199. The
   request carries a target title ID and up to 64 arguments (max 4096 bytes). The launcher calls
   `sceSystemServiceLaunchApp(target, argv, param)`. A caller that is itself the running app gets
   `0x80940010` ("queued"); the system closes the caller and then starts the target.
   Source: https://github.com/heydemoura/ps5-forwarder-format (SPEC.md section 7, launcher/launcher.c).
   Registry (registry/targets.json) lists targets with their arguments: ProsperoEden `--rom <path>
   --exit-after-game`; Porpoise `--rom`; PS5SX2 `--rom`; PS5X360 `--rom`. Porpoise and PS5SX2 were
   named as the target of forwarder launches in the forwarder manager README. No own launcher is needed
   (Forwarder Manager README: "No separate launcher payload is needed").
2. **PS5 RetroArch game mode.** `src/ps5_game.c` calls `sceSystemServiceLoadExec(PS5_GAME_EBOOT, argv)`
   with `PS5_GAME_EBOOT = /app0/eboot.bin` and writes `/app0/game-request.txt`. When RetroArch quits, the
   title writes `/app0/game-result.txt` and restarts as the frontend. Argument syntax:
   `--rom <file> --core <core> --exit-after-game`.
   Sources: https://github.com/mihawk-99/PS5_RetroArch (src/ps5_game.c, src/ps5_game.h,
   frontends/es-de/ps5/game_ps5.cpp).
   Caveat: "Arguments only reach a new process: if the title is already running, a forwarder should
   close it first." (PS5 RetroArch README, forwarder section.)

### Existing hubs (so B is not greenfield)

| Title | Id | What it does | Source |
|---|---|---|---|
| Jewel Case | PPSA75837, GPL-3.0 | home-screen ROM launcher, NES to Xbox 360, box art | github.com/SkaRift/JewelCase |
| Forwarder Manager | PPSA99930, GPL-3.0 | creates forwarder tiles with `--rom`, icons, arguments | github.com/heydemoura/ps5-forwarder-manager |
| PS5-ECAM | PPSA99360, GPL-3.0 | one launcher for Cemu, Azahar, melonDS, Eden (ProsperoEden) | github.com/premohq/PS5-ECAM |
| RommPS | PPSA76677, GPL-3.0 | RomM client, saves and game downloads | github.com/s0liton/RommPS |
| Estante | PPSA99841, GPL-3.0 | library organiser incl. emulator libraries | github.com/kauansilvalopes/estante |
| PS5 RetroArch | PPSA99169, GPL-3.0-or-later | RetroArch + EmulationStation (ES-DE) picker, WebUI, 33 cores | github.com/mihawk-99/PS5_RetroArch |

Also a native SDL2 launcher exists: `Rufidj/Nativehbl` (GPL-3.0), built with SDL2 and
`ps5-payload-sdk`, which "still relies on the same bigapp-style homebrew launch path as `websrv`"
(its README). RetroPapaPS5 was found as `zx0CF1/RetroPapaPS5-Beta` ("Native retro frontend and emulator
launcher for jailbroken PS5", pushed 2026-08-29). Not read in detail.

### B's risks

- The hub must itself be a registered title (ShadowMountPlus) and must be closed by the console to
  launch the target: forwarders "close" on success, and failure shows a notification.
- The launch API is the console's own. Firmware-specific behaviour is unverified for 12.70 beyond the
  ProsperoStore statement.
- Writing a new hub means duplicating Jewel Case, Forwarder Manager and PS5-ECAM.

## C. ES-DE fork for PS5

### ES-DE facts

- **License: MIT.** `https://gitlab.com/es-de/emulationstation-de` (LICENSE: "Permission is hereby granted,
  free of charge ..."; copyright Northwestern Software AB 2024-2026, Leon Styhre 2020-2024, Alec Lofquist
  2014). The GitHub name `ES-DE/EmulationStation-DE` does not exist; the upstream is on GitLab.
  Last activity 2026-10-22 (GitLab API).
- **Linux dependencies** (`CMakeLists.txt`, `NOT EMSCRIPTEN AND NOT ANDROID AND NOT IOS` block):
  CURL, FFmpeg, FreeImage, Freetype, HarfBuzz, ICU, Intl, Libgit2, Pugixml, SDL2, ALSA, Bluez; OpenGL
  (Desktop, or GLES via `option(GLES)`). **No libVLC and no poppler in ES-DE 3.5** (the `es-pdf-converter`
  subdirectory exists upstream). Source: gitlab.com/es-de/emulationstation-de `CMakeLists.txt`.
- ES-DE's game-start contract is the `game-start` script, not a CLI flag (landscape note
  `docs/research/2026-10-23-landscape.md`, citing `main.cpp` and `Scripting.cpp`).

### What the PS5 build already does

`mihawk-99/PS5_RetroArch` (GPL-3.0-or-later, pushed 2026-10-19; latest tag `v4.0.0-beta-4`,
2026-10-19; the README still names v1.0.0-beta.1, so the README is stale):

- `tools/build-esde.sh` builds ES-DE **v3.5.0** pinned by commit from `gitlab.com/es-de/emulationstation-de`,
  applies `frontends/es-de/patches/0001-ps5-port.patch` (10 files, 419 lines), and builds with
  `-DGL=ON -DGLES=OFF -DAPPLICATION_UPDATER=OFF -DCOMPILE_LOCALIZATIONS=OFF -DPS5_SDL2=... -DPS5_GL_SDK=...`.
- Dependencies built as static archives by the same project: FreeType, HarfBuzz, ICU, pugixml, curl with
  mbedTLS, FFmpeg 8.1.1 (software decode, filters ES-DE uses), libwebp decoder; stb headers in place of
  FreeImage (`frontends/es-de/ps5/freeimage_stb.cpp`); lunasvg, plutovg, rlottie. Source: `tools/build-esde.sh`,
  `tools/build-ffmpeg-esde.sh`, `frontends/es-de/link.sh`.
- The patch removes libgit2 (theme downloader, "the PS5 port has no theme downloader (it needs libgit2)")
  and the pdf converter (`if(NOT PS5) add_subdirectory(es-pdf-converter)`). Source:
  `frontends/es-de/patches/0001-ps5-port.patch`.
- ES-DE is a **separate executable** (`/app0/es-de/es-de.bin`), because "EmulationStation draws with
  OpenGL, which on this console is ../PS5_OpenGL's stack, Mesa, as RADV is, so the two cannot share" the
  title's own runtime (`tools/build-frontend.sh`). The GL SDK used is `ps5-opengl-gl46`, not the public
  GL 3.3 SDK.
- **Launching**: ES-DE on PS5 starts games only through RetroArch's game mode (`frontends/es-de/ps5/game_ps5.cpp`).
  Its launch command is RetroArch's own command line, and `library_ps5.cpp` writes ES-DE systems from
  RetroArch's playlists at each start. So ES-DE's per-system launch config is generated from RetroArch,
  not from heimspiel's `es_systems.xml`. This does **not** cover non-RetroArch emulators or Ports
  (unverified beyond the code read).

### Dependencies on the PS5 homebrew stack

- **SDL2** exists on the PS5: `ps5-payload-dev/pacbrew-repo` provides "prebuilt PS5 ports (SDL2, OpenSSL, ...)"
  (https://github.com/ps5-payload-dev/pacbrew-repo, pushed 2026-10-16, GPL-3.0). Griches' Ghostship port
  says "libultraship is written for SDL2; `ps5/sdl/` holds SDL2 video, audio and pad drivers for the
  console" (https://github.com/griches/PS5_Ghostship, README).
- **OpenGL**: `Gudui/ps5-opengl` (GPL-3.0) is an experimental OpenGL 3.3 Core / GLSL 3.30 stack on
  Mesa/Gallium, with an SDL2 bridge, "not Khronos-certified". The PS5 ES-DE build uses an OpenGL 4.6 SDK
  named `ps5-opengl-gl46` from a `PS5_OpenGL` checkout; I could not confirm that exact SDK's public source.
  The repo `blackbearreloaded/ps5-opengl` returned 404 when I tried it. **Unverified which repo is the
  public home of `gl46`.**
- **Vulkan/RADV**: `mihawk-99/PS5_Vulkan` (GPL-3.0, pushed 2026-10-15): "A Vulkan 1.4+ PS5 native driver,
  powered by Mesa/RADV". Vulkan 1.4 is not yet conformant (README: "its `conformanceVersion` stays
  0.0.0.0 until the pinned 1.4 CTS passes in full"). ES-DE does not use Vulkan; RetroArch does.
- **FFmpeg, FreeType, HarfBuzz, ICU, curl, pugixml, libwebp**: built by PS5 RetroArch's own scripts
  (`tools/build-*.sh`). Not in `pacbrew-repo` per the README alone. Unverified whether pacbrew has them.
- **libgit2, poppler**: not built and not needed in the port.

### C's risks

- **Fork-like maintenance**: ADR 0001 rejected forking ES-DE on PC. On PS5 the patch is 419 lines
  against ES-DE 3.5.0, applied at build time. Keeping that patch alive across ES-DE upgrades is ongoing
  work. The MIT licence allows it.
- **Single-upstream dependency**: ES-DE on PS5 lives in `mihawk-99/PS5_RetroArch`, which is one
  developer's repo with a stale README and fast commits (last push 2026-10-19). Its build expects sibling
  checkouts (`../PS5_OpenGL`, `../PS5_Mesa`, `../PS5_PayloadSDK`).
- **Launch scope**: RetroArch only. Other emulators need a separate integration.
- **Size and build time**: many libraries; FFmpeg is built per-target.

## Recommendation

1. **Start with A**, scoped to: (a) writing forwarder folders over FTP using the PS5 Forwarder Format
   (fetch the template at runtime, not vendored); (b) ROM push over FTP to each emulator's documented
   folder; (c) ShadowMountPlus `/api/v1/scan` and `/api/v1/games` for status; (d) keep the API on
   loopback. This is the only approach that needs no C, no fork, and no new on-console code, and it
   matches ADR 0001's "write files only while the frontend is closed" pattern as far as the console
   allows.
2. **Spike C as a consumer, one week.** Install PS5 RetroArch (its ES-DE picker), see whether it reads
   ROMs from a heimspiel-written folder, and whether ES-DE's gamelists in `/app0/es-de/ES-DE` survive an
   FTP write (unverified). Only then decide whether ES-DE on the console is the couch frontend.
3. **Do not start B.** Forwarders and existing hubs (Jewel Case, Forwarder Manager, PS5-ECAM) already
   cover it. Revisit only if the forwarder contract stops working on new firmware.
4. **Pin scene projects**: ShadowMountPlus (beta), ProsperoStore, forwarder format (pushed 2026-10-09),
   PS5 RetroArch, the catalog. Record the version used in each heimspiel PS5 manifest.

## Open questions (unverified)

- Whether ftpsrv and elfldr can be sent automatically at each boot on FW 12.70 (tethered jailbreak).
- Whether ShadowMountPlus 1.7beta7 mounts `.ffpkg` from `/data/homebrew` on FW 12.70 without corruption.
  The README warns about internal-drive corruption.
- Whether `sceSystemServiceLaunchApp` with `--rom` works on FW 12.70 for each target (only forwarder
  registry claims are read).
- Whether ES-DE on PS5 accepts a heimspiel-written gamelist, given that it builds systems from RetroArch
  playlists at each start.
- Which public repo holds the `ps5-opengl-gl46` SDK that the PS5 ES-DE build uses.
- Whether pacbrew-repo ships FFmpeg, FreeType, HarfBuzz, ICU and curl. Only SDL2 and OpenSSL are named in
  its README.
- The jailbreak's exact tool chain (Relapse, kstuff-lite 1.11) was not verified beyond the README of
  kstuff-lite and the user's statement.
