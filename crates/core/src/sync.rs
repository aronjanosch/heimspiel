//! `heimspiel sync`: read every Source and write the Library into ES-DE.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, bail};

use crate::config::{Config, expand};
use crate::library::{Library, SourceKind};
use crate::output::esde::EsDe;
use crate::source::{heroic, manual, steam};

/// What a sync did, for the user.
#[derive(Debug, Default)]
pub struct Report {
    pub steam: usize,
    pub heroic: usize,
    pub manual: usize,
    /// Sources that were skipped, and why.
    pub notes: Vec<String>,
}

pub fn sync(config: &Config, user_home: &Path) -> anyhow::Result<Report> {
    let es_de_home = config
        .es_de
        .home
        .as_deref()
        .map(|h| expand(h, user_home))
        .unwrap_or_else(|| user_home.join("ES-DE"));
    if !es_de_home.is_dir() {
        bail!(
            "ES-DE not found at {}; start ES-DE once or set [es-de] home in the config",
            es_de_home.display()
        );
    }
    // ES-DE reads its gamelists at startup and writes them back on exit.
    if es_de_running() {
        bail!("ES-DE is running; close it before syncing");
    }

    let mut report = Report::default();
    let mut library = Library::default();

    match steam_root(config, user_home) {
        Some(root) => {
            let command = config.steam.command.clone().unwrap_or_else(|| {
                if root.starts_with(user_home.join(".var/app")) {
                    vec![
                        "flatpak".into(),
                        "run".into(),
                        "com.valvesoftware.Steam".into(),
                    ]
                } else {
                    vec!["steam".into()]
                }
            });
            library
                .entries
                .extend(steam::entries(&root, &crate::desktop::exec(&command))?);
        }
        None => report.notes.push("Steam: no installation found".into()),
    }

    match heroic_config(config, user_home) {
        Some(dir) => match heroic_command(config, &dir, user_home) {
            Some(command) => library.entries.extend(heroic::entries(&dir, &command)?),
            None => report.notes.push(
                "Heroic: games found but no way to start Heroic; set [heroic] command in the config".into(),
            ),
        },
        None => report.notes.push("Heroic: no installation found".into()),
    }

    let files: Vec<PathBuf> = config
        .manual
        .desktop_files
        .iter()
        .map(|f| expand(f, user_home))
        .collect();
    library.entries.extend(manual::entries(&files)?);

    for entry in &library.entries {
        match entry.source {
            SourceKind::Steam => report.steam += 1,
            SourceKind::Heroic => report.heroic += 1,
            SourceKind::Manual => report.manual += 1,
        }
    }
    EsDe::locate(&es_de_home, user_home)?
        .write(&library)
        .context("writing to ES-DE")?;
    Ok(report)
}

fn steam_root(config: &Config, user_home: &Path) -> Option<PathBuf> {
    if let Some(root) = &config.steam.root {
        return Some(expand(root, user_home));
    }
    [
        ".local/share/Steam",
        ".steam/steam",
        ".var/app/com.valvesoftware.Steam/.local/share/Steam",
    ]
    .into_iter()
    .map(|p| user_home.join(p))
    .find(|p| p.join("steamapps").is_dir())
}

fn heroic_config(config: &Config, user_home: &Path) -> Option<PathBuf> {
    if let Some(dir) = &config.heroic.config {
        return Some(expand(dir, user_home));
    }
    let xdg = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| user_home.join(".config"));
    [
        xdg.join("heroic"),
        user_home.join(".var/app/com.heroicgameslauncher.hgl/config/heroic"),
    ]
    .into_iter()
    .find(|p| p.is_dir())
}

/// Heroic as Flatpak, on `PATH`, or as an AppImage in `~/Applications`, in that order.
fn heroic_command(config: &Config, heroic_config: &Path, user_home: &Path) -> Option<Vec<String>> {
    if let Some(command) = &config.heroic.command {
        return Some(command.clone());
    }
    if heroic_config.starts_with(user_home.join(".var/app")) {
        return Some(vec![
            "flatpak".into(),
            "run".into(),
            "com.heroicgameslauncher.hgl".into(),
        ]);
    }
    if env::split_paths(&env::var_os("PATH")?).any(|dir| dir.join("heroic").is_file()) {
        return Some(vec!["heroic".into()]);
    }
    let appimage = fs::read_dir(user_home.join("Applications"))
        .ok()?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .find(|p| {
            p.is_file()
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.to_lowercase().starts_with("heroic"))
        })?;
    Some(vec![appimage.to_string_lossy().into_owned()])
}

/// ES-DE's binary is called `es-de`, also inside the AppImage.
fn es_de_running() -> bool {
    let Ok(procs) = fs::read_dir("/proc") else {
        return false;
    };
    procs
        .filter_map(Result::ok)
        .any(|p| fs::read_to_string(p.path().join("comm")).is_ok_and(|c| c.trim() == "es-de"))
}
