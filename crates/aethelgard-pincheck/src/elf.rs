//! A strict ELF32 little-endian reader for one symbol: `gpio_default_config`,
//! the 308 bytes ChibiOS's `gpio_init` writes into the GPIO registers at reset
//! (`AET-WO-4400` S4). It reads no code and runs no tool.
//!
//! The bytes are located twice: through the symbol's section, and through the
//! `PT_LOAD` segment that holds its address, which is what is flashed. The two
//! must name the same file offset, so a section table that lies is refused
//! rather than read (`AET-WO-4400a` §5). The segment must run where it is
//! flashed (`p_vaddr == p_paddr`), no other segment may flash over those
//! bytes, and the section may not be writable. Every malformed input is
//! refused by name (`INV-06`).
//!
//! The symbol table itself is trusted: a `st_value` pointing at a decoy copy
//! is caught only by the `hwdef.h` cross-check (adversarial A6).

use crate::{GpioConfig, PORTS, PortRegisters, Refusal};

/// The symbol, and its size: 11 ports × 7 words × 4 bytes.
pub const SYMBOL: &[u8] = b"gpio_default_config";
pub const SYMBOL_SIZE: u32 = (PORTS.len() * 7 * 4) as u32;

const EHDR_LEN: usize = 52;
const SHDR_LEN: u32 = 40;
const PHDR_LEN: u32 = 32;
const SYM_LEN: u32 = 16;
const EM_ARM: u16 = 40;
const ET_EXEC: u16 = 2;
const SHF_WRITE: u32 = 1;
const SHT_PROGBITS: u32 = 1;
const SHT_SYMTAB: u32 = 2;
const SHT_STRTAB: u32 = 3;
const SHF_ALLOC: u32 = 2;
const PT_LOAD: u32 = 1;
const STT_OBJECT: u8 = 1;
/// Section indices from here up are reserved (`SHN_LORESERVE`).
const SHN_LORESERVE: u16 = 0xff00;

fn refuse<T>(kind: &'static str, message: impl Into<String>) -> Result<T, Refusal> {
    Err(Refusal::Elf {
        kind,
        message: message.into(),
    })
}

fn u16_at(b: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([b[at], b[at + 1]])
}

fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

/// `[offset, offset + len)` of the file, or a refusal naming `what`.
fn range<'a>(bytes: &'a [u8], offset: u64, len: u32, what: &str) -> Result<&'a [u8], Refusal> {
    match offset.checked_add(u64::from(len)) {
        Some(end) if end <= bytes.len() as u64 => Ok(&bytes[offset as usize..end as usize]),
        _ => refuse(
            "Truncated",
            format!(
                "{what} at offset {offset} of {len} bytes runs past the file's {} bytes",
                bytes.len()
            ),
        ),
    }
}

struct Section {
    kind: u32,
    flags: u32,
    addr: u32,
    offset: u32,
    size: u32,
    link: u32,
    entsize: u32,
}

fn sections(bytes: &[u8]) -> Result<Vec<Section>, Refusal> {
    let shoff = u32_at(bytes, 32);
    let shentsize = u16_at(bytes, 46);
    let shnum = u16_at(bytes, 48);
    if shoff == 0 || shnum == 0 {
        return refuse("NoSectionTable", "the ELF has no section table");
    }
    if u32::from(shentsize) != SHDR_LEN {
        return refuse(
            "MalformedElf",
            format!("e_shentsize is {shentsize}, not {SHDR_LEN}"),
        );
    }
    if shnum >= SHN_LORESERVE {
        return refuse(
            "MalformedElf",
            format!("e_shnum {shnum} is in the reserved range"),
        );
    }
    let table = range(
        bytes,
        u64::from(shoff),
        u32::from(shnum) * SHDR_LEN,
        "the section table",
    )?;
    Ok(table
        .as_chunks::<{ SHDR_LEN as usize }>()
        .0
        .iter()
        .map(|s| Section {
            kind: u32_at(s, 4),
            flags: u32_at(s, 8),
            addr: u32_at(s, 12),
            offset: u32_at(s, 16),
            size: u32_at(s, 20),
            link: u32_at(s, 24),
            entsize: u32_at(s, 36),
        })
        .collect())
}

