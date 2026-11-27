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
//! of `gpio_default_config` is caught only by the `hwdef.h` cross-check
//! (adversarial A6), and one of `var_info` by nothing (`AET-WO-4405` A6).
//!
//! The parameter tables are read as compiled defaults, so an ELF that can
//! replace them at boot is refused before any is read ([`read`],
//! [`defaults_signatures`], `AET-WO-4406`).

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
fn symbols_named(
    bytes: &[u8],
    sections: &[Section],
    symbol: &[u8],
) -> Result<Vec<(u32, u32, u8, u16)>, Refusal> {
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
    // A string table that ends in NUL terminates every name in it, so each
    // compare below reads at most `symbol.len() + 1` bytes: one scan to a NUL
    // per symbol made the read quadratic (review 2, A15.1).
    if names.last() != Some(&0) {
        return refuse("MalformedElf", "a symbol's name is not NUL-terminated");
    }
    let mut found = Vec::new();
    for sym in entries.as_chunks::<{ SYM_LEN as usize }>().0 {
        let name_at = u32_at(sym, 0) as usize;
        // An offset at the table's end names no string (its old refusal).
        let Some(tail) = names.get(name_at..).filter(|t| !t.is_empty()) else {
            return refuse(
                "MalformedElf",
                format!("a symbol's name offset {name_at} is outside its string table"),
            );
        };
        if tail[..tail.len().min(symbol.len() + 1)].strip_suffix(&[0]) == Some(symbol) {
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
///
/// An ELF that can apply parameter defaults at boot is refused
/// (`ParamDefaultsOverride`, [`defaults_signatures`]): this check reads the
/// defaults it compiles, and those would not be what runs (`AET-WO-4406`).
pub fn read(bytes: &[u8]) -> Result<GpioConfig, Refusal> {
    read_admitting(bytes, None)
}

/// [`read`], admitting one ROMFS `defaults.parm`: `defaults`, the file
/// `generate hwdef` writes for the topology, if the ELF's is byte for byte
/// that file ([`defaults_signatures`]; `AET-WO-4412`, ruled 2026-11-26T17:35Z).
pub fn read_admitting(bytes: &[u8], defaults: Option<&str>) -> Result<GpioConfig, Refusal> {
    let data = object(bytes, SYMBOL, |size| {
        (size != SYMBOL_SIZE)
            .then(|| format!("{size} bytes, not {SYMBOL_SIZE} (11 ports x 7 words)"))
    })?;
    let overrides = defaults_signatures(bytes, defaults)?;
    if !overrides.is_empty() {
        return refuse(
            "ParamDefaultsOverride",
            format!(
                "the ELF can apply parameter defaults at boot over the ones it compiles, which this check reads: {}",
                overrides.join("; ")
            ),
        );
    }
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

/// The ELF header held (a linked little-endian ELF32 `EM_ARM` firmware) and
/// its section table, each refused by name if it is not.
fn header(bytes: &[u8]) -> Result<Vec<Section>, Refusal> {
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
    sections(bytes)
}

/// The bytes of the one `STT_OBJECT` named `symbol`, in allocated, read-only
/// `PROGBITS`, located twice (its section and its `PT_LOAD`) and refused
/// unless both agree; `bad_size` names a size the caller refuses.
fn object<'a>(
    bytes: &'a [u8],
    symbol: &[u8],
    bad_size: impl Fn(u32) -> Option<String>,
) -> Result<&'a [u8], Refusal> {
    let sections = header(bytes)?;
    let name = String::from_utf8_lossy(symbol);
    let (value, size, kind, shndx) = match symbols_named(bytes, &sections, symbol)?.as_slice() {
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
    if let Some(why) = bad_size(size) {
        return refuse("SymbolSize", format!("{name} is {why}"));
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
    range(bytes, by_segment, size, &name)
}

/// ArduPilot's serial table: `ChibiOS::UARTDriver::_serial_tab`, `const`
/// in flash (`UARTDriver.cpp:53`), one `SerialDef` per SERIALn in order
/// (`AET-WO-4400b2` §1.1).
pub const SERIAL_SYMBOL: &[u8] = b"_ZN7ChibiOS10UARTDriver11_serial_tabE";

/// `SerialDef`'s size at `Copter-4.7.2` with DMA (`UARTDriver.h:58-80`,
/// `HAL_UART_NODMA` undefined): measured on the generated boards' ELFs.
pub const SERIAL_DEF_LEN: u32 = 44;

/// One `SerialDef`: an all-zero entry is ArduPilot's `{}` (no port).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SerialEntry {
    /// The USART or UART number (`instance`, offset 4), 0 for `{}`.
    pub instance: u8,
    /// `is_usb` (offset 5).
    pub is_usb: bool,
    /// `tx_line` and `rx_line` (offsets 20 and 24): `PAL_LINE`, a GPIO
    /// port's base address plus the pin.
    pub tx_line: u32,
    pub rx_line: u32,
    /// Every byte zero.
    pub empty: bool,
    /// The driver pointer (offset 0) is the address of `SD<instance>`, the
    /// serial driver the port runs on (`UARTDriver.cpp:474`): `instance`
    /// alone names only its thread (adversarial A11).
    pub driver_is_instance: bool,
}

/// `STT_FUNC` (ELF32 `st_info` low nibble).
const STT_FUNC: u8 = 2;

/// Whether the ELF defines a function named `symbol` (`AET-WO-4400d`: an
/// IMU line's driver is linked in, as its probe, under its mangled name).
pub fn defines_function(bytes: &[u8], symbol: &str) -> Result<bool, Refusal> {
    // The header and section table are held first as [`serial_table`]
    // holds them, so a short or foreign file is refused, never indexed.
    object(bytes, SERIAL_SYMBOL, |_| None)?;
    let sections = sections(bytes)?;
    Ok(symbols_named(bytes, &sections, symbol.as_bytes())?
        .iter()
        .any(|(_, _, kind, shndx)| *kind == STT_FUNC && *shndx != 0 && *shndx < SHN_LORESERVE))
}

/// ArduPilot's serial parameter table, `AP_SerialManager::var_info`
/// (`AP_SerialManager.cpp:164-400`): one `AP_Param::GroupInfo` per compiled
/// `SERIALn_*` parameter, each `#ifndef` default and `HAL_HAVE_SERIALn_PARAMS`
/// already folded in by the compiler (`AET-WO-4405`).
pub const VAR_INFO_SYMBOL: &[u8] = b"_ZN16AP_SerialManager8var_infoE";

/// `AP_Param::GroupInfo` on ARM32 (`AP_Param.h:206-218`): `name` (a
/// pointer), `offset`, a union whose `float def_value` is the default,
/// `uint16_t flags`, `uint8_t idx`, `uint8_t type`.
const GROUP_INFO_LEN: usize = 16;

/// `AP_PARAM_INT8` and `AP_PARAM_INT32` (`AP_Param.h:185-193`): the types of
/// `UARTState::protocol`, and of `baud` and `options`
/// (`AP_SerialManager.h:157-159`); `AP_PARAM_NONE` ends the table.
const AP_PARAM_NONE: u8 = 0;
const AP_PARAM_INT8: u8 = 1;
const AP_PARAM_INT32: u8 = 3;

/// The longest name read: `_PASSTIMO` and `1_PROTOCOL` are the longest the
/// table holds.
const NAME_MAX: usize = 16;

/// The rows of `var_info` that are not a port's: serial pass-through
/// (`AP_SerialManager.cpp:324`, `:331`, `:339`), each an `AP_Int8`.
const PASS_ROWS: [&str; 3] = ["_PASS1", "_PASS2", "_PASSTIMO"];

/// One compiled port's parameters, as their defaults stand in `var_info`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SerialParam {
    /// n, of `SERIALn`.
    pub port: u8,
    /// `n_PROTOCOL`, a `SerialProtocol` value.
    pub protocol: i8,
    /// `n_BAUD` as stored, before [`map_baudrate`]. Two kinds of row hold a
    /// value scaled by 1000, which `map_baudrate` undoes: `0_BAUD`, always
    /// (`DEFAULT_SERIAL0_BAUD/1000`, `AP_SerialManager.cpp:172`), and each
    /// `n_BAUD` left at its `#ifndef` default
    /// (`AP_SERIALMANAGER_MAVLINK_BAUD/1000` or `_GPS_BAUD/1000`, `:47-162`).
    pub baud: i32,
    /// `n_OPTIONS`; `None` for SERIAL0, which has no options parameter
    /// (`:165-181`).
    pub options: Option<i32>,
}

/// `AP_SerialManager::map_baudrate` (`AP_SerialManager.cpp:737-770`): a
/// `SERIALn_BAUD` value as the baud ArduPilot opens the port at. A value of
/// 0 or less is 57; the listed short codes are their bauds; above 2000 the
/// value is the baud; anything else is kbaud.
// belt-and-suspenders: `aethelgard-hwdef` holds its own copy to choose a
// define; this one judges it, and the judge shares no code with the
// generator (`RUL-20`), so the two copies are deliberate.
pub fn map_baudrate(rate: i32) -> u64 {
    let rate = if rate <= 0 { 57 } else { i64::from(rate) };
    match rate {
        1 => 1200,
        2 => 2400,
        4 => 4800,
        9 => 9600,
        19 => 19_200,
        38 => 38_400,
        57 => 57_600,
        100 => 100_000,
        111 => 111_100,
        115 => 115_200,
        230 => 230_400,
        256 => 256_000,
        460 => 460_800,
        500 => 500_000,
        921 => 921_600,
        1500 => 1_500_000,
        2000 => 2_000_000,
        r if r > 2000 => r.unsigned_abs(),
        r => r.unsigned_abs() * 1000,
    }
}

/// The NUL-terminated string at `addr`, read where it is flashed: one
/// `PT_LOAD`'s file image holds it, in place ([`loaded_offset`]: run where
/// flashed, nothing flashed over it), and it ends within `max` bytes. `what`
/// names it, and `too_long` and `malformed` are the refusals' kinds.
fn flash_cstr<'a>(
    bytes: &'a [u8],
    addr: u32,
    max: usize,
    what: &str,
    [too_long, malformed]: [&'static str; 2],
) -> Result<&'a str, Refusal> {
    let at = usize::try_from(loaded_offset(bytes, addr, 1)?).unwrap_or(usize::MAX);
    let tail = bytes.get(at..).unwrap_or_default();
    let Some(len) = tail.iter().take(max + 1).position(|b| *b == 0) else {
        return refuse(
            too_long,
            format!("the {what} at {addr:#010x} has no NUL within {max} bytes"),
        );
    };
    loaded_offset(bytes, addr, len as u32 + 1)?;
    std::str::from_utf8(&tail[..len]).or_else(|_| {
        refuse(
            malformed,
            format!("the {what} at {addr:#010x} is not UTF-8"),
        )
    })
}

/// A parameter's name ([`flash_cstr`], at most [`NAME_MAX`] bytes).
fn flash_str(bytes: &[u8], addr: u32) -> Result<&str, Refusal> {
    flash_cstr(
        bytes,
        addr,
        NAME_MAX,
        "var_info name",
        ["VarInfoNameTooLong", "MalformedVarInfo"],
    )
}

/// What a row's name says it is: a port's protocol, baud or options, or one
/// of [`PASS_ROWS`].
enum Row {
    Protocol(u8),
    Baud(u8),
    Options(u8),
    Pass,
}

/// `n_PROTOCOL`, `n_BAUD` (n 0 to 9) or `n_OPTIONS` (n 1 to 9), with the
/// `AP_PARAM_*` type its `UARTState` member has; `None` for any other name.
fn row(name: &str) -> Option<(Row, u8)> {
    if PASS_ROWS.contains(&name) {
        return Some((Row::Pass, AP_PARAM_INT8));
    }
    let (n, field) = name.split_at_checked(1)?;
    let n = n.parse::<u8>().ok()?;
    match field {
        "_PROTOCOL" => Some((Row::Protocol(n), AP_PARAM_INT8)),
        "_BAUD" => Some((Row::Baud(n), AP_PARAM_INT32)),
        "_OPTIONS" if n > 0 => Some((Row::Options(n), AP_PARAM_INT32)),
        _ => None,
    }
}

/// One `AP_Param::GroupInfo` before `AP_GROUPEND`.
struct GroupRow<'a> {
    /// Its index in the table.
    i: usize,
    name: &'a str,
    offset: i32,
    /// The union's bits: a `float` default when `flags` is 0.
    bits: u32,
    flags: u16,
    kind: u8,
}

