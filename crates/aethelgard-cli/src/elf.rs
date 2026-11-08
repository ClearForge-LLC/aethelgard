//! The firmware ELF, checked before Renode sees it (`AET-WO-2100`, ruled on
//! PR #56, R2 and R4). Renode loads an x86 ELF, or one linked to unmapped
//! memory, with only warnings (spike F12), so the shape is refused here:
//! ELF32, little-endian, `EM_ARM`, `ET_EXEC`, and every `PT_LOAD` inside the
//! part's flash or RAM. The firmware's identity is `image_sha256`, the sha256
//! of the `PT_LOAD` file bytes in physical-address order, which is what
//! Renode loads; the file's own hash also covers symbol names, which Cargo
//! derives from the checkout path (spike F17).

use sha2::{Digest, Sha256};

use aethelgard_twin::boot::{Memory, Region};

use crate::input::hex;
use crate::run::Stop;

const EHDR_LEN: usize = 52;
const PHDR_LEN: usize = 32;
const EM_ARM: u16 = 40;
const ET_EXEC: u16 = 2;
const PT_LOAD: u32 = 1;

/// One loadable segment.
struct Load {
    offset: u64,
    paddr: u64,
    vaddr: u64,
    filesz: u64,
    memsz: u64,
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

/// `image_sha256` of `bytes`, once every check has passed.
pub fn check(bytes: &[u8], memory: &Memory) -> Result<String, Stop> {
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
        });
    }
    if !loads.iter().any(|l| l.filesz > 0) {
        return Err(malformed("no PT_LOAD segment carries file bytes".into()));
    }
    let inside = |r: &Region, address: u64, len: u64| r.holds(address, len);
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
                && !inside(&memory.flash, address, l.memsz)
                && !inside(&memory.ram, address, l.memsz)
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
        }
    }
    let mut image: Vec<&Load> = loads.iter().filter(|l| l.filesz > 0).collect();
    image.sort_by_key(|l| l.paddr);
    for pair in image.windows(2) {
        if pair[0].paddr + pair[0].filesz > pair[1].paddr {
            return Err(malformed(format!(
                "PT_LOAD segments at {:#010x} and {:#010x} overlap",
                pair[0].paddr, pair[1].paddr
            )));
        }
    }
    let mut hasher = Sha256::new();
    for l in image {
        hasher.update(&bytes[l.offset as usize..(l.offset + l.filesz) as usize]);
    }
    Ok(hex(hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn memory() -> Memory {
        aethelgard_twin::boot::memory().unwrap()
    }

    /// An ELF32 LE ARM executable whose segments are (paddr, vaddr, bytes, memsz).
    pub(crate) fn elf(segments: &[(u32, u32, &[u8], u32)]) -> Vec<u8> {
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

    fn kind(bytes: &[u8]) -> &'static str {
        check(bytes, &memory()).unwrap_err().kind
    }

    #[test]
    fn a_flash_image_is_accepted_and_hashed_by_its_loaded_bytes() {
        let a = elf(&[
            (0x0800_0000, 0x0800_0000, b"abcd", 4),
            (0x2000_0000, 0x2000_0000, b"", 8),
        ]);
        let sha = check(&a, &memory()).unwrap();
        assert_eq!(sha, hex(Sha256::digest(b"abcd")));
        // The same loaded bytes in a file with a longer tail hash the same.
        let mut b = a.clone();
        b.extend_from_slice(b"symbols");
        assert_eq!(check(&b, &memory()).unwrap(), sha);
        // Segments are hashed in physical-address order, not table order.
        let c = elf(&[
            (0x0800_0004, 0x0800_0004, b"ef", 2),
            (0x0800_0000, 0x0800_0000, b"abcd", 4),
        ]);
        assert_eq!(
            check(&c, &memory()).unwrap(),
            hex(Sha256::digest(b"abcdef"))
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
        assert_eq!(
            kind(&elf(&[(0x2000_0000, 0x2000_0000, b"", 4)])),
            "MalformedFirmware"
        );
        // More file bytes than memory.
        assert_eq!(
            kind(&elf(&[(0x0800_0000, 0x0800_0000, b"abcd", 2)])),
            "MalformedFirmware"
        );
        // Overlapping images.
        assert_eq!(
            kind(&elf(&[
                (0x0800_0000, 0x0800_0000, b"abcd", 4),
                (0x0800_0002, 0x0800_0002, b"ef", 2)
            ])),
            "MalformedFirmware"
        );
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
        // The last word of RAM and of flash are inside.
        check(
            &elf(&[
                (0x081F_FFFC, 0x081F_FFFC, b"abcd", 4),
                (0x2001_FFFC, 0x2001_FFFC, b"", 4),
            ]),
            &memory(),
        )
        .unwrap();
    }
}
