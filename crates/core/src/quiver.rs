//! Quiver Launcher, which installs and runs Ports (ADR 0003). heimspiel keeps it in a folder of its
//! own, because Quiver stores its library next to its AppImage.
//!
//! Quiver 3.5.1 quirks, see docs/research/2026-10-23-tests.md (test 1): `--download` installs but
//! never exits, so completion is detected from `installed-build.txt`; two `apps.json` entries for
//! the same repository get mixed up, so there is one entry per repository. For an app in Quiver's
//! own catalog the CLI installs only a version quiverlauncher.com lists, and finds no download
//! unpinned or pinned to any other (docs/research/2026-10-24-uncharted.md); heimspiel pins the
//! catalog's version for those and the newest GitHub release for the rest.

use std::fs;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, bail};
use serde_json::{Value, json};

use crate::port::{QuiverApp, Runner};
use crate::process::stop;

pub const APPIMAGE: &str = "QuiverLauncher-linux-x64.AppImage";

/// The Quiver release heimspiel was tested with, and the SHA-256 GitHub reports for its AppImage.
const RELEASE_URL: &str = "https://github.com/tgeorgiadis/quiver-launcher/releases/download/v3.5.1/QuiverLauncher-linux-x64.AppImage";
const RELEASE_SHA256: &str = "a8cd5398b6dae924bcec09ece1781c03cbfd73298675476f26fc1c072d3c629b";

pub struct Quiver {
    dir: PathBuf,
}

/// What `ensure_entry` does with the app's `preferredVersion`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pin<'a> {
    /// Keep what is pinned, if anything.
    Keep,
    Version(&'a str),
}

/// How `install` ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Install {
    Done,
    /// Quiver exited without installing, mostly because it found no download.
    Failed,
}

impl Quiver {
    pub fn new(dir: &Path) -> Quiver {
        Quiver {
            dir: dir.to_path_buf(),
        }
    }

    pub fn appimage(&self) -> PathBuf {
        self.dir.join(APPIMAGE)
    }

    /// Downloads the tested Quiver release into the folder unless an AppImage is already there.
    /// Returns whether it downloaded.
    pub fn ensure_installed(&self) -> anyhow::Result<bool> {
        use sha2::{Digest, Sha256};
        use std::io::{Read, Write};
        use std::os::unix::fs::PermissionsExt;
        if self.appimage().is_file() {
            return Ok(false);
        }
        fs::create_dir_all(&self.dir)
            .with_context(|| format!("creating {}", self.dir.display()))?;
        let mut response = ureq::get(RELEASE_URL)
            .call()
            .context("downloading Quiver")?;
        let mut reader = response.body_mut().as_reader();
        let tmp = self.appimage().with_extension("part");
        let mut file = fs::File::create(&tmp)?;
        let mut hasher = Sha256::new();
        let mut buf = vec![0u8; 1 << 16];
        loop {
            let n = reader.read(&mut buf).context("downloading Quiver")?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
            file.write_all(&buf[..n])?;
        }
        drop(file);
        let actual = format!("{:x}", hasher.finalize());
        if actual != RELEASE_SHA256 {
            let _ = fs::remove_file(&tmp);
            bail!("the downloaded Quiver has SHA-256 {actual}, expected {RELEASE_SHA256}");
        }
        fs::set_permissions(&tmp, fs::Permissions::from_mode(0o755))?;
        fs::rename(&tmp, self.appimage())?;
        Ok(true)
    }

