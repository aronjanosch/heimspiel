# More Ports for N64 games in RomM (2026-10-25)

Candidates from `heimspiel ports` for the user's RomM library, checked against the Ports' sources
(shallow clones of the default branch) and the user's ROMs. ROMs were fetched from RomM, brought to
big-endian order and hashed with heimspiel's own logic (XXH3-64 over the padded ROM, SHA-1).

## Added

| Port | Kind | Facts from the source | User's ROM |
|---|---|---|---|
| Rainchus/Donkey-Kong-64-Recompiled v1.0.3 | librecomp | `src/main/main.cpp`: program_id `DK64Recompiled`, game_id `DK64`, rom_hash `0x4d876060f09b3fc5` | Donkey Kong 64 (USA), XXH3 matches |
| sonicdcer/MarioKart64Recomp v0.9.2 | librecomp | `src/main/main.cpp:355-358`, `src/game/config.cpp:25`: `MarioKart64Recompiled`, `mk64.us`, `0x54572403569b87c1` | Mario Kart 64 (USA), matches |
| RevoSucks/BM64Recomp v1.0.0 | librecomp | `src/main/main.cpp:355-358`: `BM64Recompiled`, `bm64_us`, `0xc4c0b74bbb696426` | Bomberman 64 (USA), matches |
| HarvestMoon64Recomp v1.2.2 | librecomp | `src/main/main.cpp:373-377,674`: `HarvestMoon64Recompiled`, `harvest_moon_64`, `0x68b2c3755c527305` | Harvest Moon 64 (USA), matches |
| 2ship2harkinian v5.0.1 | o2r | `mm/2s2h/BenPort.h:36` appShortName `2ship`; `BenPort.cpp:447-480` extracts ROMs given as arguments into `mm.o2r`; `docs/supportedHashes.json` NTSC-U 1.0 `d6133ace…` | Majora's Mask (USA), SHA-1 matches |

- All librecomp Ports keep their config in `~/.config/<program_id>` (`$XDG_CONFIG_HOME` honoured), as
  `get_app_folder_path` in `src/game/config.cpp` (MK64, BM64) or RecompFrontend (DK64, HM64) builds it;
  the ROM goes there as `<game_id>.z64`, as for Zelda64Recomp.
- 2S2H logs no "Done!" after extracting (SoH does), and it goes on into the game instead of asking.
  The archive appears complete at once: OTRExporter writes it with libzip, which creates it on
  `zip_close` through a temporary file and a rename (`OTRExporter/ExporterArchiveO2R.cpp`, commit
  32e088e). heimspiel's check "archive exists and its size is stable" is therefore safe.
- **2S2H's data folder is `$SHIP_HOME`, else its working folder.** libultraship on Linux returns
  `$SHIP_HOME`, else (only in non-portable builds) the SDL pref path, else `.`
  (`libultraship/src/ship/Context.cpp:539-583`, commit 7f9b86a). 2S2H's AppImage starts its binary
  directly (`AppRun` is a link to `usr/bin/2s2h.elf`) and sets nothing, so the first check wrote
  `mm.o2r`, `2ship2harkinian.json`, `logs/` and `mods/` into the folder heimspiel was started in; under
  Quiver that would be the app folder. Quiver passes `SHIP_HOME` on (it strips only `LD_*`, Qt and
  AppImage variables, `Services/HostProcessEnvironment.cs:15-43`). heimspiel sets `SHIP_HOME` to the
  archive's folder for the extraction and every start of an o2r Port.

## Smoke tests (`heimspiel check`, 2026-10-25)

Each Port installed through Quiver, provisioned, then run for 30 s inside
`gamescope --backend headless -W 1280 -H 720` with `SDL_AUDIODRIVER=dummy`.

| Port | Result |
|---|---|
| Zelda 64: Recompiled | keeps running, Vulkan swapchain created |
| Mario Kart 64: Recompiled | keeps running, swapchain created |
| DK64: Rekongpiled | keeps running, swapchain created |
| Bomberman 64: Recompiled | keeps running, swapchain created |
| Harvest Moon 64: Recompiled | keeps running, swapchain created |
| 2 Ship 2 Harkinian | first run: extraction wrote into the working folder (see above), heimspiel gave up after 10 min while 2S2H already played its title demo |

- gamescope ignores SIGTERM; heimspiel now sends SIGKILL to the process group after 5 s.
- Not yet started from ES-DE.

## Not added

- **Starship** (Star Fox 64, HarbourMasters/Starship v2.0.0): asks for the ROM only through dialogs: a
  yes/no box, then a file picker (`src/port/Engine.cpp:82-98`, `src/port/extractor/GameExtractor.cpp:25-42`);
  no argument, no ROM picked up from a folder on desktop systems. Needs a new provisioning kind that
  builds `sf64.o2r` with Torch (`Companion` with the bundled `config.yml`).
- **SpaghettiKart** (Mario Kart 64): the same Torch-based extraction (`src/port/GameExtractor.cpp`); it
  also looks for `baserom.us.z64` in its app folder (line 91-97), not checked further.
- **GoldenEye64Recomp** (cblock85): only a macOS build, and its rom_hash is a ROM rebuilt from the
  decompilation ("tblfree"), not the retail ROM.
- **Conker (CBFD-Recompiled), Glover-R, DKR-R, JetForceGemini-Recomp (TK22-26)**: not librecomp; each would
  need its own research. JFG and Dr. Mario 64 Recompiled ship Windows builds only.
