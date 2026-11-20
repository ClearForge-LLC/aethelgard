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
const STRTAB: &[u8] = b"\0gpio_default_config\0other\0_ZN7ChibiOS10UARTDriver11_serial_tabE\0SD1\0";

/// `_serial_tab`'s name's offset in [`STRTAB`] (`AET-WO-4400b2`).
const SERIAL_NAME: u32 = 27;

/// `SD1`'s name's offset in [`STRTAB`], and where the synthetic ELF puts it:
/// the driver a serial entry for USART1 points at.
const SD1_NAME: u32 = 65;
pub const SD1: u32 = 0x3000_1000;

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
    pub etype: u16,
    /// The segment's load address, if not where it runs.
    pub paddr: Option<u32>,
    /// A second PT_LOAD flashing 4 other bytes over the symbol's first word.
    pub overlay: bool,
    /// `.rodata` marked writable.
    pub writable: bool,
    /// `(n, len)`: `n` more symbols, each named by one name of `len` bytes
    /// appended to the string table (review 2, A15.1).
    pub crowd: Option<(u32, u32)>,
    /// The string table's last NUL left off.
    pub unterminated: bool,
    /// ArduPilot's serial table, 44 bytes per SERIALn (`AET-WO-4400b2`),
    /// after `gpio_default_config` in the same section; `None` leaves it out.
    pub serial: Option<Vec<[u8; 44]>>,
    /// Functions defined in `.rodata`'s section by name (`AET-WO-4400d`: an
    /// IMU driver's probe), each appended to the string table.
    pub functions: Vec<String>,
    /// More `STT_OBJECT`s by name and address (an `SD<n>` beyond `SD1`).
    pub objects: Vec<(String, u32)>,
    /// `AP_SerialManager::var_info` (`AET-WO-4405`): its rows, after the
    /// serial table in the same section, each name a string after them;
    /// `None` leaves the symbol out.
    pub var_info: Option<Vec<VarRow>>,
    /// The `var_info` symbol's size, if not 16 bytes per row.
    pub var_info_size: Option<u32>,
}

/// One synthetic `AP_Param::GroupInfo` (`AP_Param.h:206-218`).
#[derive(Clone, Debug)]
pub struct VarRow {
    pub name: String,
    pub offset: u32,
    /// The union's bits: a float default.
    pub value: u32,
    pub flags: u16,
    pub idx: u8,
    pub kind: u8,
    /// Where the name pointer points, if not at the row's own string.
    pub name_at: Option<u32>,
}

/// A row as `AP_GROUPINFO` writes it: no flags, `default` a float.
pub fn var_row(name: &str, kind: u8, default: f32) -> VarRow {
    VarRow {
        name: name.to_owned(),
        offset: 0,
        value: default.to_bits(),
        flags: 0,
        idx: 0,
        kind,
        name_at: None,
    }
}

/// `AP_GROUPEND` (`AP_Param.h:170`).
pub fn group_end() -> VarRow {
    VarRow {
        idx: 0xff,
        ..var_row("", 0, 0.0)
    }
}

/// One port's `(protocol, baud, options)` as `var_info` holds them, the baud
/// before `map_baudrate`; options `None` for SERIAL0.
pub type Port = (i8, i32, Option<i32>);

