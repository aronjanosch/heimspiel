use std::fs;
use std::path::PathBuf;

use heimspiel_core::library::{Launch, SourceKind};
use heimspiel_core::source::manual;

#[test]
fn reads_name_command_and_working_dir_from_listed_desktop_files() {
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("7th Heaven.desktop");
    fs::write(
        &file,
        "[Desktop Entry]\nType=Application\nName=7th Heaven\nName[de]=Siebter Himmel\n\
         Exec=\"/apps/7th Heaven\" --launch\nPath=/apps\n\n\
         [Desktop Action Settings]\nName=Settings\nExec=/apps/settings\n",
    )
    .unwrap();

    let entries = manual::entries(&[file]).unwrap();

    assert_eq!(entries.len(), 1);
    let e = &entries[0];
    assert_eq!(e.source, SourceKind::Manual);
    assert_eq!(e.id, "7th Heaven");
    assert_eq!(e.title, "7th Heaven");
    assert_eq!(
        e.launch,
        Launch::Command {
            exec: "\"/apps/7th Heaven\" --launch".into(),
            working_dir: Some(PathBuf::from("/apps")),
        }
    );
}

#[test]
fn fails_on_a_listed_file_that_is_missing() {
    let missing = PathBuf::from("/nonexistent/Gone.desktop");
    let err = manual::entries(&[missing]).unwrap_err();
    assert!(format!("{err:#}").contains("Gone.desktop"));
}
