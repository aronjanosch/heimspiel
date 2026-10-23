# Heimspiel

*Heimspiel* is German for "home game": everything you play, in one place, on your own turf.

Heimspiel brings your whole game collection under one roof and presents it in
[ES-DE](https://es-de.org) for the couch and in a desktop library for mouse and keyboard:

- **PC games** from Steam, Heroic (Epic, GOG, Amazon) and manual installs
- **Retro games** from a self-hosted [RomM](https://github.com/rommapp/romm) library or local folders,
  downloaded on demand
- **Recompilations and PC ports** (Zelda64Recomp, Ship of Harkinian, Banjo, Uncharted …), matched
  against the originals you own and set up automatically

Heimspiel does not replace these projects. It installs, configures and connects them.

## Status

Planning. Nothing to install yet. Linux first, Windows later.

## Building blocks

| Part | Project |
|---|---|
| Controller frontend | ES-DE |
| Emulators | Flatpak (Linux) / winget (Windows), configs from EmuDeck |
| Recomp install, update, launch (incl. Proton) | Quiver Launcher CLI |
| ROM library | RomM via `romm-api` |
| Recomp catalog and original hashes | recomp.fyi, awesome-decomp-games |

## License

MIT
