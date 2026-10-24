# RomM games are downloaded on demand

Entries from RomM appear in ES-DE as Placeholders with full metadata and covers; ES-DE's blocking
`game-start` script swaps in the real file from a size-limited local cache right before launch. A large
RomM library thus costs no disk space until a game is played. heimspiel's own RomM client builds on the
`romm-api` crate.

## Considered Options

- **Full sync** (as in ferry): rejected; copies the whole library to every device.

## Consequences

- The `game-start` script runs for every game in every system and receives unreliable arguments, so it
  handles only heimspiel's own Placeholders and exits immediately otherwise.
- ES-DE shows nothing while the script downloads; large images need a pre-download or a progress window.
