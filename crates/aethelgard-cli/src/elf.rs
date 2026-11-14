//! The firmware ELF, checked before Renode sees it (`AET-WO-2100`, ruled on
//! PR #56, R2 and R4). Renode loads an x86 ELF, or one linked to unmapped
//! memory, with only warnings (spike F12), so the shape is refused here:
//! ELF32, little-endian, `EM_ARM`, `ET_EXEC`, every `PT_LOAD` inside the
//! part's flash or RAM, no two overlapping, and none whose file bytes cover
//! the boot word (adversarial A1, A1c, A2: the loader, or the reset handler's
//! `.data` copy, would write the magic with no code reaching the fixed point).
//!
//! The firmware's identity is `image_sha256`: for every `PT_LOAD` with memory,
//! in physical-address order, its physical address, file size and memory size
//! (each `u32` little-endian), then its file bytes. That is the memory image
//! Renode builds. The file's own hash also covers symbol names, which Cargo
//! derives from the checkout path (spike F17); a hash of the bytes alone
//! would give one identity to two images (adversarial A4, A5).

use sha2::{Digest, Sha256};

use aethelgard_twin::boot::Memory;

use crate::input::hex;
use crate::run::Stop;

const EHDR_LEN: usize = 52;
const PHDR_LEN: usize = 32;
const EM_ARM: u16 = 40;
const ET_EXEC: u16 = 2;
const PT_LOAD: u32 = 1;
const PF_X: u32 = 1;

/// One loadable segment.
struct Load {
    offset: u64,
    paddr: u64,
    vaddr: u64,
    filesz: u64,
    memsz: u64,
    flags: u32,
}

/// What the checks give the boot.
#[derive(Debug)]
pub struct Image {
    pub sha256: String,
    /// The virtual ranges of the executable segments: where a firmware that
    /// reached its fixed point is still running (adversarial A7).
    pub executable: Vec<(u64, u64)>,
    /// Every `yield; b.n` self-loop in the executable segments' file bytes,
    /// what Renode loads (`AET-WO-2204` D3): read from the ELF, never from the
    /// machine, so a firmware cannot rewrite the code the judge reads.
    pub self_loops: Vec<u64>,
}

fn malformed(message: String) -> Stop {
    Stop::refused("MalformedFirmware", message)
}

fn u16_at(b: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([b[at], b[at + 1]])
}

fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

/// True when `[a, a + alen)` and `[b, b + blen)` share a byte.
fn overlaps(a: u64, alen: u64, b: u64, blen: u64) -> bool {
    alen > 0 && blen > 0 && a < b + blen && b < a + alen
}

