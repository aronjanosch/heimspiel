//! Provisioning: putting an Original where its Port expects it, in the form it expects, so the
//! Port starts without asking for anything.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use xxhash_rust::xxh3::xxh3_64;

use crate::port::Provide;

/// Provisions the Original for `spec`. `config_home` stands for `~/.config`. `fetch_original` is
/// only called when the Port does not have a valid copy yet.
pub fn provide(
    spec: &Provide,
    config_home: &Path,
    fetch_original: impl FnOnce() -> anyhow::Result<PathBuf>,
) -> anyhow::Result<()> {
    match spec {
        Provide::Librecomp {
            config_dir,
            game_id,
            xxh3,
        } => {
            let target = config_home.join(config_dir).join(format!("{game_id}.z64"));
            if fs::read(&target).is_ok_and(|bytes| xxh3_64(&bytes) == *xxh3) {
                return Ok(());
            }
            let original = fetch_original()?;
            let mut rom =
                fs::read(&original).with_context(|| format!("reading {}", original.display()))?;
            to_big_endian(&mut rom).with_context(|| format!("reading {}", original.display()))?;
            rom.resize(rom.len().next_multiple_of(4), 0);
            // librecomp deletes a stored ROM with the wrong hash, so only a verified copy goes in.
            let actual = xxh3_64(&rom);
            if actual != *xxh3 {
                bail!(
                    "{} does not match the release this Port needs (XXH3 {actual:016x}, expected {xxh3:016x})",
                    original.display()
                );
            }
            let dir = target.parent().unwrap();
            fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
            let tmp = target.with_extension("part");
            fs::write(&tmp, &rom).with_context(|| format!("writing {}", tmp.display()))?;
            fs::rename(&tmp, &target).with_context(|| format!("replacing {}", target.display()))
        }
    }
}

/// Brings an N64 ROM into big-endian (z64) order, detected from its header rather than its name:
/// RomM serves byte-swapped (v64) files named `.n64`.
fn to_big_endian(rom: &mut [u8]) -> anyhow::Result<()> {
    match rom.get(..4) {
        Some([0x80, 0x37, 0x12, 0x40]) => {}
        Some([0x37, 0x80, 0x40, 0x12]) => rom
            .as_chunks_mut::<2>()
            .0
            .iter_mut()
            .for_each(|pair| pair.swap(0, 1)),
        Some([0x40, 0x12, 0x37, 0x80]) => rom
            .as_chunks_mut::<4>()
            .0
            .iter_mut()
            .for_each(|word| word.reverse()),
        _ => bail!("not an N64 ROM"),
    }
    Ok(())
}
