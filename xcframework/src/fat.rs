//! Writes a Mach-O universal ("fat") file from thin static libraries, as `lipo -create` does.
//!
//! The layout follows `<mach-o/fat.h>`: a big-endian `fat_header`, one `fat_arch` per slice, then
//! each slice aligned to 8 bytes. For static libraries `lipo` uses that 8-byte alignment and orders
//! slices by CPU type; the output here is byte-for-byte what `lipo` produces.

use crate::xcframework::Arch;
use anyhow::{Context, Result};
use std::io::{Read, Write};

const FAT_MAGIC: u32 = 0xcafe_babe;
const FAT_HEADER_SIZE: u64 = 8;
const FAT_ARCH_SIZE: u64 = 20;
const ALIGN_POWER: u32 = 3;
const ARCHIVE_MAGIC: &[u8; 8] = b"!<arch>\n";

/// A thin static library to put in the fat file. Inputs are streamed, since debug builds of
/// `libwp_mobile.a` are hundreds of megabytes each.
pub struct Input {
    pub arch: Arch,
    pub len: u64,
    pub reader: Box<dyn Read>,
}

pub fn write(out: &mut impl Write, mut inputs: Vec<Input>) -> Result<()> {
    inputs.sort_by_key(|input| input.arch);
    if let Some(pair) = inputs.windows(2).find(|pair| pair[0].arch == pair[1].arch) {
        anyhow::bail!("More than one {} slice", pair[0].arch);
    }

    let align = 1u64 << ALIGN_POWER;
    let mut offset = FAT_HEADER_SIZE + FAT_ARCH_SIZE * inputs.len() as u64;
    let mut offsets = Vec::new();
    for input in &inputs {
        offset = offset.next_multiple_of(align);
        offsets.push(offset);
        offset += input.len;
    }
    // A 32-bit fat header can't address past 4GiB; `lipo` would need `-fat64`.
    if offset > u64::from(u32::MAX) {
        anyhow::bail!("Fat library would be {offset} bytes, too large for a 32-bit fat header");
    }
    // Checked above: every offset and length fits in a u32.
    let be = |v: u64| (v as u32).to_be_bytes();

    out.write_all(&FAT_MAGIC.to_be_bytes())?;
    out.write_all(&be(inputs.len() as u64))?;
    for (input, offset) in inputs.iter().zip(&offsets) {
        let (cputype, cpusubtype) = input.arch.mach_cpu();
        out.write_all(&cputype.to_be_bytes())?;
        out.write_all(&cpusubtype.to_be_bytes())?;
        out.write_all(&be(*offset))?;
        out.write_all(&be(input.len))?;
        out.write_all(&ALIGN_POWER.to_be_bytes())?;
    }

    let mut written = FAT_HEADER_SIZE + FAT_ARCH_SIZE * inputs.len() as u64;
    for (input, offset) in inputs.iter_mut().zip(&offsets) {
        out.write_all(&vec![0u8; (offset - written) as usize])?;

        let mut magic = [0u8; 8];
        input.reader.read_exact(&mut magic).with_context(|| {
            format!(
                "The {} input is too short to be a static library",
                input.arch
            )
        })?;
        if &magic != ARCHIVE_MAGIC {
            anyhow::bail!("The {} input is not a static library", input.arch);
        }
        out.write_all(&magic)?;
        let copied = 8 + std::io::copy(&mut input.reader, out)?;
        if copied != input.len {
            anyhow::bail!(
                "The {} input was {copied} bytes, expected {}",
                input.arch,
                input.len
            );
        }
        written = offset + input.len;
    }
    out.flush().context("Failed to flush fat library")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn archive(len: usize) -> Vec<u8> {
        let mut bytes = ARCHIVE_MAGIC.to_vec();
        bytes.resize(len, 0xaa);
        bytes
    }

    fn input(arch: Arch, bytes: Vec<u8>) -> Input {
        Input {
            arch,
            len: bytes.len() as u64,
            reader: Box::new(std::io::Cursor::new(bytes)),
        }
    }

    fn be(bytes: &[u8], at: usize) -> u32 {
        u32::from_be_bytes(bytes[at..at + 4].try_into().unwrap())
    }

    #[test]
    fn test_writes_slices_in_cpu_type_order_with_8_byte_alignment() {
        let mut out = Vec::new();
        write(
            &mut out,
            vec![
                input(Arch::Arm64, archive(704)),
                input(Arch::X86_64, archive(817)),
            ],
        )
        .unwrap();

        assert_eq!(be(&out, 0), FAT_MAGIC);
        assert_eq!(be(&out, 4), 2);
        // x86_64 first, as `lipo` orders it.
        assert_eq!(
            [
                be(&out, 8),
                be(&out, 12),
                be(&out, 16),
                be(&out, 20),
                be(&out, 24)
            ],
            [0x0100_0007, 3, 48, 817, 3]
        );
        // 48 + 817 = 865, rounded up to 872.
        assert_eq!(
            [
                be(&out, 28),
                be(&out, 32),
                be(&out, 36),
                be(&out, 40),
                be(&out, 44)
            ],
            [0x0100_000c, 0, 872, 704, 3]
        );
        assert_eq!(&out[48..48 + 817], archive(817).as_slice());
        assert_eq!(&out[865..872], &[0u8; 7]);
        assert_eq!(&out[872..], archive(704).as_slice());
    }

    #[test]
    fn test_single_slice_is_still_wrapped() {
        let mut out = Vec::new();
        write(&mut out, vec![input(Arch::Arm64_32, archive(100))]).unwrap();
        assert_eq!(
            [
                be(&out, 0),
                be(&out, 4),
                be(&out, 8),
                be(&out, 12),
                be(&out, 16)
            ],
            [FAT_MAGIC, 1, 0x0200_000c, 1, 32]
        );
        assert_eq!(&out[32..], archive(100).as_slice());
    }

    #[test]
    fn test_rejects_bad_inputs() {
        let mut out = Vec::new();
        assert!(
            write(
                &mut out,
                vec![
                    input(Arch::Arm64, archive(10)),
                    input(Arch::Arm64, archive(10))
                ]
            )
            .is_err()
        );
        assert!(
            write(
                &mut out,
                vec![input(Arch::Arm64, b"not an archive".to_vec())]
            )
            .is_err()
        );
        let mut short = input(Arch::Arm64, archive(10));
        short.len = 20;
        assert!(write(&mut out, vec![short]).is_err());
    }
}
