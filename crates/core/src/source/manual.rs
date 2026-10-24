//! Manual PC installs: `.desktop` files the user listed in the configuration.

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, anyhow};

use crate::desktop;
use crate::library::{Entry, Launch, SourceKind};

pub fn entries(files: &[PathBuf]) -> anyhow::Result<Vec<Entry>> {
    files
        .iter()
        .map(|file| {
            let text =
                fs::read_to_string(file).with_context(|| format!("reading {}", file.display()))?;
            let keys = desktop::main_group(&text);
            let get = |key| keys.iter().find(|(k, _)| *k == key).map(|(_, v)| *v);
            let missing = |key| anyhow!("{} has no {key}=", file.display());
            Ok(Entry {
                source: SourceKind::Manual,
                id: file
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned(),
                title: get("Name").ok_or_else(|| missing("Name"))?.to_owned(),
                launch: Launch::Command {
                    exec: get("Exec").ok_or_else(|| missing("Exec"))?.to_owned(),
                    working_dir: get("Path").map(PathBuf::from),
                },
                cover: None,
                metadata: Default::default(),
            })
        })
        .collect()
}