/// The rows of the `AP_Param::GroupInfo` array `symbol`, a whole number of
/// 16-byte entries, read up to the first whose type is `AP_PARAM_NONE`,
/// where ArduPilot stops (`AP_Param.cpp:1498-1500`); that entry must be
/// exactly `AP_GROUPEND` (`AP_Param.h:170`) and the symbol's last. `table`
/// names it in a refusal.
fn group_rows<'a>(
    bytes: &'a [u8],
    symbol: &[u8],
    table: &str,
) -> Result<Vec<GroupRow<'a>>, Refusal> {
    let data = object(bytes, symbol, |size| {
        (size == 0 || !(size as usize).is_multiple_of(GROUP_INFO_LEN)).then(|| {
            format!("{size} bytes, not whole {GROUP_INFO_LEN}-byte AP_Param::GroupInfo entries")
        })
    })?;
    let entries = data.as_chunks::<GROUP_INFO_LEN>().0;
    let mut rows = Vec::new();
    for (i, e) in entries.iter().enumerate() {
        let (name_at, flags, idx, kind) = (u32_at(e, 0), u16_at(e, 12), e[14], e[15]);
        let name = flash_str(bytes, name_at)?;
        if kind == AP_PARAM_NONE {
            if i + 1 != entries.len() || !name.is_empty() || e[4..14] != [0; 10] || idx != 0xff {
                return refuse(
                    "MalformedVarInfo",
                    format!(
                        "{table} entry {i} has type AP_PARAM_NONE but is not AP_GROUPEND ({{\"\", 0, 0, 0, 0xFF, 0}}) as the table's last entry"
                    ),
                );
            }
            return Ok(rows);
        }
        rows.push(GroupRow {
            i,
            name,
            offset: u32_at(e, 4) as i32,
            bits: u32_at(e, 8),
            flags,
            kind,
        });
    }
    refuse(
        "VarInfoUnterminated",
        format!("{table}'s {} entries hold no AP_GROUPEND", entries.len()),
    )
}

