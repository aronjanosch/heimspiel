//! Ports and their Manifests. heimspiel ships the Manifests it knows in `manifests/`; a Port is
//! offered once the Library holds its Original.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use serde::Deserialize;

use crate::catalog::{self, CatalogPort};
use crate::desktop;
use crate::dump::Dump;
use crate::library::{Entry, FileLocation, Launch, Library, Metadata, RomHashes, SourceKind};

/// heimspiel's description of one Port.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Manifest {
    pub id: String,
    pub title: String,
    pub quiver: QuiverApp,
    pub original: Original,
    /// The games of a Port that holds several, such as a collection; each becomes an Entry.
    #[serde(default)]
    pub games: Vec<Game>,
    pub provide: Provide,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Game {
    /// What the Port's launcher calls the game, e.g. `u1`.
    pub id: String,
    pub title: String,
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
    /// How a Windows build runs on Linux; native builds have none.
    #[serde(default)]
    pub runner: Option<Runner>,
    /// The Windows program heimspiel starts under Proton; a path inside the release.
    #[serde(default)]
    pub executable: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Runner {
    Proton,
}

/// The game a Port needs as its Original, and which of its releases it accepts: a ROM release
/// named by IGDB id, region and revision, or a Dump named by title IDs.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Original {
    pub platform: String,
    #[serde(default)]
    pub igdb_id: Option<u64>,
    #[serde(default)]
    pub releases: Vec<Release>,
    /// Title IDs of the accepted Dumps, e.g. `CUSA02320`.
    #[serde(default)]
    pub title_ids: Vec<String>,
    /// Files and folders an accepted Dump must hold.
    #[serde(default)]
    pub files: Vec<String>,
}

impl Original {
    /// Whether `dump` is a complete copy of an accepted title.
    pub fn accepts(&self, dump: &Dump) -> bool {
        self.check(dump).is_ok()
    }

    /// Explains why `dump` is not a complete copy of an accepted title.
    pub fn check(&self, dump: &Dump) -> anyhow::Result<()> {
        if dump.platform != self.platform || !self.title_ids.contains(&dump.title_id) {
            bail!(
                "{} is {} ({}); this Port needs {}",
                dump.path.display(),
                dump.title,
                dump.title_id,
                self.title_ids.join(" or ")
            );
        }
        let missing: Vec<&str> = self
            .files
            .iter()
            .filter(|f| !dump.path.join(f).exists())
            .map(String::as_str)
            .collect();
        if !missing.is_empty() {
            bail!(
                "the Dump {} is incomplete; missing: {}",
                dump.path.display(),
                missing.join(", ")
            );
        }
        Ok(())
    }
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
    /// Ports that come with a launcher: heimspiel writes the Dump's folder and the chosen game into
    /// the launcher's settings in the Port's Windows `%APPDATA%`, then the Port starts straight
    /// into the game.
    Dump {
        /// The settings file, relative to `%APPDATA%`.
        settings: String,
        /// Settings key for the Dump's folder, as a Windows path.
        game_dir_key: String,
        /// Settings key for the game to start, for Ports with several games.
        #[serde(default)]
        game_key: Option<String>,
        /// Further settings heimspiel fixes.
        #[serde(default)]
        set: BTreeMap<String, serde_json::Value>,
    },
}

