use std::path::PathBuf;

use heimspiel_core::library::{
    Entry, FileLocation, Identity, Launch, Library, Metadata, SourceKind,
};
use heimspiel_core::port::{self, Manifest, Provide};

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
region = "USA"

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
            location: FileLocation::RomM { rom_id },
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

    let ports = port::entries(&[manifest], &library, &heimspiel());

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

    assert!(port::entries(&[manifest], &library, &heimspiel()).is_empty());
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