/// `sizeof(AP_SerialManager::UARTState)` and where `baud`, `options` and
/// `protocol` sit in it (`AP_SerialManager.h:143-162`), `state[]` at the
/// object's offset 0: `n_BAUD` is at `24n`, `n_OPTIONS` at `24n + 4`,
/// `n_PROTOCOL` at `24n + 8`, measured on every board (`spike/4406` §2.3).
const UART_STATE_LEN: i64 = 24;

/// Each compiled port's protocol, baud and options, SERIAL0 first, read from
/// `AP_SerialManager::var_info` ([`group_rows`]). Every row must be one
/// [`row`] names, with no flags (so its union is the `float` default:
/// `AP_PARAM_FLAG_DEFAULT_POINTER` and the group pointers are flags,
/// `AP_Param.h:137-170`, `AP_Param.cpp:2619-2622`), its member's type, its
/// member's offset (ArduPilot applies a default by offset, not by name,
/// `AP_Param.cpp:1490-1497`), and a default that the member holds exactly.
/// Anything else, a row named twice, or a port without each of its rows, is
/// refused by name (`INV-06`).
pub fn serial_params(bytes: &[u8]) -> Result<Vec<SerialParam>, Refusal> {
    let malformed = |i: usize, why: String| {
        refuse::<()>("MalformedVarInfo", format!("var_info entry {i} {why}"))
    };
    let mut seen: Vec<&str> = Vec::new();
    // Per port: protocol, baud, options.
    let mut ports: [(Option<i8>, Option<i32>, Option<i32>); 10] = [(None, None, None); 10];
    for GroupRow {
        i,
        name,
        offset,
        bits,
        flags,
        kind,
    } in group_rows(bytes, VAR_INFO_SYMBOL, "var_info")?
    {
        if flags != 0 {
            malformed(
                i,
                format!("(`{name}`) has flags {flags:#06x}: its default is not a float"),
            )?;
        }
        let Some((row, want)) = row(name) else {
            return refuse(
                "UnknownVarInfoRow",
                format!("var_info entry {i} is `{name}`, not a SERIALn row this reader knows"),
            );
        };
        if kind != want {
            malformed(i, format!("(`{name}`) has type {kind}, not {want}"))?;
        }
        let at = match row {
            Row::Pass => None,
            Row::Baud(n) => Some(UART_STATE_LEN * i64::from(n)),
            Row::Options(n) => Some(UART_STATE_LEN * i64::from(n) + 4),
            Row::Protocol(n) => Some(UART_STATE_LEN * i64::from(n) + 8),
        };
        if let Some(at) = at.filter(|at| *at != i64::from(offset)) {
            return refuse(
                "VarInfoRowOffset",
                format!(
                    "var_info entry {i} (`{name}`) sets the member at offset {offset}, not {at} (24n + 0 BAUD, 4 OPTIONS, 8 PROTOCOL, as measured on every board): ArduPilot applies the default there, whatever the name says (AP_Param.cpp:1490-1497)"
                ),
            );
        }
        if seen.contains(&name) {
            return refuse("VarInfoRowTwice", format!("var_info names `{name}` twice"));
        }
        seen.push(name);
        let value = f32::from_bits(bits);
        // ArduPilot converts the float to the member's integer type
        // (`AP_Param.cpp:1471-1489`); only a value that type holds exactly
        // is a default this reader can state.
        let held = |lo: f32, hi: f32| {
            value.is_finite() && value.fract() == 0.0 && value >= lo && value < hi
        };
        let p = &mut ports;
        match row {
            Row::Pass => {}
            Row::Protocol(n) if held(-128.0, 128.0) => p[usize::from(n)].0 = Some(value as i8),
            Row::Baud(n) | Row::Options(n) if held(-2_147_483_648.0, 2_147_483_648.0) => {
                let v = Some(value as i32);
                match row {
                    Row::Baud(_) => p[usize::from(n)].1 = v,
                    _ => p[usize::from(n)].2 = v,
                }
            }
            _ => malformed(
                i,
                format!("(`{name}`) defaults to {value}, which its type does not hold exactly"),
            )?,
        }
    }
    let mut out = Vec::new();
    for (n, row) in ports.iter().enumerate() {
        let port = n as u8;
        match (*row, n) {
            ((None, None, None), _) => {}
            ((Some(protocol), Some(baud), None), 0) => out.push(SerialParam {
                port,
                protocol,
                baud,
                options: None,
            }),
            ((Some(protocol), Some(baud), Some(o)), 1..) => out.push(SerialParam {
                port,
                protocol,
                baud,
                options: Some(o),
            }),
            _ => {
                return refuse(
                    "VarInfoPartialPort",
                    format!(
                        "var_info holds some of SERIAL{n}'s rows but not all of _PROTOCOL, _BAUD{}",
                        if n == 0 { "" } else { " and _OPTIONS" }
                    ),
                );
            }
        }
    }
    Ok(out)
}

