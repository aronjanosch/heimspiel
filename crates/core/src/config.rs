//! `~/.config/heimspiel/config.toml`. Every setting is optional; heimspiel finds the usual
//! installations by itself.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct Config {
    pub es_de: EsDeConfig,
    pub steam: SteamConfig,
    pub heroic: HeroicConfig,
    pub manual: ManualConfig,
    pub romm: Option<RommConfig>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct EsDeConfig {
    /// ES-DE's application data folder; default `~/ES-DE`.
    pub home: Option<PathBuf>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SteamConfig {
    /// The folder holding `steamapps/`.
    pub root: Option<PathBuf>,
    /// Program that receives `steam://` URLs; default `steam`, or `flatpak run` for Flatpak Steam.
    pub command: Option<Vec<String>>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct HeroicConfig {
    /// Heroic's config folder.
    pub config: Option<PathBuf>,
    /// Command that starts Heroic, e.g. `["/home/me/Applications/Heroic.AppImage"]`.
    pub command: Option<Vec<String>>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct ManualConfig {
    /// `.desktop` files of PC games installed by hand that should appear in ES-DE.
    pub desktop_files: Vec<PathBuf>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct RommConfig {
    /// Base URL of the RomM server, e.g. `http://romm.lan:8080`.
    pub url: String,
    /// File holding the API token, readable only by the user; default
    /// `~/.config/heimspiel/romm-token`.
    pub token_file: Option<PathBuf>,
    /// RomM platform slugs to show in ES-DE, e.g. `["n64", "gba"]`.
    pub platforms: Vec<String>,
    /// Size limit of the cache for fetched games; default 50 GiB.
    pub cache_limit_gib: Option<u64>,
}

impl RommConfig {
    pub fn token_file(&self, user_home: &Path, config_dir: &Path) -> PathBuf {
        self.token_file
            .as_deref()
            .map(|f| expand(f, user_home))
            .unwrap_or_else(|| config_dir.join("romm-token"))
    }

    pub fn cache_limit_bytes(&self) -> u64 {
        self.cache_limit_gib.unwrap_or(50) << 30
    }
}

impl Config {
    /// Reads `path`; a missing file means the defaults.
    pub fn load(path: &Path) -> anyhow::Result<Config> {
        match fs::read_to_string(path) {
            Ok(text) => {
                toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
            Err(e) => Err(e).with_context(|| format!("reading {}", path.display())),
        }
    }
}

/// Resolves a leading `~/` against `user_home`.
pub fn expand(path: &Path, user_home: &Path) -> PathBuf {
    match path.strip_prefix("~") {
        Ok(rest) => user_home.join(rest),
        Err(_) => path.to_path_buf(),
    }
}
