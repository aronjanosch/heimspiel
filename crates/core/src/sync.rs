//! `heimspiel sync`: read every Source and write the Library into ES-DE.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, bail};

use crate::config::{Config, expand};
use crate::library::{Library, SourceKind};
use crate::output::esde::EsDe;
use crate::port;
use crate::romm::Romm;
use crate::source::{heroic, local, manual, romm, steam};

/// heimspiel's own places on this machine.
pub struct Env {
    pub user_home: PathBuf,
    /// `~/.config/heimspiel`
    pub config_dir: PathBuf,
    /// `~/.cache/heimspiel`
    pub cache_dir: PathBuf,
    /// `~/.local/share/heimspiel`
    pub data_dir: PathBuf,
    /// Command line that runs this heimspiel binary, for the ES-DE hook.
    pub heimspiel: Vec<String>,
}

impl Env {
    /// Where the `game-start` hook keeps games fetched from RomM.
    pub fn game_cache(&self) -> PathBuf {
        self.cache_dir.join("romm/games")
    }

    /// heimspiel's own Quiver installation.
    pub fn quiver_dir(&self) -> PathBuf {
        self.data_dir.join("quiver")
    }
}

/// What a sync did, for the user.
#[derive(Debug, Default)]
pub struct Report {
    pub steam: usize,
    pub heroic: usize,
    pub manual: usize,
    pub romm: usize,
    pub local: usize,
    pub ports: usize,
    /// Sources that were skipped, and why.
    pub notes: Vec<String>,
}

pub fn sync(config: &Config, env: &Env) -> anyhow::Result<Report> {
    let user_home = env.user_home.as_path();
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
    if es_de_running(&es_de_home, user_home) {
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

    if let Some(romm_config) = &config.romm {
        let token = read_token(&romm_config.token_file(user_home, &env.config_dir))?;
        let client = Romm::new(&romm_config.url, &token);
        library.entries.extend(romm::entries(
            &client,
            &romm_config.platforms,
            &env.cache_dir.join("romm/covers"),
        )?);
    }

    // Ports find their Originals among the RomM games.
    let ports = port::entries(&port::builtin(), &library, &env.heimspiel);
    library.entries.extend(ports);

    let local_folders: Vec<PathBuf> = config
        .local
        .folders
        .iter()
        .map(|f| expand(f, user_home))
        .collect();
    library.entries.extend(local::entries(&local_folders)?);

    for entry in &library.entries {
        match entry.source {
            SourceKind::Steam => report.steam += 1,
            SourceKind::Heroic => report.heroic += 1,
            SourceKind::Manual => report.manual += 1,
            SourceKind::RomM => report.romm += 1,
            SourceKind::Local => report.local += 1,
            SourceKind::Port => report.ports += 1,
        }
    }
    let esde = local_folders.iter().fold(
        EsDe::locate(&es_de_home, user_home)?.game_cache(&env.game_cache()),
        |esde, f| esde.local_folder(f),
    );
    esde.write(&library).context("writing to ES-DE")?;
    if config.romm.is_some() && esde.install_hook(&env.heimspiel)? {
        report.notes.push(
            "ES-DE: turned on event scripts and installed the game-start hook for RomM".into(),
        );
    }
    Ok(report)
}

/// Reads the RomM token, refusing a file that others can read.
pub fn read_token(path: &Path) -> anyhow::Result<String> {
    use std::os::unix::fs::PermissionsExt;
    let mode = fs::metadata(path)
        .with_context(|| format!("reading the RomM token from {}", path.display()))?
        .permissions()
        .mode();
    if mode & 0o077 != 0 {
        bail!(
            "{} is readable by others; run `chmod 600` on it",
            path.display()
        );
    }
    let token = fs::read_to_string(path)?.trim().to_owned();
    if token.is_empty() {
        bail!("{} is empty", path.display());
    }
    Ok(token)
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

/// Whether an ES-DE with this application data folder runs. ES-DE's binary is called `es-de`, also
/// inside the AppImage; `--home <dir>` makes it use `<dir>/ES-DE` instead of `~/ES-DE`.
fn es_de_running(es_de_home: &Path, user_home: &Path) -> bool {
    let Ok(procs) = fs::read_dir("/proc") else {
        return false;
    };
    procs.filter_map(Result::ok).any(|p| {
        if !fs::read_to_string(p.path().join("comm")).is_ok_and(|c| c.trim() == "es-de") {
            return false;
        }
        let cmdline = fs::read(p.path().join("cmdline")).unwrap_or_default();
        let args: Vec<&[u8]> = cmdline.split(|b| *b == 0).collect();
        let home = args
            .windows(2)
            .find(|w| w[0] == b"--home")
            .map(|w| PathBuf::from(String::from_utf8_lossy(w[1]).into_owned()))
            .unwrap_or_else(|| user_home.to_path_buf());
        same_dir(&home.join("ES-DE"), es_de_home)
    })
}

fn same_dir(a: &Path, b: &Path) -> bool {
    a == b || matches!((a.canonicalize(), b.canonicalize()), (Ok(a), Ok(b)) if a == b)
}
