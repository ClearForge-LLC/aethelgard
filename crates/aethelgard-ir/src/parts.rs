//! The parts table (`AET-WO-4200` D3, review B7): what each part is, as data.
//!
//! The table is `data/parts.json`, embedded in this crate and pinned by its
//! sha256 in a tier-1 test. One parser, [`parse_parts`], reads it and every
//! test fixture; it never panics, and refuses a malformed entry by naming the
//! part and the field (`INV-06`). The embedded table reaches a user only
//! through [`table`], whose error `validate` returns like any other refusal.
//!
//! Each entry holds the part's values: its bus (I2C addresses, or SPI modes
//! and clock), its identity read, its supply range, the largest capacitance of
//! any of its signal pins where the datasheet prints a maximum
//! (`AET-WO-4300c` D3), and the Renode model that stands in for it, or none. Each value group carries its datasheet citation:
//! the document, revision and section, and the document's sha256 when it is
//! held, or the URL it must be fetched from (`pending`), never a value from
//! memory. What a Renode model logs is not a part value: that stays code in
//! `aethelgard-twin`, matched on the model (`AET-WO-2204` D2). The generators
//! and the judge read the same entry (`INV-01`); neither takes an expected
//! value from the other (`INV-07`).
//!
//! Schema 3 (`AET-WO-4210`, ruled on PR #131) adds what a device's bring-up
//! needs beyond its identity: a part's [`Lifecycle`], cited to the vendor page
//! held (D1); whether it is a twin stand-in, never meant for a purchase; its
//! I/O range where the datasheet separates it from the supply (D7); and its
//! transaction [`Script`]: one configuration write and one data read after
//! the identity, each cited. A data read's expected value is the datasheet's
//! conversion of the stimulus the twin sets (D3), computed here from the
//! table alone, so the generator and the judge share it and neither takes it
//! from the other.

use std::collections::BTreeSet;
use std::sync::LazyLock;

use serde::Deserialize;

use crate::IrValidationError;

/// The table as embedded: `data/parts.json`.
pub const PARTS_JSON: &str = include_str!("../data/parts.json");

/// The `schema_version` the parser reads.
/// Version 2 added `pin_capacitance` and named the flash for its ordering
/// code (`AET-WO-4300c` D3, F11). Version 3 made the identity's write a byte
/// list with a mask, and added `lifecycle`, `stand_in`, `io_millivolts` and
/// `script` (`AET-WO-4210`).
pub const PARTS_SCHEMA_VERSION: u32 = 3;

/// A bus kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BusKind {
    I2c,
    Spi,
}

/// The Renode 1.17.0 models a part may name, each with the buses it attaches
/// to. A name outside this set is refused; `aethelgard-twin` maps each to its
/// own model, exhaustively.
pub const RENODE_MODELS: [(&str, &[BusKind]); 4] = [
    ("I2C.BME280", &[BusKind::I2c]),
    ("SPI.GenericSpiFlash", &[BusKind::Spi]),
    ("Sensors.ICM20948", &[BusKind::I2c, BusKind::Spi]),
    ("Sensors.ICP_101xx", &[BusKind::I2c]),
];

/// The most bytes a step writes: the ICP-10111's 16-bit commands are 2.
pub const MAX_STEP_WRITE: usize = 4;

/// The most bytes a data read takes: two per channel, at most four channels,
/// so one record's value fits 64 bits (`AET-WO-4210` S2, F18's `n <= 9`).
pub const MAX_CHANNELS: usize = 4;

/// The highest SPI clock an entry may state: no part in the table comes near
/// it, and anything above is a typo.
pub const MAX_SPI_CLOCK_HZ: u32 = 1_000_000_000;

/// The highest supply an entry may state, in millivolts.
pub const MAX_SUPPLY_MILLIVOLTS: u32 = 60_000;

/// The highest pin capacitance an entry may state, in femtofarads: no
/// part's signal pin comes near 100 pF, and anything above is a typo.
pub const MAX_PIN_FEMTOFARADS: u64 = 100_000;

