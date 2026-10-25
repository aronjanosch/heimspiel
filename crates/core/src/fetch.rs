//! Fetching RomM games into the cache, shared by the `game-start` hook and Port launches.

use std::fs;
use std::path::PathBuf;

use anyhow::Context;

use crate::cache::GameCache;
use crate::progress::Progress;
use crate::romm::{RomDetail, Romm, Start};

pub struct Fetch<'a> {
    pub client: &'a Romm,
    pub cache: &'a GameCache,
    /// Where `heimspiel sync` keeps the covers of RomM games, as `<rom id>.<ext>`.
    pub covers: PathBuf,
    pub progress: &'a dyn Progress,
    /// Downloads from this size on are shown through `progress`.
    pub show_from_bytes: u64,
}

impl Fetch<'_> {
    /// What to start of a RomM game, from the cache or downloaded into it: its file, the
    /// playlist of its discs, or its folder. Downloads cut short go on where they stopped.
    pub fn game(&self, rom_id: u64) -> anyhow::Result<PathBuf> {
        let rom = self.client.rom(rom_id)?;
        self.fetch(&rom)
            .with_context(|| format!("could not download {}", rom.name))
    }

    /// The cover of a RomM game, if `heimspiel sync` stored one.
    pub fn cover(&self, rom_id: u64) -> Option<PathBuf> {
        fs::read_dir(&self.covers)
            .ok()?
            .filter_map(|item| item.ok().map(|i| i.path()))
            .find(|path| {
                path.file_stem()
                    .is_some_and(|s| s == rom_id.to_string().as_str())
            })
    }

    fn fetch(&self, rom: &RomDetail) -> anyhow::Result<PathBuf> {
        let start = rom
            .start()
            .context("heimspiel cannot tell which of its files to start")?;
        let mut missing = Vec::new();
        for file in &rom.files {
            let relative = rom.relative_path(file)?;
            let path = self.cache.path(rom.id, &relative);
            if fs::metadata(&path).map_or(true, |m| m.len() != file.file_size_bytes) {
                missing.push((file, relative));
            }
        }
        let total: u64 = missing.iter().map(|(f, _)| f.file_size_bytes).sum();
        let shown = !missing.is_empty() && total >= self.show_from_bytes;
        if shown {
            self.progress
                .start(&rom.name, self.cover(rom.id).as_deref(), "Downloading");
        }
        let mut done = 0;
        for (file, relative) in missing {
            self.cache
                .store(rom.id, &relative, file.file_size_bytes, |part| {
                    self.client.download_file(file, part, &|bytes| {
                        if shown {
                            self.progress.advance(done + bytes, total);
                        }
                    })
                })?;
            done += file.file_size_bytes;
        }
        let first = rom.files.first().context("RomM lists no files")?;
        self.cache
            .mark_played(&self.cache.path(rom.id, &rom.relative_path(first)?));
        Ok(match start {
            Start::File(name) => self.cache.path(rom.id, &name),
            Start::Playlist(discs) => {
                let playlist = self.cache.path(rom.id, &format!("{}.m3u", rom.fs_name));
                let text: String = discs.iter().map(|d| format!("{d}\n")).collect();
                fs::write(&playlist, text)
                    .with_context(|| format!("writing {}", playlist.display()))?;
                playlist
            }
            Start::Folder => self.cache.game_dir(rom.id),
        })
    }
}
