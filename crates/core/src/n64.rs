//! N64 ROMs in the form Ports check them: big-endian (z64) and padded to whole words.

use std::fs;
use std::path::Path;

use anyhow::{Context, bail};
use sha1::{Digest, Sha1};
use xxhash_rust::xxh3::xxh3_64;

use crate::library::RomHashes;

/// The largest N64 ROM; bigger files are not read.
pub const MAX_SIZE: u64 = 64 << 20;

/// Reads the ROM at `path` and brings it into the form Ports check.
pub fn read(path: &Path) -> anyhow::Result<Vec<u8>> {
    let mut rom = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    to_big_endian(&mut rom).with_context(|| format!("reading {}", path.display()))?;
    rom.resize(rom.len().next_multiple_of(4), 0);
    Ok(rom)
}

pub fn hashes(rom: &[u8]) -> RomHashes {
    RomHashes {
        xxh3: xxh3_64(rom),
        sha1: format!("{:x}", Sha1::digest(rom)),
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
