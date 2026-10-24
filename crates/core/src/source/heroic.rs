//! Heroic: installed Epic (legendary), GOG and Amazon (nile) games.

use std::fs;
use std::path::Path;

use anyhow::Context;
use serde_json::Value;

use crate::desktop;
use crate::library::{Entry, SourceKind};

/// Reads the installed games from Heroic's config folder (`~/.config/heroic` or its Flatpak twin).
/// `command` starts Heroic itself (AppImage path, `heroic`, or `flatpak run …`).
pub fn entries(config: &Path, command: &[String]) -> anyhow::Result<Vec<Entry>> {
    let mut entries = Vec::new();
    for runner in RUNNERS {
        let titles = read_json(&config.join(runner.library))?;
        let Some(installed) = read_json(&config.join(runner.installed))? else {
            continue;
        };
        for (app_name, game) in (runner.games)(&installed) {
            if game["is_dlc"].as_bool() == Some(true) {
                continue;
            }
            let title = game["title"]
                .as_str()
                .or_else(|| titles.as_ref().and_then(|t| title(t, app_name)))
                .unwrap_or(app_name);
            let url = format!("heroic://launch?appName={app_name}&runner={}", runner.name);
            let mut args = command.to_vec();
            args.extend(["--no-gui".to_owned(), url]);
            entries.push(Entry {
                source: SourceKind::Heroic,
                id: format!("{}:{app_name}", runner.name),
                title: title.to_owned(),
                exec: desktop::exec(&args),
                working_dir: None,
                cover: None,
            });
        }
    }
    Ok(entries)
}

struct Runner {
    name: &'static str,
    installed: &'static str,
    library: &'static str,
    /// Yields `(appName, game)` pairs from the runner's `installed.json`.
    games: fn(&Value) -> Vec<(&str, &Value)>,
}

const RUNNERS: [Runner; 3] = [
    Runner {
        name: "legendary",
        installed: "legendaryConfig/legendary/installed.json",
        library: "store_cache/legendary_library.json",
        games: |v| {
            v.as_object()
                .into_iter()
                .flatten()
                .map(|(k, g)| (k.as_str(), g))
                .collect()
        },
    },
    Runner {
        name: "gog",
        installed: "gog_store/installed.json",
        library: "store_cache/gog_library.json",
        games: |v| keyed_list(&v["installed"], "appName"),
    },
    Runner {
        name: "nile",
        installed: "nile_config/nile/installed.json",
        library: "store_cache/nile_library.json",
        games: |v| keyed_list(v, "id"),
    },
];

fn keyed_list<'a>(list: &'a Value, key: &str) -> Vec<(&'a str, &'a Value)> {
    list.as_array()
        .into_iter()
        .flatten()
        .filter_map(|g| Some((g[key].as_str()?, g)))
        .collect()
}

/// Looks up a title in a store cache (`games` for GOG, `library` for the others).
fn title<'a>(cache: &'a Value, app_name: &str) -> Option<&'a str> {
    let list = cache.get("games").or_else(|| cache.get("library"))?;
    list.as_array()?
        .iter()
        .find(|g| g["app_name"].as_str() == Some(app_name))?["title"]
        .as_str()
}

fn read_json(path: &Path) -> anyhow::Result<Option<Value>> {
    let Ok(text) = fs::read_to_string(path) else {
        return Ok(None);
    };
    let value =
        serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
    Ok(Some(value))
}