/// `var_info` as the tag's source orders it (`AP_SerialManager.cpp:164-400`)
/// for `ports`, SERIAL0 first: `0_BAUD`, `0_PROTOCOL`; `n_PROTOCOL`, `n_BAUD`
/// for 1 to 6; `n_OPTIONS` for 1 to 6; the three pass-through rows; then
/// `n_PROTOCOL`, `n_BAUD`, `n_OPTIONS` for 7 to 9; and `AP_GROUPEND`.
pub fn var_info(ports: &[Port]) -> Vec<VarRow> {
    let (int8, int32) = (1, 3);
    let mut rows = Vec::new();
    let options =
        |n: usize, o: Option<i32>| var_row(&format!("{n}_OPTIONS"), int32, o.unwrap() as f32);
    for (n, &(protocol, baud, _)) in ports.iter().enumerate().take(7) {
        if n == 0 {
            rows.push(var_row("0_BAUD", int32, baud as f32));
            rows.push(var_row("0_PROTOCOL", int8, f32::from(protocol)));
        } else {
            rows.push(var_row(&format!("{n}_PROTOCOL"), int8, f32::from(protocol)));
            rows.push(var_row(&format!("{n}_BAUD"), int32, baud as f32));
        }
    }
    for (n, &(.., o)) in ports.iter().enumerate().take(7).skip(1) {
        rows.push(options(n, o));
    }
    rows.push(var_row("_PASS1", int8, 0.0));
    rows.push(var_row("_PASS2", int8, -1.0));
    rows.push(var_row("_PASSTIMO", int8, 15.0));
    for (n, &(protocol, baud, o)) in ports.iter().enumerate().skip(7) {
        rows.push(var_row(&format!("{n}_PROTOCOL"), int8, f32::from(protocol)));
        rows.push(var_row(&format!("{n}_BAUD"), int32, baud as f32));
        rows.push(options(n, o));
    }
    rows.push(group_end());
    rows
}

/// `var_info` with SERIAL0 alone, MAVLink 2 at `115200/1000`: sentinel-01's,
/// measured (`spike/4405`). [`Elf::default`] holds no `var_info`, so the
/// GPIO tests' section is the GPIO image alone.
pub const SERIAL0_ONLY: [Port; 1] = [(2, 115, None)];

/// Step G's registers as the `4400` spike decoded them by hand (FEEDBACK §5):
/// GPIOA `MODER` `0x0000A801` (PA0 output, PA5–PA7 alternate) on AF5; PB6 and
/// PB7 alternate on AF4; PD8 and PD9 alternate on AF7. Every other pin input.
/// The drive is what the topology implies (`AET-WO-4400b` §1.3): PB6 and PB7
/// open-drain (`OTYPER` B `0xC0`), PA5 and PA7 at the stored `01` (`OSPEEDR`
/// A `0x4400`), every other field 0. [`step_g_defaults`] is the drive
/// ArduPilot gives a hand-written step G instead.
pub fn step_g_words() -> [[u32; 7]; 11] {
    let mut w = [[0u32; 7]; 11];
    w[0][0] = 0x0000_A801;
    w[0][2] = 0x0000_4400;
    w[0][5] = 0x5550_0000;
    w[1][0] = 0x0000_A000;
    w[1][1] = 0x0000_00C0;
    w[1][5] = 0x4400_0000;
    w[3][0] = 0x000A_0000;
    w[3][6] = 0x0000_0077;
    w
}