/// The SPI command that reads a JEDEC identity (`9Fh`), the identity's width,
/// and the capacity codes (`2^code` bytes) a `SPI.GenericSpiFlash` is built
/// from: the shape the twin's emitter requires (`aethelgard-twin`
/// `FlashIdentity`), checked here so a malformed one never reaches
/// `generate` (`AET-WO-4300c` F20).
pub const JEDEC_READ: u8 = 0x9F;
pub const JEDEC_WIDTH: u8 = 3;
pub const FLASH_CAPACITY_CODES: std::ops::RangeInclusive<u8> = 0x10..=0x19;

/// The name the W25Q64JV had before its ordering code was part of it
/// (`AET-WO-4300c` F11). Its JEDEC identity depends on that code, so a
/// specification naming it is refused, naming both.
pub const BARE_W25Q64JV: &str = "W25Q64JV";

/// How a part sits on its bus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Bus {
    /// The 7-bit addresses its straps allow, each from 0x08 to 0x77.
    I2c { addresses: Vec<u8> },
    /// The SPI modes it accepts (CPOL × 2 + CPHA), and its highest clock.
    Spi { modes: Vec<u8>, max_clock_hz: u32 },
}

impl Bus {
    pub fn kind(&self) -> BusKind {
        match self {
            Bus::I2c { .. } => BusKind::I2c,
            Bus::Spi { .. } => BusKind::Spi,
        }
    }
}

/// One identity read: the bytes written first (an I2C register or a 16-bit
/// command, or an SPI command with the read bit as the part needs it), then
/// `width` bytes (1 to 4) read back, which a present part answers with
/// `value` under `mask`, the first byte on the wire as the most significant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    pub write: Vec<u8>,
    pub width: u8,
    pub value: u32,
    /// The bits the identity fixes; all of them unless the table says less
    /// (the ICP-10111's bits 15:6 are unspecified, `AET-WO-4210` F6).
    pub mask: u32,
    /// `false` when no held datasheet states the value: allowed only for a
    /// twin stand-in, whose identity is then cited to the model's source and
    /// a driver (`AET-WO-4210` D2).
    pub datasheet_value: bool,
}

/// A part's production status on its vendor's page (`AET-WO-4210` D1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lifecycle {
    Active,
    /// Not recommended for new designs.
    Nrnd,
    /// End of life: refused on the fabrication path, by name.
    Eol,
}

impl Lifecycle {
    pub fn name(self) -> &'static str {
        match self {
            Lifecycle::Active => "active",
            Lifecycle::Nrnd => "nrnd",
            Lifecycle::Eol => "eol",
        }
    }
}

/// A physical quantity the twin can set on a model before a run (the
/// stimulus, `AET-WO-4210` D3), each in one unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quantity {
    /// In g.
    AccelerationX,
    AccelerationY,
    AccelerationZ,
    /// In degrees Celsius.
    Temperature,
}

/// Every quantity, its table name and its unit.
pub const QUANTITIES: [(Quantity, &str, &str); 4] = [
    (Quantity::AccelerationX, "acceleration_x", "g"),
    (Quantity::AccelerationY, "acceleration_y", "g"),
    (Quantity::AccelerationZ, "acceleration_z", "g"),
    (Quantity::Temperature, "temperature", "celsius"),
];

impl Quantity {
    pub fn name(self) -> &'static str {
        QUANTITIES.iter().find(|q| q.0 == self).map_or("", |q| q.1)
    }
}

/// One 16-bit word of a data read, high byte first: the quantity the twin
/// sets to `stimulus`, and the datasheet's conversion,
/// `raw = round((stimulus - zero) * lsb.0 / lsb.1)`, in two's complement when
/// `signed`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Channel {
    pub quantity: Quantity,
    pub stimulus: i32,
    pub zero: i32,
    pub lsb: (u32, u32),
    pub signed: bool,
}

impl Channel {
    /// The word the part answers for the stimulus, rounded half away from
    /// zero. The parser refused a channel whose word does not fit 16 bits.
    pub fn raw(&self) -> u16 {
        // Two's complement in 16 bits; `channel` checked the range.
        (self.rounded() & 0xFFFF) as u16
    }

