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
    /// The ranges of the executable segments, at their physical address,
    /// which is their virtual one: where a firmware that reached its fixed
    /// point is still running (adversarial A7), sorted by address, at most
    /// [`aethelgard_chaos::record::MAX_EXECUTABLE_SEGMENTS`].
    pub executable: Vec<(u64, u64)>,
    /// Every `yield; b.n` self-loop in the executable segments' file bytes,
    /// what Renode loads (`AET-WO-2204` D3), at most [`MAX_SELF_LOOPS`]. The
    /// judge also reads each back from the machine after the run, since a
    /// firmware can rewrite its own flash in Renode (adversarial A1).
    pub self_loops: Vec<u64>,
}

/// The most `yield; b.n` self-loops a firmware may hold: each is read back
/// after the run. The generated firmware has three (`fault`, `Board::take`'s
/// `else`, the fixed point).
pub const MAX_SELF_LOOPS: usize = 64;

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

/// The ELF header's checks, then its `PT_LOAD` segments: what both judges'
/// checks start from.
fn loads(bytes: &[u8]) -> Result<Vec<Load>, Stop> {
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
    Ok(loads)
}

/// The checks, then the image.
pub fn check(bytes: &[u8], memory: &Memory, boot_word: u32) -> Result<Image, Stop> {
    let loads = loads(bytes)?;
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
    // Cortex-M has no MMU: code runs where Renode loads it, at the physical
    // address. An executable segment that names another virtual one would let
    // the judge read code that is not where the core runs (`AET-WO-2204`
    // adversarial A1).
    if let Some(l) = loads
        .iter()
        .find(|l| l.flags & PF_X != 0 && l.memsz > 0 && l.vaddr != l.paddr)
    {
        return Err(malformed(format!(
            "an executable PT_LOAD's virtual address {:#010x} is not its physical one {:#010x}",
            l.vaddr, l.paddr
        )));
    }
    let self_loops: Vec<u64> = loads
        .iter()
        .filter(|l| l.flags & PF_X != 0)
        .flat_map(|l| {
            aethelgard_twin::boot::self_loops(
                l.paddr,
                &bytes[l.offset as usize..(l.offset + l.filesz) as usize],
            )
        })
        .collect();
    // Disjoint, since no two segments overlap in memory (above).
    let mut executable: Vec<(u64, u64)> = loads
        .iter()
        .filter(|l| l.flags & PF_X != 0 && l.memsz > 0)
        .map(|l| (l.paddr, l.paddr + l.memsz))
        .collect();
    executable.sort_unstable();
    // Kind 10 archives them (`AET-WO-2205` §1.5).
    if executable.len() > aethelgard_chaos::record::MAX_EXECUTABLE_SEGMENTS {
        return Err(malformed(format!(
            "the firmware has {} executable segments, more than {}",
            executable.len(),
            aethelgard_chaos::record::MAX_EXECUTABLE_SEGMENTS
        )));
    }
    if self_loops.len() > MAX_SELF_LOOPS {
        return Err(malformed(format!(
            "the executable segments hold {} `yield; b.n` self-loops, more than {MAX_SELF_LOOPS}",
            self_loops.len()
        )));
    }
    Ok(Image {
        sha256: hex(hasher.finalize()),
        executable,
        self_loops,
    })
}

/// Where a foreign firmware may load (`AET-WO-4400e`, ruled on PR #212): the
/// part's flash and every RAM region ST's header states, each from its own
/// `#define` and the size its comment states ([`aethelgard_svd_hal::cmsis`]),
/// never typed in (`RUL-16`). Renode 1.17.0's `stm32h7.repl` maps each at the
/// same base with the same size (`AET-WO-4400e` §1.1, read at source).
pub const FOREIGN_RAM: [&str; 6] = [
    "D1_ITCMRAM_BASE",
    "D1_DTCMRAM_BASE",
    "D1_AXISRAM_BASE",
    "D2_AHBSRAM_BASE",
    "D3_SRAM_BASE",
    "D3_BKPSRAM_BASE",
];

