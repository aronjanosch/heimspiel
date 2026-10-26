//! Local folders laid out like RomM's library: `<folder>/<platform>/<game file>`, with RomM's
//! platform slugs as folder names. A game of several files, such as a disc image with its tracks,
//! has a folder of its own: `<folder>/<platform>/<game>/<files>`.
//!
//! N64 ROMs are hashed, so Ports can recognise their Originals without an IGDB id. The hashes are
//! kept in `hash_cache` and a file is only read again when its size or modification time changed.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::library::{Entry, FileLocation, Identity, Launch, Metadata, RomHashes, SourceKind};
use crate::n64;
use crate::source::romm::is_game;
use crate::start;

pub fn entries(folders: &[PathBuf], hash_cache: &Path) -> anyhow::Result<Vec<Entry>> {
    let mut known = HashCache::load(hash_cache);
    let mut seen = HashCache::default();
    let mut entries = Vec::new();
    for folder in folders {
        let platforms =
            fs::read_dir(folder).with_context(|| format!("reading {}", folder.display()))?;
        for platform in platforms {
            let platform = platform?;
            if !platform.file_type()?.is_dir() {
                continue;
            }
            let slug = platform.file_name().to_string_lossy().into_owned();
            for file in fs::read_dir(platform.path())? {
                let file = file?;
                let name = file.file_name().to_string_lossy().into_owned();
                let path = file.path();
                if name.starts_with('.') {
                    continue;
                }
                let (location, file_name) = if path.is_dir() {
                    match game_folder(&path, &name)? {
                        Some(found) => found,
                        None => continue,
                    }
                } else if path.is_file() && is_game(&name) {
                    (FileLocation::Local { path: path.clone() }, name.clone())
                } else {
                    continue;
                };
                let rom = if slug == "n64" && path.is_file() {
                    let hashed = known.take_or_hash(&path);
                    let rom = hashed.as_ref().and_then(|h| h.rom.clone());
                    seen.0.extend(hashed.map(|h| (path.clone(), h)));
                    rom
                } else {
                    None
                };
                entries.push(Entry {
                    source: SourceKind::Local,
                    id: path.display().to_string(),
                    title: title(&name),
                    launch: Launch::File {
                        platform: slug.clone(),
                        file_name,
                        location,
                    },
                    cover: None,
                    metadata: Metadata::default(),
                    identity: Identity {
                        rom,
                        ..Default::default()
                    },
                });
            }
        }
    }
    // A cache that cannot be written only costs time on the next run.
    let _ = seen.save(hash_cache);
    entries.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(entries)
}

/// The game of several files in `path` and the name it is shown under, or `None` for a folder
/// that holds no game, such as one of artwork.
fn game_folder(path: &Path, name: &str) -> anyhow::Result<Option<(FileLocation, String)>> {
    let mut top = Vec::new();
    for file in fs::read_dir(path).with_context(|| format!("reading {}", path.display()))? {
        let file = file?;
        let name = file.file_name().to_string_lossy().into_owned();
        if !name.starts_with('.') && file.path().is_file() {
            top.push(name);
        }
    }
    let top: Vec<&str> = top.iter().map(String::as_str).collect();
    let Some(start) = start::of(name, &top) else {
        return Ok(None);
    };
    Ok(start.name(name).map(|file_name| {
        let path = path.to_path_buf();
        (FileLocation::LocalFolder { path, start }, file_name)
    }))
}

/// ROM hashes of earlier runs, by path.
#[derive(Default, Serialize, Deserialize)]
struct HashCache(BTreeMap<PathBuf, Hashed>);

#[derive(Serialize, Deserialize)]
struct Hashed {
    size: u64,
    modified: SystemTime,
    /// `None` for a file that is no N64 ROM.
    rom: Option<RomHashes>,
}

impl HashCache {
    fn load(path: &Path) -> HashCache {
        fs::read(path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }

    fn save(&self, path: &Path) -> anyhow::Result<()> {
        fs::create_dir_all(path.parent().unwrap())?;
        let tmp = path.with_extension("part");
        fs::write(&tmp, serde_json::to_vec(self)?)?;
        fs::rename(&tmp, path)?;
        Ok(())
    }

    /// The hashes of `path`, from the cache while the file is unchanged; `None` when it cannot be
    /// read.
    fn take_or_hash(&mut self, path: &Path) -> Option<Hashed> {
        let meta = fs::metadata(path).ok()?;
        let (size, modified) = (meta.len(), meta.modified().ok()?);
        if let Some(cached) = self.0.remove(path)
            && cached.size == size
            && cached.modified == modified
        {
            return Some(cached);
        }
        let rom = (size <= n64::MAX_SIZE)
            .then(|| n64::read(path).ok())
            .flatten()
            .map(|rom| n64::hashes(&rom));
        Some(Hashed {
            size,
            modified,
            rom,
        })
    }
}

/// The file name without extension and No-Intro tags: `Zelda (USA) [!].sfc` → `Zelda`.
fn title(file_name: &str) -> String {
    let stem = file_name
        .rsplit_once('.')
        .map_or(file_name, |(stem, _)| stem);
    let end = [stem.find(" ("), stem.find(" [")]
        .into_iter()
        .flatten()
        .min()
        .unwrap_or(stem.len());
    stem[..end].to_owned()
}