/// The checks, then the image.
pub fn check(bytes: &[u8], memory: &Memory, boot_word: u32) -> Result<Image, Stop> {
    if bytes.len() < EHDR_LEN || &bytes[..4] != b"\x7fELF" {
        return Err(malformed("not an ELF file".into()));
    }
    let (class, data, version) = (bytes[4], bytes[5], bytes[6]);
    if class != 1 {
        return Err(malformed(format!("ELF class {class}, not ELF32 (1)")));
    }
    if data != 1 {
        return Err(malformed(format!(
            "ELF data encoding {data}, not little-endian (1)"
        )));
    }
    if version != 1 || u32_at(bytes, 20) != 1 {
        return Err(malformed("ELF version is not 1".into()));
    }
    let e_type = u16_at(bytes, 16);
    if e_type != ET_EXEC {
        return Err(malformed(format!("ELF type {e_type}, not ET_EXEC (2)")));
    }
    let machine = u16_at(bytes, 18);
    if machine != EM_ARM {
        return Err(malformed(format!("ELF machine {machine}, not EM_ARM (40)")));
    }
    let phoff = u32_at(bytes, 28) as usize;
    let phentsize = usize::from(u16_at(bytes, 42));
    let phnum = usize::from(u16_at(bytes, 44));
    if phentsize != PHDR_LEN {
        return Err(malformed(format!(
            "program header entries are {phentsize} bytes, not {PHDR_LEN}"
        )));
    }
    let table_end = phnum
        .checked_mul(PHDR_LEN)
        .and_then(|n| n.checked_add(phoff))
        .filter(|&end| end <= bytes.len())
        .ok_or_else(|| malformed("the program header table is outside the file".into()))?;
    let mut loads = Vec::new();
    for at in (phoff..table_end).step_by(PHDR_LEN) {
        if u32_at(bytes, at) != PT_LOAD {
            continue;
        }
        let field = |i: usize| u64::from(u32_at(bytes, at + 4 * i));
        loads.push(Load {
            offset: field(1),
            vaddr: field(2),
            paddr: field(3),
            filesz: field(4),
            memsz: field(5),
            flags: u32_at(bytes, at + 24),
        });
    }
    if !loads.iter().any(|l| l.filesz > 0) {
        return Err(malformed("no PT_LOAD segment carries file bytes".into()));
    }
    let word = u64::from(boot_word);
    for l in &loads {
        if l.offset + l.filesz > bytes.len() as u64 {
            return Err(malformed(format!(
                "a PT_LOAD's {} file bytes at offset {:#x} run past the file",
                l.filesz, l.offset
            )));
        }
        if l.filesz > l.memsz {
            return Err(malformed(format!(
                "a PT_LOAD at {:#010x} has more file bytes ({}) than memory ({})",
                l.paddr, l.filesz, l.memsz
            )));
        }
        // Renode writes `memsz` bytes at the physical address; the code runs
        // at the virtual one. Both must be the part's memory.
        for (what, address) in [("physical", l.paddr), ("virtual", l.vaddr)] {
            if l.memsz > 0
                && !memory.flash.holds(address, l.memsz)
                && !memory.ram.holds(address, l.memsz)
            {
                return Err(Stop::refused(
                    "FirmwareOutsideMemory",
                    format!(
                        "a PT_LOAD of {} bytes at {what} address {address:#010x} is outside flash \
                         [{:#010x}, {:#010x}) and RAM [{:#010x}, {:#010x})",
                        l.memsz,
                        memory.flash.start,
                        memory.flash.end,
                        memory.ram.start,
                        memory.ram.end
                    ),
                ));
            }
            if overlaps(address, l.filesz, word, 4) {
                return Err(Stop::refused(
                    "FirmwareLoadsBootWord",
                    format!(
                        "a PT_LOAD's file bytes at {what} address {address:#010x} cover the boot \
                         word at {boot_word:#010x}: only code at its fixed point may write it"
                    ),
                ));
            }
        }
    }
    let mut image: Vec<&Load> = loads.iter().filter(|l| l.memsz > 0).collect();
    image.sort_by_key(|l| l.paddr);
    for pair in image.windows(2) {
        if overlaps(pair[0].paddr, pair[0].memsz, pair[1].paddr, pair[1].memsz) {
            return Err(malformed(format!(
                "PT_LOAD segments at {:#010x} and {:#010x} overlap in memory",
                pair[0].paddr, pair[1].paddr
            )));
        }
    }
    let mut hasher = Sha256::new();
    for l in &image {
        for v in [l.paddr, l.filesz, l.memsz] {
            hasher.update((v as u32).to_le_bytes());
        }
        hasher.update(&bytes[l.offset as usize..(l.offset + l.filesz) as usize]);
    }
    Ok(Image {
        sha256: hex(hasher.finalize()),
        executable: loads
            .iter()
            .filter(|l| l.flags & PF_X != 0 && l.memsz > 0)
            .map(|l| (l.vaddr, l.vaddr + l.memsz))
            .collect(),
        self_loops: loads
            .iter()
            .filter(|l| l.flags & PF_X != 0)
            .flat_map(|l| {
                aethelgard_twin::boot::self_loops(
                    l.vaddr,
                    &bytes[l.offset as usize..(l.offset + l.filesz) as usize],
                )
            })
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const WORD: u32 = 0x2000_0000;

    fn memory() -> Memory {
        aethelgard_twin::boot::memory().unwrap()
    }

    /// An ELF32 LE ARM executable whose segments are (paddr, vaddr, bytes, memsz).
    fn elf(segments: &[(u32, u32, &[u8], u32)]) -> Vec<u8> {
        let mut b = vec![0u8; EHDR_LEN];
        b[..7].copy_from_slice(b"\x7fELF\x01\x01\x01");
        b[16..18].copy_from_slice(&ET_EXEC.to_le_bytes());
        b[18..20].copy_from_slice(&EM_ARM.to_le_bytes());
        b[20..24].copy_from_slice(&1u32.to_le_bytes());
        b[28..32].copy_from_slice(&(EHDR_LEN as u32).to_le_bytes());
        b[40..42].copy_from_slice(&(EHDR_LEN as u16).to_le_bytes());
        b[42..44].copy_from_slice(&(PHDR_LEN as u16).to_le_bytes());
        b[44..46].copy_from_slice(&(segments.len() as u16).to_le_bytes());
        let mut offset = (EHDR_LEN + PHDR_LEN * segments.len()) as u32;
        let mut data = Vec::new();
        for &(paddr, vaddr, bytes, memsz) in segments {
            for v in [
                PT_LOAD,
                offset,
                vaddr,
                paddr,
                bytes.len() as u32,
                memsz,
                5,
                4,
            ] {
                b.extend_from_slice(&v.to_le_bytes());
            }
            offset += bytes.len() as u32;
            data.extend_from_slice(bytes);
        }
        b.extend_from_slice(&data);
        b
    }

    fn sha(bytes: &[u8]) -> String {
        check(bytes, &memory(), WORD).unwrap().sha256
    }

    fn kind(bytes: &[u8]) -> &'static str {
        check(bytes, &memory(), WORD).unwrap_err().kind
    }

    #[test]
    fn the_image_hash_binds_bytes_addresses_and_sizes_but_not_the_rest_of_the_file() {
        let a = elf(&[(0x0800_0000, 0x0800_0000, b"abcd", 4), (WORD, WORD, b"", 8)]);
        let mut expected = Sha256::new();
        for v in [0x0800_0000u32, 4, 4] {
            expected.update(v.to_le_bytes());
        }
        expected.update(b"abcd");
        for v in [WORD, 0, 8] {
            expected.update(v.to_le_bytes());
        }
        assert_eq!(sha(&a), hex(expected.finalize()));
        // A longer tail (symbols) does not change it.
        let mut b = a.clone();
        b.extend_from_slice(b"symbols");
        assert_eq!(sha(&b), sha(&a));
        // Table order does not change it.
        let c = elf(&[(WORD, WORD, b"", 8), (0x0800_0000, 0x0800_0000, b"abcd", 4)]);
        assert_eq!(sha(&c), sha(&a));
        // The same bytes at another address are another image (adversarial A5),
        // and so is more zero-fill.
        let moved = elf(&[(0x0800_1000, 0x0800_1000, b"abcd", 4), (WORD, WORD, b"", 8)]);
        assert_ne!(sha(&moved), sha(&a));
        let filled = elf(&[(0x0800_0000, 0x0800_0000, b"abcd", 8), (WORD, WORD, b"", 8)]);
        assert_ne!(sha(&filled), sha(&a));
        // Only executable segments are where the core may run.
        let image = check(&a, &memory(), WORD).unwrap();
        assert_eq!(
            image.executable,
            [(0x0800_0000, 0x0800_0004), (0x2000_0000, 0x2000_0008)]
        );
    }

    #[test]
    fn every_shape_outside_elf32_le_arm_exec_is_refused() {
        let good = elf(&[(0x0800_0000, 0x0800_0000, b"abcd", 4)]);
        let with = |at: usize, v: &[u8]| {
            let mut b = good.clone();
            b[at..at + v.len()].copy_from_slice(v);
            b
        };
        assert_eq!(kind(b"junk"), "MalformedFirmware");
        assert_eq!(kind(&good[..40]), "MalformedFirmware");
        assert_eq!(kind(&with(0, b"\x7fELG")), "MalformedFirmware");
        assert_eq!(kind(&with(4, &[2])), "MalformedFirmware");
        assert_eq!(kind(&with(5, &[2])), "MalformedFirmware");
        assert_eq!(kind(&with(6, &[0])), "MalformedFirmware");
        assert_eq!(kind(&with(16, &3u16.to_le_bytes())), "MalformedFirmware");
        assert_eq!(kind(&with(18, &62u16.to_le_bytes())), "MalformedFirmware");
        assert_eq!(kind(&with(42, &40u16.to_le_bytes())), "MalformedFirmware");
        assert_eq!(kind(&with(44, &200u16.to_le_bytes())), "MalformedFirmware");
        // A segment whose file bytes run past the end of the file.
        assert_eq!(kind(&good[..good.len() - 1]), "MalformedFirmware");
        // Nothing to load.
        assert_eq!(kind(&elf(&[(WORD, WORD, b"", 4)])), "MalformedFirmware");
        // More file bytes than memory.
        assert_eq!(
            kind(&elf(&[(0x0800_0000, 0x0800_0000, b"abcd", 2)])),
            "MalformedFirmware"
        );
    }

    #[test]
    fn overlapping_segments_are_refused_by_their_memory_not_their_file_bytes() {
        assert_eq!(
            kind(&elf(&[
                (0x0800_0000, 0x0800_0000, b"abcd", 4),
                (0x0800_0002, 0x0800_0002, b"ef", 2)
            ])),
            "MalformedFirmware"
        );
        // Adversarial A4: a zero-fill (memsz past filesz) over the next segment.
        assert_eq!(
            kind(&elf(&[
                (0x0800_0000, 0x0800_0000, b"ab", 0x40),
                (0x0800_0010, 0x0800_0010, b"cd", 2)
            ])),
            "MalformedFirmware"
        );
        // Adjacent is not overlapping.
        sha(&elf(&[
            (0x0800_0000, 0x0800_0000, b"ab", 4),
            (0x0800_0004, 0x0800_0004, b"cd", 2),
        ]));
    }

    #[test]
    fn file_bytes_over_the_boot_word_are_refused() {
        // Adversarial A1/A2: the magic loaded straight into the boot word.
        let loaded = elf(&[
            (0x0800_0000, 0x0800_0000, b"abcd", 4),
            (WORD, WORD, &0x2100_B007u32.to_le_bytes(), 4),
        ]);
        assert_eq!(kind(&loaded), "FirmwareLoadsBootWord");
        // Adversarial A1c: `.data` placed over it, copied in by the reset handler.
        let copied = elf(&[(0x0800_0000, WORD, b"abcd", 4)]);
        assert_eq!(kind(&copied), "FirmwareLoadsBootWord");
        // The reference's boot section: memory but no file bytes, accepted.
        sha(&elf(&[
            (0x0800_0000, 0x0800_0000, b"abcd", 4),
            (WORD, WORD, b"", 4),
        ]));
        // File bytes right after it are not over it.
        sha(&elf(&[
            (0x0800_0000, 0x0800_0000, b"abcd", 4),
            (WORD + 4, WORD + 4, b"ab", 2),
        ]));
    }

    #[test]
    fn a_segment_outside_flash_and_ram_is_refused() {
        // The spike's relinked firmware: flash at 0x0A000000, unmapped.
        assert_eq!(
            kind(&elf(&[(0x0A00_0000, 0x0A00_0000, b"abcd", 4)])),
            "FirmwareOutsideMemory"
        );
        // Straddling the end of RAM, physically or virtually.
        assert_eq!(
            kind(&elf(&[(0x0800_0000, 0x2001_FFFE, b"abcd", 4)])),
            "FirmwareOutsideMemory"
        );
        assert_eq!(
            kind(&elf(&[(0x2001_FFFE, 0x2001_FFFE, b"abcd", 4)])),
            "FirmwareOutsideMemory"
        );
        // A zero-file-byte segment is checked too: Renode writes its memory.
        assert_eq!(
            kind(&elf(&[
                (0x0800_0000, 0x0800_0000, b"abcd", 4),
                (0x0000_0000, 0x0000_0000, b"", 4)
            ])),
            "FirmwareOutsideMemory"
        );
        // The last word of flash and of RAM are inside.
        sha(&elf(&[
            (0x081F_FFFC, 0x081F_FFFC, b"abcd", 4),
            (0x2001_FFFC, 0x2001_FFFC, b"", 4),
        ]));
    }
}
