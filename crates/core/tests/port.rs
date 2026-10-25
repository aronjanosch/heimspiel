mod common;

use std::path::PathBuf;

use heimspiel_core::catalog::CatalogPort;
use heimspiel_core::dump;
use heimspiel_core::library::{
    Entry, FileLocation, Identity, Launch, Layout, Library, Metadata, SourceKind,
};
use heimspiel_core::port::{self, Manifest, OriginalLocation, Provide};

const MANIFEST: &str = r#"
id = "zelda64recomp"
title = "Zelda 64: Recompiled"

[quiver]
name = "Zelda64Recomp"
repository = "Zelda64Recomp/Zelda64Recomp"
asset-filter = "Linux-X64.zip"

[original]
platform = "n64"
igdb-id = 1030
releases = [{ region = "USA" }]

[provide]
kind = "librecomp"
config-dir = "Zelda64Recompiled"
game-id = "mm.n64.us.1.0"
xxh3 = "0xEF18B4A9E2386169"
"#;

fn romm(rom_id: u64, igdb_id: u64, region: &str, revision: Option<&str>) -> Entry {
    Entry {
        source: SourceKind::RomM,
        id: rom_id.to_string(),
        title: "The Legend of Zelda: Majora's Mask".into(),
        launch: Launch::File {
            platform: "n64".into(),
            file_name: "MM.n64".into(),
            location: FileLocation::RomM {
                rom_id,
                layout: Layout::File,
            },
        },
        cover: Some(PathBuf::from("/cache/covers/3037.png")),
        metadata: Metadata::default(),
        identity: Identity {
            igdb_id: Some(igdb_id),
            regions: vec![region.into()],
            revision: revision.map(Into::into),
        },
    }
}

fn heimspiel() -> Vec<String> {
    vec!["/usr/bin/heimspiel".into()]
}

#[test]
fn offers_a_port_whose_original_is_in_the_library() {
    let manifest = Manifest::parse(MANIFEST).unwrap();
    let library = Library {
        entries: vec![romm(3037, 1030, "USA", None)],
    };

    let ports = port::entries(&[manifest], &library, &[], &heimspiel());

    assert_eq!(ports.len(), 1);
    let p = &ports[0];
    assert_eq!(p.source, SourceKind::Port);
    assert_eq!(p.id, "zelda64recomp");
    assert_eq!(p.title, "Zelda 64: Recompiled");
    assert_eq!(
        p.exec(),
        Some("/usr/bin/heimspiel launch zelda64recomp --romm 3037")
    );
    assert_eq!(p.cover, Some(PathBuf::from("/cache/covers/3037.png")));
}

#[test]
fn needs_the_exact_release_of_the_original() {
    let manifest = Manifest::parse(MANIFEST).unwrap();
    let library = Library {
        entries: vec![
            romm(1, 1030, "Europe", None),
            romm(2, 1030, "USA", Some("A")),
            romm(3, 1029, "USA", None),
        ],
    };

    assert!(port::entries(&[manifest], &library, &[], &heimspiel()).is_empty());
}

#[test]
fn ships_a_manifest_for_zelda64recomp() {
    let builtin = port::builtin();

    let zelda = builtin.iter().find(|m| m.id == "zelda64recomp").unwrap();
    assert_eq!(
        zelda.provide,
        Provide::Librecomp {
            config_dir: "Zelda64Recompiled".into(),
            game_id: "mm.n64.us.1.0".into(),
            xxh3: 0xEF18B4A9E2386169,
        }
    );
}

#[test]
fn built_in_manifests_have_distinct_ids() {
    let builtin = port::builtin();
    let mut ids: Vec<_> = builtin.iter().map(|m| m.id.as_str()).collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), builtin.len());
    assert!(ids.contains(&"banjorecomp"));
}

const COLLECTION: &str = r#"
id = "testcollection"
title = "Test Collection"

[quiver]
name = "TestCollection"
repository = "test/collection"
asset-filter = "windows.zip"
runner = "proton"
executable = "Play.exe"

[original]
platform = "ps4"
title-ids = ["CUSA02320", "CUSA02343"]
files = ["eboot.bin", "u1data"]

[[games]]
id = "u1"
title = "Test: First"

[[games]]
id = "u2"
title = "Test: Second"

[provide]
kind = "dump"
settings = "test-launcher/settings.json"
game-dir-key = "game_dir"
game-key = "start"
set = { check_updates = false }
"#;

#[test]
fn offers_each_game_of_a_port_whose_dump_is_there() {
    let tmp = tempfile::tempdir().unwrap();
    let path = common::ps4_dump(tmp.path(), "CUSA02320", &["eboot.bin", "u1data"]);
    let dumps = dump::dumps(&[tmp.path().to_path_buf()]).unwrap();
    let manifest = Manifest::parse(COLLECTION).unwrap();

    let ports = port::entries(&[manifest], &Library::default(), &dumps, &heimspiel());

    let titles: Vec<_> = ports.iter().map(|p| p.title.as_str()).collect();
    assert_eq!(titles, ["Test: First", "Test: Second"]);
    assert_eq!(ports[0].id, "testcollection/u1");
    assert_eq!(
        ports[0].exec().unwrap(),
        format!(
            "/usr/bin/heimspiel launch testcollection --dump {} --game u1",
            path.display()
        )
    );
}

