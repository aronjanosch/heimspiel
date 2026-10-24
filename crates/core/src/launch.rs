//! `heimspiel launch`: install a Port, provision its Original, and run it. ES-DE waits for this
//! command, so the Port runs in the foreground until it exits.

use std::path::Path;
use std::time::Duration;

use anyhow::Context;

use crate::fetch::Fetch;
use crate::port::Manifest;
use crate::provision;
use crate::quiver::Quiver;

pub struct PortLaunch<'a> {
    pub manifests: &'a [Manifest],
    pub quiver: &'a Quiver,
    pub fetch: &'a Fetch<'a>,
    /// Stands for `~/.config`.
    pub config_home: &'a Path,
    pub install_timeout: Duration,
}

/// Launches the Port `port_id` with the RomM game `rom_id` as its Original.
pub fn run(ctx: &PortLaunch, port_id: &str, rom_id: u64) -> anyhow::Result<()> {
    let manifest = ctx
        .manifests
        .iter()
        .find(|m| m.id == port_id)
        .with_context(|| format!("no Port called {port_id}"))?;
    ctx.quiver.ensure_entry(&manifest.quiver)?;
    if !ctx.quiver.is_installed(&manifest.quiver) {
        (ctx.fetch.notify)(&format!("Installing {} …", manifest.title));
        ctx.quiver.install(&manifest.quiver, ctx.install_timeout)?;
    }
    provision::provide(&manifest.provide, ctx.config_home, || {
        ctx.fetch.game(rom_id)
    })?;
    ctx.quiver.run(&manifest.quiver)
}
