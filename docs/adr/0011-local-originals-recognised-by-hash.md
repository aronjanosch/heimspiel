# ROMs in local folders are recognised as Originals by their hash

Without RomM, a Port's Original is a ROM in a local folder, and nothing about it names the release: the
file name is whatever the user chose and there is no IGDB id. heimspiel therefore hashes the N64 ROMs in
local folders the way Ports check them (big-endian, padded to whole words) and compares the hash with the
one in the Manifest (`xxh3` for librecomp, `sha1` for o2r). RomM games keep being matched by IGDB id,
region and revision, because RomM's list has no hashes and downloading every candidate would cost more
than the match is worth. When both hold the Original, the local file is used: it needs no download.

## Considered Options

- **Matching by title from the file name** (No-Intro names, Catalog game keys): rejected; names vary and
  say nothing reliable about region and revision, which decide whether a Port accepts the ROM.
- **Hashing every local file**: rejected; only N64 ROMs feed Ports that check a ROM, and other platforms
  would read gigabytes of disc images for nothing.

## Consequences

- The first `sync` reads every local N64 ROM once (at most 64 MiB each). The hashes are kept in
  `~/.cache/heimspiel/local/rom-hashes.json`, and a file is only read again when its size or modification
  time changes.
- `heimspiel launch <port> --file <rom>` names such an Original and checks it before the Port is installed,
  as with a Dump.
- A Port for another platform that checks a ROM needs that platform's normal form before its local ROMs
  can be matched.