/// Step G with ArduPilot's own defaults, as `4400a` §2.4 measured them on
/// the CI ELF: every `OSPEEDR` `10` (`0xAAAAAAAA`), and pull-ups on PA0 (a CS)
/// and on PD8 and PD9 (a USART).
pub fn step_g_defaults() -> [[u32; 7]; 11] {
    let mut w = step_g_words();
    for port in &mut w {
        port[2] = 0xAAAA_AAAA;
    }
    w[0][3] = 0x0000_0001;
    w[3][3] = 0x0005_0000;
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
            etype: 2,
            paddr: None,
            overlay: false,
            writable: false,
            crowd: None,
            unterminated: false,
            serial: None,
            functions: Vec::new(),
            objects: Vec::new(),
            var_info: None,
            var_info_size: None,
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
        let serial_at = ADDR + data.len() as u32;
        if let Some(entries) = &self.serial {
            for e in entries {
                data.extend_from_slice(e);
            }
        }
        let var_at = ADDR + data.len() as u32;
        if let Some(rows) = &self.var_info {
            let names_at = var_at + 16 * rows.len() as u32;
            let (mut entries, mut names) = (Vec::new(), Vec::new());
            for r in rows {
                put32(
                    &mut entries,
                    r.name_at.unwrap_or(names_at + names.len() as u32),
                );
                put32(&mut entries, r.offset);
                put32(&mut entries, r.value);
                put16(&mut entries, r.flags);
                entries.push(r.idx);
                entries.push(r.kind);
                names.extend_from_slice(r.name.as_bytes());
                names.push(0);
            }
            data.extend(entries);
            data.extend(names);
        }
        let mut syms = Vec::new();
        symbol(&mut syms, 0, 0, 0, 0, 0);
        if let Some(entries) = &self.serial {
            symbol(
                &mut syms,
                SERIAL_NAME,
                serial_at,
                44 * entries.len() as u32,
                1,
                1,
            );
            symbol(&mut syms, SD1_NAME, SD1, 100, 1, 1);
        }
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
        let mut strtab = STRTAB.to_vec();
        for f in &self.functions {
            symbol(&mut syms, strtab.len() as u32, ADDR, 4, 2, 1);
            strtab.extend_from_slice(f.as_bytes());
            strtab.push(0);
        }
        for (name, at) in &self.objects {
            symbol(&mut syms, strtab.len() as u32, *at, 100, 1, 1);
            strtab.extend_from_slice(name.as_bytes());
            strtab.push(0);
        }
        if let Some((n, len)) = self.crowd {
            for _ in 0..n {
                symbol(&mut syms, STRTAB.len() as u32, ADDR, 4, 1, 1);
            }
            strtab.extend(std::iter::repeat_n(b'g', len as usize));
            strtab.push(0);
        }
        // Last, so the names before it keep their offsets.
        if let Some(rows) = &self.var_info {
            let size = self.var_info_size.unwrap_or(16 * rows.len() as u32);
            symbol(&mut syms, strtab.len() as u32, var_at, size, 1, 1);
            strtab.extend_from_slice(aethelgard_pincheck::elf::VAR_INFO_SYMBOL);
            strtab.push(0);
        }
        if self.unterminated {
            strtab.pop();
        }
        let symtab_off = DATA_OFF + data.len();
        let strtab_off = symtab_off + syms.len();
        let shoff = strtab_off + strtab.len();

        let mut b = Vec::new();
        b.extend_from_slice(b"\x7fELF");
        b.extend_from_slice(&[self.class, self.data, 1, 0]);
        b.extend_from_slice(&[0; 8]);
        put16(&mut b, self.etype);
        put16(&mut b, self.machine);
        put32(&mut b, 1);
        put32(&mut b, ADDR); // entry
        // The program headers go after the section headers, so a second one
        // moves no other offset; the PHDR bytes at EHDR stay zero.
        let phoff = shoff + 4 * 40;
        let phnum: u16 = if self.overlay { 2 } else { 1 };
        put32(&mut b, phoff as u32);
        put32(&mut b, shoff as u32);
        put32(&mut b, 0x0500_0400);
        for v in [EHDR as u16, PHDR as u16, phnum, 40, 4, 0] {
            put16(&mut b, v);
        }
        b.extend_from_slice(&[0; PHDR]);
        b.extend_from_slice(&data);
        b.extend_from_slice(&syms);
        b.extend_from_slice(&strtab);
        section(&mut b, [0, 0, 0, 0, 0, 0, 0, 0]);
        let rodata = self.section_offset.unwrap_or(DATA_OFF as u32);
        let flags = if self.writable { 3 } else { 2 };
        section(&mut b, [0, 1, flags, ADDR, rodata, data.len() as u32, 0, 0]);
        section(
            &mut b,
            [0, 2, 0, 0, symtab_off as u32, syms.len() as u32, 3, 16],
        );
        section(
            &mut b,
            [0, 3, 0, 0, strtab_off as u32, strtab.len() as u32, 0, 0],
        );
        // One PT_LOAD: the data, run at ADDR and flashed at `paddr`.
        let len = data.len() as u32;
        let paddr = self.paddr.unwrap_or(ADDR);
        for v in [1, DATA_OFF as u32, ADDR, paddr, len, len, 4, 4] {
            put32(&mut b, v);
        }
        if self.overlay {
            // Four bytes of the symbol table, flashed at ADDR, run in RAM.
            for v in [1, symtab_off as u32, 0x3000_0000, ADDR, 4, 4, 6, 4] {
                put32(&mut b, v);
            }
        }
        b
    }
}

