//! `Specification` + `SolvedTopology` → `.kicad_sch` and bill of materials.
//!
//! How each pin is connected follows ERC's pin-type rules, as `AET-WO-1300`
//! S2 measured them on KiCad 10.0.7:
//! - every unused pin gets a no-connect;
//! - every rail gets a `PWR_FLAG`;
//! - each `power_out` pin (the MCU's VCAPs) gets its own net and capacitor;
//! - an input, a passive or a power-in pin may sit on a flagged rail;
//! - a bidirectional pin that must be strapped to a rail goes through a
//!   resistor (`pin_to_pin` warns otherwise).
//!
//! Every symbol sits at rotation 0. A connected pin gets a 2.54 mm wire stub,
//! outward from the body, ending in a local label or a power symbol; stacked
//! pins (one point, several numbers) are connected once. Every UUID is a
//! sha256 of a content path. Every collection is walked in key order, so the
//! text is a function of the two inputs and the vendored library alone
//! (`INV-01`).

use std::collections::{BTreeMap, BTreeSet};

use aethelgard_ir::{PinTable, SolvedTopology, Specification};
use sha2::{Digest, Sha256};

use crate::library::{self, LibraryError};
use crate::sexpr::{self, Sx, mm};

/// KiCad 10's own schematic format version (ruled D7 on PR #40).
pub const VERSION: &str = "20260306";

const GRID: i64 = 1_270_000;
const STUB: i64 = 2 * GRID;
const GND: &str = "GND";
const RAIL_SYMBOL: &str = "power:+3V3";
const GND_SYMBOL: &str = "power:GND";
const FLAG_SYMBOL: &str = "power:PWR_FLAG";
const R: &str = "Device:R";
const C: &str = "Device:C";
const R_FOOTPRINT: &str = "Resistor_SMD:R_0402_1005Metric";
const C_FOOTPRINT: &str = "Capacitor_SMD:C_0402_1005Metric";

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EmitError {
    #[error(transparent)]
    Library(#[from] LibraryError),
    #[error("`{reference}` is a `{part}`, which has no vendored model; known: {known}")]
    UnknownPart {
        reference: String,
        part: String,
        known: &'static str,
    },
    #[error("`{reference}` ({part}) cannot answer at address {address}; it can at {allowed}")]
    UnsupportedAddress {
        reference: String,
        part: String,
        address: u8,
        allowed: &'static str,
    },
    #[error("the solved topology has no pin for net `{0}`")]
    UnsolvedNet(String),
    #[error("net `{net}` is solved to pin `{pin}`, which the MCU's symbol does not have")]
    UnknownPin { net: String, pin: String },
    #[error("`{0}` names a net, rail or reference the emitter uses for itself; rename it")]
    ReservedName(String),
    #[error("`{0}` is used twice as a reference")]
    DuplicateReference(String),
    #[error("{0}")]
    Internal(String),
}

impl EmitError {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Library(_) => "LibraryCorrupt",
            Self::UnknownPart { .. } => "UnknownPart",
            Self::UnsupportedAddress { .. } => "UnsupportedAddress",
            Self::UnsolvedNet(_) => "UnsolvedNet",
            Self::UnknownPin { .. } => "UnknownPin",
            Self::ReservedName(_) => "ReservedName",
            Self::DuplicateReference(_) => "DuplicateReference",
            Self::Internal(_) => "EmitterError",
        }
    }

    pub fn id(&self) -> Option<&str> {
        match self {
            Self::UnknownPart { reference, .. } | Self::UnsupportedAddress { reference, .. } => {
                Some(reference)
            }
            Self::UnsolvedNet(id) | Self::ReservedName(id) | Self::DuplicateReference(id) => {
                Some(id)
            }
            Self::UnknownPin { net, .. } => Some(net),
            Self::Library(_) | Self::Internal(_) => None,
        }
    }
}

/// What `emit` produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Emission {
    pub schematic: String,
    pub bom: String,
}

/// A symbol pin: library coordinates in nanometres, Y up.
#[derive(Clone, Debug)]
pub struct Pin {
    pub number: String,
    pub name: String,
    pub etype: String,
    pub at: (i64, i64),
    pub angle: i64,
}

