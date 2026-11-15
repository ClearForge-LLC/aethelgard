//! A synthetic ELF32 little-endian `EM_ARM` firmware holding one
//! `gpio_default_config`, built byte by byte so tier 1 needs no toolchain and
//! commits no ArduPilot binary. Each field a test may corrupt is a knob.

#![allow(dead_code)]

use std::collections::BTreeMap;

use aethelgard_ir::{Pin, PinSource, PinTable, SolvedTopology};
use aethelgard_pincheck::PORTS;

pub const ADDR: u32 = 0x0800_1000;
pub const EHDR: usize = 52;
pub const PHDR: usize = 32;
pub const DATA_OFF: usize = EHDR + PHDR;
const STRTAB: &[u8] = b"\0gpio_default_config\0other\0";

/// The knobs.
#[derive(Clone)]
pub struct Elf {
    pub words: [[u32; 7]; 11],
    pub machine: u16,
    pub class: u8,
    pub data: u8,
    pub sym_size: u32,
    pub sym_value: u32,
    /// A second symbol named `gpio_default_config`.
    pub duplicate: bool,
    /// What `.rodata`'s section header says its file offset is.
    pub section_offset: Option<u32>,
    /// The section index the symbol names.
    pub shndx: u16,
    /// Make the symbol a function.
    pub function: bool,
}

/// Step G's registers as the `4400` spike decoded them by hand (FEEDBACK §5):
/// GPIOA `MODER` `0x0000A801` (PA0 output, PA5–PA7 alternate) on AF5; PB6 and
/// PB7 alternate on AF4; PD8 and PD9 alternate on AF7. Every other pin input.
pub fn step_g_words() -> [[u32; 7]; 11] {
    let mut w = [[0u32; 7]; 11];
    w[0][0] = 0x0000_A801;
    w[0][5] = 0x5550_0000;
    w[1][0] = 0x0000_A000;
    w[1][5] = 0x4400_0000;
    w[3][0] = 0x000A_0000;
    w[3][6] = 0x0000_0077;
    w
}

impl Default for Elf {
    fn default() -> Self {
        Self {
            words: step_g_words(),
            machine: 40,
            class: 1,
            data: 1,
            sym_size: 308,
            sym_value: ADDR,
            duplicate: false,
            section_offset: None,
            shndx: 1,
            function: false,
        }
    }
}

fn put16(b: &mut Vec<u8>, v: u16) {
    b.extend_from_slice(&v.to_le_bytes());
}

fn put32(b: &mut Vec<u8>, v: u32) {
    b.extend_from_slice(&v.to_le_bytes());
}

/// One section header: name, type, flags, addr, offset, size, link, entsize.
fn section(b: &mut Vec<u8>, [name, kind, flags, addr, off, size, link, entsize]: [u32; 8]) {
    for v in [name, kind, flags, addr, off, size, link, 0, 4, entsize] {
        put32(b, v);
    }
}

fn symbol(b: &mut Vec<u8>, name: u32, value: u32, size: u32, info: u8, shndx: u16) {
    put32(b, name);
    put32(b, value);
    put32(b, size);
    b.push(info);
    b.push(0);
    put16(b, shndx);
}