/// A foreign firmware's regions: flash first, then [`FOREIGN_RAM`], each
/// `(name, region)`.
pub fn foreign_memory() -> Result<Vec<(&'static str, aethelgard_twin::boot::Region)>, Stop> {
    let facts = aethelgard_svd_hal::cmsis::register_facts()
        .map_err(|e| Stop::tool_error("aethelgard-svd-hal", e.to_string()))?;
    let fact = |r: Result<u32, aethelgard_svd_hal::cmsis::RegisterError>| {
        r.map(u64::from)
            .map_err(|e| Stop::tool_error("aethelgard-svd-hal", e.to_string()))
    };
    let mut regions = vec![(
        "FLASH_BANK1_BASE",
        aethelgard_twin::boot::Region {
            start: fact(facts.value("FLASH_BANK1_BASE"))?,
            end: fact(facts.value("FLASH_END"))? + 1,
        },
    )];
    for name in FOREIGN_RAM {
        let start = fact(facts.value(name))?;
        let kib = fact(facts.region_size_kib(name))?;
        regions.push((
            name,
            aethelgard_twin::boot::Region {
                start,
                end: start + kib * 1024,
            },
        ));
    }
    Ok(regions)
}

/// A loaded flash segment of a foreign firmware: its physical address, its
/// file bytes' length and their sha256 (`AET-WO-4400e`, ruled on PR #212,
/// comment 6965511448: no write inside its own loaded ranges).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlashLoad {
    pub address: u64,
    pub len: u64,
    pub sha256: String,
}

/// Every `PT_LOAD` whose file bytes Renode writes into `flash`, in address
/// order: what the foreign judge reads back after the run.
pub fn flash_loads(
    bytes: &[u8],
    flash: &aethelgard_twin::boot::Region,
) -> Result<Vec<FlashLoad>, Stop> {
    let mut out: Vec<FlashLoad> = loads(bytes)?
        .iter()
        .filter(|l| l.filesz > 0 && flash.holds(l.paddr, l.filesz))
        .map(|l| FlashLoad {
            address: l.paddr,
            len: l.filesz,
            sha256: hex(Sha256::digest(
                &bytes[l.offset as usize..(l.offset + l.filesz) as usize],
            )),
        })
        .collect();
    out.sort_by_key(|f| f.address);
    Ok(out)
}

/// The section header's size in an ELF32 file, and the symbol table's
/// type and entry size (System V ABI, ELF32: `Elf32_Shdr`, `SHT_SYMTAB`,
/// `Elf32_Sym`).
const SHDR_LEN: usize = 40;
const SHT_SYMTAB: u32 = 2;
const SYM_LEN: usize = 16;
const STT_FUNC: u8 = 2;