/// The pins of unit 1 (and the common unit 0), body style 1.
pub fn pins(sym: &Sx) -> Result<Vec<Pin>, String> {
    let mut out = Vec::new();
    for sub in sym.children("symbol") {
        let sub_name = sub.arg(1).unwrap_or("");
        let mut parts = sub_name.rsplitn(3, '_');
        let body = parts.next().and_then(|b| b.parse::<u32>().ok());
        let unit = parts.next().and_then(|u| u.parse::<u32>().ok());
        let (Some(unit), Some(body)) = (unit, body) else {
            return Err(format!(
                "sub-symbol `{sub_name}` has no unit and body style"
            ));
        };
        if unit > 1 || body > 1 {
            continue;
        }
        for pin in sub.children("pin") {
            let at = pin.child("at").ok_or("a pin without `at`")?;
            let coord = |i| {
                at.arg(i)
                    .and_then(sexpr::nm)
                    .ok_or_else(|| format!("pin `at` {i} unreadable"))
            };
            let angle = at.arg(3).unwrap_or("0");
            out.push(Pin {
                number: pin
                    .child("number")
                    .and_then(|n| n.arg(1))
                    .ok_or("a pin without a number")?
                    .to_owned(),
                name: pin
                    .child("name")
                    .and_then(|n| n.arg(1))
                    .ok_or("a pin without a name")?
                    .to_owned(),
                etype: pin.arg(1).ok_or("a pin without a type")?.to_owned(),
                at: (coord(1)?, coord(2)?),
                angle: angle
                    .parse::<i64>()
                    .map_err(|_| format!("pin angle `{angle}`"))?,
            });
        }
    }
    Ok(out)
}

/// What one pin is connected to.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Conn {
    Net(String),
    /// A rail, by domain id, or `GND`.
    Rail(String),
    NoConnect,
}

#[derive(Clone, Debug)]
struct Part {
    lib_id: &'static str,
    reference: String,
    value: String,
    footprint: String,
    /// By pin number; pins absent here get a no-connect.
    conns: BTreeMap<String, Conn>,
    note: Option<&'static str>,
}

/// A UUID derived from content: sha256 of a domain-separated key, the first
/// 16 bytes, with RFC 9562's version 8 and variant bits set.
pub fn uuid(key: &str) -> String {
    let h = Sha256::digest(format!("aethelgard/eda\0{key}").as_bytes());
    let mut b = [0u8; 16];
    b.copy_from_slice(&h[..16]);
    b[6] = (b[6] & 0x0f) | 0x80;
    b[8] = (b[8] & 0x3f) | 0x80;
    let hex: String = b.iter().map(|x| format!("{x:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

/// The UUID of a symbol instance, which a board's footprint `path` names.
pub fn symbol_uuid(project: &str, reference: &str) -> String {
    uuid(&format!("{project}/symbol/{reference}"))
}

/// Resistances the way schematics write them: `2k2`, `10k`, `330`, `1M`.
fn ohms(v: u32) -> String {
    let (div, unit) = match v {
        1_000_000.. => (1_000_000, 'M'),
        1_000.. => (1_000, 'k'),
        _ => return v.to_string(),
    };
    let (whole, frac) = (v / div, v % div);
    if frac == 0 {
        return format!("{whole}{unit}");
    }
    let mut digits = format!("{frac:0width$}", width = div.ilog10() as usize);
    while digits.ends_with('0') {
        digits.pop();
    }
    format!("{whole}{unit}{digits}")
}

const KNOWN_PARTS: &str = "BME280, TMP117, W25Q64JV";
const STRAP: u32 = 10_000;

struct Builder<'a> {
    spec: &'a Specification,
    parts: Vec<Part>,
    used: BTreeSet<String>,
    next: BTreeMap<char, u32>,
    rails: BTreeSet<String>,
    names: BTreeSet<String>,
}

impl Builder<'_> {
    /// The next free `U<n>`, `R<n>` or `C<n>`, skipping every reference the
    /// specification uses.
    fn reference(&mut self, prefix: char) -> String {
        let n = self.next.entry(prefix).or_insert(0);
        loop {
            *n += 1;
            let r = format!("{prefix}{n}");
            if !self.used.contains(&r) {
                self.used.insert(r.clone());
                return r;
            }
        }
    }

    /// An emitter-chosen net, refused if the specification already uses the
    /// name for something else.
    fn own_net(&mut self, name: String) -> Result<String, EmitError> {
        if !self.names.insert(name.clone()) {
            return Err(EmitError::ReservedName(name));
        }
        Ok(name)
    }

    fn passive(&mut self, kind: char, value: String, one: Conn, two: Conn) {
        let reference = self.reference(kind);
        let (lib_id, footprint) = if kind == 'R' {
            (R, R_FOOTPRINT)
        } else {
            (C, C_FOOTPRINT)
        };
        self.parts.push(Part {
            lib_id,
            reference,
            value,
            footprint: footprint.to_owned(),
            conns: BTreeMap::from([("1".into(), one), ("2".into(), two)]),
            note: None,
        });
    }

    fn rail(&mut self, domain: &str) -> Conn {
        self.rails.insert(domain.to_owned());
        Conn::Rail(domain.to_owned())
    }

    fn ground(&mut self) -> Conn {
        self.rails.insert(GND.to_owned());
        Conn::Rail(GND.to_owned())
    }

    /// A bidirectional pin strapped to a rail, through a resistor.
    fn strap(&mut self, reference: &str, pin: &str, rail: Conn) -> Result<Conn, EmitError> {
        let net = self.own_net(format!("{reference}_{pin}"))?;
        self.passive('R', ohms(STRAP), Conn::Net(net.clone()), rail);
        Ok(Conn::Net(net))
    }

    fn decouple(&mut self, rail: &str, value: &str) {
        let r = self.rail(rail);
        let g = self.ground();
        self.passive('C', value.to_owned(), r, g);
    }
}

fn by_name<'p>(pins: &'p [Pin], name: &str) -> impl Iterator<Item = &'p Pin> + 'p {
    let name = name.to_owned();
    pins.iter().filter(move |p| p.name == name)
}

