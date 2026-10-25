//! The cache of game files fetched from RomM, limited in size. Each game lives in its own folder
//! `<dir>/<rom id>/`; the modification time of its files records when it was last played.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use anyhow::{Context, bail};

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

    /// The folder of a game. The ROM id is part of every path inside, so a link to a game that
    /// was dropped from the cache still tells which game to fetch again.
    pub fn game_dir(&self, rom_id: u64) -> PathBuf {
        self.dir.join(rom_id.to_string())
    }

    /// Where a file of a game is or would be stored; `relative` may name subfolders with `/`.
    pub fn path(&self, rom_id: u64, relative: &str) -> PathBuf {
        relative
            .split(['/', '\\'])
            .map(|part| part.trim_start_matches('.'))
            .filter(|part| !part.is_empty())
            .fold(self.game_dir(rom_id), |path, part| path.join(part))
    }

    /// The ROM id of a path inside this cache, as returned by [`GameCache::path`].
    pub fn rom_id_of(&self, path: &Path) -> Option<u64> {
        let dir = path.strip_prefix(&self.dir).ok()?.components().next()?;
        dir.as_os_str().to_str()?.parse().ok()
    }

    /// Marks the game holding the file at `path` as just played; false if the file is not there.
    pub fn mark_played(&self, path: &Path) -> bool {
        let Ok(file) = fs::File::options().write(true).open(path) else {
            return false;
        };
        let _ = file.set_modified(SystemTime::now());
        true
    }

    /// Stores a file of `size` bytes at [`GameCache::path`]. Makes room first, then lets `write`
    /// fill the file's `.part`, which may already hold the start of an earlier attempt. A failed
    /// attempt keeps the part for the next one; only a part of exactly `size` bytes becomes the
    /// file.
    pub fn store(
        &self,
        rom_id: u64,
        relative: &str,
        size: u64,
        write: impl FnOnce(&Path) -> anyhow::Result<()>,
    ) -> anyhow::Result<PathBuf> {
        let path = self.path(rom_id, relative);
        let mut part = path.clone().into_os_string();
        part.push(".part");
        let part = PathBuf::from(part);
        let mut have = fs::metadata(&part).map_or(0, |m| m.len());
        if have > size {
            fs::remove_file(&part).with_context(|| format!("removing {}", part.display()))?;
            have = 0;
        }
        self.make_room(rom_id, size - have)?;
        let dir = path.parent().unwrap();
        fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
        if have < size {
            write(&part)?;
        }
        let got = fs::metadata(&part).map_or(0, |m| m.len());
        if got != size {
            let _ = fs::remove_file(&part);
            bail!(
                "received {got} bytes of {}, expected {size}",
                path.display()
            );
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
            if game.dir == self.game_dir(keep) {
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
            let mut game = CachedGame {
                dir: dir.clone(),
                size: 0,
                played: SystemTime::UNIX_EPOCH,
            };
            add_files(&dir, &mut game)?;
            games.push(game);
        }
        Ok(games)
    }
}

/// Adds the size and latest modification of every file below `dir` to `game`.
fn add_files(dir: &Path, game: &mut CachedGame) -> anyhow::Result<()> {
    let Ok(items) = fs::read_dir(dir) else {
        return Ok(());
    };
    for item in items {
        let item = item?;
        let meta = item.metadata()?;
        if meta.is_dir() {
            add_files(&item.path(), game)?;
        } else {
            game.size += meta.len();
            game.played = game.played.max(meta.modified()?);
        }
    }
    Ok(())
}

struct CachedGame {
    dir: PathBuf,
    size: u64,
    played: SystemTime,
}