/// Every symbol's value, size, type and section index whose name is [`SYMBOL`].
fn symbols_named(bytes: &[u8], sections: &[Section]) -> Result<Vec<(u32, u32, u8, u16)>, Refusal> {
    let mut tables = sections.iter().filter(|s| s.kind == SHT_SYMTAB);
    let symtab = match (tables.next(), tables.next()) {
        (Some(t), None) => t,
        (None, _) => return refuse("NoSymbolTable", "the ELF has no .symtab"),
        (Some(_), Some(_)) => {
            return refuse("MalformedElf", "the ELF has more than one SHT_SYMTAB");
        }
    };
    if symtab.entsize != SYM_LEN || symtab.size % SYM_LEN != 0 {
        return refuse(
            "MalformedElf",
            format!(
                ".symtab has entries of {} bytes over {} bytes, not whole {SYM_LEN}-byte entries",
                symtab.entsize, symtab.size
            ),
        );
    }
    let strtab = match sections.get(symtab.link as usize) {
        Some(s) if s.kind == SHT_STRTAB => s,
        _ => {
            return refuse(
                "MalformedElf",
                format!(".symtab's sh_link {} is not a string table", symtab.link),
            );
        }
    };
    let names = range(
        bytes,
        u64::from(strtab.offset),
        strtab.size,
        "the symbol string table",
    )?;
    let entries = range(bytes, u64::from(symtab.offset), symtab.size, ".symtab")?;
    let mut found = Vec::new();
    for sym in entries.as_chunks::<{ SYM_LEN as usize }>().0 {
        let name_at = u32_at(sym, 0) as usize;
        let Some(tail) = names.get(name_at..) else {
            return refuse(
                "MalformedElf",
                format!("a symbol's name offset {name_at} is outside its string table"),
            );
        };
        let Some(len) = tail.iter().position(|&b| b == 0) else {
            return refuse("MalformedElf", "a symbol's name is not NUL-terminated");
        };
        if &tail[..len] == SYMBOL {
            found.push((
                u32_at(sym, 4),
                u32_at(sym, 8),
                sym[12] & 0xf,
                u16_at(sym, 14),
            ));
        }
    }
    Ok(found)
}

/// The file offset of `[value, value + size)` through the one `PT_LOAD` whose
/// file image holds it.
fn loaded_offset(bytes: &[u8], value: u32, size: u32) -> Result<u64, Refusal> {
    let phoff = u32_at(bytes, 28);
    let phentsize = u16_at(bytes, 42);
    let phnum = u16_at(bytes, 44);
    if phoff == 0 || phnum == 0 {
        return refuse("NoProgramHeaders", "the ELF has no program header table");
    }
    if u32::from(phentsize) != PHDR_LEN {
        return refuse(
            "MalformedElf",
            format!("e_phentsize is {phentsize}, not {PHDR_LEN}"),
        );
    }
    let table = range(
        bytes,
        u64::from(phoff),
        u32::from(phnum) * PHDR_LEN,
        "the program header table",
    )?;
    // Each PT_LOAD with file bytes: (offset, vaddr, paddr, filesz).
    let mut loads = Vec::new();
    for p in table.as_chunks::<{ PHDR_LEN as usize }>().0 {
        if u32_at(p, 0) != PT_LOAD || u32_at(p, 16) == 0 {
            continue;
        }
        let (offset, vaddr, paddr, filesz) =
            (u32_at(p, 4), u32_at(p, 8), u32_at(p, 12), u32_at(p, 16));
        range(bytes, u64::from(offset), filesz, "a PT_LOAD segment")?;
        loads.push((offset, vaddr, paddr, filesz));
    }
    let holds = |start: u32, len: u32, at: u32| {
        at >= start && u64::from(at) + u64::from(size) <= u64::from(start) + u64::from(len)
    };
    let holding: Vec<_> = loads
        .iter()
        .filter(|&&(_, v, _, f)| holds(v, f, value))
        .collect();
    let &&(offset, vaddr, paddr, _) = match holding.as_slice() {
        [one] => one,
        [] => {
            return refuse(
                "SymbolNotLoaded",
                format!(
                    "no PT_LOAD's file image holds {value:#010x}..+{size}: nothing flashed carries it"
                ),
            );
        }
        _ => {
            return refuse(
                "MalformedElf",
                format!("{} PT_LOAD segments hold {value:#010x}", holding.len()),
            );
        }
    };
    // The image is flashed by physical address and read in place at reset
    // (ChibiOS's __early_init runs before .data is copied): the bytes must sit
    // where they are read, and nothing else may be flashed over them
    // (AET-WO-4400a adversarial A2, A5).
    if vaddr != paddr {
        return refuse(
            "SymbolNotInPlace",
            format!(
                "{value:#010x} is in a PT_LOAD loaded at {paddr:#010x} and run at {vaddr:#010x}: what reset reads is not what was flashed"
            ),
        );
    }
    let overlaps = loads
        .iter()
        .filter(|&&(_, _, p, f)| {
            u64::from(p) < u64::from(value) + u64::from(size)
                && u64::from(value) < u64::from(p) + u64::from(f)
        })
        .count();
    if overlaps != 1 {
        return refuse(
            "SegmentsOverlap",
            format!("{overlaps} PT_LOAD segments flash bytes over {value:#010x}..+{size}"),
        );
    }
    Ok(u64::from(offset) + u64::from(value - vaddr))
}