/// The address of the function symbol `name` in a foreign firmware's
/// symbol table, with Thumb's low bit cleared, and only where it runs from an
/// executable `PT_LOAD` (`AET-WO-1609`): the judge hooks it there. `None`
/// when no symbol table names it. Two different addresses under the one
/// name, or a table outside the file, are refused (`MalformedFirmware`).
pub fn function(bytes: &[u8], name: &str) -> Result<Option<u64>, Stop> {
    let loads = loads(bytes)?;
    let shoff = u32_at(bytes, 32) as usize;
    let shentsize = usize::from(u16_at(bytes, 46));
    let shnum = usize::from(u16_at(bytes, 48));
    if shnum == 0 {
        return Ok(None);
    }
    if shentsize != SHDR_LEN {
        return Err(malformed(format!(
            "section header entries are {shentsize} bytes, not {SHDR_LEN}"
        )));
    }
    let range = |off: usize, len: usize, what: &str| {
        off.checked_add(len)
            .filter(|&end| end <= bytes.len())
            .map(|end| off..end)
            .ok_or_else(|| malformed(format!("the {what} is outside the file")))
    };
    let headers = range(
        shoff,
        shnum.saturating_mul(SHDR_LEN),
        "section header table",
    )?;
    let section = |i: usize| {
        let at = headers.start + i * SHDR_LEN;
        let field = |k: usize| u32_at(bytes, at + 4 * k) as usize;
        (u32_at(bytes, at + 4), field(4), field(5), field(6))
    };
    let mut found: Vec<u64> = Vec::new();
    for i in 0..shnum {
        let (kind, offset, size, link) = section(i);
        if kind != SHT_SYMTAB {
            continue;
        }
        if link >= shnum {
            return Err(malformed(format!("symbol table {i} links section {link}")));
        }
        let symbols = range(offset, size - size % SYM_LEN, "symbol table")?;
        let (_, str_off, str_size, _) = section(link);
        let strings = range(str_off, str_size, "string table")?;
        for at in symbols.step_by(SYM_LEN) {
            let start = u32_at(bytes, at) as usize;
            let info = bytes[at + 12];
            if info & 0xF != STT_FUNC || start >= strings.len() {
                continue;
            }
            let text = &bytes[strings.start + start..strings.end];
            let end = text.iter().position(|&b| b == 0).unwrap_or(text.len());
            if &text[..end] == name.as_bytes() {
                let address = u64::from(u32_at(bytes, at + 4) & !1);
                if !found.contains(&address) {
                    found.push(address);
                }
            }
        }
    }
    let address = match found.as_slice() {
        [] => return Ok(None),
        [one] => *one,
        many => {
            return Err(malformed(format!(
                "`{name}` names {} addresses, not one",
                many.len()
            )));
        }
    };
    let runs = loads.iter().any(|l| {
        l.flags & PF_X != 0 && l.vaddr <= address && address < l.vaddr + l.memsz.max(l.filesz)
    });
    if !runs {
        return Err(malformed(format!(
            "`{name}` is at {address:#010x}, outside every executable PT_LOAD"
        )));
    }
    Ok(Some(address))
}

/// The `count` little-endian words at `address` in the file bytes one
/// `PT_LOAD` loads there, by physical address (`AET-WO-2314`: the vector
/// table the foreign judge reads its fault handlers from). `None` when no
/// single segment's file bytes hold all of them.
pub fn loaded_words(bytes: &[u8], address: u64, count: usize) -> Result<Option<Vec<u32>>, Stop> {
    let len = 4 * count as u64;
    Ok(loads(bytes)?
        .iter()
        .find(|l| {
            l.paddr <= address
                && address + len <= l.paddr + l.filesz
                // belt-and-suspenders: `check_foreign` refuses a segment
                // whose file bytes pass the file's end before this is read,
                // so no test reaches it; it keeps a later caller from
                // panicking on a slice past the end.
                && l.offset + l.filesz <= bytes.len() as u64
        })
        .map(|l| {
            let at = (l.offset + address - l.paddr) as usize;
            (0..count).map(|i| u32_at(bytes, at + 4 * i)).collect()
        }))
}

