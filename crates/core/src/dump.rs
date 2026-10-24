//! Dumps: extracted, decrypted folders of console games the user made on their own console
//! (ADR 0007). A PS4 Dump is recognised by its `sce_sys/param.sfo`.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, bail};

/// One Dump, as its `param.sfo` describes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dump {
    pub platform: String,
    /// `CUSA02320`
    pub title_id: String,
    pub title: String,
    /// The game's patch level, `01.00` for the first release.
    pub version: String,
    pub path: PathBuf,
}

/// The Dumps directly inside `folders`; anything else there, such as PKG files or incomplete
/// extractions, is skipped.
pub fn dumps(folders: &[PathBuf]) -> anyhow::Result<Vec<Dump>> {
    let mut dumps = Vec::new();
    for folder in folders {
        for dir in fs::read_dir(folder).with_context(|| format!("reading {}", folder.display()))? {
            let dir = dir?;
            if dir.file_type()?.is_dir()
                && let Ok(dump) = read(&dir.path())
            {
                dumps.push(dump);
            }
        }
    }
    dumps.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(dumps)
}

/// Reads the Dump at `path`.
pub fn read(path: &Path) -> anyhow::Result<Dump> {
    let sfo_path = path.join("sce_sys/param.sfo");
    let bytes =
        fs::read(&sfo_path).with_context(|| format!("{} is no PS4 Dump", path.display()))?;
    let mut values =
        param_sfo(&bytes).with_context(|| format!("reading {}", sfo_path.display()))?;
    let mut take = |key: &str| {
        values
            .remove(key)
            .with_context(|| format!("{} has no {key}", sfo_path.display()))
    };
    Ok(Dump {
        platform: "ps4".into(),
        title_id: take("TITLE_ID")?,
        title: take("TITLE")?,
        version: take("APP_VER")?,
        path: path.to_path_buf(),
    })
}

/// The text values of a `param.sfo`: a header, an index of 16-byte entries, a key table and a
/// data table, all little-endian.
fn param_sfo(bytes: &[u8]) -> anyhow::Result<HashMap<String, String>> {
    let u16_at = |at: usize| -> anyhow::Result<usize> {
        let b = bytes.get(at..at + 2).context("truncated")?;
        Ok(u16::from_le_bytes([b[0], b[1]]) as usize)
    };
    let u32_at = |at: usize| -> anyhow::Result<usize> {
        let b = bytes.get(at..at + 4).context("truncated")?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize)
    };
    if !bytes.starts_with(b"\0PSF") {
        bail!("not a param.sfo");
    }
    let (key_table, data_table, count) = (u32_at(8)?, u32_at(12)?, u32_at(16)?);
    let mut values = HashMap::new();
    for i in 0..count {
        let entry = 20 + 16 * i;
        // 0x0204 is UTF-8 text with a terminating zero; numbers (0x0404) are not needed.
        if u16_at(entry + 2)? != 0x0204 {
            continue;
        }
        let key_start = key_table + u16_at(entry)?;
        let key = bytes
            .get(key_start..)
            .and_then(|rest| rest.split(|b| *b == 0).next())
            .context("truncated")?;
        let data_start = data_table + u32_at(entry + 12)?;
        let data = bytes
            .get(data_start..data_start + u32_at(entry + 4)?)
            .context("truncated")?;
        let text = data.split(|b| *b == 0).next().unwrap_or_default();
        values.insert(
            String::from_utf8_lossy(key).into_owned(),
            String::from_utf8_lossy(text).into_owned(),
        );
    }
    Ok(values)
}
