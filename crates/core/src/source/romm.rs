//! RomM: games on a RomM server, shown as Placeholders until they are played (ADR 0009).

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Context;

use crate::library::{Entry, FileLocation, Identity, Launch, Layout, Metadata, SourceKind};
use crate::romm::{Rom, Romm, Start};

/// Files RomM keeps next to games that are not games: saves, screenshots, notes.
const NOT_GAMES: &[&str] = &[
    "eep", "mpk", "sra", "srm", "fla", "sav", "state", "rtc", "png", "jpg", "txt", "nfo",
];

/// Reads the games of the given platforms (RomM slugs such as `n64`). Covers are downloaded once
/// into `cover_cache` and reused on later syncs. Games made of several files are left out when
/// their files do not tell what to start.
pub fn entries(
    client: &Romm,
    platforms: &[String],
    cover_cache: &Path,
) -> anyhow::Result<Vec<Entry>> {
    let mut entries = Vec::new();
    for platform in client.platforms()? {
        if !platforms.contains(&platform.slug) {
            continue;
        }
        for rom in client.roms(platform.id)? {
            if !is_game(&rom.fs_name) {
                continue;
            }
            let Some((file_name, layout)) = placement(client, &rom)? else {
                continue;
            };
            let cover = cover(client, &rom, cover_cache)?;
            entries.push(entry(rom, &platform.slug, file_name, layout, cover));
        }
    }
    Ok(entries)
}

/// Whether a file in a game folder is a game rather than a save, screenshot or note.
pub(crate) fn is_game(file_name: &str) -> bool {
    let ext = file_name
        .rsplit_once('.')
        .map(|(_, e)| e.to_lowercase())
        .unwrap_or_default();
    !NOT_GAMES.contains(&ext.as_str())
}

/// The game's name in a frontend's ROM folder and the layout of its files there. A game made of
/// several files becomes a folder named after what is started, such as `Metal Slug 6.gdi`.
fn placement(client: &Romm, rom: &Rom) -> anyhow::Result<Option<(String, Layout)>> {
    if !rom.has_multiple_files {
        return Ok(Some((rom.fs_name.clone(), Layout::File)));
    }
    let extension = match client.rom(rom.id)?.start() {
        None => return Ok(None),
        Some(Start::Folder) => return Ok(Some((rom.fs_name.clone(), Layout::Folder))),
        Some(Start::File(file)) => match file.rsplit_once('.') {
            Some((_, extension)) => extension.to_owned(),
            None => return Ok(None),
        },
        Some(Start::Playlist(_)) => "m3u".to_owned(),
    };
    Ok(Some((
        format!("{}.{extension}", rom.fs_name),
        Layout::StartInFolder,
    )))
}

fn entry(
    rom: Rom,
    platform: &str,
    file_name: String,
    layout: Layout,
    cover: Option<PathBuf>,
) -> Entry {
    let meta = rom.metadatum.unwrap_or_default();
    let identity = Identity {
        igdb_id: rom.igdb_id,
        regions: rom.regions,
        revision: rom.revision.filter(|r| !r.is_empty()),
        rom: None,
    };
    Entry {
        source: SourceKind::RomM,
        id: rom.id.to_string(),
        title: rom.name,
        launch: Launch::File {
            platform: platform.to_owned(),
            file_name,
            location: FileLocation::RomM {
                rom_id: rom.id,
                layout,
            },
        },
        cover,
        metadata: Metadata {
            description: rom.summary.map(|s| s.replace('\r', "")),
            release_date: meta.first_release_date.map(date),
            genre: (!meta.genres.is_empty()).then(|| meta.genres.join(", ")),
            players: meta.player_count,
            rating: meta.average_rating.map(|r| (r / 100.0) as f32),
            ..Metadata::default()
        },
        identity,
    }
}

fn cover(client: &Romm, rom: &Rom, cache: &Path) -> anyhow::Result<Option<PathBuf>> {
    // RomM sends an empty path for games without a cover.
    let Some(path) = rom.path_cover_large.as_deref().filter(|p| !p.is_empty()) else {
        return Ok(None);
    };
    let file_part = path.split('?').next().unwrap_or(path);
    let ext = file_part
        .rsplit_once('.')
        .map(|(_, e)| e)
        .filter(|e| !e.contains('/'))
        .unwrap_or("png");
    let target = cache.join(format!("{}.{ext}", rom.id));
    if !target.exists() {
        fs::create_dir_all(cache).with_context(|| format!("creating {}", cache.display()))?;
        // A broken cover is no reason to drop the game; ES-DE's scraper can still find one.
        if client.download_media(path, &target).is_err() {
            return Ok(None);
        }
    }
    Ok(Some(target))
}

/// Milliseconds since the Unix epoch as `YYYY-MM-DD` (UTC), after Howard Hinnant's `civil_from_days`.
fn date(millis: i64) -> String {
    let z = millis.div_euclid(86_400_000) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}