/// The reference topology as `aethelgard solve` gives it for
/// `examples/sentinel-01-minimal.json`; regenerate it with `aethelgard solve`
/// when the solved schema changes (schema 5 since `AET-WO-4300d`).
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
                    ..Default::default()
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
    s + IDENTITY
}

/// The reference board's device tables as ArduPilot writes them into
/// `hwdef.h` (`AET-WO-4400b2`): the flash on SPI1 with its CS on PA0 at the
/// achieved 16 MHz in mode 0, I2C1 with the BME280 probed as BMP280 at 0x76,
/// and the I2C clock at its declared 400 kHz.
pub const IDENTITY: &str = "\
#define MHZ (1000U*1000U)
#define KHZ (1000U)
#define HAL_I2C_MAX_CLOCK 400000
#define HAL_SPI1_CONFIG { &SPID1, 1, STM32_SPI_SPI1_DMA_STREAMS, PAL_LINE(GPIOA,5U) }
#define HAL_SPI_BUS_LIST HAL_SPI1_CONFIG
#define HAL_SPI_DEVICE0  SPIDesc(\"dataflash\"      ,  0,  1, PAL_LINE(GPIOA,0U) , SPIDEV_MODE0,  16*MHZ,  16*MHZ)
#define HAL_SPI_DEVICE_LIST \\
   HAL_SPI_DEVICE0
#define HAL_BARO_PROBE1 probe_i2c_dev(AP_Baro_BMP280::probe, 0, 118); RETURN_IF_NO_SPACE;
#define HAL_BARO_PROBE_LIST HAL_BARO_PROBE1
#if defined(STM32_I2C_I2C1_RX_DMA_STREAM) && defined(STM32_I2C_I2C1_TX_DMA_STREAM)
#define HAL_I2C1_CONFIG { &I2CD1, 1, STM32_I2C_I2C1_RX_DMA_STREAM, STM32_I2C_I2C1_TX_DMA_STREAM, PAL_LINE(GPIOB,6U), PAL_LINE(GPIOB,7U) }
#else
#define HAL_I2C1_CONFIG { &I2CD1, 1, SHARED_DMA_NONE, SHARED_DMA_NONE, PAL_LINE(GPIOB,6U), PAL_LINE(GPIOB,7U) }
#endif
#define HAL_I2C_DEVICE_LIST \\
   HAL_I2C1_CONFIG
";

pub fn allow() -> aethelgard_pincheck::AllowList {
    aethelgard_pincheck::AllowList::parse(include_bytes!("../../fixtures/step-g.allow.json"))
        .unwrap()
}

/// Invensensev3's probe as the CI-built ELF names it (`AET-WO-4400d`,
/// measured with `nm` on the generated sentinel-02's `arducopter.elf`).
pub const INVENSENSEV3_PROBE: &str = "_ZN30AP_InertialSensor_Invensensev35probeER17AP_InertialSensorN6AP_HAL6OwnPtrINS2_6DeviceEEE8Rotation";

/// Where the synthetic ELF puts `SD<n>`: `SD1` at [`SD1`], each other `n`
/// 0x100 further per instance.
pub fn sd_address(instance: u8) -> u32 {
    SD1 + 0x100 * (u32::from(instance).max(1) - 1)
}

/// A decoded serial entry back as its 44 bytes: the driver pointer
/// `SD<instance>` exactly when it is the instance's own driver, else another
/// address; the instance, USB flag and lines.
pub fn serial_bytes(e: &aethelgard_pincheck::elf::SerialEntry) -> [u8; 44] {
    let mut b = [0u8; 44];
    if e.empty {
        return b;
    }
    let driver = if e.driver_is_instance {
        sd_address(e.instance)
    } else {
        sd_address(e.instance) + 0x64
    };
    b[..4].copy_from_slice(&driver.to_le_bytes());
    b[4] = e.instance;
    b[5] = u8::from(e.is_usb);
    b[20..24].copy_from_slice(&e.tx_line.to_le_bytes());
    b[24..28].copy_from_slice(&e.rx_line.to_le_bytes());
    b
}
