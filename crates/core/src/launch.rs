//! `heimspiel launch`: install a Port, provision its Original, and run it. ES-DE waits for this
//! command, so the Port runs in the foreground until it exits.

use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, bail};

use crate::dump;
use crate::fetch::Fetch;
use crate::port::{Manifest, QuiverApp, Runner};
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
    pub notify: &'a dyn Fn(&str),
    pub homes: &'a Homes,
    pub install_timeout: Duration,
    /// The newest release tag of a Port's repository; heimspiel pins it in Quiver before
    /// installing.
    pub latest_version: &'a dyn Fn(&QuiverApp) -> anyhow::Result<String>,
}

/// Where the Port's Original is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OriginalLocation {
    /// A game on the RomM server, by its id.
    RomM(u64),
    /// A Dump folder.
    Dump(PathBuf),
}

/// Launches the Port `port_id` with its Original at `original`; `game` picks one of the Port's
/// games, for Ports with several.
pub fn run(
    ctx: &PortLaunch,
    port_id: &str,
    original: &OriginalLocation,
    game: Option<&str>,
) -> anyhow::Result<()> {
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
        (ctx.notify)(&format!("Installing {} …", manifest.title));
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
            let exe = ctx.quiver.executable(app);
            provision::provide(&manifest.provide, ctx.homes, exe.as_deref(), || {
                fetch.game(*rom_id)
            })?;
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
            proton.run(
                &program,
                &ctx.quiver.prefix(app),
                &format!("heimspiel-{}", app.name),
            )
        }
        None => ctx.quiver.run(app),
    }
}