    /// `round((stimulus - zero) * lsb.0 / lsb.1)`, half away from zero, in
    /// 128 bits, where no `i32` and `u32` inputs overflow (adversarial A5).
    fn rounded(&self) -> i128 {
        let num = (i128::from(self.stimulus) - i128::from(self.zero)) * i128::from(self.lsb.0);
        let den = i128::from(self.lsb.1);
        (2 * num + num.signum() * den) / (2 * den)
    }
}

/// What a device's bring-up does after its identity (`AET-WO-4210` S2): one
/// configuration write, then one data read, each in its own transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Script {
    /// The bytes the configuration writes; nothing is read back.
    pub configure: Vec<u8>,
    /// The bytes the data read writes first (a register or a command).
    pub data: Vec<u8>,
    /// The words it reads back, in order: two bytes each.
    pub channels: Vec<Channel>,
}

/// Which step of a device's script.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StepKind {
    Identity,
    Configure,
    Data,
}

impl StepKind {
    pub fn name(self) -> &'static str {
        match self {
            StepKind::Identity => "identity",
            StepKind::Configure => "configure",
            StepKind::Data => "data",
        }
    }
}

/// One transaction of a device's script, as the firmware makes it and the
/// judge expects it: write `write`, then read `width` bytes (none for a
/// configuration), which a present part answers with `expected` under
/// `mask`, the first byte on the wire as the most significant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub kind: StepKind,
    pub write: Vec<u8>,
    pub width: u8,
    pub expected: u64,
    pub mask: u64,
}

/// Where a value group was read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Citation {
    pub document: String,
    pub revision: String,
    pub section: String,
    pub held: Held,
}

/// Whether the cited document is held.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Held {
    /// The document's sha256, 64 lowercase hex digits.
    Sha256(String),
    /// Not held: the URL it must be fetched from (`AET-WO-4200` D8). The
    /// value stands, marked, until it is.
    Pending(String),
}

/// One citation per value group; `io`, `configure` and `data` exactly when
/// the part has an I/O range and a script.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Citations {
    pub bus: Citation,
    pub identity: Citation,
    pub supply: Citation,
    pub io: Option<Citation>,
    pub pin_capacitance: Citation,
    pub lifecycle: Citation,
    pub configure: Option<Citation>,
    pub data: Option<Citation>,
}

impl Citations {
    /// Every citation the entry holds, with its value group's name.
    pub fn all(&self) -> Vec<(&'static str, &Citation)> {
        let mut v = vec![
            ("bus", &self.bus),
            ("identity", &self.identity),
            ("supply", &self.supply),
        ];
        v.extend(self.io.as_ref().map(|c| ("io", c)));
        v.push(("pin_capacitance", &self.pin_capacitance));
        v.push(("lifecycle", &self.lifecycle));
        v.extend(self.configure.as_ref().map(|c| ("configure", c)));
        v.extend(self.data.as_ref().map(|c| ("data", c)));
        v
    }
}

/// One part.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Part {
    pub part: String,
    pub bus: Bus,
    pub identity: Identity,
    /// The vendor's production status (`AET-WO-4210` D1).
    pub lifecycle: Lifecycle,
    /// In the table to exercise the twin, never for a purchase
    /// (`AET-WO-4210` D1). Only a stand-in may carry an identity no held
    /// datasheet states.
    pub stand_in: bool,
    /// The supply range, (min, max) in millivolts.
    pub supply_millivolts: (u32, u32),
    /// The range its signal pins accept, (min, max) in millivolts, where the
    /// datasheet separates it from the supply (`VDDIO`); `None` when the
    /// supply range is the I/O range (`AET-WO-4210` D7).
    pub io_millivolts: Option<(u32, u32)>,
    /// The largest maximum capacitance the datasheet prints for any of the
    /// part's signal pins, in femtofarads, or `None` when it prints none: a
    /// typical value is never promoted to a maximum (`AET-WO-4300c` D3).
    pub pin_capacitance_femtofarads: Option<u64>,
    /// The Renode model, as `platforms` names it, or `None`: unmodelled.
    pub renode: Option<String>,
    /// The bring-up after the identity, or `None`: the identity alone.
    pub script: Option<Script>,
    pub citations: Citations,
}