/// ArduPilot's GPS parameter table, `AP_GPS::var_info` (`AP_GPS.cpp:135-290`).
pub const GPS_VAR_INFO_SYMBOL: &[u8] = b"_ZN6AP_GPS8var_infoE";

/// `AP_PARAM_GROUP`, and `AP_PARAM_FLAG_NESTED_OFFSET`: a receiver's row is
/// `AP_SUBGROUPINFO(params[i], "<i+1>_", …)` (`AP_GPS.cpp:280-286`,
/// `AP_Param.h:159`, `:81`, `:192`).
const AP_PARAM_GROUP: u8 = 6;
const AP_PARAM_FLAG_NESTED_OFFSET: u16 = 1;

/// How many GPS receivers the ELF compiles, read from `AP_GPS::var_info`
/// ([`group_rows`]): one subgroup row per receiver, `1_`, `2_`, … in order
/// (`GPS_MAX_RECEIVERS`, `AP_GPS_config.h:16`; `AP_GPS.cpp:280-286`).
/// `AP_GPS::init` gives a serial port only to a receiver (`AP_GPS.cpp:345-349`),
/// so this is the most GPS ports ArduPilot can bind. `None` when the ELF has
/// no `AP_GPS::var_info` (AP_GPS is not compiled). A group row that is not
/// the next receiver's, with any other flags, is refused by name.
pub fn gps_receivers(bytes: &[u8]) -> Result<Option<u8>, Refusal> {
    if symbols_named(bytes, &header(bytes)?, GPS_VAR_INFO_SYMBOL)?.is_empty() {
        return Ok(None);
    }
    let mut receivers: u8 = 0;
    for r in group_rows(bytes, GPS_VAR_INFO_SYMBOL, "AP_GPS::var_info")? {
        if r.kind != AP_PARAM_GROUP {
            continue;
        }
        let next = receivers.saturating_add(1);
        if r.name != format!("{next}_") || r.flags != AP_PARAM_FLAG_NESTED_OFFSET {
            return refuse(
                "MalformedGpsVarInfo",
                format!(
                    "AP_GPS::var_info entry {} is a group `{}` with flags {:#06x}, not receiver {next}'s `{next}_` with AP_PARAM_FLAG_NESTED_OFFSET (AP_GPS.cpp:280-286)",
                    r.i, r.name, r.flags
                ),
            );
        }
        receivers = next;
    }
    Ok(Some(receivers))
}

