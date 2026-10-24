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

Early. `heimspiel sync` already fills ES-DE from Steam, Heroic, hand-picked `.desktop` files, a RomM
server and local folders, and offers the Ports whose originals you own: Zelda 64: Recompiled, Banjo:
Recompiled and Ship of Harkinian from RomM ROMs, Uncharted: The Nathan Drake Collection from a PS4
dump. The setup wizard is still to come. Linux first, Windows later.

## Usage

Build with `cargo build --release`, then describe your setup in `~/.config/heimspiel/config.toml`.
Every section is optional; Steam and Heroic are found on their own.

```toml
[manual]
# PC games installed by hand that should appear in ES-DE
desktop-files = ["~/.local/share/applications/My Game.desktop"]

[romm]
url = "http://romm.lan:8080"
platforms = ["n64", "snes", "gba"]   # RomM platform slugs
# token-file = "~/.config/heimspiel/romm-token"   # chmod 600
# cache-limit-gib = 50

[local]
# folders with one subfolder per RomM platform slug, e.g. ~/Games/ROMs/n64/
folders = ["~/Games/ROMs"]

[dumps]
# decrypted dumps of your own PS4 games, one folder each, e.g. ~/Games/PS4/CUSA02320/
folders = ["~/Games/PS4"]
```

Close ES-DE and run `heimspiel sync`. RomM games show up with metadata and covers and are downloaded
the first time you start them, through an ES-DE event script that `sync` installs. Ports appear in
ES-DE's `ports` system; the first start installs the port through Quiver and hands it your original.

## Building blocks

| Part | Project |
|---|---|
| Controller frontend | ES-DE |
| Emulators | Flatpak (Linux) / winget (Windows), configs from EmuDeck |
| Recomp install, update, launch (incl. Proton) | Quiver Launcher CLI |
| ROM library | RomM, through its REST API |
| Recomp catalog and original hashes | recomp.fyi, awesome-decomp-games |

## License

MIT
