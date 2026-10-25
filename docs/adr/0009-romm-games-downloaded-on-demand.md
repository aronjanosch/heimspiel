# RomM games are downloaded on demand

Entries from RomM appear in ES-DE as Placeholders with full metadata and covers; ES-DE's blocking
`game-start` script swaps in the real file from a size-limited local cache right before launch. A large
RomM library thus costs no disk space until a game is played. The RomM client is our own (ADR 0010).

## Considered Options

- **Full sync** (as in ferry): rejected; copies the whole library to every device.

## Consequences

- The `game-start` script runs for every game in every system and receives unreliable arguments, so it
  handles only heimspiel's own Placeholders and exits immediately otherwise.
- ES-DE shows nothing while the script downloads; large images need a pre-download or a progress window.
- A game of several files (a disc with tracks, several discs, a ScummVM folder) becomes a folder that ES-DE
  treats as one game, holding a Placeholder named like the folder for the file that starts; the hook
  links the game's cached files into it (docs/research/2026-10-25-multi-file-and-loading.md). Games whose
  files do not tell what to start are left out.
- Every file is downloaded on its own and a cut-short download stays as `.part`; the next start continues
  it with a range request.
