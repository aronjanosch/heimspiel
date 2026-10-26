//! The Library: every Entry heimspiel knows about, across all Sources.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::start::Start;

/// All Entries heimspiel knows about.
#[derive(Debug, Default)]
pub struct Library {
    pub entries: Vec<Entry>,
}

/// One playable game, as delivered by exactly one Source.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub source: SourceKind,
    /// Identifies the Entry within its Source (Steam app ID, Heroic `runner:appName`, …).
    pub id: String,
    pub title: String,
    pub launch: Launch,
    /// Local cover image, if the Source has one.
    pub cover: Option<PathBuf>,
    pub metadata: Metadata,
    pub identity: Identity,
}

impl Entry {
    /// The command line, for Entries started by one.
    pub fn exec(&self) -> Option<&str> {
        match &self.launch {
            Launch::Command { exec, .. } => Some(exec),
            Launch::File { .. } => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Launch {
    /// A PC game started by a command line in `.desktop` `Exec` syntax.
    Command {
        exec: String,
        working_dir: Option<PathBuf>,
    },
    /// A game file for an emulator, named `file_name`, for a Platform such as `n64`.
    File {
        platform: String,
        file_name: String,
        location: FileLocation,
    },
}

/// Where the game file of a `Launch::File` Entry lives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileLocation {
    /// On a RomM server, fetched when the game is started.
    RomM { rom_id: u64, layout: Layout },
    /// In a local folder.
    Local { path: PathBuf },
    /// A game of several files in their own folder inside a local folder.
    LocalFolder { path: PathBuf, start: Start },
}

/// How the files of a game fetched from RomM are laid out where a frontend looks for games.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    /// One file.
    File,
    /// A folder named like the file in it that is started, next to the game's other files.
    StartInFolder,
    /// A folder that is started as a whole, such as a ScummVM `.svm` folder.
    Folder,
}

/// Descriptive data for frontends; every field is optional.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Metadata {
    pub description: Option<String>,
    /// First release, as `YYYY-MM-DD`.
    pub release_date: Option<String>,
    pub developer: Option<String>,
    pub publisher: Option<String>,
    pub genre: Option<String>,
    pub players: Option<String>,
    /// Between 0.0 and 1.0.
    pub rating: Option<f32>,
}

/// What identifies the game itself across Sources, used to find the Original of a Port.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Identity {
    pub igdb_id: Option<u64>,
    pub regions: Vec<String>,
    /// Revision of the release, e.g. `A` for "Rev A"; `None` for the first release.
    pub revision: Option<String>,
    /// Hashes of a ROM in a local folder, for Sources without an IGDB id.
    pub rom: Option<RomHashes>,
}

/// Hashes of a ROM in the form Ports check it (see [`crate::n64`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RomHashes {
    pub xxh3: u64,
    /// Lower-case hex.
    pub sha1: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceKind {
    Steam,
    Heroic,
    Manual,
    RomM,
    Local,
    /// Ports matched against Originals in the Library.
    Port,
}
