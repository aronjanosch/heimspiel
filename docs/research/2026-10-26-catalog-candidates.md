# Catalog candidates for the next Manifests (2026-10-26)

`heimspiel ports` on the test setup lists 45 games the user owns that have a Port with a release in
awesome-decomp-games (`data/games.json`), beyond the nine Manifests. For each, the newest GitHub release's
assets were listed (`gh api repos/<repo>/releases`) and, for N64Recomp Ports, `src/main/main.cpp` was read for
librecomp's `GameEntry`, and the user's ROMs were fetched from RomM, converted to big-endian, padded and
hashed (XXH3-64 with `xxh3sum -H3`, SHA-1).

## Ready for the `librecomp` kind

A `GameEntry` with `rom_hash` (XXH3-64 of the big-endian ROM) and `game_id`, and a Linux x64 release asset.
**All eight match the user's USA ROMs** (Star Fox 64: Rev A) and have Manifests now; the config folder is
`~/.config/<program_id>` as for the earlier librecomp Ports:

| Game | Repository | Release | Linux asset | `game_id` | `rom_hash` |
|---|---|---|---|---|---|
| Star Fox 64 | sonicdcer/Starfox64Recomp | 1.0.3 | `Starfox64Recompiled-v1.0.3-Linux-X64-Release.zip` | `sf64.n64.us.1.1` | `0x163fd3fc3813f54e` |
| Mystical Ninja Starring Goemon | klorfmorf/Goemon64Recomp | v0.2.0-dev | `Goemon64Recompiled-Linux-X64-Release.zip` | `mnsg.us` | `0xDB1BC7EE0E6BEBA1` |
| Mega Man 64 | MegaMan64Recomp/MegaMan64Recompiled | v0.9.1 | `MegaMan64Recompiled-v0.9.1-Linux-X64.zip` | `megaman.n64.us.1.0` | `0x0c74c7489d01643b` |
| Duke Nukem: Zero Hour | sonicdcer/DNZHRecomp | 0.0.3 | `DNZHRecompiled-0.0.3-Linux-X64-Release.zip` | `dnzh.us` | `0xafc33da7101fec88` |
| Snowboard Kids | cdlewis/snowboardkids-recomp | v1.0.6 | `SnowboardKidsRecompiled-Linux-X64.tar.gz` | `snowboardkids.n64.us` | `0xf384619787b78d4b` |
| Snowboard Kids 2 | cdlewis/snowboardkids2-recomp | v2.0.2 | `SnowboardKids2Recompiled-Linux-X64.tar.gz` | `snowboardkids2.n64.us` | `0x81f434ff431162eb` |
| Space Station Silicon Valley | Cellenseres/SSSV_Recomp | v0.2.0 | `SSSVRecompiled_v0.2.0_Linux.tar.gz` | `sssv.n64.us.1.0` | `0x912A068AADB0D0C5` |
| Castlevania: Legacy of Darkness | fliperama86/cvlod_recomp | v0.2.29 | `LodRecomp-v0.2.29-linux-x64.tar.gz` | `castlevania2.n64.us` | `0x0BF827D4AC27A631` (`include/lod/target_rom.hpp`) |

Asset filters avoid the version in the name. Castlevania uses the glob `*linux-x64.tar.gz`: a plain filter
matches anywhere in the name and would also pick the `.sha256` file
(`lib/QuiverLauncher.Core/Services/RepositorySourceHelper.cs:89-102`). program_id per Port: `Starfox64Recompiled`,
`Goemon64Recompiled`, `MegaMan64Recompiled`, `DNZHRecompiled`, `SnowboardKidsRecompiled`,
`SnowboardKids2Recompiled`, `SSSVRecompiled`, `LodRecomp`. None was started yet.

## Linux release, but not plain librecomp

- Built on a patched N64ModernRuntime with their own `main` (no `src/main/main.cpp`): DKR-R and Rocket-R
  (ThatGuyMcd, AppImage), Glover-R, AeroGauge and Automobili Lamborghini (alondero), Jet Force Gemini
  (VibinToast), Rush 2049, Ogre Battle 64, Conker's Bad Fur Day (CBFD-Recompiled), Blast Corps (Power-Dozer),
  ISS 2000, Tetrisphere (Blocksphere), Body Harvest, Pilotwings 64, Hybrid Heaven, StarCraft 64 (launcher),
  Road Rash 64 ("Linux-Experimental"), Rush 2 (AppImage). Each needs its ROM location and check read from its
  source.
- libultraship (`o2r` kind, like SoH/2S2H): Ghostship (Super Mario 64, `Nautilus-Alfa-Linux.zip`),
  Lighthouse (Banjo-Kazooie, `lighthouse.appimage`), BattleShip (Super Smash Bros., AppImage).
- Torch extraction (no kind yet): Starship (Star Fox 64), SpaghettiKart (Mario Kart 64).
- Others: Perfect Dark (`pd-x86_64-linux.tar.gz`, only a `ci-dev-build` tag), sm64coopdx (`sm64coopdx_Linux.zip`,
  builds its own ROM-free assets from the ROM), Golden Balloon (Diddy Kong Racing decomp, AppImage).

## No Linux build

Beetle Adventure Racing, Dr. Mario 64, Jet Force Gemini (TK22-26), Wayne Gretzky's 3D Hockey: Windows only.
GoldenEye64Recomp: macOS only (see 2026-10-25-more-ports.md). StarHaven (Paper Mario): 3DS only.
SotE-Recomp ships one zip whose platform is not named.