/// A foreign firmware's checks, then its image (`AET-WO-4400e`): every
/// `PT_LOAD` with a size lies, at its physical and its virtual address,
/// inside one region of [`foreign_memory`]; no two segments' **file bytes**
/// overlap (a zero-filled segment may share its physical address with another,
/// as ArduPilot's `.crash_log`, `.bss` and `.heap` do: accepted on PR #212,
/// comment 6965511448); an executable segment runs where it is loaded. There is no
/// boot word to keep clear and no self-loop to find: a foreign firmware is not
/// held to the generated runtime's contract.
pub fn check_foreign(
    bytes: &[u8],
    regions: &[(&'static str, aethelgard_twin::boot::Region)],
) -> Result<Image, Stop> {
    let loads = loads(bytes)?;
    if !loads.iter().any(|l| l.filesz > 0) {
        return Err(malformed("no PT_LOAD segment carries file bytes".into()));
    }
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
        for (what, address) in [("physical", l.paddr), ("virtual", l.vaddr)] {
            if l.memsz > 0 && !regions.iter().any(|(_, r)| r.holds(address, l.memsz)) {
                let named: Vec<String> = regions
                    .iter()
                    .map(|(n, r)| format!("{n} [{:#010x}, {:#010x})", r.start, r.end))
                    .collect();
                return Err(Stop::refused(
                    "FirmwareOutsideMemory",
                    format!(
                        "a PT_LOAD of {} bytes at {what} address {address:#010x} is outside every \
                         region a foreign firmware may load into: {}",
                        l.memsz,
                        named.join(", ")
                    ),
                ));
            }
        }
    }
    let mut filled: Vec<&Load> = loads.iter().filter(|l| l.filesz > 0).collect();
    filled.sort_by_key(|l| l.paddr);
    for pair in filled.windows(2) {
        if overlaps(pair[0].paddr, pair[0].filesz, pair[1].paddr, pair[1].filesz) {
            return Err(malformed(format!(
                "PT_LOAD segments' file bytes at {:#010x} and {:#010x} overlap",
                pair[0].paddr, pair[1].paddr
            )));
        }
    }
    let mut image: Vec<&Load> = loads.iter().filter(|l| l.memsz > 0).collect();
    image.sort_by_key(|l| (l.paddr, l.vaddr, l.memsz));
    let mut hasher = Sha256::new();
    for l in &image {
        for v in [l.paddr, l.filesz, l.memsz] {
            hasher.update((v as u32).to_le_bytes());
        }
        hasher.update(&bytes[l.offset as usize..(l.offset + l.filesz) as usize]);
    }
    if let Some(l) = loads
        .iter()
        .find(|l| l.flags & PF_X != 0 && l.memsz > 0 && l.vaddr != l.paddr)
    {
        return Err(malformed(format!(
            "an executable PT_LOAD's virtual address {:#010x} is not its physical one {:#010x}",
            l.vaddr, l.paddr
        )));
    }
    let mut executable: Vec<(u64, u64)> = loads
        .iter()
        .filter(|l| l.flags & PF_X != 0 && l.memsz > 0)
        .map(|l| (l.paddr, l.paddr + l.memsz))
        .collect();
    executable.sort_unstable();
    if executable.windows(2).any(|p| p[0].1 > p[1].0) {
        return Err(malformed(
            "executable PT_LOAD segments overlap in memory".into(),
        ));
    }
    if executable.len() > aethelgard_chaos::record::MAX_EXECUTABLE_SEGMENTS {
        return Err(malformed(format!(
            "the firmware has {} executable segments, more than {}",
            executable.len(),
            aethelgard_chaos::record::MAX_EXECUTABLE_SEGMENTS
        )));
    }
    Ok(Image {
        sha256: hex(hasher.finalize()),
        executable,
        self_loops: Vec::new(),
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

    /// `base` with a symbol table of `symbols` (name, value, `st_info`) and
    /// its string table appended, and the section headers that name them:
    /// null, `.symtab`, `.strtab`.
    fn with_symbols(base: &[u8], symbols: &[(&str, u32, u8)]) -> Vec<u8> {
        let mut b = base.to_vec();
        let mut strings = vec![0u8];
        let mut table = vec![0u8; SYM_LEN];
        for &(name, value, info) in symbols {
            let at = strings.len() as u32;
            strings.extend_from_slice(name.as_bytes());
            strings.push(0);
            table.extend_from_slice(&at.to_le_bytes());
            table.extend_from_slice(&value.to_le_bytes());
            table.extend_from_slice(&4u32.to_le_bytes());
            table.extend_from_slice(&[info, 0, 1, 0]);
        }
        let symtab_at = b.len() as u32;
        b.extend_from_slice(&table);
        let strtab_at = b.len() as u32;
        b.extend_from_slice(&strings);
        let shoff = b.len() as u32;
        let header = |kind: u32, offset: u32, size: u32, link: u32| {
            let mut h = vec![0u8; SHDR_LEN];
            for (i, v) in [(1, kind), (4, offset), (5, size), (6, link)] {
                h[4 * i..4 * i + 4].copy_from_slice(&v.to_le_bytes());
            }
            h
        };
        b.extend(header(0, 0, 0, 0));
        b.extend(header(SHT_SYMTAB, symtab_at, table.len() as u32, 2));
        b.extend(header(3, strtab_at, strings.len() as u32, 0));
        b[32..36].copy_from_slice(&shoff.to_le_bytes());
        b[46..48].copy_from_slice(&(SHDR_LEN as u16).to_le_bytes());
        b[48..50].copy_from_slice(&3u16.to_le_bytes());
        b
    }

    /// `AET-WO-1609`: the panic hook's address is the function symbol's, its
    /// Thumb bit cleared, and only inside an executable `PT_LOAD`; absent is
    /// `None` (the caller refuses it by name), and a name at two addresses,
    /// or a symbol outside every executable segment, is refused.
    #[test]
    fn a_function_symbol_is_found_once_inside_executable_code() {
        const PANIC: &str = "_ZN6AP_HAL5panicEPKcz";
        let base = elf(&[(0x0800_0000, 0x0800_0000, &[0u8; 64], 64)]);
        let one = with_symbols(&base, &[("other", 0x0800_0011, 2), (PANIC, 0x0800_0021, 2)]);
        assert_eq!(function(&one, PANIC).unwrap(), Some(0x0800_0020));
        assert_eq!(function(&base, PANIC).unwrap(), None, "no section headers");
        let object = with_symbols(&base, &[(PANIC, 0x0800_0021, 1)]);
        assert_eq!(
            function(&object, PANIC).unwrap(),
            None,
            "an STT_OBJECT is not the function"
        );
        let prefix = with_symbols(&base, &[("_ZN6AP_HAL5panicEPKc", 0x0800_0021, 2)]);
        assert_eq!(
            function(&prefix, PANIC).unwrap(),
            None,
            "a prefix is another name"
        );
        let twice = with_symbols(&base, &[(PANIC, 0x0800_0021, 2), (PANIC, 0x0800_0031, 2)]);
        assert_eq!(
            function(&twice, PANIC).unwrap_err().kind,
            "MalformedFirmware"
        );
        let same = with_symbols(
            &base,
            &[(PANIC, 0x0800_0021, 2), (PANIC, 0x0800_0021, 0x12)],
        );
        assert_eq!(
            function(&same, PANIC).unwrap(),
            Some(0x0800_0020),
            "one address twice is one"
        );
        let outside = with_symbols(&base, &[(PANIC, 0x0900_0001, 2)]);
        assert_eq!(
            function(&outside, PANIC).unwrap_err().kind,
            "MalformedFirmware"
        );
        let mut cut = one.clone();
        cut.truncate(cut.len() - 1);
        assert_eq!(function(&cut, PANIC).unwrap_err().kind, "MalformedFirmware");
    }

    /// `AET-WO-4400e`: a foreign firmware's segment must lie inside a region
    /// at its virtual address too, and no two segments' file bytes may
    /// overlap; zero-filled segments may share an address.
    #[test]
    fn a_foreign_segment_is_judged_at_both_addresses_and_file_bytes_never_overlap() {
        let regions = foreign_memory().unwrap();
        let code = [0u8; 64];
        let virtual_outside = elf(&[(0x0800_0000, 0x6000_0000, &code, 64)]);
        let e = check_foreign(&virtual_outside, &regions).unwrap_err();
        assert_eq!(e.kind, "FirmwareOutsideMemory", "{}", e.message);
        assert!(
            e.message.contains("virtual address 0x60000000"),
            "{}",
            e.message
        );
        // Two read-write data segments whose file bytes overlap (an
        // executable pair falls to the executable-overlap rule first).
        let mut overlapping = elf(&[
            (0x0800_0000, 0x0800_0000, &code, 64),
            (0x3000_0000, 0x3000_0000, &code, 64),
            (0x3000_0020, 0x3000_0020, &code, 64),
        ]);
        for i in [1, 2] {
            let at = EHDR_LEN + PHDR_LEN * i + 24;
            overlapping[at..at + 4].copy_from_slice(&6u32.to_le_bytes());
        }
        let e = check_foreign(&overlapping, &regions).unwrap_err();
        assert_eq!(e.kind, "MalformedFirmware", "{}", e.message);
        assert!(e.message.contains("overlap"), "{}", e.message);
        let mut zero_filled = elf(&[
            (0x0800_0000, 0x0800_0000, &code, 64),
            (0x3000_0000, 0x3000_0000, &[], 0x100),
            (0x3000_0000, 0x3000_0000, &[], 0x200),
        ]);
        // The zero-filled segments read-write (PF_R | PF_W), as ArduPilot's are.
        for i in [1, 2] {
            let at = EHDR_LEN + PHDR_LEN * i + 24;
            zero_filled[at..at + 4].copy_from_slice(&6u32.to_le_bytes());
        }
        if let Err(e) = check_foreign(&zero_filled, &regions) {
            panic!("{}: {}", e.kind, e.message);
        }
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

    /// `AET-WO-2204` D3, adversarial A1: the self-loops are read at the
    /// physical address, where Renode loads them; an executable segment
    /// naming another virtual address is refused (Cortex-M has no MMU), and
    /// so are more loops than the judge reads back.
    #[test]
    fn self_loops_are_found_where_renode_loads_them() {
        let lp: &[u8] = &[0, 0, 0x10, 0xBF, 0xFD, 0xE7, 0, 0];
        let a = elf(&[(0x0800_0000, 0x0800_0000, lp, 8)]);
        assert_eq!(
            check(&a, &memory(), WORD).unwrap().self_loops,
            [0x0800_0002]
        );
        // Not a loop: the `b.n` goes elsewhere, or no `yield` before it.
        for code in [
            &[0x10u8, 0xBF, 0xFE, 0xE7][..],
            &[0x08, 0x60, 0xFD, 0xE7][..],
        ] {
            let e = elf(&[(0x0800_0000, 0x0800_0000, code, 4)]);
            assert!(check(&e, &memory(), WORD).unwrap().self_loops.is_empty());
        }
        // The decoy: a loop's bytes at one virtual address, loaded at another.
        let decoy = elf(&[(0x080F_0000, 0x0800_0040, lp, 8)]);
        assert_eq!(kind(&decoy), "MalformedFirmware");
        let many = [0x10u8, 0xBF, 0xFD, 0xE7].repeat(MAX_SELF_LOOPS + 1);
        assert_eq!(
            kind(&elf(&[(
                0x0800_0000,
                0x0800_0000,
                &many,
                many.len() as u32
            )])),
            "MalformedFirmware"
        );
        let enough = [0x10u8, 0xBF, 0xFD, 0xE7].repeat(MAX_SELF_LOOPS);
        let e = elf(&[(0x0800_0000, 0x0800_0000, &enough, enough.len() as u32)]);
        assert_eq!(
            check(&e, &memory(), WORD).unwrap().self_loops.len(),
            MAX_SELF_LOOPS
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

    /// `AET-WO-2205` §1.5: kind 10 archives the executable segments, sorted,
    /// at most 16; a firmware with more is refused before it boots.
    #[test]
    fn executable_segments_are_sorted_and_at_most_sixteen() {
        let at = |k: u32| 0x0800_0000 + 0x100 * k;
        let segs: Vec<(u32, u32, &[u8], u32)> = (0..16)
            .rev()
            .map(|k| (at(k), at(k), &b"abcd"[..], 4))
            .collect();
        let image = check(&elf(&segs), &memory(), WORD).unwrap();
        let want: Vec<(u64, u64)> = (0..16)
            .map(|k| (at(k).into(), (at(k) + 4).into()))
            .collect();
        assert_eq!(image.executable, want, "sorted by address");
        let segs: Vec<(u32, u32, &[u8], u32)> =
            (0..17).map(|k| (at(k), at(k), &b"abcd"[..], 4)).collect();
        assert_eq!(kind(&elf(&segs)), "MalformedFirmware", "17 segments");
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