impl Elf {
    pub fn bytes(&self) -> Vec<u8> {
        let mut data = Vec::new();
        for port in &self.words {
            for &w in port {
                put32(&mut data, w);
            }
        }
        let mut syms = Vec::new();
        symbol(&mut syms, 0, 0, 0, 0, 0);
        let info = if self.function { 2 } else { 1 };
        symbol(
            &mut syms,
            1,
            self.sym_value,
            self.sym_size,
            info,
            self.shndx,
        );
        symbol(&mut syms, 21, ADDR, 4, 1, 1);
        if self.duplicate {
            symbol(&mut syms, 1, ADDR, 308, 1, 1);
        }
        let symtab_off = DATA_OFF + data.len();
        let strtab_off = symtab_off + syms.len();
        let shoff = strtab_off + STRTAB.len();

        let mut b = Vec::new();
        b.extend_from_slice(b"\x7fELF");
        b.extend_from_slice(&[self.class, self.data, 1, 0]);
        b.extend_from_slice(&[0; 8]);
        put16(&mut b, 2); // ET_EXEC
        put16(&mut b, self.machine);
        put32(&mut b, 1);
        put32(&mut b, ADDR); // entry
        put32(&mut b, EHDR as u32); // phoff
        put32(&mut b, shoff as u32);
        put32(&mut b, 0x0500_0400);
        for v in [EHDR as u16, PHDR as u16, 1, 40, 4, 0] {
            put16(&mut b, v);
        }
        // One PT_LOAD: the data, flashed at ADDR.
        for v in [
            1,
            DATA_OFF as u32,
            ADDR,
            ADDR,
            data.len() as u32,
            data.len() as u32,
            4,
            4,
        ] {
            put32(&mut b, v);
        }
        b.extend_from_slice(&data);
        b.extend_from_slice(&syms);
        b.extend_from_slice(STRTAB);
        section(&mut b, [0, 0, 0, 0, 0, 0, 0, 0]);
        let rodata = self.section_offset.unwrap_or(DATA_OFF as u32);
        section(&mut b, [0, 1, 2, ADDR, rodata, data.len() as u32, 0, 0]);
        section(
            &mut b,
            [0, 2, 0, 0, symtab_off as u32, syms.len() as u32, 3, 16],
        );
        section(
            &mut b,
            [0, 3, 0, 0, strtab_off as u32, STRTAB.len() as u32, 0, 0],
        );
        b
    }
}

/// The reference topology as `aethelgard solve` gives it for
/// `examples/sentinel-01-minimal.json` (the spike's input, schema 3).
pub fn topology() -> SolvedTopology {
    serde_json::from_str(include_str!("topology.json")).unwrap()
}

/// A pin table holding `pins` and no other: the check reads only which pins exist.
pub fn package(pins: &[&str]) -> PinTable {
    PinTable {
        part: aethelgard_pincheck::PART.into(),
        package: "LQFP144".into(),
        source: PinSource {
            repository: String::new(),
            commit: String::new(),
            files: BTreeMap::new(),
        },
        pins: pins
            .iter()
            .map(|p| {
                let pin = Pin {
                    position: 0,
                    alternate_functions: BTreeMap::new(),
                };
                ((*p).to_owned(), pin)
            })
            .collect(),
    }
}

/// Every pin of ports A–H, as on LQFP144 to the check's eye; no port I, J or K.
pub fn lqfp_like() -> PinTable {
    let names: Vec<String> = PORTS[..8]
        .iter()
        .flat_map(|p| (0..16).map(move |n| format!("P{p}{n}")))
        .collect();
    package(&names.iter().map(String::as_str).collect::<Vec<_>>())
}

/// A `hwdef.h` holding exactly the three macros of every port for `words`, in
/// ArduPilot's layout.
pub fn header(words: &[[u32; 7]; 11]) -> String {
    let mode = ["INPUT", "OUTPUT", "ALTERNATE", "ANALOG"];
    let mut s = String::from("// generated\n\n");
    for (port, w) in PORTS.iter().zip(words) {
        let terms: Vec<String> = (0..16)
            .map(|n| format!("PIN_MODE_{}({n}U)", mode[((w[0] >> (2 * n)) & 3) as usize]))
            .collect();
        s += &format!(
            "#define VAL_GPIO{port}_MODER   ({})\n\n",
            terms.join(" | \\\n                           ")
        );
        for (name, word, base) in [("AFRL", w[5], 0), ("AFRH", w[6], 8)] {
            let terms: Vec<String> = (0..8)
                .map(|k| format!("PIN_AFIO_AF({}U, {}U)", base + k, (word >> (4 * k)) & 0xf))
                .collect();
            s += &format!(
                "#define VAL_GPIO{port}_{name}    ({})\n\n",
                terms.join(" | \\\n                           ")
            );
        }
    }
    s
}

pub fn allow() -> aethelgard_pincheck::AllowList {
    aethelgard_pincheck::AllowList::parse(include_bytes!("../../fixtures/step-g.allow.json"))
        .unwrap()
}