/// The MCU's connection for a pin, by name. Signal pins are set later.
fn mcu_conn(b: &mut Builder<'_>, name: &str) -> Option<Conn> {
    let supply = b.spec.mcu.supply.clone();
    if name.starts_with("VDD") || name == "VBAT" || name == "VREF+" || name == "PDR_ON" {
        Some(b.rail(&supply))
    } else if name.starts_with("VSS") || name == "BOOT0" {
        Some(b.ground())
    } else {
        None
    }
}

/// Emits the schematic and bill of materials. `table` is the MCU's pin table
/// the topology was solved against: each solved pin is drawn on the symbol
/// pin whose number is the table's package position, so ST's data, not a
/// name match, decides it (KiCad names pins 28 and 29 `PC2_C` and `PC3_C`
/// where ST's table says `PC2` and `PC3`).
pub fn emit(
    spec: &Specification,
    solved: &SolvedTopology,
    table: &PinTable,
) -> Result<Emission, EmitError> {
    if spec.mcu.part != "STM32H753ZITx" || table.part != spec.mcu.part {
        return Err(EmitError::UnknownPart {
            reference: "mcu".into(),
            part: spec.mcu.part.clone(),
            known: "STM32H753ZITx",
        });
    }
    let mut used = BTreeSet::new();
    for r in spec
        .i2c_devices
        .iter()
        .map(|d| &d.reference)
        .chain(spec.spi_devices.iter().map(|d| &d.reference))
    {
        // KiCad treats a reference starting with `#` as a virtual symbol (power,
        // flag) and drops it from the netlist and BOM, and the emitter names its
        // own power symbols and flags there.
        if r.starts_with('#') {
            return Err(EmitError::ReservedName(r.clone()));
        }
        if !used.insert(r.clone()) {
            return Err(EmitError::DuplicateReference(r.clone()));
        }
    }
    // Every name the specification gives a net or a rail; an emitter net may
    // not take one.
    let mut names: BTreeSet<String> = spec.voltage_domains.iter().map(|d| d.id.clone()).collect();
    for bus in &spec.i2c_buses {
        names.extend([bus.sda.net.clone(), bus.scl.net.clone()]);
    }
    for bus in &spec.spi_buses {
        names.extend([
            bus.sck.net.clone(),
            bus.mosi.net.clone(),
            bus.miso.net.clone(),
        ]);
    }
    for d in &spec.spi_devices {
        names.insert(d.cs.net.clone());
    }
    if names.contains(GND) {
        return Err(EmitError::ReservedName(GND.into()));
    }
    names.insert(GND.into());
    let mut b = Builder {
        spec,
        parts: Vec::new(),
        used,
        next: BTreeMap::new(),
        rails: BTreeSet::new(),
        names,
    };

    // The MCU.
    let mcu_ref = b.reference('U');
    let mcu_sym = library::symbol("MCU_ST_STM32H7:STM32H753ZITx")?;
    let mcu_pins = pins(&mcu_sym).map_err(EmitError::Internal)?;
    let mut conns = BTreeMap::new();
    for p in &mcu_pins {
        if let Some(c) = mcu_conn(&mut b, &p.name) {
            conns.insert(p.number.clone(), c);
        }
    }
    let solved_pins = solved
        .assignments
        .iter()
        .map(|(net, a)| (net, &a.pin))
        .chain(solved.gpio.iter());
    for (net, pin) in solved_pins {
        let unknown = || EmitError::UnknownPin {
            net: net.clone(),
            pin: pin.clone(),
        };
        let position = table
            .pins
            .get(pin)
            .ok_or_else(unknown)?
            .position
            .to_string();
        let p = mcu_pins
            .iter()
            .find(|p| p.number == position)
            .ok_or_else(unknown)?;
        conns.insert(p.number.clone(), Conn::Net(net.clone()));
    }
    let mut vcaps: Vec<&Pin> = by_name(&mcu_pins, "VCAP").collect();
    vcaps.sort_by_key(|p| p.number.parse::<u32>().unwrap_or(u32::MAX));
    for (i, p) in vcaps.iter().enumerate() {
        let net = b.own_net(format!("{mcu_ref}_VCAP{}", i + 1))?;
        conns.insert(p.number.clone(), Conn::Net(net.clone()));
        let g = b.ground();
        b.passive('C', "2u2".into(), Conn::Net(net), g);
    }
    b.decouple(&spec.mcu.supply.clone(), "100n");
    b.parts.push(Part {
        lib_id: "MCU_ST_STM32H7:STM32H753ZITx",
        reference: mcu_ref,
        value: spec.mcu.part.clone(),
        footprint: mcu_sym.property("Footprint").unwrap_or("").to_owned(),
        conns,
        note: None,
    });

    // The I2C buses' pull-ups, then their devices.
    for bus in &spec.i2c_buses {
        for net in [&bus.sda.net, &bus.scl.net] {
            if !solved.assignments.contains_key(net) {
                return Err(EmitError::UnsolvedNet(net.clone()));
            }
            let r = b.rail(&bus.domain);
            b.passive('R', ohms(bus.pull_up_ohms), r, Conn::Net(net.clone()));
        }
    }
    for d in &spec.i2c_devices {
        let bus = spec
            .i2c_buses
            .iter()
            .find(|x| x.id == d.bus)
            .ok_or_else(|| EmitError::Internal(format!("{}: no bus {}", d.reference, d.bus)))?;
        let (sda, scl) = (
            Conn::Net(bus.sda.net.clone()),
            Conn::Net(bus.scl.net.clone()),
        );
        let rail = b.rail(&d.supply);
        let gnd = b.ground();
        let (lib_id, conns) = match d.part.as_str() {
            "BME280" => {
                let sdo = match d.address {
                    0x76 => b.strap(&d.reference, "SDO", gnd.clone())?,
                    0x77 => b.strap(&d.reference, "SDO", rail.clone())?,
                    address => {
                        return Err(EmitError::UnsupportedAddress {
                            reference: d.reference.clone(),
                            part: d.part.clone(),
                            address,
                            allowed: "118 (0x76) and 119 (0x77)",
                        });
                    }
                };
                (
                    "Sensor:BME280",
                    [
                        ("1", gnd.clone()),
                        ("7", gnd.clone()),
                        ("6", rail.clone()),
                        ("8", rail.clone()),
                        ("2", rail.clone()),
                        ("3", sda),
                        ("4", scl),
                        ("5", sdo),
                    ]
                    .to_vec(),
                )
            }
            "TMP117" => {
                let add0 = match d.address {
                    0x48 => gnd.clone(),
                    0x49 => rail.clone(),
                    0x4A => sda.clone(),
                    0x4B => scl.clone(),
                    address => {
                        return Err(EmitError::UnsupportedAddress {
                            reference: d.reference.clone(),
                            part: d.part.clone(),
                            address,
                            allowed: "72 to 75 (0x48 to 0x4B)",
                        });
                    }
                };
                (
                    "Sensor_Temperature:TMP117xxDRV",
                    [
                        ("1", scl),
                        ("2", gnd.clone()),
                        ("4", add0),
                        ("5", rail.clone()),
                        ("6", sda),
                    ]
                    .to_vec(),
                )
            }
            _ => {
                return Err(EmitError::UnknownPart {
                    reference: d.reference.clone(),
                    part: d.part.clone(),
                    known: KNOWN_PARTS,
                });
            }
        };
        device(&mut b, &d.reference, lib_id, conns)?;
        b.decouple(&d.supply, "100n");
    }
    for d in &spec.spi_devices {
        let bus = spec
            .spi_buses
            .iter()
            .find(|x| x.id == d.bus)
            .ok_or_else(|| EmitError::Internal(format!("{}: no bus {}", d.reference, d.bus)))?;
        for net in [&bus.sck.net, &bus.mosi.net, &bus.miso.net] {
            if !solved.assignments.contains_key(net) {
                return Err(EmitError::UnsolvedNet(net.clone()));
            }
        }
        if !solved.gpio.contains_key(&d.cs.net) {
            return Err(EmitError::UnsolvedNet(d.cs.net.clone()));
        }
        if d.part != "W25Q64JV" {
            return Err(EmitError::UnknownPart {
                reference: d.reference.clone(),
                part: d.part.clone(),
                known: KNOWN_PARTS,
            });
        }
        let rail = b.rail(&d.supply);
        let gnd = b.ground();
        let wp = b.strap(&d.reference, "WP", rail.clone())?;
        let hold = b.strap(&d.reference, "HOLD", rail.clone())?;
        let conns = [
            ("1", Conn::Net(d.cs.net.clone())),
            ("2", Conn::Net(bus.miso.net.clone())),
            ("3", wp),
            ("4", gnd),
            ("5", Conn::Net(bus.mosi.net.clone())),
            ("6", Conn::Net(bus.sck.net.clone())),
            ("7", hold),
            ("8", rail),
        ]
        .to_vec();
        device(&mut b, &d.reference, "Memory_Flash:W25Q32JVSS", conns)?;
        b.decouple(&d.supply, "100n");
    }
    let rails = b.rails.clone();
    let parts = b.parts;
    Ok(Emission {
        schematic: write_sheet(&spec.project_name, &parts, &rails)?,
        bom: crate::bom::write(&parts.iter().map(BomRow::from).collect::<Vec<_>>()),
    })
}

