# heimspiel

heimspiel fills ES-DE (and later other Outputs) from several Sources and provisions Originals for
Ports. It connects existing tools (ES-DE, Quiver, RomM, Heroic, emulators) instead of replacing them.

## Language

Capitalised terms (Entry, Source, Output, Port, Original, Provisioning …) are defined in
`GLOSSARY.md`. Name types, functions and docs after them; read the glossary before naming anything new
or using a word from its _Avoid_ lists.

This repository is public: code, comments, docs, commits, issues and PRs are written in English.

## Before changing behaviour

- **Decisions** live in `docs/adr/`. Read the ADRs that touch the area first; a change that contradicts
  one needs a new ADR that supersedes it.
- **Upstream facts** (file formats, CLI quirks of ES-DE, Quiver, RomM, Heroic, librecomp, SoH) live in
  `docs/research/`, with file:line references and test results. Check there before assuming how an
  upstream tool behaves.

## Workspace

- `crates/core`: all logic (Library, Sources, Outputs, Provisioning). Library crate, no UI.
- `crates/cli`: the `heimspiel` binary, called by ES-DE and scripts. Thin layer over `core`.
- `crates/app`: Tauri setup wizard. Thin layer over `core`.

New behaviour goes into `core` with tests; the CLI and app only parse input and call it.

GPL programs (Heroic, extractors, EmuDeck configs) are run as external processes or fetched at runtime,
never added as dependencies or copied in (ADR 0005).

Done means `cargo fmt --check`, `cargo clippy --workspace` and `cargo test --workspace` pass.