impl Part {
    /// The range a device's bus domain must lie in: the I/O range where the
    /// datasheet gives one, else the supply's (`AET-WO-4210` D7).
    pub fn io_range(&self) -> ((u32, u32), &Citation) {
        match (self.io_millivolts, &self.citations.io) {
            (Some(r), Some(c)) => (r, c),
            _ => (self.supply_millivolts, &self.citations.supply),
        }
    }

    /// The device's transactions, in the order the firmware makes them and
    /// the judge expects them: the identity, then the script's
    /// configuration and data read, if it has one.
    pub fn steps(&self) -> Vec<Step> {
        let id = &self.identity;
        let mut steps = vec![Step {
            kind: StepKind::Identity,
            write: id.write.clone(),
            width: id.width,
            expected: u64::from(id.value),
            mask: u64::from(id.mask),
        }];
        if let Some(s) = &self.script {
            steps.push(Step {
                kind: StepKind::Configure,
                write: s.configure.clone(),
                width: 0,
                expected: 0,
                mask: 0,
            });
            let width = u8::try_from(2 * s.channels.len()).unwrap_or(u8::MAX);
            let expected = s
                .channels
                .iter()
                .fold(0u64, |v, c| (v << 16) | u64::from(c.raw()));
            steps.push(Step {
                kind: StepKind::Data,
                write: s.data.clone(),
                width,
                expected,
                mask: if width >= 8 {
                    u64::MAX
                } else {
                    (1u64 << (8 * u32::from(width))) - 1
                },
            });
        }
        steps
    }
}

/// The parsed table, sorted by part name, each name once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartsTable {
    parts: Vec<Part>,
}

impl PartsTable {
    pub fn parts(&self) -> &[Part] {
        &self.parts
    }

    pub fn get(&self, part: &str) -> Option<&Part> {
        self.parts
            .binary_search_by(|p| p.part.as_str().cmp(part))
            .ok()
            .map(|i| &self.parts[i])
    }