/// A device's part, with the part's default footprint and the D2 note.
fn device(
    b: &mut Builder<'_>,
    reference: &str,
    lib_id: &'static str,
    conns: Vec<(&str, Conn)>,
) -> Result<(), EmitError> {
    let sym = library::symbol(lib_id)?;
    let (value, note) = if lib_id == "Memory_Flash:W25Q32JVSS" {
        (
            "W25Q64JVSS".to_owned(),
            Some(
                "symbol Memory_Flash:W25Q32JVSS stands in for W25Q64JV: same family, same SOIC-8 208-mil pinout (AET-WO-1300 D2)",
            ),
        )
    } else {
        (sym.property("Value").unwrap_or("").to_owned(), None)
    };
    b.parts.push(Part {
        lib_id,
        reference: reference.to_owned(),
        value,
        footprint: sym.property("Footprint").unwrap_or("").to_owned(),
        conns: conns.into_iter().map(|(n, c)| (n.to_owned(), c)).collect(),
        note,
    });
    Ok(())
}

pub struct BomRow {
    pub reference: String,
    pub value: String,
    pub footprint: String,
    pub symbol: String,
    pub note: String,
}

impl From<&Part> for BomRow {
    fn from(p: &Part) -> Self {
        BomRow {
            reference: p.reference.clone(),
            value: p.value.clone(),
            footprint: p.footprint.clone(),
            symbol: p.lib_id.to_owned(),
            note: p.note.unwrap_or("").to_owned(),
        }
    }
}

