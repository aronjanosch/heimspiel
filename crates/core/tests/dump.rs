mod common;

use std::fs;

use heimspiel_core::dump;

#[test]
fn finds_ps4_dumps_by_their_param_sfo() {
    let tmp = tempfile::tempdir().unwrap();
    let path = common::ps4_dump(tmp.path(), "CUSA02320", &["eboot.bin"]);
    // Neither a PKG nor a folder without param.sfo is a Dump.
    fs::write(tmp.path().join("Game CUSA02320 v1.00.pkg"), b"\x7fCNT").unwrap();
    fs::create_dir_all(tmp.path().join("CUSA01623/sce_module")).unwrap();

    let dumps = dump::dumps(&[tmp.path().to_path_buf()]).unwrap();

    assert_eq!(dumps.len(), 1);
    let d = &dumps[0];
    assert_eq!(d.platform, "ps4");
    assert_eq!(d.title_id, "CUSA02320");
    assert_eq!(d.title, "Test Collection™");
    assert_eq!(d.version, "01.00");
    assert_eq!(d.path, path);
}

#[test]
fn reads_one_dump_by_its_path() {
    let tmp = tempfile::tempdir().unwrap();
    let path = common::ps4_dump(tmp.path(), "CUSA02343", &[]);

    assert_eq!(dump::read(&path).unwrap().title_id, "CUSA02343");
    assert!(dump::read(tmp.path()).is_err());
}

#[test]
fn skips_a_damaged_param_sfo() {
    let tmp = tempfile::tempdir().unwrap();
    let path = common::ps4_dump(tmp.path(), "CUSA02320", &[]);
    fs::write(path.join("sce_sys/param.sfo"), b"\0PSF\x01\x01").unwrap();

    assert!(dump::dumps(&[tmp.path().to_path_buf()]).unwrap().is_empty());
}