impl Provide {
    /// Whether a ROM with these hashes is a release this Port accepts.
    pub fn accepts(&self, rom: &RomHashes) -> bool {
        match self {
            Provide::Librecomp { xxh3, .. } => rom.xxh3 == *xxh3,
            Provide::O2r { sha1, .. } => sha1.iter().any(|s| s.eq_ignore_ascii_case(&rom.sha1)),
            Provide::Dump { .. } => false,
        }
    }
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
        ("2s2h", include_str!("../manifests/2s2h.toml")),
        ("banjorecomp", include_str!("../manifests/banjorecomp.toml")),
        ("bm64recomp", include_str!("../manifests/bm64recomp.toml")),
        ("dk64recomp", include_str!("../manifests/dk64recomp.toml")),
        ("dnzhrecomp", include_str!("../manifests/dnzhrecomp.toml")),
        (
            "goemon64recomp",
            include_str!("../manifests/goemon64recomp.toml"),
        ),
        ("hm64recomp", include_str!("../manifests/hm64recomp.toml")),
        ("lodrecomp", include_str!("../manifests/lodrecomp.toml")),
        ("mk64recomp", include_str!("../manifests/mk64recomp.toml")),
        ("mm64recomp", include_str!("../manifests/mm64recomp.toml")),
        ("sbk2recomp", include_str!("../manifests/sbk2recomp.toml")),
        ("sbkrecomp", include_str!("../manifests/sbkrecomp.toml")),
        ("sf64recomp", include_str!("../manifests/sf64recomp.toml")),
        ("soh", include_str!("../manifests/soh.toml")),
        ("sssvrecomp", include_str!("../manifests/sssvrecomp.toml")),
        ("uncharted", include_str!("../manifests/uncharted.toml")),
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

/// The Entries of every Port whose Original is in the Library or among the Dumps: one per Port,
/// or one per game for a Port with several. `heimspiel` is the command line that runs heimspiel
/// itself; the Entries launch through it.
pub fn entries(
    manifests: &[Manifest],
    library: &Library,
    dumps: &[Dump],
    heimspiel: &[String],
) -> Vec<Entry> {
    let mut entries = Vec::new();
    for manifest in manifests {
        let launch = |original: &OriginalLocation, game: Option<&Game>| {
            let mut args = heimspiel.to_vec();
            args.extend(["launch".into(), manifest.id.clone()]);
            args.extend(original.args());
            if let Some(game) = game {
                args.extend(["--game".into(), game.id.clone()]);
            }
            Launch::Command {
                exec: desktop::exec(&args),
                working_dir: None,
            }
        };
        if let Some((original, location)) = in_library(manifest, library) {
            let from = match location {
                OriginalLocation::RomM(_) => " from RomM",
                _ => "",
            };
            entries.push(Entry {
                source: SourceKind::Port,
                id: manifest.id.clone(),
                title: manifest.title.clone(),
                launch: launch(&location, None),
                cover: original.cover.clone(),
                metadata: Metadata {
                    description: Some(format!(
                        "A native PC port of {}. heimspiel installs it and provides your copy{from} \
                         on the first start.",
                        original.title
                    )),
                    ..original.metadata.clone()
                },
                identity: Default::default(),
            });
        } else if let Some(dump) = dumps.iter().find(|d| manifest.original.accepts(d)) {
            let original = OriginalLocation::Dump(dump.path.clone());
            let icon = dump.path.join("sce_sys/icon0.png");
            let entry = |id: String, title: &str, game: Option<&Game>| Entry {
                source: SourceKind::Port,
                id,
                title: title.to_owned(),
                launch: launch(&original, game),
                cover: icon.is_file().then(|| icon.clone()),
                metadata: Metadata {
                    description: Some(format!(
                        "A native PC port of {}. heimspiel installs it and points it to your Dump \
                         on the first start.",
                        dump.title
                    )),
                    ..Default::default()
                },
                identity: Default::default(),
            };
            if manifest.games.is_empty() {
                entries.push(entry(manifest.id.clone(), &manifest.title, None));
            }
            for game in &manifest.games {
                let id = format!("{}/{}", manifest.id, game.id);
                entries.push(entry(id, &game.title, Some(game)));
            }
        }
    }
    entries
}

/// Where a Port's Original is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OriginalLocation {
    /// A game on the RomM server, by its id.
    RomM(u64),
    /// A ROM in a local folder.
    File(PathBuf),
    /// A Dump folder.
    Dump(PathBuf),
}

impl OriginalLocation {
    /// The arguments of `heimspiel launch` that name this Original.
    fn args(&self) -> [String; 2] {
        match self {
            OriginalLocation::RomM(rom_id) => ["--romm".into(), rom_id.to_string()],
            OriginalLocation::File(path) => ["--file".into(), path.to_string_lossy().into()],
            OriginalLocation::Dump(path) => ["--dump".into(), path.to_string_lossy().into()],
        }
    }
}

/// Where the Original of `manifest` is: a matching ROM in the Library, else an accepted Dump.
pub fn original(
    manifest: &Manifest,
    library: &Library,
    dumps: &[Dump],
) -> Option<OriginalLocation> {
    in_library(manifest, library)
        .map(|(_, location)| location)
        .or_else(|| {
            dumps
                .iter()
                .find(|d| manifest.original.accepts(d))
                .map(|d| OriginalLocation::Dump(d.path.clone()))
        })
}

/// What `heimspiel ports` shows.
#[derive(Debug)]
pub struct Overview<'a> {
    /// Every Port heimspiel has a Manifest for.
    pub manifests: Vec<ManifestState<'a>>,
    /// Catalog Ports for games the user owns that heimspiel has no Manifest for.
    pub more: Vec<CatalogMatch<'a>>,
}