#[test]
fn needs_a_complete_dump_of_an_accepted_title() {
    let tmp = tempfile::tempdir().unwrap();
    common::ps4_dump(tmp.path(), "CUSA02343", &["eboot.bin"]);
    common::ps4_dump(tmp.path(), "CUSA01623", &["eboot.bin", "u1data"]);
    let dumps = dump::dumps(&[tmp.path().to_path_buf()]).unwrap();
    let manifest = Manifest::parse(COLLECTION).unwrap();

    assert!(port::entries(&[manifest], &Library::default(), &dumps, &heimspiel()).is_empty());
}

fn catalog_port(title: &str, key: &str, platform: &str, repository: &str) -> CatalogPort {
    CatalogPort {
        title: title.into(),
        game_key: key.into(),
        platforms: vec![platform.into()],
        kind: "decompilation".into(),
        repository: repository.into(),
        release: Some("1.0".into()),
    }
}

#[test]
fn gives_an_overview_of_the_ports_and_what_they_need() {
    let zelda = Manifest::parse(MANIFEST).unwrap();
    let collection = Manifest::parse(COLLECTION).unwrap();
    let mut owned = romm(3037, 1030, "USA", None);
    owned.title = "The Legend of Zelda: Majora's Mask".into();
    let library = Library {
        entries: vec![owned],
    };
    let catalog = [
        // The Port heimspiel already knows.
        catalog_port(
            "MM (Zelda64Recomp)",
            "thelegendofzeldamajorasmask",
            "n64",
            "Zelda64Recomp/Zelda64Recomp",
        ),
        catalog_port(
            "MM (2 Ship 2 Harkinian)",
            "thelegendofzeldamajorasmask",
            "n64",
            "2ship2harkinian/2ship2harkinian",
        ),
        // Same title, other Platform.
        catalog_port(
            "MM 3D (Restoration)",
            "thelegendofzeldamajorasmask",
            "3ds",
            "leoetlino/project-restoration",
        ),
        catalog_port(
            "Banjo-Kazooie",
            "banjokazooie",
            "n64",
            "BanjoRecomp/BanjoRecomp",
        ),
    ];
    let manifests = [zelda, collection];

    let overview = port::overview(&manifests, &library, &[], &catalog, &|app| {
        app.name == "Zelda64Recomp"
    });

    let states: Vec<_> = overview
        .manifests
        .iter()
        .map(|s| (s.manifest.id.as_str(), s.original.as_deref(), s.installed))
        .collect();
    assert_eq!(
        states,
        [
            (
                "zelda64recomp",
                Some("The Legend of Zelda: Majora's Mask (RomM)"),
                true
            ),
            ("testcollection", None, false),
        ]
    );
    let more: Vec<_> = overview
        .more
        .iter()
        .map(|m| (m.port.repository.as_str(), m.original.as_str()))
        .collect();
    assert_eq!(
        more,
        [(
            "2ship2harkinian/2ship2harkinian",
            "The Legend of Zelda: Majora's Mask"
        )]
    );
}

#[test]
fn finds_catalog_ports_for_dumps_too() {
    let tmp = tempfile::tempdir().unwrap();
    common::ps4_dump(tmp.path(), "CUSA02320", &["eboot.bin", "u1data"]);
    let dumps = dump::dumps(&[tmp.path().to_path_buf()]).unwrap();
    let catalog = [catalog_port(
        "Test Collection PC",
        "testcollection",
        "ps4",
        "other/port",
    )];
    let manifests = [Manifest::parse(COLLECTION).unwrap()];

    let overview = port::overview(&manifests, &Library::default(), &dumps, &catalog, &|_| {
        false
    });

    assert_eq!(
        overview.manifests[0].original.as_deref(),
        Some("Test Collection™ (Dump CUSA02320)")
    );
    assert_eq!(overview.more[0].original, "Test Collection™");
}

#[test]
fn finds_where_a_ports_original_is() {
    let zelda = Manifest::parse(MANIFEST).unwrap();
    let collection = Manifest::parse(COLLECTION).unwrap();
    let library = Library {
        entries: vec![romm(3037, 1030, "USA", None)],
    };
    let tmp = tempfile::tempdir().unwrap();
    let path = common::ps4_dump(tmp.path(), "CUSA02320", &["eboot.bin", "u1data"]);
    let dumps = dump::dumps(&[tmp.path().to_path_buf()]).unwrap();

    assert_eq!(
        port::original(&zelda, &library, &dumps),
        Some(OriginalLocation::RomM(3037))
    );
    assert_eq!(
        port::original(&collection, &library, &dumps),
        Some(OriginalLocation::Dump(path))
    );
    assert_eq!(port::original(&collection, &library, &[]), None);
}
