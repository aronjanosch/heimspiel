//! ES-DE's `game-start` event: fetch a RomM game before the emulator opens it (ADR 0009).
//!
//! ES-DE runs the hook for every game it starts, blocks until it exits, and passes arguments
//! unreliably: the game path shell-escaped, or the expanded `Exec` line for `.desktop` games. So
//! only argument 1 is used, and anything that is not one of heimspiel's RomM games ends the hook
//! at once.

use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

use anyhow::{Context, bail};

use crate::cache::GameCache;
use crate::fetch::Fetch;
use crate::library::Layout;
use crate::output::esde::{FOLDER_MARKER, PLACEHOLDER, placeholder};

pub fn game_start(fetch: &Fetch, args: &[String]) -> anyhow::Result<()> {
    let Some(first) = args.first() else {
        return Ok(());
    };
    let Some(game) = Placed::at(&PathBuf::from(unescape(first)), fetch.cache) else {
        return Ok(());
    };
    if game.ready(fetch.cache) {
        return Ok(());
    }
    let start = fetch.game(game.rom_id)?;
    game.link(&start)?;
    if let Some(roms) = game.roms_dir() {
        restore_placeholders(roms, fetch.cache);
    }
    fetch.progress.finish();
    Ok(())
}

/// Puts the Placeholders back for games the cache dropped to make room: ES-DE skips links that
/// point nowhere when it starts. Games of several files keep their folder and need nothing.
fn restore_placeholders(roms: &Path, cache: &GameCache) {
    let systems = fs::read_dir(roms).into_iter().flatten().flatten();
    for item in systems.flat_map(|s| fs::read_dir(s.path()).into_iter().flatten().flatten()) {
        let path = item.path();
        if let Ok(target) = fs::read_link(&path)
            && !path.exists()
            && let Some(rom_id) = cache.rom_id_of(&target)
        {
            let tmp = path.with_extension("heimspiel-tmp");
            let _ = fs::write(&tmp, placeholder(rom_id)).and_then(|()| fs::rename(&tmp, &path));
        }
    }
}

/// One of heimspiel's RomM games in an ES-DE ROM folder, at the path ES-DE passes: a file, the
/// file in a folder named like it, or a folder.
struct Placed {
    rom_id: u64,
    path: PathBuf,
    layout: Layout,
}

impl Placed {
    fn at(path: &Path, cache: &GameCache) -> Option<Placed> {
        let parent = path.parent()?;
        let (rom_id, layout) = if path.is_dir() {
            (marked(path)?, Layout::Folder)
        } else if parent.file_name() == path.file_name()
            && let Some(rom_id) = marked(parent)
        {
            (rom_id, Layout::StartInFolder)
        } else {
            (file_rom_id(path, cache)?, Layout::File)
        };
        Some(Placed {
            rom_id,
            path: path.to_path_buf(),
            layout,
        })
    }

    /// ES-DE's ROM folder, holding one folder per system.
    fn roms_dir(&self) -> Option<&Path> {
        let system = match self.layout {
            Layout::File | Layout::Folder => self.path.parent()?,
            Layout::StartInFolder => self.path.parent()?.parent()?,
        };
        system.parent()
    }

    /// Whether the game is linked to its files in the cache; marks it as played if so.
    fn ready(&self, cache: &GameCache) -> bool {
        let target = match self.layout {
            Layout::File | Layout::StartInFolder => fs::read_link(&self.path).ok(),
            // A folder of links is complete or entirely dangling: the cache drops whole games.
            Layout::Folder => fs::read_dir(&self.path)
                .into_iter()
                .flatten()
                .filter_map(|item| fs::read_link(item.ok()?.path()).ok())
                .find(|target| target.is_file()),
        };
        target.is_some_and(|t| cache.rom_id_of(&t) == Some(self.rom_id) && cache.mark_played(&t))
    }

    /// Links the game to `start`, what [`Fetch::game`] returned.
    fn link(&self, start: &Path) -> anyhow::Result<()> {
        match self.layout {
            Layout::File => link(&self.path, start),
            Layout::StartInFolder => {
                link_children(start.parent().unwrap(), self.path.parent().unwrap())?;
                link(&self.path, start)
            }
            Layout::Folder => link_children(start, &self.path),
        }
    }
}

/// The RomM id in the marker of a folder heimspiel wrote.
fn marked(folder: &Path) -> Option<u64> {
    let text = fs::read_to_string(folder.join(FOLDER_MARKER)).ok()?;
    rom_id_in(&text)
}

/// The RomM id behind a game file: a Placeholder, or a link into the cache (even a dangling one).
fn file_rom_id(path: &Path, cache: &GameCache) -> Option<u64> {
    let meta = fs::symlink_metadata(path).ok()?;
    if meta.file_type().is_symlink() {
        return cache.rom_id_of(&fs::read_link(path).ok()?);
    }
    if !meta.is_file() || meta.len() > 1024 {
        return None;
    }
    rom_id_in(&fs::read_to_string(path).ok()?)
}

fn rom_id_in(placeholder: &str) -> Option<u64> {
    placeholder
        .strip_prefix(PLACEHOLDER)?
        .strip_prefix("romm:")?
        .trim()
        .parse()
        .ok()
}

/// Links everything in `from` into `into` under the same names, skipping unfinished downloads.
fn link_children(from: &Path, into: &Path) -> anyhow::Result<()> {
    for item in fs::read_dir(from).with_context(|| format!("reading {}", from.display()))? {
        let item = item?;
        if item.path().extension().is_none_or(|e| e != "part") {
            link(&into.join(item.file_name()), &item.path())?;
        }
    }
    Ok(())
}

/// Replaces `path` with a link to `target`, atomically.
fn link(path: &Path, target: &Path) -> anyhow::Result<()> {
    let tmp = path.with_extension("heimspiel-link");
    let _ = fs::remove_file(&tmp);
    symlink(target, &tmp).with_context(|| format!("linking {}", path.display()))?;
    fs::rename(&tmp, path).with_context(|| format!("replacing {}", path.display()))?;
    if !path.exists() {
        bail!("{} points nowhere", path.display());
    }
    Ok(())
}

/// Undoes the shell escaping ES-DE applies to paths (`\ `, `\(`, …).
fn unescape(arg: &str) -> String {
    let mut out = String::with_capacity(arg.len());
    let mut chars = arg.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => out.extend(chars.next()),
            c => out.push(c),
        }
    }
    out
}