// ---- the sheet ----

fn at(x: i64, y: i64, angle: i64) -> Sx {
    Sx::List(vec![
        Sx::atom("at"),
        Sx::atom(mm(x)),
        Sx::atom(mm(y)),
        Sx::atom(angle.to_string()),
    ])
}

fn uuid_sx(key: &str) -> Sx {
    Sx::List(vec![Sx::atom("uuid"), Sx::str(uuid(key))])
}

fn effects(justify: Option<&str>) -> Sx {
    let mut v = vec![
        Sx::atom("effects"),
        Sx::List(vec![Sx::atom("font"), Sx::node("size", &["1.27", "1.27"])]),
    ];
    if let Some(j) = justify {
        let mut jv = vec![Sx::atom("justify")];
        jv.extend(j.split(' ').map(Sx::atom));
        v.push(Sx::List(jv));
    }
    Sx::List(v)
}

/// Outward from the body, schematic coordinates (Y down), from the pin's
/// library angle (from its point into the body, Y up).
fn outward(angle: i64) -> Result<(i64, i64), EmitError> {
    match angle.rem_euclid(360) {
        0 => Ok((-1, 0)),
        90 => Ok((0, 1)),
        180 => Ok((1, 0)),
        270 => Ok((0, -1)),
        a => Err(EmitError::Internal(format!(
            "pin angle {a} is not a multiple of 90"
        ))),
    }
}

struct Sheet<'a> {
    project: &'a str,
    root: String,
    syms: BTreeMap<&'static str, Sx>,
    items: Vec<Sx>,
    power: usize,
}

