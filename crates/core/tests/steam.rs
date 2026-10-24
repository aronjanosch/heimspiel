use std::fs;
use std::path::Path;

use heimspiel_core::library::SourceKind;
use heimspiel_core::source::steam;

fn manifest(library: &Path, appid: &str, name: &str, state_flags: &str) {
    let dir = library.join("steamapps");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join(format!("appmanifest_{appid}.acf")),
        format!(
            "\"AppState\"\n{{\n\t\"appid\"\t\t\"{appid}\"\n\t\"name\"\t\t\"{name}\"\n\t\"StateFlags\"\t\t\"{state_flags}\"\n}}\n"
        ),
    )
    .unwrap();
}

fn library_folders(root: &Path, libraries: &[&Path]) {
    let mut vdf = String::from("\"libraryfolders\"\n{\n");
    for (i, lib) in libraries.iter().enumerate() {
        vdf += &format!(
            "\t\"{i}\"\n\t{{\n\t\t\"path\"\t\t\"{}\"\n\t\t\"apps\"\n\t\t{{\n\t\t}}\n\t}}\n",
            lib.display()
        );
    }
    vdf += "}\n";
    fs::create_dir_all(root.join("steamapps")).unwrap();
    fs::write(root.join("steamapps/libraryfolders.vdf"), vdf).unwrap();
}

#[test]
fn reads_installed_games_from_every_library_folder() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("Steam");
    let second = tmp.path().join("SteamLibrary");
    library_folders(&root, &[&root, &second]);
    manifest(&root, "590380", "Into the Breach", "4");
    manifest(&second, "570", "Dota 2", "4");

    let mut entries = steam::entries(&root, "steam").unwrap();
    entries.sort_by(|a, b| a.id.cmp(&b.id));

    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].source, SourceKind::Steam);
    assert_eq!(entries[0].id, "570");
    assert_eq!(entries[0].title, "Dota 2");
    assert_eq!(entries[0].exec(), Some("steam steam://rungameid/570"));
    assert_eq!(entries[1].title, "Into the Breach");
}

#[test]
fn skips_tools_and_unfinished_installs() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("Steam");
    library_folders(&root, &[&root]);
    manifest(&root, "590380", "Into the Breach", "4");
    manifest(&root, "1493710", "Proton - Experimental", "4");
    manifest(&root, "1628350", "Steam Linux Runtime 3.0 (sniper)", "4");
    manifest(&root, "228980", "Steamworks Common Redistributables", "4");
    manifest(&root, "4183110", "Half Downloaded", "1026");

    let entries = steam::entries(&root, "steam").unwrap();

    let titles: Vec<_> = entries.iter().map(|e| e.title.as_str()).collect();
    assert_eq!(titles, ["Into the Breach"]);
}

#[test]
fn uses_the_cover_from_the_library_cache() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("Steam");
    library_folders(&root, &[&root]);
    manifest(&root, "590380", "Into the Breach", "4");
    manifest(&root, "570", "Dota 2", "4");
    let cover = root.join("appcache/librarycache/590380/library_600x900.jpg");
    fs::create_dir_all(cover.parent().unwrap()).unwrap();
    fs::write(&cover, b"jpg").unwrap();

    let entries = steam::entries(&root, "steam").unwrap();

    let breach = entries.iter().find(|e| e.id == "590380").unwrap();
    let dota = entries.iter().find(|e| e.id == "570").unwrap();
    assert_eq!(breach.cover.as_deref(), Some(cover.as_path()));
    assert_eq!(dota.cover, None);
}
