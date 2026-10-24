use std::fs;
use std::path::Path;

use heimspiel_core::library::SourceKind;
use heimspiel_core::source::heroic;

fn write(config: &Path, file: &str, json: &str) {
    let path = config.join(file);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, json).unwrap();
}

fn command() -> Vec<String> {
    vec!["/opt/Heroic Games/Heroic.AppImage".into()]
}

#[test]
fn reads_installed_games_of_all_three_runners() {
    let tmp = tempfile::tempdir().unwrap();
    let config = tmp.path();
    write(
        config,
        "gog_store/installed.json",
        r#"{"installed": [{"appName": "1584866499", "is_dlc": false, "platform": "windows"}]}"#,
    );
    write(
        config,
        "store_cache/gog_library.json",
        r#"{"games": [{"app_name": "1584866499", "title": "Beat Cop"}, {"app_name": "1", "title": "Not Installed"}]}"#,
    );
    write(
        config,
        "legendaryConfig/legendary/installed.json",
        r#"{"Fortnite": {"app_name": "Fortnite", "title": "Fortnite", "is_dlc": false}}"#,
    );
    write(
        config,
        "nile_config/nile/installed.json",
        r#"[{"id": "amzn1.adg.product.42", "path": "/games/x"}]"#,
    );
    write(
        config,
        "store_cache/nile_library.json",
        r#"{"library": [{"app_name": "amzn1.adg.product.42", "title": "Amazon Game"}]}"#,
    );

    let mut entries = heroic::entries(config, &command()).unwrap();
    entries.sort_by(|a, b| a.title.cmp(&b.title));

    let got: Vec<_> = entries
        .iter()
        .map(|e| (e.id.as_str(), e.title.as_str()))
        .collect();
    assert_eq!(
        got,
        [
            ("nile:amzn1.adg.product.42", "Amazon Game"),
            ("gog:1584866499", "Beat Cop"),
            ("legendary:Fortnite", "Fortnite"),
        ]
    );
    assert!(entries.iter().all(|e| e.source == SourceKind::Heroic));
}

#[test]
fn launches_through_heroic_without_its_window() {
    let tmp = tempfile::tempdir().unwrap();
    write(
        tmp.path(),
        "gog_store/installed.json",
        r#"{"installed": [{"appName": "1584866499", "is_dlc": false}]}"#,
    );

    let entries = heroic::entries(tmp.path(), &command()).unwrap();

    assert_eq!(
        entries[0].exec(),
        Some(
            r#""/opt/Heroic Games/Heroic.AppImage" --no-gui "heroic://launch?appName=1584866499&runner=gog""#
        )
    );
}

#[test]
fn skips_dlc_and_falls_back_to_the_app_name_without_a_title() {
    let tmp = tempfile::tempdir().unwrap();
    write(
        tmp.path(),
        "gog_store/installed.json",
        r#"{"installed": [{"appName": "1", "is_dlc": false}, {"appName": "2", "is_dlc": true}]}"#,
    );

    let entries = heroic::entries(tmp.path(), &command()).unwrap();

    let titles: Vec<_> = entries.iter().map(|e| e.title.as_str()).collect();
    assert_eq!(titles, ["1"]);
}

#[test]
fn has_no_entries_without_heroic() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(
        heroic::entries(&tmp.path().join("missing"), &command())
            .unwrap()
            .is_empty()
    );
}