/// The parameter-defaults signatures (`AET-WO-4405` D4, measured in
/// `spike/4406` §2.1): what lets ArduPilot replace the defaults `var_info`
/// compiles before it reads parameter storage (`AP_Param::load_all` calls
/// `reload_defaults_file`, `AP_Param.cpp:1563`, `:1599-1627`;
/// `get_default_value` takes an override first, `:2612-2622`).
pub const LOAD_DEFAULTS_FILE: &[u8] = b"_ZN8AP_Param18load_defaults_fileEPKcb";
pub const LOAD_EMBEDDED_PARAM_DEFAULTS: &[u8] = b"_ZN8AP_Param28load_embedded_param_defaultsEb";
pub const PARAM_DEFAULTS_DATA: &[u8] = b"_ZN8AP_Param19param_defaults_dataE";
pub const ROMFS_FILES: &[u8] = b"_ZN8AP_ROMFS5filesE";

/// `param_defaults_struct` (`AP_Param.h:682-688`, `AP_Param.cpp:134-139`):
/// `"PARMDEF"` and eight magic bytes, `uint16_t max_length`, `uint16_t
/// length`, then `max_length` bytes of `NAME value` lines.
const PARAM_DEFAULTS_MAGIC: [u8; 16] = *b"PARMDEF\0\x55\x37\xf4\xa0\x38\x5d\x48\x5b";
const PARAM_DEFAULTS_HEADER: u32 = 20;

