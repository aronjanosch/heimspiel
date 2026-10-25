//! The Catalog: the Ports known from awesome-decomp-games (`data/games.json`, CC0), which lists
//! recompilations, decompilations and source ports with their repository and latest release.

use std::fs;
use std::path::Path;
use std::time::Duration;

use anyhow::Context;
use serde::Deserialize;

/// Where heimspiel fetches the Catalog from.
pub const URL: &str =
    "https://raw.githubusercontent.com/SubmitGame/awesome-decomp-games/main/data/games.json";

/// One Port in the Catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogPort {
    pub title: String,
    /// The original game's title in lower-case letters and digits, as [`game_key`] makes it.
    pub game_key: String,
    /// RomM slugs of the Platforms the original was released on; `pc` for PC games.
    pub platforms: Vec<String>,
    /// `static-recomp`, `decompilation`, `source-port` or `engine-remake`.
    pub kind: String,
    /// `owner/repo` on GitHub.
    pub repository: String,
    /// The latest release; `None` when there are only sources to build.
    pub release: Option<String>,
}

#[derive(Deserialize)]
struct Games {
    games: Vec<Game>,
}

#[derive(Deserialize)]
struct Game {
    title: String,
    game_key: String,
    platform: String,
    #[serde(rename = "type")]
    kind: String,
    /// Missing for projects that are not on GitHub.
    repo: Option<String>,
    release: Option<String>,
}

/// Reads `games.json`. Browser ports, disassemblies, tools and projects outside GitHub are left
/// out: there is nothing heimspiel could install.
pub fn parse(json: &str) -> anyhow::Result<Vec<CatalogPort>> {
    let games: Games = serde_json::from_str(json).context("reading the Catalog")?;
    Ok(games
        .games
        .into_iter()
        .filter(|g| !matches!(g.kind.as_str(), "browser-port" | "disassembly" | "tool"))
        .filter_map(|g| {
            Some(CatalogPort {
                platforms: g
                    .platform
                    .split(" / ")
                    .flat_map(slugs)
                    .map(|s| s.to_string())
                    .collect(),
                title: g.title,
                game_key: g.game_key,
                kind: g.kind,
                repository: g.repo?,
                release: g.release.filter(|r| !r.is_empty()),
            })
        })
        .collect())
}

/// The Catalog from `url`, kept in `cache` and fetched again once it is older than `max_age`. A
/// stale copy is used when the download fails.
pub fn load(url: &str, cache: &Path, max_age: Duration) -> anyhow::Result<Vec<CatalogPort>> {
    let fresh = fs::metadata(cache)
        .and_then(|m| m.modified())
        .is_ok_and(|t| t.elapsed().is_ok_and(|age| age < max_age));
    if !fresh {
        match download(url) {
            Ok(text) => {
                parse(&text)?;
                if let Some(dir) = cache.parent() {
                    fs::create_dir_all(dir)?;
                }
                let tmp = cache.with_extension("part");
                fs::write(&tmp, &text)?;
                fs::rename(&tmp, cache)?;
            }
            Err(e) if cache.is_file() => {
                eprintln!("Using the cached Catalog: {e:#}");
            }
            Err(e) => return Err(e),
        }
    }
    parse(&fs::read_to_string(cache).with_context(|| format!("reading {}", cache.display()))?)
}

fn download(url: &str) -> anyhow::Result<String> {
    Ok(ureq::get(url)
        .call()
        .context("downloading the Catalog")?
        .body_mut()
        .with_config()
        .limit(64 << 20)
        .read_to_string()?)
}

/// A title as the Catalog keys games: lower-case ASCII letters and digits only.
pub fn game_key(title: &str) -> String {
    title
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

/// RomM slugs for a Platform name of the Catalog.
fn slugs(platform: &str) -> &'static [&'static str] {
    match platform {
        "PC" | "DOS" => &["pc"],
        "N64" => &["n64"],
        "GameCube" => &["ngc"],
        "Wii" => &["wii"],
        "Wii U" => &["wiiu"],
        "Switch" => &["switch"],
        "NES" => &["nes"],
        "SNES" => &["snes"],
        "Game Boy" => &["gb"],
        "Game Boy Color" => &["gbc"],
        "GBA" => &["gba"],
        "DS" => &["nds"],
        "3DS" => &["3ds"],
        "PS1" => &["psx"],
        "PS2" => &["ps2"],
        "PS3" => &["ps3"],
        "PS4" | "PlayStation 4" => &["ps4"],
        "PS5" => &["ps5"],
        "PSP" => &["psp"],
        "Xbox" | "og xbox" => &["xbox"],
        "Xbox 360" => &["xbox360"],
        "Genesis" => &["genesis"],
        "Master System" => &["sms"],
        "Game Gear" => &["gamegear"],
        "Master System/Game Gear" => &["sms", "gamegear"],
        "32X" => &["sega32"],
        "Sega CD/32X" => &["segacd", "sega32"],
        "Saturn" => &["saturn"],
        "Dreamcast" => &["dc"],
        "Arcade" => &["arcade"],
        _ => &[],
    }
}
