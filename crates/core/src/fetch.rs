//! Fetching RomM games into the cache, shared by the `game-start` hook and Port launches.

use std::path::PathBuf;

use anyhow::anyhow;

use crate::cache::GameCache;
use crate::romm::Romm;

pub struct Fetch<'a> {
    pub client: &'a Romm,
    pub cache: &'a GameCache,
    /// Shows a message to the user, e.g. as a desktop notification.
    pub notify: &'a dyn Fn(&str),
    /// Downloads from this size on are announced through `notify`.
    pub notify_from_bytes: u64,
}

impl Fetch<'_> {
    /// The game file of a RomM game, from the cache or downloaded into it.
    pub fn game(&self, rom_id: u64) -> anyhow::Result<PathBuf> {
        let rom = self.client.rom(rom_id)?;
        if let Some(file) = self.cache.get(rom.id, &rom.fs_name) {
            return Ok(file);
        }
        let fail = |e: anyhow::Error| {
            (self.notify)(&format!("Could not download {}: {e:#}", rom.name));
            e
        };
        if rom.has_multiple_files {
            return Err(fail(anyhow!(
                "games made of several files are not supported yet"
            )));
        }
        let large = rom.fs_size_bytes >= self.notify_from_bytes;
        if large {
            (self.notify)(&format!(
                "Downloading {} ({}) …",
                rom.name,
                size(rom.fs_size_bytes)
            ));
        }
        let file = self
            .cache
            .store(rom.id, &rom.fs_name, rom.fs_size_bytes, |part| {
                self.client.download_rom(rom.id, &rom.fs_name, part)
            })
            .map_err(fail)?;
        if large {
            (self.notify)(&format!("{} is ready", rom.name));
        }
        Ok(file)
    }
}

fn size(bytes: u64) -> String {
    match bytes {
        b if b >= 1 << 30 => format!("{:.1} GB", b as f64 / (1u64 << 30) as f64),
        b => format!("{} MB", b.div_ceil(1 << 20)),
    }
}