/// ArduPilot's scripting parameter table, `AP_Scripting::var_info`
/// (`AP_Scripting.cpp:65-170`), whose `ENABLE` row is `SCR_ENABLE`
/// (`AP_GROUPINFO_FLAGS(..., SCRIPTING_ENABLE_DEFAULT, AP_PARAM_FLAG_ENABLE)`,
/// `:72`; `AP_Param.h:88`).
pub const SCRIPTING_VAR_INFO: &[u8] = b"_ZN12AP_Scripting8var_infoE";
const AP_PARAM_FLAG_ENABLE: u16 = 1 << 2;

/// `AP_ROMFS::embedded_file` on ARM32 (`AP_ROMFS.h:30-36`): `filename` (a
/// pointer), `compressed_size`, `decompressed_size`, `crc`, `contents`.
const ROMFS_ENTRY_LEN: u32 = 20;

/// The longest ROMFS file name read: a path such as `scripts/<name>.lua`.
const ROMFS_NAME_MAX: usize = 128;

/// `load_defaults_file`'s signature line: admitted only with an admitted
/// `defaults.parm`, since on ChibiOS its one caller loads that file
/// (`reload_defaults_file`, `AP_Param.cpp:1599-1632`; the SITL callers do not
/// build for this target).
const LOAD_DEFAULTS_FILE_LINKED: &str = "AP_Param::load_defaults_file is linked: a defaults file is read at boot (AP_Param.cpp:1599-1616, :2406)";

/// The file ChibiOS loads defaults from: `get_custom_defaults_file` is
/// `@ROMFS/defaults.parm` on every board (`AP_HAL_ChibiOS/Util.h:43-45`), and
/// ROMFS finds it by this name (`AP_Param.cpp:1647-1666`, `AP_ROMFS.cpp:37-46`).
const ROMFS_DEFAULTS: &str = "defaults.parm";

