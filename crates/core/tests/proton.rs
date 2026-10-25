use std::fs;
use std::path::Path;

use heimspiel_core::proton;

fn install(steam: &Path, dir: &str) {
    fs::create_dir_all(steam.join(dir)).unwrap();
    fs::write(steam.join(dir).join("proton"), "").unwrap();
}

#[test]
fn prefers_proton_experimental_then_the_newest_proton() {
    let tmp = tempfile::tempdir().unwrap();
    let steam = tmp.path();
    install(steam, "steamapps/common/Proton 9.0 (Beta)");
    install(steam, "steamapps/common/Proton 10.0");
    install(steam, "compatibilitytools.d/GE-Proton10-25");

    let found = proton::find(steam).unwrap();
    assert_eq!(found.exe, steam.join("steamapps/common/Proton 10.0/proton"));
    assert_eq!(found.steam_root, steam);

    install(steam, "steamapps/common/Proton - Experimental");
    assert_eq!(
        proton::find(steam).unwrap().exe,
        steam.join("steamapps/common/Proton - Experimental/proton")
    );
}

#[test]
fn falls_back_to_a_custom_proton() {
    let tmp = tempfile::tempdir().unwrap();
    install(tmp.path(), "compatibilitytools.d/GE-Proton10-25");

    assert_eq!(
        proton::find(tmp.path()).unwrap().exe,
        tmp.path()
            .join("compatibilitytools.d/GE-Proton10-25/proton")
    );
    assert!(proton::find(&tmp.path().join("none")).is_none());
}
