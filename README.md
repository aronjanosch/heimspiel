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
Recompiled, Ship of Harkinian and more from N64 ROMs in RomM or a local folder, Uncharted: The Nathan
Drake Collection from a PS4 dump. The setup wizard is still to come. Linux first, Windows later.

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
# folders with one subfolder per RomM platform slug, e.g. ~/Games/ROMs/n64/;
# no RomM needed: N64 ROMs here are recognised as Port originals by their hash
folders = ["~/Games/ROMs"]

[dumps]
# decrypted dumps of your own PS4 games, one folder each, e.g. ~/Games/PS4/CUSA02320/
folders = ["~/Games/PS4"]
```

Close ES-DE and run `heimspiel sync`. RomM games show up with metadata and covers and are downloaded
the first time you start them, through an ES-DE event script that `sync` installs. Games made of
several files (a disc with its tracks, several discs, a ScummVM folder) appear as one game; a download
cut short continues where it stopped on the next start. Ports appear in ES-DE's `ports` system; the
first start installs the port through Quiver and hands it your original.

While a download or an installation runs, ES-DE only shows black. heimspiel shows a loading window
with the game's cover and progress instead, and the reason when a game cannot start. The window is
`heimspiel-app progress`, found next to the `heimspiel` binary or on `PATH`; without it heimspiel falls
back to desktop notifications.

`heimspiel ports` lists the ports heimspiel sets up and what each still needs, and the other ports for
games you own, from the [awesome-decomp-games](https://github.com/SubmitGame/awesome-decomp-games)
catalog (`--all` includes projects without a release).

## Desktop environments

heimspiel never moves or focuses windows itself. Where a desktop needs help to bring ES-DE back to
fullscreen after a game, the repository has a snippet you add yourself:

- **Hyprland**: a game takes ES-DE's fullscreen slot and Hyprland does not hand it back.
  `contrib/hyprland/es-de.lua` makes ES-DE fullscreen again once every window that opened on its
  workspace has closed, and floats windowed games centred instead of tiling them next to ES-DE. Add
  `dofile("/path/to/heimspiel/contrib/hyprland/es-de.lua")` at the end of `~/.config/hypr/hyprland.lua`.
  A window that stays open on ES-DE's workspace, such as a Steam client opened there, keeps ES-DE from
  returning to fullscreen until it closes.
- **KDE Plasma** (untested): `contrib/kde/heimspiel-es-de` is a KWin script that does the same and
  activates ES-DE. Install it with
  `kpackagetool6 --type KWin/Script --install contrib/kde/heimspiel-es-de`, then enable it under
  System Settings → Window Management → KWin Scripts.
- **GNOME** (untested): GNOME keeps fullscreen windows fullscreen, so nothing is expected to be needed.

## Building blocks

| Part | Project |
|---|---|
| Controller frontend | ES-DE |
| Emulators | Flatpak (Linux) / winget (Windows), configs from EmuDeck |
| Recomp install, update, launch (incl. Proton) | Quiver Launcher CLI |
| ROM library | RomM, through its REST API |
| Port catalog | awesome-decomp-games (CC0); recomp.fyi for original hashes later |

## License

MIT
