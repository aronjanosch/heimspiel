//! `heimspiel launch`: install a Port, provision its Original, and run it. ES-DE waits for this
//! command, so the Port runs in the foreground until it exits.

use std::time::Duration;

use anyhow::Context;

use crate::fetch::Fetch;
use crate::port::{Manifest, QuiverApp};
use crate::provision::{self, Homes};
use crate::quiver::Quiver;

pub struct PortLaunch<'a> {
    pub manifests: &'a [Manifest],
    pub quiver: &'a Quiver,
    pub fetch: &'a Fetch<'a>,
    pub homes: &'a Homes,
    pub install_timeout: Duration,
    /// The newest release tag of a Port's repository; heimspiel pins it in Quiver before
    /// installing.
    pub latest_version: &'a dyn Fn(&QuiverApp) -> anyhow::Result<String>,
}

/// Launches the Port `port_id` with the RomM game `rom_id` as its Original.
pub fn run(ctx: &PortLaunch, port_id: &str, rom_id: u64) -> anyhow::Result<()> {
    let manifest = ctx
        .manifests
        .iter()
        .find(|m| m.id == port_id)
        .with_context(|| format!("no Port called {port_id}"))?;
    let app = &manifest.quiver;
    if ctx.quiver.is_installed(app) {
        ctx.quiver.ensure_entry(app, None)?;
    } else {
        (ctx.fetch.notify)(&format!("Installing {} …", manifest.title));
        let version = (ctx.latest_version)(app)?;
        ctx.quiver.ensure_entry(app, Some(&version))?;
        ctx.quiver.install(app, ctx.install_timeout)?;
    }
    let exe = ctx.quiver.executable(&manifest.quiver);
    provision::provide(&manifest.provide, ctx.homes, exe.as_deref(), || {
        ctx.fetch.game(rom_id)
    })?;
    ctx.quiver.run(&manifest.quiver)
}