impl Sheet<'_> {
    fn instance(
        &mut self,
        lib_id: &'static str,
        reference: &str,
        value: &str,
        footprint: Option<&str>,
        pos: (i64, i64),
        key: &str,
    ) -> Result<(), EmitError> {
        let sym = self.syms[lib_id].clone();
        let mut v = vec![
            Sx::atom("symbol"),
            Sx::List(vec![Sx::atom("lib_id"), Sx::str(lib_id)]),
            at(pos.0, pos.1, 0),
            Sx::node("unit", &["1"]),
        ];
        for flag in ["exclude_from_sim", "in_bom", "on_board"] {
            let val = sym.child(flag).and_then(|f| f.arg(1)).unwrap_or("yes");
            v.push(Sx::node(flag, &[val]));
        }
        v.push(Sx::node("dnp", &["no"]));
        v.push(uuid_sx(key));
        for prop in sym.children("property") {
            let name = prop.arg(1).unwrap_or("");
            let text = match name {
                "Reference" => reference.to_owned(),
                "Value" => value.to_owned(),
                "Footprint" => footprint
                    .map(str::to_owned)
                    .unwrap_or_else(|| prop.arg(2).unwrap_or("").to_owned()),
                _ => prop.arg(2).unwrap_or("").to_owned(),
            };
            let (px, py, pa) = match prop.child("at") {
                Some(a) => (
                    a.arg(1).and_then(sexpr::nm).unwrap_or(0),
                    a.arg(2).and_then(sexpr::nm).unwrap_or(0),
                    a.arg(3).and_then(|s| s.parse().ok()).unwrap_or(0),
                ),
                None => (0, 0, 0),
            };
            let mut p = vec![
                Sx::atom("property"),
                Sx::str(name),
                Sx::str(text),
                at(pos.0 + px, pos.1 - py, pa),
            ];
            p.extend(
                prop.items()
                    .iter()
                    .filter(|c| matches!(c.head(), Some("effects" | "hide")))
                    .cloned(),
            );
            v.push(Sx::List(p));
        }
        let mut numbers: Vec<String> = pins(&sym)
            .map_err(EmitError::Internal)?
            .into_iter()
            .map(|p| p.number)
            .collect();
        numbers.sort();
        numbers.dedup();
        for n in numbers {
            v.push(Sx::List(vec![
                Sx::atom("pin"),
                Sx::str(n.clone()),
                uuid_sx(&format!("{key}/pin/{n}")),
            ]));
        }
        v.push(Sx::List(vec![
            Sx::atom("instances"),
            Sx::List(vec![
                Sx::atom("project"),
                Sx::str(self.project),
                Sx::List(vec![
                    Sx::atom("path"),
                    Sx::str(format!("/{}", self.root)),
                    Sx::List(vec![Sx::atom("reference"), Sx::str(reference)]),
                    Sx::node("unit", &["1"]),
                ]),
            ]),
        ]));
        self.items.push(Sx::List(v));
        Ok(())
    }

    fn power_symbol(&mut self, rail: &str, pos: (i64, i64)) -> Result<(), EmitError> {
        self.power += 1;
        let r = format!("#PWR{:03}", self.power);
        let lib = if rail == GND { GND_SYMBOL } else { RAIL_SYMBOL };
        let key = format!("{}/power/{r}", self.project);
        self.instance(lib, &r, rail, None, pos, &key)
    }

    fn wire(&mut self, a: (i64, i64), b: (i64, i64), key: &str) {
        self.items.push(Sx::List(vec![
            Sx::atom("wire"),
            Sx::List(vec![
                Sx::atom("pts"),
                Sx::node("xy", &[&mm(a.0), &mm(a.1)]),
                Sx::node("xy", &[&mm(b.0), &mm(b.1)]),
            ]),
            Sx::List(vec![
                Sx::atom("stroke"),
                Sx::node("width", &["0"]),
                Sx::node("type", &["default"]),
            ]),
            uuid_sx(key),
        ]));
    }

    fn label(&mut self, text: &str, pos: (i64, i64), dir: (i64, i64), key: &str) {
        let (angle, justify) = match dir {
            (1, 0) => (0, "left bottom"),
            (-1, 0) => (180, "right bottom"),
            (0, -1) => (90, "left bottom"),
            _ => (270, "right bottom"),
        };
        self.items.push(Sx::List(vec![
            Sx::atom("label"),
            Sx::str(text),
            at(pos.0, pos.1, angle),
            Sx::node("fields_autoplaced", &["yes"]),
            effects(Some(justify)),
            uuid_sx(key),
        ]));
    }

    fn part(&mut self, part: &Part, pos: (i64, i64)) -> Result<(), EmitError> {
        let key = format!("{}/symbol/{}", self.project, part.reference);
        self.instance(
            part.lib_id,
            &part.reference,
            &part.value,
            Some(&part.footprint),
            pos,
            &key,
        )?;
        let pins = pins(&self.syms[part.lib_id]).map_err(EmitError::Internal)?;
        for n in part.conns.keys() {
            if !pins.iter().any(|p| &p.number == n) {
                return Err(EmitError::Internal(format!(
                    "{}: `{}` has no pin {n}",
                    part.reference, part.lib_id
                )));
            }
        }
        let mut groups: BTreeMap<(i64, i64), Vec<&Pin>> = BTreeMap::new();
        for p in &pins {
            groups.entry(p.at).or_default().push(p);
        }
        for (pt, mut group) in groups {
            group.sort_by(|a, b| a.number.cmp(&b.number));
            let conns: Vec<(&str, &Conn)> = group
                .iter()
                .filter_map(|p| part.conns.get(&p.number).map(|c| (p.number.as_str(), c)))
                .collect();
            let first = group[0].number.as_str();
            let conn = match conns.first() {
                None => &Conn::NoConnect,
                Some((_, c)) => c,
            };
            if conns.iter().any(|(_, c)| c != &conn)
                || (!conns.is_empty() && conns.len() != group.len())
            {
                return Err(EmitError::Internal(format!(
                    "{}: stacked pins at one point disagree",
                    part.reference
                )));
            }
            let p = (pos.0 + pt.0, pos.1 - pt.1);
            let pkey = format!("{key}/pin/{first}");
            let dir = outward(group[0].angle)?;
            let end = (p.0 + dir.0 * STUB, p.1 + dir.1 * STUB);
            match conn {
                Conn::NoConnect => self.items.push(Sx::List(vec![
                    Sx::atom("no_connect"),
                    Sx::node("at", &[&mm(p.0), &mm(p.1)]),
                    uuid_sx(&format!("{pkey}/nc")),
                ])),
                Conn::Net(net) => {
                    self.wire(p, end, &format!("{pkey}/wire"));
                    self.label(net, end, dir, &format!("{pkey}/label"));
                }
                Conn::Rail(rail) => {
                    self.wire(p, end, &format!("{pkey}/wire"));
                    self.power_symbol(rail, end)?;
                }
            }
        }
        Ok(())
    }
}

