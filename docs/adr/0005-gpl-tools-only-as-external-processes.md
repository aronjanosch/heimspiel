# GPL tools only as external processes

heimspiel is MIT licensed. GPL programs it relies on (Heroic, `ps4-pkg-tool`, extractors, EmuDeck
configs) are only run as separate processes or fetched at runtime; their code is never linked, vendored
or copied into this repository.
