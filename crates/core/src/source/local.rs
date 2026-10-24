//! Local folders laid out like RomM's library: `<folder>/<platform>/<game file>`, with RomM's
//! platform slugs as folder names.

use std::fs;
use std::path::PathBuf;

use anyhow::Context;

use crate::library::{Entry, FileLocation, Launch, Metadata, SourceKind};
use crate::source::romm::is_game;

pub fn entries(folders: &[PathBuf]) -> anyhow::Result<Vec<Entry>> {
    let mut entries = Vec::new();
    for folder in folders {
        let platforms =
            fs::read_dir(folder).with_context(|| format!("reading {}", folder.display()))?;
        for platform in platforms {
            let platform = platform?;
            if !platform.file_type()?.is_dir() {
                continue;
            }
            let slug = platform.file_name().to_string_lossy().into_owned();
            for file in fs::read_dir(platform.path())? {
                let file = file?;
                let name = file.file_name().to_string_lossy().into_owned();
                if name.starts_with('.') || !file.file_type()?.is_file() || !is_game(&name) {
                    continue;
                }
                entries.push(Entry {
                    source: SourceKind::Local,
                    id: file.path().display().to_string(),
                    title: title(&name),
                    launch: Launch::File {
                        platform: slug.clone(),
                        file_name: name,
                        location: FileLocation::Local { path: file.path() },
                    },
                    cover: None,
                    metadata: Metadata::default(),
                });
            }
        }
    }
    entries.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(entries)
}

/// The file name without extension and No-Intro tags: `Zelda (USA) [!].sfc` → `Zelda`.
fn title(file_name: &str) -> String {
    let stem = file_name
        .rsplit_once('.')
        .map_or(file_name, |(stem, _)| stem);
    let end = [stem.find(" ("), stem.find(" [")]
        .into_iter()
        .flatten()
        .min()
        .unwrap_or(stem.len());
    stem[..end].to_owned()
}
