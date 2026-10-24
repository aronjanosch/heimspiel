//! The cache of game files fetched from RomM, limited in size. Each game lives in
//! `<dir>/<rom id>/<file name>`; its modification time records when it was last played.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use anyhow::Context;

pub struct GameCache {
    dir: PathBuf,
    limit: u64,
}

impl GameCache {
    pub fn new(dir: &Path, limit_bytes: u64) -> GameCache {
        GameCache {
            dir: dir.to_path_buf(),
            limit: limit_bytes,
        }
    }

    /// Where a game is or would be stored. The ROM id is part of the path, so a link to a game
    /// that was dropped from the cache still tells which game to fetch again.
    pub fn path(&self, rom_id: u64, file_name: &str) -> PathBuf {
        let name = file_name.replace(['/', '\\'], "-");
        self.dir
            .join(rom_id.to_string())
            .join(name.trim_start_matches('.'))
    }

    /// The ROM id of a game path inside this cache, as returned by [`GameCache::path`].
    pub fn rom_id_of(&self, path: &Path) -> Option<u64> {
        let dir = path.strip_prefix(&self.dir).ok()?.components().next()?;
        dir.as_os_str().to_str()?.parse().ok()
    }

    /// The cached game, marked as just played.
    pub fn get(&self, rom_id: u64, file_name: &str) -> Option<PathBuf> {
        let path = self.path(rom_id, file_name);
        let file = fs::File::options().write(true).open(&path).ok()?;
        let _ = file.set_modified(SystemTime::now());
        Some(path)
    }

    /// Makes room for `size` bytes, then lets `write` fill the game's file. Nothing is kept if
    /// `write` fails.
    pub fn store(
        &self,
        rom_id: u64,
        file_name: &str,
        size: u64,
        write: impl FnOnce(&Path) -> anyhow::Result<()>,
    ) -> anyhow::Result<PathBuf> {
        self.make_room(rom_id, size)?;
        let path = self.path(rom_id, file_name);
        let dir = path.parent().unwrap();
        fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
        let part = path.with_extension("part");
        if let Err(e) = write(&part) {
            let _ = fs::remove_file(&part);
            return Err(e);
        }
        fs::rename(&part, &path).with_context(|| format!("replacing {}", path.display()))?;
        Ok(path)
    }

    /// Drops the least recently played games, except `keep`, until `size` more bytes fit.
    fn make_room(&self, keep: u64, size: u64) -> anyhow::Result<()> {
        let mut games = self.games()?;
        let mut total: u64 = games.iter().map(|g| g.size).sum();
        games.sort_by_key(|g| g.played);
        for game in games {
            if total + size <= self.limit {
                break;
            }
            if game
                .dir
                .file_name()
                .is_some_and(|n| n == keep.to_string().as_str())
            {
                continue;
            }
            fs::remove_dir_all(&game.dir)
                .with_context(|| format!("removing {}", game.dir.display()))?;
            total -= game.size;
        }
        Ok(())
    }

    fn games(&self) -> anyhow::Result<Vec<CachedGame>> {
        let Ok(dirs) = fs::read_dir(&self.dir) else {
            return Ok(Vec::new());
        };
        let mut games = Vec::new();
        for dir in dirs {
            let dir = dir?.path();
            let Ok(files) = fs::read_dir(&dir) else {
                continue;
            };
            let mut game = CachedGame {
                dir: dir.clone(),
                size: 0,
                played: SystemTime::UNIX_EPOCH,
            };
            for file in files {
                let meta = file?.metadata()?;
                game.size += meta.len();
                game.played = game.played.max(meta.modified()?);
            }
            games.push(game);
        }
        Ok(games)
    }
}

struct CachedGame {
    dir: PathBuf,
    size: u64,
    played: SystemTime,
}