/// Every way the ELF can apply parameter defaults at boot over what
/// `var_info` compiles, one line each (each line below cites its source;
/// when each is linked, `spike/4406` §1.1), empty when it can apply none.
/// Lua is two of them, ruled on the work order's PR (D-c): a script in
/// ROMFS, and scripting compiled on by default, which runs a script from
/// the SD card as well.
///
/// `AP_Param::reload_defaults_file` is defined in every build and is not a
/// signature. A symbol that is absent is not read: a board without ROMFS
/// lists no file. A symbol that is present but malformed is refused by name.
///
/// One file is admitted (`AET-WO-4412`, ruled 2026-11-26T17:35Z): a ROMFS
/// `defaults.parm` that inflates to exactly `defaults`, the file `generate
/// hwdef` writes for the topology, and with it `load_defaults_file`, which
/// loads only that file here. Any other content is a signature naming its
/// first differing line; with no `defaults`, the file is a signature as
/// `AET-WO-4406` built it.
pub fn defaults_signatures(bytes: &[u8], defaults: Option<&str>) -> Result<Vec<String>, Refusal> {
    let sections = header(bytes)?;
    let named = |symbol: &[u8]| symbols_named(bytes, &sections, symbol);
    let mut found = Vec::new();
    let mut admitted = false;
    for (symbol, what) in [
        (LOAD_DEFAULTS_FILE, LOAD_DEFAULTS_FILE_LINKED),
        (
            LOAD_EMBEDDED_PARAM_DEFAULTS,
            "AP_Param::load_embedded_param_defaults is linked: defaults embedded in the image, which apj_tool can write after the build, are read at boot (AP_Param.cpp:1618-1622, :2602-2605)",
        ),
    ] {
        if !named(symbol)?.is_empty() {
            found.push(what.to_owned());
        }
    }
    if !named(PARAM_DEFAULTS_DATA)?.is_empty() {
        let data = object(bytes, PARAM_DEFAULTS_DATA, |size| {
            (size < PARAM_DEFAULTS_HEADER).then(|| {
                format!("{size} bytes, shorter than its {PARAM_DEFAULTS_HEADER}-byte header")
            })
        })?;
        let (max, length) = (u16_at(data, 16), u16_at(data, 18));
        let room = data.len() as u32 - PARAM_DEFAULTS_HEADER;
        if data[..16] != PARAM_DEFAULTS_MAGIC || u32::from(max) != room || length > max {
            return refuse(
                "MalformedParamDefaults",
                format!(
                    "AP_Param::param_defaults_data is not \"PARMDEF\" and its magic, then max_length {room} and a length within it (AP_Param.h:682-688): magic {}, max_length {max}, length {length}",
                    if data[..16] == PARAM_DEFAULTS_MAGIC {
                        "right"
                    } else {
                        "wrong"
                    }
                ),
            );
        }
        if length != 0 {
            found.push(format!(
                "AP_Param::param_defaults_data holds {length} bytes of defaults (AP_Param.cpp:1618-1622)"
            ));
        }
    }
    if !named(ROMFS_FILES)?.is_empty() {
        let data = object(bytes, ROMFS_FILES, |size| {
            // A board with no ROMFS has no such symbol (`spike/4406`
            // `out/measure-advnoromfs.txt`); an empty one is not a form
            // ArduPilot was measured to write.
            (size == 0 || size % ROMFS_ENTRY_LEN != 0).then(|| {
                format!("{size} bytes, not 1 or more whole {ROMFS_ENTRY_LEN}-byte AP_ROMFS::embedded_file entries")
            })
        })?;
        for e in data.as_chunks::<{ ROMFS_ENTRY_LEN as usize }>().0 {
            let name = flash_cstr(
                bytes,
                u32_at(e, 0),
                ROMFS_NAME_MAX,
                "ROMFS file name",
                ["RomfsNameTooLong", "MalformedRomfs"],
            )?;
            if name == ROMFS_DEFAULTS {
                let listed = format!(
                    "ROMFS lists {ROMFS_DEFAULTS}, which ChibiOS loads at boot as @ROMFS/{ROMFS_DEFAULTS} (AP_HAL_ChibiOS/Util.h:43-45, AP_Param.cpp:1647-1666)"
                );
                match defaults {
                    None => found.push(listed),
                    // The stated size is held to the topology's file first,
                    // so nothing is inflated past it: a file stating more is
                    // not the topology's, and a bomb is refused before any
                    // allocation (`AET-WO-4412` review, 23:39Z (1); INV-06).
                    Some(expected) if u32_at(e, 8) as usize != expected.len() => found.push(format!(
                        "{listed}, and it is not the one generate hwdef writes for this topology: it states {} bytes, the topology's is {}",
                        u32_at(e, 8),
                        expected.len()
                    )),
                    Some(expected) => {
                        let text = romfs_inflated(bytes, e)?;
                        match first_difference(&text, expected.as_bytes()) {
                            None => admitted = true,
                            Some((n, got, want)) => found.push(format!(
                                "{listed}, and it is not the one generate hwdef writes for this topology: its line {n} is `{got}`, the topology's `{want}`"
                            )),
                        }
                    }
                }
            } else if name.starts_with("scripts") && name.ends_with(".lua") {
                found.push(format!(
                    "ROMFS lists {name}, a Lua script ArduPilot runs from @ROMFS/scripts when scripting is enabled, and a script can set parameters (boards.py:669-672, lua_scripts.cpp:548-550)"
                ));
            }
        }
    }
    if !named(SCRIPTING_VAR_INFO)?.is_empty() {
        let rows = group_rows(bytes, SCRIPTING_VAR_INFO, "AP_Scripting::var_info")?;
        let enable: Vec<_> = rows.iter().filter(|r| r.name == "ENABLE").collect();
        let [r] = enable.as_slice() else {
            return refuse(
                "MalformedScriptingVarInfo",
                format!(
                    "AP_Scripting::var_info holds {} ENABLE rows, not one (AP_Scripting.cpp:72)",
                    enable.len()
                ),
            );
        };
        if r.kind != AP_PARAM_INT8 || r.flags != AP_PARAM_FLAG_ENABLE {
            return refuse(
                "MalformedScriptingVarInfo",
                format!(
                    "AP_Scripting::var_info entry {} (`ENABLE`) has type {} and flags {:#06x}, not AP_Int8 flagged AP_PARAM_FLAG_ENABLE alone, so its default is not a float this reader can state (AP_Scripting.cpp:72, AP_Param.h:88)",
                    r.i, r.kind, r.flags
                ),
            );
        }
        let enabled = f32::from_bits(r.bits);
        if enabled != 0.0 {
            found.push(format!(
                "AP_Scripting::var_info's ENABLE (SCR_ENABLE) defaults to {enabled}: Lua scripts, from ROMFS or the SD card, run at boot and can set parameters (AP_Scripting.cpp:55-72, :220; lua_scripts.cpp:548-550)"
            ));
        }
    }
    if admitted {
        found.retain(|line| line != LOAD_DEFAULTS_FILE_LINKED);
    }
    Ok(found)
}

