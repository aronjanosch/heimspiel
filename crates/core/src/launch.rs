//! `heimspiel launch`: install a Port, provision its Original, and run it. ES-DE waits for this
//! command, so the Port runs in the foreground until it exits.

use std::path::PathBuf;
use std::time::Duration;

use anyhow::Context;

use crate::dump;
use crate::fetch::Fetch;
use crate::port::{Manifest, QuiverApp};
use crate::provision::{self, Homes};
use crate::quiver::Quiver;

pub struct PortLaunch<'a> {
    pub manifests: &'a [Manifest],
    pub quiver: &'a Quiver,
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
        ctx.quiver.ensure_entry(app, None)?;
    } else {
        (ctx.notify)(&format!("Installing {} …", manifest.title));
        let version = (ctx.latest_version)(app)?;
        ctx.quiver.ensure_entry(app, Some(&version))?;
        ctx.quiver.install(app, ctx.install_timeout)?;
    }
    ctx.quiver.select_executable(app)?;
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
    ctx.quiver.run(app)
}
