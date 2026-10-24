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

/// First line of the Placeholders the ES-DE output writes for RomM games.
const PLACEHOLDER: &str = "heimspiel-placeholder romm:";

pub fn game_start(fetch: &Fetch, args: &[String]) -> anyhow::Result<()> {
    let Some(first) = args.first() else {
        return Ok(());
    };
    let path = PathBuf::from(unescape(first));
    let Some(rom_id) = romm_game(&path, fetch.cache) else {
        return Ok(());
    };
    if let Ok(target) = fs::read_link(&path)
        && let Some(name) = target.file_name().and_then(|n| n.to_str())
        && fetch.cache.get(rom_id, name).is_some()
    {
        return Ok(());
    }
    let file = fetch.game(rom_id)?;
    link(&path, &file)
}
/// The RomM id behind a game path: a Placeholder, or a link into the cache (even a dangling one).
fn romm_game(path: &Path, cache: &GameCache) -> Option<u64> {
    let meta = fs::symlink_metadata(path).ok()?;
    if meta.file_type().is_symlink() {
        return cache.rom_id_of(&fs::read_link(path).ok()?);
    }
    if !meta.is_file() || meta.len() > 1024 {
        return None;
    }
    let text = fs::read_to_string(path).ok()?;
    text.strip_prefix(PLACEHOLDER)?.trim().parse().ok()
}

/// Replaces the game path with a link to `file`, atomically.
fn link(path: &Path, file: &Path) -> anyhow::Result<()> {
    let tmp = path.with_extension("heimspiel-link");
    let _ = fs::remove_file(&tmp);
    symlink(file, &tmp).with_context(|| format!("linking {}", path.display()))?;
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