    /// Adds or updates the app's entry in `apps.json`, keeping every other entry.
    pub fn ensure_entry(&self, app: &QuiverApp, pin: Pin) -> anyhow::Result<()> {
        let path = self.dir.join("apps.json");
        let mut doc: Value = match fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str(&text)
                .with_context(|| format!("parsing {}", path.display()))?,
            Err(_) => json!({ "apps": [] }),
        };
        let apps = doc["apps"]
            .as_array_mut()
            .context("apps.json has no apps list")?;
        let existing = apps
            .iter()
            .position(|a| a["repository"] == app.repository.as_str());
        let pinned = match pin {
            Pin::Version(version) => Some(Value::from(version)),
            Pin::Keep => existing.and_then(|i| apps[i].get("preferredVersion").cloned()),
        };
        let mut entry = json!({
            "name": app.name,
            "repository": app.repository,
            "folderName": app.name,
            "releaseAssetFilter": app.asset_filter,
            // An update check inside `--run` can hang like `--download`.
            "autoUpdate": false,
        });
        if let Some(Runner::Proton) = app.runner {
            // heimspiel runs Windows Ports under Proton itself; this keeps Quiver's own launch
            // (from its window) on the same prefix.
            entry["linuxRunner"] = "proton".into();
            entry["linuxPrefixPath"] = self.prefix(app).to_string_lossy().into();
        }
        if let Some(pinned) = pinned {
            entry["preferredVersion"] = pinned;
        }
        match existing {
            Some(i) if apps[i] == entry => return Ok(()),
            Some(i) => apps[i] = entry,
            None => apps.push(entry),
        }
        let tmp = path.with_extension("json.heimspiel-tmp");
        fs::write(&tmp, serde_json::to_string_pretty(&doc)? + "\n")?;
        fs::rename(&tmp, &path).with_context(|| format!("replacing {}", path.display()))
    }

    /// The installed program: as Quiver noted it in `selected_executable.txt`, or the only
    /// executable file in the app folder (Quiver writes no note for a single AppImage).
    pub fn executable(&self, app: &QuiverApp) -> Option<PathBuf> {
        use std::os::unix::fs::PermissionsExt;
        let folder = self.dir.join("Apps").join(&app.name);
        if let Ok(note) = fs::read_to_string(folder.join("selected_executable.txt")) {
            let path = PathBuf::from(note.trim());
            return path.is_file().then_some(path);
        }
        let mut programs = fs::read_dir(&folder).ok()?.filter_map(|f| {
            let path = f.ok()?.path();
            let mode = fs::metadata(&path).ok()?.permissions().mode();
            (path.is_file() && mode & 0o111 != 0).then_some(path)
        });
        let program = programs.next()?;
        programs.next().is_none().then_some(program)
    }

    /// The version Quiver's catalog verified for the app, from the copy Quiver caches on its first
    /// run; `None` for apps outside the catalog or before that run.
    pub fn catalog_version(&self, app: &QuiverApp) -> Option<String> {
        let text = fs::read_to_string(self.dir.join("Cache/catalog-release-status.json")).ok()?;
        let apps: Vec<Value> = serde_json::from_str(text.trim_start_matches('\u{feff}')).ok()?;
        apps.iter().find(|a| {
            a["repository"]
                .as_str()
                .is_some_and(|r| r.eq_ignore_ascii_case(&app.repository))
        })?["verified"]["version"]
            .as_str()
            .map(str::to_owned)
    }

    /// The folder Quiver installs the app into.
    pub fn app_dir(&self, app: &QuiverApp) -> PathBuf {
        self.dir.join("Apps").join(&app.name)
    }

    /// The Proton prefix (Proton's compat data folder) of a Windows Port.
    pub fn prefix(&self, app: &QuiverApp) -> PathBuf {
        self.dir.join("Prefixes").join(&app.name)
    }

    pub fn is_installed(&self, app: &QuiverApp) -> bool {
        self.dir
            .join("Apps")
            .join(&app.name)
            .join("installed-build.txt")
            .is_file()
    }

    /// Runs `--download` until the build is installed, then stops Quiver.
    pub fn install(&self, app: &QuiverApp, timeout: Duration) -> anyhow::Result<Install> {
        let mut child = self.spawn(&["--download", &app.name])?;
        let started = Instant::now();
        loop {
            if self.is_installed(app) {
                stop(&mut child);
                return Ok(Install::Done);
            }
            if child.try_wait()?.is_some() {
                return Ok(Install::Failed);
            }
            if started.elapsed() > timeout {
                stop(&mut child);
                bail!(
                    "Quiver did not install {} within {} s",
                    app.name,
                    timeout.as_secs()
                );
            }
            thread::sleep(Duration::from_millis(100));
        }
    }

    /// The command that runs the app; it blocks until the app exits.
    pub fn run_command(&self, app: &QuiverApp) -> Command {
        let mut command = Command::new(self.appimage());
        command.args(["--run", &app.name]).current_dir(&self.dir);
        command
    }

    /// Where Quiver's own output goes, appended.
    pub fn open_log(&self) -> anyhow::Result<fs::File> {
        Ok(fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.log())?)
    }

    pub fn log(&self) -> PathBuf {
        self.dir.join("heimspiel-quiver.log")
    }

    /// Starts Quiver in a process group of its own, so it can be stopped with all its children.
    fn spawn(&self, args: &[&str]) -> anyhow::Result<Child> {
        let log = self.open_log()?;
        Command::new(self.appimage())
            .args(args)
            .current_dir(&self.dir)
            .stdin(Stdio::null())
            .stdout(log.try_clone()?)
            .stderr(log)
            .process_group(0)
            .spawn()
            .with_context(|| format!("starting {}", self.appimage().display()))
    }
}

/// The tag of the newest release of `owner/repo` on GitHub.
pub fn latest_tag(repository: &str) -> anyhow::Result<String> {
    let url = format!("https://api.github.com/repos/{repository}/releases/latest");
    let text = ureq::get(&url)
        .header("Accept", "application/vnd.github+json")
        .call()
        .with_context(|| format!("asking GitHub for the latest release of {repository}"))?
        .body_mut()
        .read_to_string()?;
    let release: Value = serde_json::from_str(&text)?;
    release["tag_name"]
        .as_str()
        .map(str::to_owned)
        .with_context(|| format!("GitHub lists no release for {repository}"))
}
