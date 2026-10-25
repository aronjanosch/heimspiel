//! Proton, which runs the Windows builds of Ports. heimspiel starts them itself instead of through
//! `quiver --run`: Quiver runs any file in the release that starts with `#!/` natively instead of
//! the chosen `.exe` (docs/research/2026-10-24-uncharted.md), and passes no environment of ours.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::Context;

/// One Proton installation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proton {
    /// The `proton` script.
    pub exe: PathBuf,
    /// The Steam installation it belongs to, which Proton needs for its runtime.
    pub steam_root: PathBuf,
}

/// The Proton to use from `steam_root`: Proton Experimental, else the newest Valve Proton, else a
/// custom build in `compatibilitytools.d` (GE-Proton).
pub fn find(steam_root: &Path) -> Option<Proton> {
    let protons_in = |dir: PathBuf, prefix: &str| -> Vec<PathBuf> {
        let mut found: Vec<PathBuf> = fs::read_dir(dir)
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| {
                p.file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with(prefix))
                    && p.join("proton").is_file()
            })
            .collect();
        // Newest first, by the numbers in the name: "Proton 10.0" before "Proton 9.0 (Beta)".
        found.sort_by_key(|p| std::cmp::Reverse(version(p)));
        found
    };
    let common = steam_root.join("steamapps/common");
    let experimental = common.join("Proton - Experimental");
    let exe = [experimental]
        .into_iter()
        .filter(|p| p.join("proton").is_file())
        .chain(protons_in(common, "Proton "))
        .chain(protons_in(steam_root.join("compatibilitytools.d"), ""))
        .next()?
        .join("proton");
    Some(Proton {
        exe,
        steam_root: steam_root.to_path_buf(),
    })
}

impl Proton {
    /// The command that runs the Windows `program` in the prefix `prefix` (Proton's compat data
    /// folder); it blocks until the program and everything it started have exited. `id` names
    /// the game towards Proton.
    pub fn command(&self, program: &Path, prefix: &Path, id: &str) -> anyhow::Result<Command> {
        fs::create_dir_all(prefix).with_context(|| format!("creating {}", prefix.display()))?;
        let mut command = Command::new(&self.exe);
        command
            .arg("waitforexitandrun")
            .arg(program)
            .current_dir(program.parent().unwrap_or(Path::new("/")))
            .env("STEAM_COMPAT_DATA_PATH", prefix)
            .env("STEAM_COMPAT_CLIENT_INSTALL_PATH", &self.steam_root)
            .env("SteamGameId", id)
            // Xalia, Proton's controller helper for launcher windows, crashed on the Uncharted
            // launcher; the Ports start straight into their games.
            .env("PROTON_USE_XALIA", "0");
        Ok(command)
    }
}

/// The numbers in a folder name, `[10, 25]` for `GE-Proton10-25`.
fn version(dir: &Path) -> Vec<u64> {
    dir.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .split(|c: char| !c.is_ascii_digit())
        .filter_map(|n| n.parse().ok())
        .collect()
}
