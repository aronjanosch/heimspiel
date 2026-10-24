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
use crate::romm::Romm;

/// First line of the Placeholders the ES-DE output writes for RomM games.
const PLACEHOLDER: &str = "heimspiel-placeholder romm:";

pub struct GameStart<'a> {
    pub client: &'a Romm,
    pub cache: &'a GameCache,
    /// Shows a message to the user, e.g. as a desktop notification.
    pub notify: &'a dyn Fn(&str),
    /// Downloads from this size on are announced through `notify`.
    pub notify_from_bytes: u64,
}

pub fn game_start(ctx: &GameStart, args: &[String]) -> anyhow::Result<()> {
    let Some(first) = args.first() else {
        return Ok(());
    };
    let path = PathBuf::from(unescape(first));
    let Some(rom_id) = romm_game(&path, ctx.cache) else {
        return Ok(());
    };
    if let Ok(target) = fs::read_link(&path)
        && let Some(name) = target.file_name().and_then(|n| n.to_str())
        && ctx.cache.get(rom_id, name).is_some()
    {
        return Ok(());
    }

    let rom = ctx.client.rom(rom_id)?;
    let fail = |e: anyhow::Error| {
        (ctx.notify)(&format!("Could not download {}: {e:#}", rom.name));
        e
    };
    if rom.has_multiple_files {
        return Err(fail(anyhow::anyhow!(
            "games made of several files are not supported yet"
        )));
    }
    let large = rom.fs_size_bytes >= ctx.notify_from_bytes;
    if large {
        (ctx.notify)(&format!(
            "Downloading {} ({}) …",
            rom.name,
            size(rom.fs_size_bytes)
        ));
    }
    let file = match ctx.cache.get(rom.id, &rom.fs_name) {
        Some(file) => file,
        None => ctx
            .cache
            .store(rom.id, &rom.fs_name, rom.fs_size_bytes, |part| {
                ctx.client.download_rom(rom.id, &rom.fs_name, part)
            })
            .map_err(fail)?,
    };
    link(&path, &file).map_err(fail)?;
    if large {
        (ctx.notify)(&format!("{} is ready", rom.name));
    }
    Ok(())
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

fn size(bytes: u64) -> String {
    match bytes {
        b if b >= 1 << 30 => format!("{:.1} GB", b as f64 / (1u64 << 30) as f64),
        b => format!("{} MB", b.div_ceil(1 << 20)),
    }
}
