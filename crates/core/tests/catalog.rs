mod common;

use std::time::Duration;

use common::{FakeServer, Response};
use heimspiel_core::catalog::{self, CatalogPort};

const GAMES: &str = r#"{
  "generated_at": "2026-10-24T08:00:00Z",
  "games": [
    {"title": "The Legend of Zelda: Majora's Mask (Zelda64Recomp)", "game_key": "thelegendofzeldamajorasmask",
     "platform": "N64", "type": "static-recomp", "repo": "Zelda64Recomp/Zelda64Recomp",
     "release": "v1.2.2", "archived": false},
    {"title": "The Legend of Zelda: Majora's Mask (2 Ship 2 Harkinian)", "game_key": "thelegendofzeldamajorasmask",
     "platform": "N64 / GameCube", "type": "decompilation", "repo": "2ship2harkinian/2ship2harkinian",
     "release": "5.0.1", "archived": false},
    {"title": "The Legend of Zelda: Majora's Mask", "game_key": "thelegendofzeldamajorasmask",
     "platform": "N64", "type": "decompilation", "repo": "zeldaret/mm", "release": null, "archived": false},
    {"title": "Super Mario 64 (browser)", "game_key": "supermario64", "platform": "N64",
     "type": "browser-port", "repo": "x/sm64-web", "release": null, "archived": false},
    {"title": "Pokémon Red", "game_key": "pokemonred", "platform": "Game Boy",
     "type": "disassembly", "repo": "pret/pokered", "release": null, "archived": false},
    {"title": "Website Only", "game_key": "websiteonly", "platform": "PC", "type": "source-port",
     "repo": null, "release": null, "archived": false},
    {"title": "Some Tool", "game_key": "sometool", "platform": "", "type": "tool",
     "repo": "x/tool", "release": "1.0", "archived": false}
  ],
  "mods": []
}"#;

#[test]
fn reads_the_ports_of_awesome_decomp_games() {
    let ports = catalog::parse(GAMES).unwrap();

    let repos: Vec<_> = ports.iter().map(|p| p.repository.as_str()).collect();
    // Browser ports, disassemblies and tools are no Ports to install.
    assert_eq!(
        repos,
        [
            "Zelda64Recomp/Zelda64Recomp",
            "2ship2harkinian/2ship2harkinian",
            "zeldaret/mm"
        ]
    );
    assert_eq!(
        ports[1],
        CatalogPort {
            title: "The Legend of Zelda: Majora's Mask (2 Ship 2 Harkinian)".into(),
            game_key: "thelegendofzeldamajorasmask".into(),
            platforms: vec!["n64".into(), "ngc".into()],
            kind: "decompilation".into(),
            repository: "2ship2harkinian/2ship2harkinian".into(),
            release: Some("5.0.1".into()),
        }
    );
    assert_eq!(ports[2].release, None);
}

#[test]
fn keys_titles_like_the_catalog() {
    assert_eq!(
        catalog::game_key("The Legend of Zelda: Majora's Mask"),
        "thelegendofzeldamajorasmask"
    );
    assert_eq!(
        catalog::game_key("Uncharted: The Nathan Drake Collection™"),
        "unchartedthenathandrakecollection"
    );
}

#[test]
fn keeps_the_catalog_and_falls_back_to_it_when_the_download_fails() {
    let server = FakeServer::start(|_| Response::json(GAMES));
    let tmp = tempfile::tempdir().unwrap();
    let cache = tmp.path().join("catalog/games.json");
    let url = format!("{}/games.json", server.url);

    assert_eq!(
        catalog::load(&url, &cache, Duration::from_secs(3600))
            .unwrap()
            .len(),
        3
    );
    catalog::load(&url, &cache, Duration::from_secs(3600)).unwrap();
    assert_eq!(server.requests().len(), 1);

    let down = FakeServer::start(|_| Response::status(503));
    let url = format!("{}/games.json", down.url);
    let ports = catalog::load(&url, &cache, Duration::ZERO).unwrap();
    assert_eq!(ports.len(), 3);
}
