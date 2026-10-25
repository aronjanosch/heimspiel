//! `heimspiel launch`: install a Port, provision its Original, and run it. ES-DE waits for this
//! command, so the Port runs in the foreground until it exits. `heimspiel check` does the same
//! inside a wrapper such as a headless gamescope and only checks that the Port keeps running.

use std::ffi::OsStr;
use std::fs;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, bail};

use crate::dump;
use crate::fetch::Fetch;
pub use crate::port::OriginalLocation;
use crate::port::{Manifest, QuiverApp, Runner};
use crate::process::stop;
use crate::progress::Progress;
use crate::proton::Proton;
use crate::provision::{self, Homes};
use crate::quiver::{Install, Pin, Quiver};

pub struct PortLaunch<'a> {
    pub manifests: &'a [Manifest],
    pub quiver: &'a Quiver,
    /// Runs the Windows builds of Ports.
    pub proton: Option<&'a Proton>,
    /// Access to RomM, for Ports whose Original is there.
    pub fetch: Option<&'a Fetch<'a>>,
    /// Shows the installation of a Port; finished right before the Port starts.
    pub progress: &'a dyn Progress,
    pub homes: &'a Homes,
    pub install_timeout: Duration,
    /// A command line every program heimspiel starts for the Port runs inside, such as a
    /// headless gamescope; empty to run them directly.
    pub wrapper: &'a [String],
    /// The newest release tag of a Port's repository; heimspiel pins it in Quiver before
    /// installing.
    pub latest_version: &'a dyn Fn(&QuiverApp) -> anyhow::Result<String>,
}

/// Launches the Port `port_id` with its Original at `original`; `game` picks one of the Port's
/// games, for Ports with several.
pub fn run(
    ctx: &PortLaunch,
    port_id: &str,
    original: &OriginalLocation,
    game: Option<&str>,
) -> anyhow::Result<()> {
    let mut command = prepare(ctx, port_id, original, game)?;
    ctx.progress.finish();
    let log = ctx.quiver.open_log()?;
    let status = command
        .stdin(Stdio::null())
        .stdout(log.try_clone()?)
        .stderr(log)
        .process_group(0)
        .status()
        .with_context(|| format!("starting {port_id}"))?;
    if !status.success() {
        bail!(
            "{port_id} ended with {status}; see {}",
            ctx.quiver.log().display()
        );
    }
    Ok(())
}

/// Installs and provisions like [`run`], then runs the Port (inside `ctx.wrapper`, such as a
/// headless gamescope) and checks that it is still running after `duration`; then stops it. The
/// output goes to `log`. Sound through SDL is muted.
pub fn check(
    ctx: &PortLaunch,
    port_id: &str,
    original: &OriginalLocation,
    game: Option<&str>,
    duration: Duration,
    log: &Path,
) -> anyhow::Result<()> {
    let mut command = prepare(ctx, port_id, original, game)?;
    ctx.progress.finish();
    let output = fs::File::create(log).with_context(|| format!("creating {}", log.display()))?;
    let mut child = command
        .env("SDL_AUDIODRIVER", "dummy")
        .stdin(Stdio::null())
        .stdout(output.try_clone()?)
        .stderr(output)
        .process_group(0)
        .spawn()
        .with_context(|| format!("starting {port_id}"))?;
    let started = Instant::now();
    while started.elapsed() < duration {
        if let Some(status) = child.try_wait()? {
            bail!(
                "{port_id} ended after {} s with {status}; see {}",
                started.elapsed().as_secs(),
                log.display()
            );
        }
        thread::sleep(Duration::from_millis(100));
    }
    stop(&mut child);
    Ok(())
}

/// `command` run by `wrapper`, keeping its environment and folder.
fn wrap(command: &Command, wrapper: &[String]) -> Command {
    let mut parts: Vec<&OsStr> = wrapper.iter().map(OsStr::new).collect();
    parts.push(command.get_program());
    parts.extend(command.get_args());
    let mut wrapped = Command::new(parts[0]);
    wrapped.args(&parts[1..]);
    for (key, value) in command.get_envs() {
        match value {
            Some(value) => wrapped.env(key, value),
            None => wrapped.env_remove(key),
        };
    }
    if let Some(dir) = command.get_current_dir() {
        wrapped.current_dir(dir);
    }
    wrapped
}

/// Installs the Port and provisions its Original; returns the command that runs it.
fn prepare(
    ctx: &PortLaunch,
    port_id: &str,
    original: &OriginalLocation,
    game: Option<&str>,
) -> anyhow::Result<Command> {
    let manifest = ctx
        .manifests
        .iter()
        .find(|m| m.id == port_id)
        .with_context(|| format!("no Port called {port_id}"))?;
    // A Dump is checked before anything is installed: heimspiel cannot fetch a missing one.
    if let OriginalLocation::Dump(path) = original {
        manifest.original.check(&dump::read(path)?)?;
    }
    let app = &manifest.quiver;
    if ctx.quiver.is_installed(app) {
        ctx.quiver.ensure_entry(app, Pin::Keep)?;
    } else {
        let cover = match (original, ctx.fetch) {
            (OriginalLocation::RomM(rom_id), Some(fetch)) => fetch.cover(*rom_id),
            _ => None,
        };
        ctx.progress
            .start(&manifest.title, cover.as_deref(), "Installing");
        let mut version = match ctx.quiver.catalog_version(app) {
            Some(listed) => listed,
            None => (ctx.latest_version)(app)?,
        };
        loop {
            ctx.quiver.ensure_entry(app, Pin::Version(&version))?;
            if ctx.quiver.install(app, ctx.install_timeout)? == Install::Done {
                break;
            }
            // Quiver fetches its catalog on its first run; a catalog app needs the version listed there.
            match ctx.quiver.catalog_version(app) {
                Some(listed) if listed != version => version = listed,
                _ => bail!(
                    "Quiver could not install {} {version}; see {}",
                    app.name,
                    ctx.quiver.log().display()
                ),
            }
        }
    }
    match original {
        OriginalLocation::RomM(rom_id) => {
            let fetch = ctx
                .fetch
                .context("this Port's Original is in RomM; set up [romm]")?;
            let port = ctx
                .quiver
                .executable(app)
                .map(|exe| wrap(&Command::new(exe), ctx.wrapper));
            provision::provide(&manifest.provide, ctx.homes, port, || fetch.game(*rom_id))?;
        }
        OriginalLocation::Dump(path) => {
            provision::provide_dump(&manifest.provide, &ctx.quiver.prefix(app), path, game)?;
        }
    }
    match app.runner {
        Some(Runner::Proton) => {
            let proton = ctx.proton.context(
                "this Port is a Windows build and needs Proton; install it through Steam",
            )?;
            let executable = app
                .executable
                .as_deref()
                .context("the Manifest names no program to start")?;
            let program = ctx.quiver.app_dir(app).join(executable);
            let command = proton.command(
                &program,
                &ctx.quiver.prefix(app),
                &format!("heimspiel-{}", app.name),
            )?;
            Ok(wrap(&command, ctx.wrapper))
        }
        None => {
            let mut command = ctx.quiver.run_command(app);
            if let Some(home) = provision::ship_home(&manifest.provide, ctx.homes) {
                command.env("SHIP_HOME", home);
            }
            Ok(wrap(&command, ctx.wrapper))
        }
    }
}
