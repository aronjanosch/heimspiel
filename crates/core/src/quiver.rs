//! Quiver Launcher, which installs and runs Ports (ADR 0003). heimspiel keeps it in a folder of its
//! own, because Quiver stores its library next to its AppImage.
//!
//! Quiver 3.5.1 quirks, see docs/research/2026-10-23-tests.md (test 1): `--download` installs but
//! never exits, so completion is detected from `installed-build.txt`; two `apps.json` entries for
//! the same repository get mixed up, so there is one entry per repository.

use std::fs;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, bail};
use serde_json::{Value, json};

use crate::port::QuiverApp;

pub const APPIMAGE: &str = "QuiverLauncher-linux-x64.AppImage";

/// The Quiver release heimspiel was tested with, and the SHA-256 GitHub reports for its AppImage.
const RELEASE_URL: &str = "https://github.com/tgeorgiadis/quiver-launcher/releases/download/v3.5.1/QuiverLauncher-linux-x64.AppImage";
const RELEASE_SHA256: &str = "a8cd5398b6dae924bcec09ece1781c03cbfd73298675476f26fc1c072d3c629b";

pub struct Quiver {
    dir: PathBuf,
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
    pub fn ensure_entry(&self, app: &QuiverApp) -> anyhow::Result<()> {
        let path = self.dir.join("apps.json");
        let mut doc: Value = match fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str(&text)
                .with_context(|| format!("parsing {}", path.display()))?,
            Err(_) => json!({ "apps": [] }),
        };
        let apps = doc["apps"]
            .as_array_mut()
            .context("apps.json has no apps list")?;
        let entry = json!({
            "name": app.name,
            "repository": app.repository,
            "folderName": app.name,
            "releaseAssetFilter": app.asset_filter,
            // An update check inside `--run` can hang like `--download`.
            "autoUpdate": false,
        });
        match apps
            .iter_mut()
            .find(|a| a["repository"] == app.repository.as_str())
        {
            Some(existing) if *existing == entry => return Ok(()),
            Some(existing) => *existing = entry,
            None => apps.push(entry),
        }
        let tmp = path.with_extension("json.heimspiel-tmp");
        fs::write(&tmp, serde_json::to_string_pretty(&doc)? + "\n")?;
        fs::rename(&tmp, &path).with_context(|| format!("replacing {}", path.display()))
    }

    pub fn is_installed(&self, app: &QuiverApp) -> bool {
        self.dir
            .join("Apps")
            .join(&app.name)
            .join("installed-build.txt")
            .is_file()
    }

    /// Runs `--download` until the build is installed, then stops Quiver.
    pub fn install(&self, app: &QuiverApp, timeout: Duration) -> anyhow::Result<()> {
        let mut child = self.spawn(&["--download", &app.name])?;
        let started = Instant::now();
        loop {
            if self.is_installed(app) {
                stop(&mut child);
                return Ok(());
            }
            if child.try_wait()?.is_some() {
                bail!(
                    "Quiver ended without installing {}; see {}",
                    app.name,
                    self.log().display()
                );
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

    /// Runs the app and waits until it exits.
    pub fn run(&self, app: &QuiverApp) -> anyhow::Result<()> {
        let status = self.spawn(&["--run", &app.name])?.wait()?;
        if !status.success() {
            bail!(
                "{} ended with {status}; see {}",
                app.name,
                self.log().display()
            );
        }
        Ok(())
    }

    fn log(&self) -> PathBuf {
        self.dir.join("heimspiel-quiver.log")
    }

    /// Starts Quiver in a process group of its own, so it can be stopped with all its children.
    fn spawn(&self, args: &[&str]) -> anyhow::Result<Child> {
        let log = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.log())?;
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

fn stop(child: &mut Child) {
    let group = format!("-{}", child.id());
    let killed = Command::new("kill").args(["-TERM", "--", &group]).status();
    if !killed.is_ok_and(|s| s.success()) {
        let _ = child.kill();
    }
    let _ = child.wait();
}
