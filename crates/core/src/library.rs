//! The Library: every Entry heimspiel knows about, across all Sources.

use std::path::PathBuf;

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
    RomM { rom_id: u64 },
    /// In a local folder.
    Local { path: PathBuf },
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceKind {
    Steam,
    Heroic,
    Manual,
    RomM,
    Local,
}
