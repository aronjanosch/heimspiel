# Our own RomM client instead of the `romm-api` crate

heimspiel talks to RomM through a small synchronous client of its own (ureq) instead of the `romm-api`
crate that ADR 0009 named. `romm-api` 1.4 is async (reqwest/tokio) and always pulls in about 300 crates,
including `self_update`, `keyring` and `indicatif` from its CLI, while heimspiel needs four calls:
platforms, ROM pages, file content and cover images. Owning the client also puts the behaviour test 3
asked for under our control: one keep-alive connection, a request limit for media, retries with backoff
and timeouts. `romm-api` stays a reference for the API's shapes.