/// Reads the reset GPIO configuration out of a linked ELF.
pub fn read(bytes: &[u8]) -> Result<GpioConfig, Refusal> {
    if bytes.len() < EHDR_LEN {
        return refuse(
            "Truncated",
            format!("{} bytes is shorter than an ELF32 header", bytes.len()),
        );
    }
    if &bytes[..4] != b"\x7fELF" {
        return refuse("NotElf", "the file does not start with \\x7fELF");
    }
    if bytes[4] != 1 {
        return refuse(
            "WrongClass",
            format!("EI_CLASS is {}, not 1 (ELF32)", bytes[4]),
        );
    }
    if bytes[5] != 1 {
        return refuse(
            "WrongByteOrder",
            format!("EI_DATA is {}, not 1 (little-endian)", bytes[5]),
        );
    }
    let kind = u16_at(bytes, 16);
    if kind != ET_EXEC {
        return refuse(
            "WrongType",
            format!("e_type is {kind}, not {ET_EXEC} (ET_EXEC): not a linked firmware"),
        );
    }
    let machine = u16_at(bytes, 18);
    if machine != EM_ARM {
        return refuse(
            "WrongMachine",
            format!("e_machine is {machine}, not {EM_ARM} (EM_ARM)"),
        );
    }
    let sections = sections(bytes)?;
    let name = String::from_utf8_lossy(SYMBOL);
    let (value, size, kind, shndx) = match symbols_named(bytes, &sections)?.as_slice() {
        [one] => *one,
        [] => return refuse("SymbolMissing", format!("no symbol {name} in .symtab")),
        many => {
            return refuse(
                "SymbolDuplicated",
                format!("{} symbols named {name} in .symtab", many.len()),
            );
        }
    };
    if kind != STT_OBJECT {
        return refuse(
            "SymbolNotObject",
            format!("{name} has type {kind}, not STT_OBJECT"),
        );
    }
    if size != SYMBOL_SIZE {
        return refuse(
            "SymbolSize",
            format!("{name} is {size} bytes, not {SYMBOL_SIZE} (11 ports x 7 words)"),
        );
    }
    let section = match sections.get(usize::from(shndx)) {
        Some(s) if shndx != 0 && shndx < SHN_LORESERVE => s,
        _ => {
            return refuse(
                "SymbolOutsideSection",
                format!("{name}'s section index {shndx} names no section"),
            );
        }
    };
    if section.kind != SHT_PROGBITS || section.flags & SHF_ALLOC == 0 {
        return refuse(
            "SymbolOutsideSection",
            format!(
                "{name}'s section is not allocated PROGBITS: it holds no bytes the part receives"
            ),
        );
    }
    if section.flags & SHF_WRITE != 0 {
        return refuse(
            "SymbolOutsideSection",
            format!(
                "{name}'s section is writable: RAM the firmware may change before or after reset reads it"
            ),
        );
    }
    let inside = value >= section.addr
        && u64::from(value) + u64::from(size) <= u64::from(section.addr) + u64::from(section.size);
    if !inside {
        return refuse(
            "SymbolOutsideSection",
            format!(
                "{name} at {value:#010x}..+{size} is outside its section {:#010x}..+{}",
                section.addr, section.size
            ),
        );
    }
    range(
        bytes,
        u64::from(section.offset),
        section.size,
        "the symbol's section",
    )?;
    let by_section = u64::from(section.offset) + u64::from(value - section.addr);
    let by_segment = loaded_offset(bytes, value, size)?;
    if by_section != by_segment {
        return refuse(
            "SectionTableDisagrees",
            format!(
                "{name} is at file offset {by_section} by its section and {by_segment} by its PT_LOAD"
            ),
        );
    }
    let data = range(bytes, by_segment, size, &name)?;
    let words: Vec<u32> = data
        .as_chunks::<4>()
        .0
        .iter()
        .map(|w| u32::from_le_bytes(*w))
        .collect();
    let mut ports = [PortRegisters::default(); PORTS.len()];
    for (port, w) in ports.iter_mut().zip(words.as_chunks::<7>().0) {
        *port = PortRegisters {
            moder: w[0],
            otyper: w[1],
            ospeedr: w[2],
            pupdr: w[3],
            odr: w[4],
            afrl: w[5],
            afrh: w[6],
        };
    }
    Ok(GpioConfig { ports })
}