/// A part's extent in sheet units around its origin: (min x, min y, max x,
/// max y), schematic Y down, stubs included.
fn extent(sym: &Sx) -> Result<(i64, i64, i64, i64), EmitError> {
    let pins = pins(sym).map_err(EmitError::Internal)?;
    let mut e = (0, 0, 0, 0);
    for p in pins {
        let (x, y) = (p.at.0, -p.at.1);
        e.0 = e.0.min(x - STUB);
        e.1 = e.1.min(y - STUB);
        e.2 = e.2.max(x + STUB);
        e.3 = e.3.max(y + STUB);
    }
    Ok(e)
}

fn snap(v: i64) -> i64 {
    v.div_euclid(GRID) * GRID
}

fn write_sheet(
    project: &str,
    parts: &[Part],
    rails: &BTreeSet<String>,
) -> Result<String, EmitError> {
    let root = uuid(&format!("{project}/root"));
    let mut ids: BTreeSet<&'static str> = parts.iter().map(|p| p.lib_id).collect();
    ids.extend([RAIL_SYMBOL, GND_SYMBOL, FLAG_SYMBOL]);
    let mut syms = BTreeMap::new();
    for id in ids {
        syms.insert(id, library::symbol(id)?);
    }
    let mut sheet = Sheet {
        project,
        root: root.clone(),
        syms,
        items: Vec::new(),
        power: 0,
    };
    // Flags: one row at the top, a power symbol and a PWR_FLAG on one point.
    let mut flags = 0;
    for rail in rails {
        flags += 1;
        let pos = (GRID * 20 * flags, GRID * 10);
        sheet.power_symbol(rail, pos)?;
        let r = format!("#FLG{flags:03}");
        let key = format!("{project}/flag/{r}");
        sheet.instance(FLAG_SYMBOL, &r, "PWR_FLAG", None, pos, &key)?;
    }
    // Parts in reference order, in rows of cells sized to the largest part
    // but the MCU, which has a cell of its own on the left. A cell's margin
    // keeps every pin, stub and label point apart from every other part's.
    let mut sorted: Vec<&Part> = parts.iter().collect();
    sorted.sort_by(|a, b| a.reference.cmp(&b.reference));
    for w in sorted.windows(2) {
        if w[0].reference == w[1].reference {
            return Err(EmitError::DuplicateReference(w[0].reference.clone()));
        }
    }
    let margin = 8 * GRID;
    let top = GRID * 30;
    let mut cell = (0, 0);
    let mut extents = BTreeMap::new();
    for p in &sorted {
        let e = extent(&sheet.syms[p.lib_id])?;
        extents.insert(p.reference.clone(), e);
        if !p.lib_id.starts_with("MCU_") {
            cell.0 = cell.0.max(e.2 - e.0 + margin);
            cell.1 = cell.1.max(e.3 - e.1 + margin);
        }
    }
    let mut x0 = GRID * 20;
    for p in sorted.iter().filter(|p| p.lib_id.starts_with("MCU_")) {
        let e = extents[&p.reference];
        sheet.part(p, (snap(x0 - e.0), snap(top - e.1)))?;
        x0 += e.2 - e.0 + margin;
    }
    let per_row = 6;
    for (i, p) in sorted
        .iter()
        .filter(|p| !p.lib_id.starts_with("MCU_"))
        .enumerate()
    {
        let e = extents[&p.reference];
        let (cx, cy) = (
            x0 + cell.0 * (i as i64 % per_row),
            top + cell.1 * (i as i64 / per_row),
        );
        sheet.part(p, (snap(cx - e.0), snap(cy - e.1)))?;
    }
    let mut lib = vec![Sx::atom("lib_symbols")];
    lib.extend(sheet.syms.values().cloned());
    let mut out = vec![
        Sx::atom("kicad_sch"),
        Sx::node("version", &[VERSION]),
        Sx::List(vec![Sx::atom("generator"), Sx::str("aethelgard")]),
        Sx::List(vec![
            Sx::atom("generator_version"),
            Sx::str(env!("CARGO_PKG_VERSION")),
        ]),
        Sx::List(vec![Sx::atom("uuid"), Sx::str(root)]),
        Sx::List(vec![Sx::atom("paper"), Sx::str("A0")]),
        Sx::List(vec![
            Sx::atom("title_block"),
            Sx::List(vec![Sx::atom("title"), Sx::str(project)]),
        ]),
        Sx::List(lib),
    ];
    out.extend(sheet.items);
    out.push(Sx::List(vec![
        Sx::atom("sheet_instances"),
        Sx::List(vec![
            Sx::atom("path"),
            Sx::str("/"),
            Sx::List(vec![Sx::atom("page"), Sx::str("1")]),
        ]),
    ]));
    Ok(sexpr::write(&Sx::List(out)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resistances_read_as_schematics_write_them() {
        assert_eq!(ohms(2200), "2k2");
        assert_eq!(ohms(4700), "4k7");
        assert_eq!(ohms(10_000), "10k");
        assert_eq!(ohms(330), "330");
        assert_eq!(ohms(1_000_000), "1M");
        assert_eq!(ohms(1_500_000), "1M5");
        assert_eq!(ohms(1_050), "1k05");
    }

    #[test]
    fn uuids_are_version_8_and_a_function_of_the_key() {
        let a = uuid("p/symbol/R1");
        assert_eq!(a, uuid("p/symbol/R1"));
        assert_ne!(a, uuid("p/symbol/R2"));
        assert_eq!(a.len(), 36);
        assert_eq!(&a[14..15], "8");
        assert!(matches!(&a[19..20], "8" | "9" | "a" | "b"));
    }

    #[test]
    fn every_mcu_pin_rule_names_pins_the_symbol_has() {
        let sym = library::symbol("MCU_ST_STM32H7:STM32H753ZITx").unwrap();
        let pins = pins(&sym).unwrap();
        assert_eq!(pins.len(), 144);
        for name in [
            "VDD", "VBAT", "VREF+", "PDR_ON", "VSS", "BOOT0", "VCAP", "PB8", "PB9",
        ] {
            assert!(pins.iter().any(|p| p.name == name), "{name}");
        }
        let vcap: Vec<&str> = by_name(&pins, "VCAP").map(|p| p.etype.as_str()).collect();
        assert_eq!(vcap, ["power_out", "power_out"]);
    }

    #[test]
    fn device_pin_numbers_match_their_symbols() {
        for (id, expect) in [
            (
                "Sensor:BME280",
                vec![
                    ("1", "GND"),
                    ("2", "CSB"),
                    ("3", "SDI"),
                    ("4", "SCK"),
                    ("5", "SDO"),
                    ("6", "VDDIO"),
                    ("7", "GND"),
                    ("8", "VDD"),
                ],
            ),
            (
                "Sensor_Temperature:TMP117xxDRV",
                vec![
                    ("1", "SCL"),
                    ("2", "GND"),
                    ("3", "ALERT"),
                    ("4", "ADD0"),
                    ("5", "V+"),
                    ("6", "SDA"),
                ],
            ),
            (
                "Memory_Flash:W25Q32JVSS",
                vec![
                    ("1", "~{CS}"),
                    ("2", "DO/IO_{1}"),
                    ("3", "~{WP}/IO_{2}"),
                    ("4", "GND"),
                    ("5", "DI/IO_{0}"),
                    ("6", "CLK"),
                    ("7", "~{HOLD}/~{RESET}/IO_{3}"),
                    ("8", "VCC"),
                ],
            ),
        ] {
            let sym = library::symbol(id).unwrap();
            let mut got: Vec<(String, String)> = pins(&sym)
                .unwrap()
                .into_iter()
                .map(|p| (p.number, p.name))
                .collect();
            got.sort();
            let want: Vec<(String, String)> = expect
                .iter()
                .map(|(n, m)| ((*n).into(), (*m).into()))
                .collect();
            assert_eq!(got, want, "{id}");
        }
    }
}
