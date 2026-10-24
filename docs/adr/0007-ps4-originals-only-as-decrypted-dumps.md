# PS4 Originals only as decrypted Dumps

PS4 Ports accept only an already decrypted Dump made on the user's own console; heimspiel does not try to
turn a retail disc or PKG into one. The test with `ps4-pkg-tool` on a retail PKG crashed after writing
only the unencrypted metadata, because the game data needs keys only a console can derive.

## Consequences

- heimspiel checks a supplied Dump (`eboot.bin`, `sce_sys/param.sfo` with the expected title ID and
  version) and explains what is missing otherwise.
- heimspiel can still read title ID and version from a disc image or PKG without decrypting, to show the
  user that they own the game and which Dump the Port needs.
- A PKG the user made decrypted on their own console is not a retail PKG: `ps4-pkg-tool` extracts it into a
  complete Dump (docs/research/2026-10-24-uncharted.md). heimspiel may later offer that extraction; it still
  never decrypts.