    /// Every citation whose document is not held: (part, value group, URL).
    pub fn pending(&self) -> Vec<(&str, &'static str, &str)> {
        let mut out = Vec::new();
        for p in &self.parts {
            for (group, c) in p.citations.all() {
                if let Held::Pending(url) = &c.held {
                    out.push((p.part.as_str(), group, url.as_str()));
                }
            }
        }
        out
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawTable {
    schema_version: u32,
    /// Each entry's own text, parsed strictly on its own (`RawPart`), so a
    /// key given twice in an entry is refused, never collapsed (`AET-WO-4200`
    /// adversarial A2).
    parts: Vec<Box<serde_json::value::RawValue>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPart {
    part: String,
    lifecycle: String,
    stand_in: bool,
    bus: RawBus,
    identity: RawIdentity,
    supply_millivolts: RawSupply,
    /// Required: `null` says the supply range is the I/O range.
    #[serde(deserialize_with = "Option::deserialize")]
    io_millivolts: Option<RawSupply>,
    pin_capacitance: RawPinCapacitance,
    /// Required: `null` says unmodelled; an absent key is refused, never read
    /// as unmodelled (adversarial A2).
    #[serde(deserialize_with = "Option::deserialize")]
    renode: Option<String>,
    /// Required: `null` says the identity alone.
    #[serde(deserialize_with = "Option::deserialize")]
    script: Option<RawScript>,
    citations: RawCitations,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawScript {
    configure: RawWrite,
    data: RawData,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawWrite {
    write: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawData {
    write: Vec<String>,
    channels: Vec<RawChannel>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawChannel {
    quantity: String,
    unit: String,
    stimulus: i32,
    zero: i32,
    lsb: [u32; 2],
    signed: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "lowercase")]
enum RawBus {
    I2c(RawI2c),
    Spi(RawSpi),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawI2c {
    addresses: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSpi {
    modes: Vec<u8>,
    max_clock_hz: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawIdentity {
    write: Vec<String>,
    read: u8,
    value: String,
    /// Required: `null` says every bit of the value is fixed.
    #[serde(deserialize_with = "Option::deserialize")]
    mask: Option<String>,
    datasheet_value: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSupply {
    min: u32,
    max: u32,
}

/// Required: `null` says the datasheet prints no maximum; an absent key is
/// refused, never read as none.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPinCapacitance {
    #[serde(deserialize_with = "Option::deserialize")]
    max_femtofarads: Option<u64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCitations {
    bus: RawCitation,
    identity: RawCitation,
    supply: RawCitation,
    /// Required: `null` exactly when `io_millivolts` is.
    #[serde(deserialize_with = "Option::deserialize")]
    io: Option<RawCitation>,
    pin_capacitance: RawCitation,
    lifecycle: RawCitation,
    /// Required: `null` exactly when `script` is.
    #[serde(deserialize_with = "Option::deserialize")]
    configure: Option<RawCitation>,
    #[serde(deserialize_with = "Option::deserialize")]
    data: Option<RawCitation>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawCitation {
    document: String,
    revision: String,
    section: String,
    sha256: Option<String>,
    pending: Option<String>,
}

fn refused(part: Option<&str>, reason: impl Into<String>) -> IrValidationError {
    IrValidationError::PartsTable {
        part: part.map(str::to_owned),
        reason: reason.into(),
    }
}

/// `0x` then 1 to 8 upper-case hexadecimal digits, at most `max`.
fn hex(text: &str, field: &str, max: u32) -> Result<u32, String> {
    let digits = text
        .strip_prefix("0x")
        .filter(|d| (1..=8).contains(&d.len()))
        .filter(|d| {
            d.bytes()
                .all(|b| b.is_ascii_digit() || (b'A'..=b'F').contains(&b))
        })
        .ok_or_else(|| {
            format!("`{field}` is `{text}`, not `0x` then 1 to 8 upper-case hex digits")
        })?;
    let v = u32::from_str_radix(digits, 16).map_err(|e| format!("`{field}`: {e}"))?;
    if v > max {
        return Err(format!("`{field}` is {text}, above {max:#X}"));
    }
    Ok(v)
}

/// Printable ASCII, 1 to `max` bytes.
fn text(value: &str, field: &str, max: usize) -> Result<(), String> {
    if value.is_empty() || value.len() > max || !value.bytes().all(|b| (0x20..=0x7E).contains(&b)) {
        return Err(format!(
            "`{field}` is not 1 to {max} bytes of printable ASCII"
        ));
    }
    Ok(())
}

pub(crate) fn citation(raw: RawCitation, group: &str) -> Result<Citation, String> {
    text(&raw.document, &format!("citations.{group}.document"), 128)?;
    text(&raw.revision, &format!("citations.{group}.revision"), 64)?;
    text(&raw.section, &format!("citations.{group}.section"), 256)?;
    let held = match (raw.sha256, raw.pending) {
        (Some(h), None) => {
            if h.len() != 64 || !h.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) {
                return Err(format!(
                    "`citations.{group}.sha256` is not 64 lower-case hex digits"
                ));
            }
            Held::Sha256(h)
        }
        (None, Some(url)) => {
            text(&url, &format!("citations.{group}.pending"), 256)?;
            if url.strip_prefix("https://").is_none_or(str::is_empty) {
                return Err(format!("`citations.{group}.pending` is not an https URL"));
            }
            Held::Pending(url)
        }
        _ => {
            return Err(format!(
                "`citations.{group}` has neither or both of `sha256` and `pending`"
            ));
        }
    };
    Ok(Citation {
        document: raw.document,
        revision: raw.revision,
        section: raw.section,
        held,
    })
}

/// 1 to [`MAX_STEP_WRITE`] bytes, each `0x` then 1 or 2 upper-case hex digits.
fn bytes(list: &[String], field: &str) -> Result<Vec<u8>, String> {
    if list.is_empty() || list.len() > MAX_STEP_WRITE {
        return Err(format!("`{field}` is not 1 to {MAX_STEP_WRITE} bytes"));
    }
    list.iter()
        .map(|b| u8::try_from(hex(b, field, 0xFF)?).map_err(|e| e.to_string()))
        .collect()
}

/// A millivolt range: 1 <= min <= max <= [`MAX_SUPPLY_MILLIVOLTS`].
fn millivolts(raw: &RawSupply, field: &str) -> Result<(u32, u32), String> {
    let (min, max) = (raw.min, raw.max);
    if min == 0 || min > max || max > MAX_SUPPLY_MILLIVOLTS {
        return Err(format!(
            "`{field}` is {min} to {max}, not 1 <= min <= max <= {MAX_SUPPLY_MILLIVOLTS}"
        ));
    }
    Ok((min, max))
}

fn channel(raw: &RawChannel, at: usize) -> Result<Channel, String> {
    let field = format!("script.data.channels[{at}]");
    let Some(&(quantity, _, unit)) = QUANTITIES.iter().find(|q| q.1 == raw.quantity) else {
        return Err(format!(
            "`{field}.quantity` is `{}`, not a quantity the twin can set",
            raw.quantity
        ));
    };
    if raw.unit != unit {
        return Err(format!(
            "`{field}.unit` is `{}`, but `{}` is in `{unit}`",
            raw.unit, raw.quantity
        ));
    }
    let [num, den] = raw.lsb;
    if num == 0 || den == 0 {
        return Err(format!("`{field}.lsb` holds a zero"));
    }
    let c = Channel {
        quantity,
        stimulus: raw.stimulus,
        zero: raw.zero,
        lsb: (num, den),
        signed: raw.signed,
    };
    let rounded = c.rounded();
    let range = if c.signed {
        -32_768..=32_767
    } else {
        0..=65_535
    };
    if !range.contains(&rounded) {
        return Err(format!(
            "`{field}`: the stimulus converts to {rounded}, which a {} 16-bit word cannot hold",
            if c.signed { "signed" } else { "unsigned" }
        ));
    }
    Ok(c)
}

fn script(raw: &RawScript) -> Result<Script, String> {
    let configure = bytes(&raw.configure.write, "script.configure.write")?;
    let data = bytes(&raw.data.write, "script.data.write")?;
    if raw.data.channels.is_empty() || raw.data.channels.len() > MAX_CHANNELS {
        return Err(format!(
            "`script.data.channels` is not 1 to {MAX_CHANNELS} channels"
        ));
    }
    let channels = raw
        .data
        .channels
        .iter()
        .enumerate()
        .map(|(i, c)| channel(c, i))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Script {
        configure,
        data,
        channels,
    })
}

/// A citation present exactly when its value group is.
fn paired(
    raw: Option<RawCitation>,
    present: bool,
    group: &str,
    value: &str,
) -> Result<Option<Citation>, String> {
    match (raw, present) {
        (Some(c), true) => citation(c, group).map(Some),
        (None, false) => Ok(None),
        (Some(_), false) => Err(format!(
            "`citations.{group}` is given but `{value}` is null"
        )),
        (None, true) => Err(format!(
            "`citations.{group}` is null but `{value}` is given"
        )),
    }
}

fn part(raw: RawPart) -> Result<Part, String> {
    let lifecycle = match raw.lifecycle.as_str() {
        "active" => Lifecycle::Active,
        "nrnd" => Lifecycle::Nrnd,
        "eol" => Lifecycle::Eol,
        other => {
            return Err(format!(
                "`lifecycle` is `{other}`, not `active`, `nrnd` or `eol`"
            ));
        }
    };
    let bus = match raw.bus {
        RawBus::I2c(i2c) => {
            if i2c.addresses.is_empty() {
                return Err("`bus.i2c.addresses` is empty".to_owned());
            }
            let mut addresses = Vec::new();
            for a in &i2c.addresses {
                let v = hex(a, "bus.i2c.addresses", 0x77)?;
                if v < 0x08 {
                    return Err(format!("`bus.i2c.addresses` holds {a}, below 0x08"));
                }
                let v = u8::try_from(v).map_err(|e| e.to_string())?;
                if addresses.contains(&v) {
                    return Err(format!("`bus.i2c.addresses` holds {a} twice"));
                }
                addresses.push(v);
            }
            Bus::I2c { addresses }
        }
        RawBus::Spi(spi) => {
            if spi.modes.is_empty() {
                return Err("`bus.spi.modes` is empty".to_owned());
            }
            let mut seen = BTreeSet::new();
            for &m in &spi.modes {
                if m > 3 {
                    return Err(format!("`bus.spi.modes` holds {m}, not 0 to 3"));
                }
                if !seen.insert(m) {
                    return Err(format!("`bus.spi.modes` holds {m} twice"));
                }
            }
            if !(1..=MAX_SPI_CLOCK_HZ).contains(&spi.max_clock_hz) {
                return Err(format!(
                    "`bus.spi.max_clock_hz` is {}, not 1 to {MAX_SPI_CLOCK_HZ}",
                    spi.max_clock_hz
                ));
            }
            Bus::Spi {
                modes: spi.modes,
                max_clock_hz: spi.max_clock_hz,
            }
        }
    };
    let width = raw.identity.read;
    if !(1..=4).contains(&width) {
        return Err(format!("`identity.read` is {width}, not 1 to 4"));
    }
    let write = bytes(&raw.identity.write, "identity.write")?;
    let max = if width == 4 {
        u32::MAX
    } else {
        (1u32 << (8 * width)) - 1
    };
    let value = hex(&raw.identity.value, "identity.value", max)?;
    let mask = match &raw.identity.mask {
        None => max,
        Some(m) => {
            let m = hex(m, "identity.mask", max)?;
            if m == 0 || value & !m != 0 {
                return Err(format!(
                    "`identity.mask` is {m:#X}: zero, or it leaves bits of the value unfixed"
                ));
            }
            m
        }
    };
    // A stand-in is in the table to exercise the twin, never for a purchase,
    // so it must carry a lifecycle the specification has to acknowledge and
    // the banner names (adversarial A6).
    if raw.stand_in && lifecycle == Lifecycle::Active {
        return Err("`stand_in` is true for an `active` part: a twin stand-in is `eol` or `nrnd`, so a board that uses it must acknowledge it".to_owned());
    }
    if !raw.identity.datasheet_value && !raw.stand_in {
        return Err(
            "`identity.datasheet_value` is false, which only a twin stand-in may be".to_owned(),
        );
    }
    let (min, max) = millivolts(&raw.supply_millivolts, "supply_millivolts")?;
    let io = raw
        .io_millivolts
        .as_ref()
        .map(|r| millivolts(r, "io_millivolts"))
        .transpose()?;
    let script = raw.script.as_ref().map(script).transpose()?;
    let pin_capacitance = raw.pin_capacitance.max_femtofarads;
    if pin_capacitance.is_some_and(|c| !(1..=MAX_PIN_FEMTOFARADS).contains(&c)) {
        return Err(format!(
            "`pin_capacitance.max_femtofarads` is {pin_capacitance:?}, not null or 1 to {MAX_PIN_FEMTOFARADS}"
        ));
    }
    if let Some(model) = &raw.renode {
        let Some((_, buses)) = RENODE_MODELS.iter().find(|(m, _)| m == model) else {
            return Err(format!(
                "`renode` names `{model}`, which is not a known model"
            ));
        };
        if !buses.contains(&bus.kind()) {
            return Err(format!(
                "`renode` names `{model}`, which does not attach to the part's bus"
            ));
        }
        let capacity = u8::try_from(value & 0xFF).map_err(|e| e.to_string())?;
        if model == "SPI.GenericSpiFlash"
            && (write != [JEDEC_READ]
                || width != JEDEC_WIDTH
                || !FLASH_CAPACITY_CODES.contains(&capacity))
        {
            return Err(format!(
                "`identity` is not a JEDEC identity a `SPI.GenericSpiFlash` is built from: write {JEDEC_READ:#04X}, read {JEDEC_WIDTH}, a capacity code {:#04X} to {:#04X} in the last byte",
                FLASH_CAPACITY_CODES.start(),
                FLASH_CAPACITY_CODES.end()
            ));
        }
    }
    let c = raw.citations;
    let citations = Citations {
        bus: citation(c.bus, "bus")?,
        identity: citation(c.identity, "identity")?,
        supply: citation(c.supply, "supply")?,
        io: paired(c.io, io.is_some(), "io", "io_millivolts")?,
        pin_capacitance: citation(c.pin_capacitance, "pin_capacitance")?,
        lifecycle: citation(c.lifecycle, "lifecycle")?,
        configure: paired(c.configure, script.is_some(), "configure", "script")?,
        data: paired(c.data, script.is_some(), "data", "script")?,
    };
    Ok(Part {
        part: raw.part,
        lifecycle,
        stand_in: raw.stand_in,
        bus,
        identity: Identity {
            write,
            width,
            value,
            mask,
            datasheet_value: raw.identity.datasheet_value,
        },
        supply_millivolts: (min, max),
        io_millivolts: io,
        pin_capacitance_femtofarads: pin_capacitance,
        renode: raw.renode,
        script,
        citations,
    })
}

/// Parses a parts table, refusing the first malformed entry by naming its
/// part and the field: an unknown key, a field of the wrong kind or out of
/// range, a model that is not known or does not attach to the part's bus, a
/// citation that is neither held nor pending, a key given twice in an entry,
/// a missing `renode` (`null` is unmodelled), a part named twice in any case,
/// or an empty table. It never panics.
pub fn parse_parts(json: &str) -> Result<PartsTable, IrValidationError> {
    let raw: RawTable = serde_json::from_str(json).map_err(|e| refused(None, e.to_string()))?;
    if raw.schema_version != PARTS_SCHEMA_VERSION {
        return Err(refused(
            None,
            format!(
                "schema_version is {}, not {PARTS_SCHEMA_VERSION}",
                raw.schema_version
            ),
        ));
    }
    if raw.parts.is_empty() {
        return Err(refused(None, "`parts` is empty"));
    }
    let mut parts: Vec<Part> = Vec::with_capacity(raw.parts.len());
    for (i, text) in raw.parts.iter().enumerate() {
        // The name, read leniently, only to say which entry is refused.
        let entry: serde_json::Value =
            serde_json::from_str(text.get()).map_err(|e| refused(None, e.to_string()))?;
        let name = entry
            .get("part")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| refused(None, format!("entry {i} has no string `part`")))?;
        let named = |reason: String| refused(Some(&name), reason);
        crate::check_identifier(crate::IdentifierKind::Plain, &name)
            .map_err(|rule| named(format!("`part` is not an identifier: {rule}")))?;
        let one_bus = entry
            .get("bus")
            .and_then(serde_json::Value::as_object)
            .is_some_and(|b| b.len() == 1);
        if !one_bus {
            return Err(named(
                "`bus` is not an object holding exactly one of `i2c` and `spi`".to_owned(),
            ));
        }
        let raw: RawPart = serde_json::from_str(text.get()).map_err(|e| named(e.to_string()))?;
        let p = part(raw).map_err(named)?;
        if parts.iter().any(|q| q.part.eq_ignore_ascii_case(&p.part)) {
            return Err(named(
                "the part is in the table twice, in some case".to_owned(),
            ));
        }
        parts.push(p);
    }
    parts.sort_by(|a, b| a.part.cmp(&b.part));
    Ok(PartsTable { parts })
}

static TABLE: LazyLock<Result<PartsTable, IrValidationError>> =
    LazyLock::new(|| parse_parts(PARTS_JSON));

/// The embedded table, parsed once. A malformed table is an error here, which
/// every caller returns, never a panic.
pub fn table() -> Result<&'static PartsTable, IrValidationError> {
    TABLE.as_ref().map_err(Clone::clone)
}

/// The identity of `part`, or `None`: a part with no entry has no identity read.
pub fn identity(part: &str) -> Result<Option<Identity>, IrValidationError> {
    Ok(table()?.get(part).map(|p| p.identity.clone()))
}

/// The steps of `part`'s script ([`Part::steps`]), or `None`: a part with no
/// entry makes no transaction.
pub fn steps(part: &str) -> Result<Option<Vec<Step>>, IrValidationError> {
    Ok(table()?.get(part).map(Part::steps))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_embedded_table_parses_and_holds_the_four_parts() {
        let t = table().unwrap();
        let names: Vec<&str> = t.parts().iter().map(|p| p.part.as_str()).collect();
        assert_eq!(
            names,
            ["BME280", "ICM20948", "ICP10111", "TMP117", "W25Q64JVxIQ"]
        );
        assert_eq!(identity("BME280").unwrap().unwrap().value, 0x60);
        assert_eq!(identity("TMP117").unwrap().unwrap().width, 2);
        assert_eq!(identity("bme280").unwrap(), None);
        assert_eq!(identity("W25Q128").unwrap(), None);
        assert_eq!(identity(BARE_W25Q64JV).unwrap(), None);
    }
}
