//! Ports and their Manifests. heimspiel ships the Manifests it knows in `manifests/`; a Port is
//! offered once the Library holds its Original.

use anyhow::Context;
use serde::Deserialize;

use crate::desktop;
use crate::library::{Entry, FileLocation, Launch, Library, Metadata, SourceKind};

/// heimspiel's description of one Port.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Manifest {
    pub id: String,
    pub title: String,
    pub quiver: QuiverApp,
    pub original: Original,
    pub provide: Provide,
}

/// How Quiver finds the Port's releases.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct QuiverApp {
    /// Name of the entry in Quiver's `apps.json`.
    pub name: String,
    /// `owner/repo` on GitHub.
    pub repository: String,
    /// Substring (or glob) that picks the release asset for this system.
    pub asset_filter: String,
}

/// The game a Port needs as its Original, and which of its releases it accepts.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Original {
    pub platform: String,
    pub igdb_id: u64,
    pub releases: Vec<Release>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Release {
    pub region: String,
    /// `None` for the first release, `A` for "Rev A".
    #[serde(default)]
    pub revision: Option<String>,
}

/// How the Original is provisioned.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "kebab-case"
)]
pub enum Provide {
    /// N64Recomp ports: a big-endian `<game id>.z64` in the Port's config folder (`~/.config/<dir>`),
    /// checked by librecomp with XXH3-64.
    Librecomp {
        config_dir: String,
        game_id: String,
        #[serde(deserialize_with = "hex")]
        xxh3: u64,
    },
    /// HarbourMasters ports: started once with the ROM, they extract it into an archive in the
    /// user data folder (`~/.local/share/<archive>`) and then ask whether to run; heimspiel stops
    /// them there.
    O2r {
        archive: String,
        /// SHA-1 of every accepted release, as big-endian ROM.
        sha1: Vec<String>,
    },
}

fn hex<'de, D: serde::Deserializer<'de>>(d: D) -> Result<u64, D::Error> {
    let text = String::deserialize(d)?;
    u64::from_str_radix(text.trim_start_matches("0x"), 16).map_err(serde::de::Error::custom)
}

impl Manifest {
    pub fn parse(text: &str) -> anyhow::Result<Manifest> {
        Ok(toml::from_str(text)?)
    }
}

/// The Manifests that ship with heimspiel.
pub fn builtin() -> Vec<Manifest> {
    const FILES: &[(&str, &str)] = &[
        ("banjorecomp", include_str!("../manifests/banjorecomp.toml")),
        ("soh", include_str!("../manifests/soh.toml")),
        (
            "zelda64recomp",
            include_str!("../manifests/zelda64recomp.toml"),
        ),
    ];
    FILES
        .iter()
        .map(|(name, text)| {
            Manifest::parse(text)
                .with_context(|| format!("manifest {name}"))
                .expect("built-in manifests are valid")
        })
        .collect()
}

/// One Entry per Port whose Original is in the Library. `heimspiel` is the command line that runs
/// heimspiel itself; the Entry launches through it.
pub fn entries(manifests: &[Manifest], library: &Library, heimspiel: &[String]) -> Vec<Entry> {
    manifests
        .iter()
        .filter_map(|manifest| {
            let (original, rom_id) = library
                .entries
                .iter()
                .find_map(|e| Some((e, romm_original(e, &manifest.original)?)))?;
            let mut args = heimspiel.to_vec();
            args.extend([
                "launch".into(),
                manifest.id.clone(),
                "--romm".into(),
                rom_id.to_string(),
            ]);
            Some(Entry {
                source: SourceKind::Port,
                id: manifest.id.clone(),
                title: manifest.title.clone(),
                launch: Launch::Command {
                    exec: desktop::exec(&args),
                    working_dir: None,
                },
                cover: original.cover.clone(),
                metadata: Metadata {
                    description: Some(format!(
                        "A native PC port of {}. heimspiel installs it and provides your copy \
                         from RomM on the first start.",
                        original.title
                    )),
                    ..original.metadata.clone()
                },
                identity: Default::default(),
            })
        })
        .collect()
}

/// The RomM id of `entry` if it is the release `original` names.
fn romm_original(entry: &Entry, original: &Original) -> Option<u64> {
    let Launch::File {
        platform,
        location: FileLocation::RomM { rom_id },
        ..
    } = &entry.launch
    else {
        return None;
    };
    let id = &entry.identity;
    let matches = *platform == original.platform
        && id.igdb_id == Some(original.igdb_id)
        && original
            .releases
            .iter()
            .any(|r| id.regions.contains(&r.region) && id.revision == r.revision);
    matches.then_some(*rom_id)
}
