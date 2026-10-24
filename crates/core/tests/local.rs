use std::fs;

use heimspiel_core::library::{FileLocation, Launch, SourceKind};
use heimspiel_core::source::local;

#[test]
fn reads_games_from_platform_folders() {
    let tmp = tempfile::tempdir().unwrap();
    let game = tmp.path().join("n64/Super Mario 64 (USA).z64");
    fs::create_dir_all(game.parent().unwrap()).unwrap();
    fs::write(&game, b"rom").unwrap();

    let entries = local::entries(&[tmp.path().to_path_buf()]).unwrap();

    assert_eq!(entries.len(), 1);
    let e = &entries[0];
    assert_eq!(e.source, SourceKind::Local);
    assert_eq!(e.title, "Super Mario 64");
    assert_eq!(
        e.launch,
        Launch::File {
            platform: "n64".into(),
            file_name: "Super Mario 64 (USA).z64".into(),
            location: FileLocation::Local { path: game },
        }
    );
}

#[test]
fn skips_saves_hidden_files_and_subfolders() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("snes");
    fs::create_dir_all(dir.join("media")).unwrap();
    for file in ["Zelda (USA).sfc", "Zelda (USA).srm", ".DS_Store"] {
        fs::write(dir.join(file), b"x").unwrap();
    }
    fs::write(tmp.path().join("readme.txt"), b"x").unwrap();

    let entries = local::entries(&[tmp.path().to_path_buf()]).unwrap();

    let titles: Vec<_> = entries.iter().map(|e| e.title.as_str()).collect();
    assert_eq!(titles, ["Zelda"]);
}

#[test]
fn fails_on_a_folder_that_is_missing() {
    let err = local::entries(&["/nonexistent/ROMs".into()]).unwrap_err();
    assert!(format!("{err:#}").contains("/nonexistent/ROMs"));
}