/// A ROMFS entry's contents, inflated as ArduPilot does (raw deflate,
/// `AP_ROMFS.cpp`'s `find_decompress`) to exactly its `decompressed_size`;
/// anything else, a build with `HAL_ROMFS_UNCOMPRESSED` included, is refused
/// by name. Its `crc` is not read: a wrong one makes ArduPilot drop the file
/// (`AP_ROMFS.cpp:96`), never apply it.
fn romfs_inflated(bytes: &[u8], entry: &[u8]) -> Result<Vec<u8>, Refusal> {
    let (compressed, decompressed, at) = (u32_at(entry, 4), u32_at(entry, 8), u32_at(entry, 16));
    let off = usize::try_from(loaded_offset(bytes, at, compressed)?).unwrap_or(usize::MAX);
    let raw = bytes
        .get(off..off + compressed as usize)
        .unwrap_or_default();
    match miniz_oxide::inflate::decompress_to_vec_with_limit(raw, decompressed as usize) {
        Ok(text) if text.len() == decompressed as usize => Ok(text),
        _ => refuse(
            "MalformedRomfs",
            format!(
                "ROMFS {ROMFS_DEFAULTS}'s {compressed} bytes at {at:#010x} do not inflate (raw deflate, AP_ROMFS.cpp find_decompress) to its {decompressed} bytes"
            ),
        ),
    }
}

/// The first line, numbered from 1, where `got` and `want` differ, each
/// shown as text (`(none)` past either's end); `None` when they are equal.
fn first_difference(got: &[u8], want: &[u8]) -> Option<(usize, String, String)> {
    if got == want {
        return None;
    }
    let mut a = got.split(|b| *b == b'\n');
    let mut b = want.split(|b| *b == b'\n');
    (1..).find_map(|n| match (a.next(), b.next()) {
        (x, y) if x == y && x.is_some() => None,
        (x, y) => {
            let show = |l: Option<&[u8]>| {
                l.map_or("(none)".to_owned(), |l| {
                    String::from_utf8_lossy(l).into_owned()
                })
            };
            Some((n, show(x), show(y)))
        }
    })
}

/// The serial table, SERIAL0 first: a whole number of `SerialDef`s, at most
/// ten (SERIAL0 to SERIAL9), each refused unless its padding is zero.
pub fn serial_table(bytes: &[u8]) -> Result<Vec<SerialEntry>, Refusal> {
    let data = object(bytes, SERIAL_SYMBOL, |size| {
        (size == 0 || size % SERIAL_DEF_LEN != 0 || size > 10 * SERIAL_DEF_LEN).then(|| {
            format!("{size} bytes, not 1 to 10 whole {SERIAL_DEF_LEN}-byte SerialDef entries")
        })
    })?;
    let sections = sections(bytes)?;
    let mut out = Vec::new();
    for (i, e) in data
        .as_chunks::<{ SERIAL_DEF_LEN as usize }>()
        .0
        .iter()
        .enumerate()
    {
        // The layout's two padding gaps (after `dma_tx_stream_id`, and at the
        // end) are zero in every entry ArduPilot writes.
        if e[14..16] != [0, 0] || e[42..44] != [0, 0] {
            return refuse(
                "MalformedElf",
                format!(
                    "SerialDef entry {i} has bytes in its padding: not the layout this reader knows"
                ),
            );
        }
        let empty = e.iter().all(|b| *b == 0);
        let driver_is_instance = !empty && {
            let name = format!("SD{}", e[4]);
            match symbols_named(bytes, &sections, name.as_bytes())?.as_slice() {
                [(value, ..)] => *value == u32_at(e, 0),
                _ => false,
            }
        };
        out.push(SerialEntry {
            instance: e[4],
            is_usb: e[5] != 0,
            tx_line: u32_at(e, 20),
            rx_line: u32_at(e, 24),
            empty,
            driver_is_instance,
        });
    }
    Ok(out)
}