#[derive(Debug)]
pub struct ManifestState<'a> {
    pub manifest: &'a Manifest,
    /// The Original found for it, as `Title (RomM)` or `Title (Dump CUSA02320)`.
    pub original: Option<String>,
    pub installed: bool,
}

#[derive(Debug)]
pub struct CatalogMatch<'a> {
    pub port: &'a CatalogPort,
    /// The title of the user's copy.
    pub original: String,
}

/// The state of every Manifest, and the Catalog Ports that match games in the Library or the
/// Dumps by title and Platform. `installed` says whether Quiver has installed a Port.
pub fn overview<'a>(
    manifests: &'a [Manifest],
    library: &Library,
    dumps: &[Dump],
    catalog: &'a [CatalogPort],
    installed: &dyn Fn(&QuiverApp) -> bool,
) -> Overview<'a> {
    let states = manifests
        .iter()
        .map(|manifest| {
            let from_library = in_library(manifest, library).map(|(e, location)| match location {
                OriginalLocation::File(path) => format!("{} ({})", e.title, path.display()),
                _ => format!("{} (RomM)", e.title),
            });
            let from_dump = || {
                dumps
                    .iter()
                    .find(|d| manifest.original.accepts(d))
                    .map(|d| format!("{} (Dump {})", d.title, d.title_id))
            };
            ManifestState {
                manifest,
                original: from_library.or_else(from_dump),
                installed: installed(&manifest.quiver),
            }
        })
        .collect();
    // What the user owns, as (Platform, title).
    let owned: Vec<(&str, &str)> = library
        .entries
        .iter()
        .filter_map(|e| match (&e.launch, e.source) {
            (_, SourceKind::Port) => None,
            (Launch::File { platform, .. }, _) => Some((platform.as_str(), e.title.as_str())),
            (Launch::Command { .. }, _) => Some(("pc", e.title.as_str())),
        })
        .chain(
            dumps
                .iter()
                .map(|d| (d.platform.as_str(), d.title.as_str())),
        )
        .collect();
    let mut more = Vec::new();
    for (platform, title) in owned {
        let key = catalog::game_key(title);
        for port in catalog {
            let known = manifests
                .iter()
                .any(|m| m.quiver.repository.eq_ignore_ascii_case(&port.repository));
            let listed = more
                .iter()
                .any(|m: &CatalogMatch| std::ptr::eq(m.port, port));
            if !known
                && !listed
                && port.game_key == key
                && port.platforms.iter().any(|p| p == platform)
            {
                more.push(CatalogMatch {
                    port,
                    original: title.to_owned(),
                });
            }
        }
    }
    Overview {
        manifests: states,
        more,
    }
}

/// The Library's Entry that is the Original of `manifest`, and where it is. A ROM in a local
/// folder comes before one in RomM: it needs no download.
fn in_library<'a>(
    manifest: &Manifest,
    library: &'a Library,
) -> Option<(&'a Entry, OriginalLocation)> {
    let find = |found: &dyn Fn(&Entry) -> Option<OriginalLocation>| {
        library.entries.iter().find_map(|e| Some((e, found(e)?)))
    };
    find(&|e| local_original(e, manifest).map(|path| OriginalLocation::File(path.to_owned())))
        .or_else(|| find(&|e| romm_original(e, &manifest.original).map(OriginalLocation::RomM)))
}

/// The path of `entry` if it is a ROM in a local folder that `manifest` accepts.
fn local_original<'a>(entry: &'a Entry, manifest: &Manifest) -> Option<&'a Path> {
    let Launch::File {
        platform,
        location: FileLocation::Local { path },
        ..
    } = &entry.launch
    else {
        return None;
    };
    let rom = entry.identity.rom.as_ref()?;
    (*platform == manifest.original.platform && manifest.provide.accepts(rom)).then_some(path)
}

/// The RomM id of `entry` if it is the release `original` names.
fn romm_original(entry: &Entry, original: &Original) -> Option<u64> {
    let Launch::File {
        platform,
        location: FileLocation::RomM { rom_id, .. },
        ..
    } = &entry.launch
    else {
        return None;
    };
    let id = &entry.identity;
    let matches = *platform == original.platform
        && original.igdb_id.is_some()
        && id.igdb_id == original.igdb_id
        && original
            .releases
            .iter()
            .any(|r| id.regions.contains(&r.region) && id.revision == r.revision);
    matches.then_some(*rom_id)
}
