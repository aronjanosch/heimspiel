use std::fs;
use std::path::Path;

use heimspiel_core::library::{FileLocation, Launch, SourceKind};
use heimspiel_core::n64;
use heimspiel_core::source::local;

#[test]
fn reads_games_from_platform_folders() {
    let tmp = tempfile::tempdir().unwrap();
    let game = tmp.path().join("n64/Super Mario 64 (USA).z64");
    fs::create_dir_all(game.parent().unwrap()).unwrap();
    fs::write(&game, b"rom").unwrap();

    let entries =
        local::entries(&[tmp.path().to_path_buf()], &tmp.path().join("hashes.json")).unwrap();

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

    let entries =
        local::entries(&[tmp.path().to_path_buf()], &tmp.path().join("hashes.json")).unwrap();

    let titles: Vec<_> = entries.iter().map(|e| e.title.as_str()).collect();
    assert_eq!(titles, ["Zelda"]);
}

#[test]
fn fails_on_a_folder_that_is_missing() {
    let err = local::entries(
        &["/nonexistent/ROMs".into()],
        Path::new("/nonexistent/hashes.json"),
    )
    .unwrap_err();
    assert!(format!("{err:#}").contains("/nonexistent/ROMs"));
}

#[test]
fn hashes_n64_roms_in_the_form_ports_check() {
    let tmp = tempfile::tempdir().unwrap();
    let roms = tmp.path().join("roms");
    fs::create_dir_all(roms.join("n64")).unwrap();
    fs::create_dir_all(roms.join("snes")).unwrap();
    // Byte-swapped (v64), as RomM serves it, and not padded to whole words.
    fs::write(
        roms.join("n64/Test (USA).n64"),
        [0x37, 0x80, 0x40, 0x12, 2, 1],
    )
    .unwrap();
    fs::write(roms.join("n64/notes.z64"), b"no rom").unwrap();
    fs::write(roms.join("snes/Test (USA).sfc"), [0x80, 0x37, 0x12, 0x40]).unwrap();
    let cache = tmp.path().join("cache/hashes.json");

    let entries = local::entries(std::slice::from_ref(&roms), &cache).unwrap();

    let expected = n64::hashes(&[0x80, 0x37, 0x12, 0x40, 1, 2, 0, 0]);
    let rom_of = |name: &str| {
        let entry = entries.iter().find(|e| e.id.ends_with(name)).unwrap();
        entry.identity.rom.clone()
    };
    assert_eq!(rom_of("Test (USA).n64"), Some(expected.clone()));
    assert_eq!(rom_of("notes.z64"), None);
    assert_eq!(rom_of("Test (USA).sfc"), None);
    assert!(cache.is_file());

    // The second run takes the hashes from the cache, even with the file unreadable.
    let rom = roms.join("n64/Test (USA).n64");
    let mode = |m| fs::set_permissions(&rom, std::os::unix::fs::PermissionsExt::from_mode(m));
    mode(0o000).unwrap();
    let again = local::entries(std::slice::from_ref(&roms), &cache);
    mode(0o644).unwrap();
    let again = again.unwrap();
    let entry = again
        .iter()
        .find(|e| e.id.ends_with("Test (USA).n64"))
        .unwrap();
    assert_eq!(entry.identity.rom, Some(expected));
}
