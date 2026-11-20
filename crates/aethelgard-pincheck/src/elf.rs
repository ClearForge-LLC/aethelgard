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
pub fn read(bytes: &[u8]) -> Result<GpioConfig, Refusal> {
    let data = object(bytes, SYMBOL, |size| {
        (size != SYMBOL_SIZE)
            .then(|| format!("{size} bytes, not {SYMBOL_SIZE} (11 ports x 7 words)"))
    })?;
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

/// The bytes of the one `STT_OBJECT` named `symbol`, in allocated, read-only
/// `PROGBITS`, located twice (its section and its `PT_LOAD`) and refused
/// unless both agree; `bad_size` names a size the caller refuses.
fn object<'a>(
    bytes: &'a [u8],
    symbol: &[u8],
    bad_size: impl Fn(u32) -> Option<String>,
) -> Result<&'a [u8], Refusal> {
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

/// The NUL-terminated string at `addr`, read from flash: it must lie in an
/// allocated, read-only `PROGBITS` section and in the one `PT_LOAD` that
/// flashes it, at the same file offset by both (as [`object`] holds a
/// symbol), and end within [`NAME_MAX`] bytes.
fn flash_str<'a>(bytes: &'a [u8], sections: &[Section], addr: u32) -> Result<&'a str, Refusal> {
    let outside = || {
        refuse(
            "VarInfoNameOutside",
            format!(
                "a var_info name at {addr:#010x} is not a string of at most {NAME_MAX} bytes in flashed, read-only PROGBITS"
            ),
        )
    };
    let Some(s) = sections.iter().find(|s| {
        s.kind == SHT_PROGBITS
            && s.flags & SHF_ALLOC != 0
            && s.flags & SHF_WRITE == 0
            && addr >= s.addr
            && u64::from(addr) < u64::from(s.addr) + u64::from(s.size)
    }) else {
        return outside();
    };
    let within = range(
        bytes,
        u64::from(s.offset),
        s.size,
        "a var_info name's section",
    )?;
    let tail = &within[(addr - s.addr) as usize..];
    let Some(len) = tail.iter().take(NAME_MAX + 1).position(|b| *b == 0) else {
        return outside();
    };
    let by_segment = loaded_offset(bytes, addr, len as u32 + 1)?;
    if by_segment != u64::from(s.offset) + u64::from(addr - s.addr) {
        return refuse(
            "SectionTableDisagrees",
            format!(
                "the var_info name at {addr:#010x} is at different file offsets by its section and its PT_LOAD"
            ),
        );
    }
    std::str::from_utf8(&tail[..len]).or_else(|_| outside())
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

/// Each compiled port's protocol, baud and options, SERIAL0 first, read from
/// `AP_SerialManager::var_info`. Whole 16-byte entries are read up to the
/// first whose type is `AP_PARAM_NONE`, where ArduPilot stops
/// (`AP_Param.cpp:1498-1500`); that entry must be exactly `AP_GROUPEND`
/// (`AP_Param.h:170`) and the symbol's last. Every other row must be one
/// [`row`] names, with no flags (so its union is the `float` default:
/// `AP_PARAM_FLAG_DEFAULT_POINTER` and the group pointers are flags,
/// `AP_Param.h:137-170`, `AP_Param.cpp:2619-2622`), its member's type, and a
/// default that the member holds exactly. Anything else, a row named twice,
/// or a port without each of its rows, is refused by name (`INV-06`).
pub fn serial_params(bytes: &[u8]) -> Result<Vec<SerialParam>, Refusal> {
    let data = object(bytes, VAR_INFO_SYMBOL, |size| {
        (size == 0 || !(size as usize).is_multiple_of(GROUP_INFO_LEN)).then(|| {
            format!("{size} bytes, not whole {GROUP_INFO_LEN}-byte AP_Param::GroupInfo entries")
        })
    })?;
    let sections = sections(bytes)?;
    let entries = data.as_chunks::<GROUP_INFO_LEN>().0;
    let malformed = |i: usize, why: String| {
        refuse::<()>("MalformedVarInfo", format!("var_info entry {i} {why}"))
    };
    let mut seen: Vec<String> = Vec::new();
    // Per port: protocol, baud, options.
    let mut ports: [(Option<i8>, Option<i32>, Option<i32>); 10] = [(None, None, None); 10];
    let mut ended = false;
    for (i, e) in entries.iter().enumerate() {
        let (name_at, flags, idx, kind) = (u32_at(e, 0), u16_at(e, 12), e[14], e[15]);
        let name = flash_str(bytes, &sections, name_at)?;
        if kind == AP_PARAM_NONE {
            if i + 1 != entries.len() || !name.is_empty() || e[4..14] != [0; 10] || idx != 0xff {
                malformed(
                    i,
                    "has type AP_PARAM_NONE but is not AP_GROUPEND ({\"\", 0, 0, 0, 0xFF, 0}) as the table's last entry".to_owned(),
                )?;
            }
            ended = true;
            break;
        }
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
        if seen.iter().any(|s| s == name) {
            return refuse("VarInfoRowTwice", format!("var_info names `{name}` twice"));
        }
        seen.push(name.to_owned());
        let value = f32::from_bits(u32_at(e, 8));
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
    if !ended {
        return refuse(
            "VarInfoUnterminated",
            format!("var_info's {} entries hold no AP_GROUPEND", entries.len()),
        );
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
