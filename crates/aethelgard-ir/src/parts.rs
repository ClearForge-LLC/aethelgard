//! The parts table (`AET-WO-4200` D3, review B7): what each part is, as data.
//!
//! The table is `data/parts.json`, embedded in this crate and pinned by its
//! sha256 in a tier-1 test. One parser, [`parse_parts`], reads it and every
//! test fixture; it never panics, and refuses a malformed entry by naming the
//! part and the field (`INV-06`). The embedded table reaches a user only
//! through [`table`], whose error `validate` returns like any other refusal.
//!
//! Each entry holds the part's values: its bus (I2C addresses, or SPI modes
//! and clock), its identity read, its supply range, and the Renode model that
//! stands in for it, or none. Each value group carries its datasheet citation:
//! the document, revision and section, and the document's sha256 when it is
//! held, or the URL it must be fetched from (`pending`), never a value from
//! memory. What a Renode model logs is not a part value: that stays code in
//! `aethelgard-twin`, matched on the model (`AET-WO-2204` D2). The generators
//! and the judge read the same entry (`INV-01`); neither takes an expected
//! value from the other (`INV-07`).

use std::collections::BTreeSet;
use std::sync::LazyLock;

use serde::Deserialize;

use crate::IrValidationError;

/// The table as embedded: `data/parts.json`.
pub const PARTS_JSON: &str = include_str!("../data/parts.json");

/// The `schema_version` the parser reads.
pub const PARTS_SCHEMA_VERSION: u32 = 1;

/// A bus kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BusKind {
    I2c,
    Spi,
}

/// The Renode 1.17.0 models a part may name, each with the buses it attaches
/// to. A name outside this set is refused; `aethelgard-twin` maps each to its
/// own model, exhaustively.
pub const RENODE_MODELS: [(&str, &[BusKind]); 3] = [
    ("I2C.BME280", &[BusKind::I2c]),
    ("SPI.GenericSpiFlash", &[BusKind::Spi]),
    ("Sensors.ICM20948", &[BusKind::I2c, BusKind::Spi]),
];

/// The highest SPI clock an entry may state: no part in the table comes near
/// it, and anything above is a typo.
pub const MAX_SPI_CLOCK_HZ: u32 = 1_000_000_000;

/// The highest supply an entry may state, in millivolts.
pub const MAX_SUPPLY_MILLIVOLTS: u32 = 60_000;

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

/// One identity read: the byte written first (an I2C register, or an SPI
/// command with the read bit as the part needs it), then `width` bytes (1 to
/// 4) read back, which a present part answers with `value`, the first byte on
/// the wire as the most significant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Identity {
    pub write: u8,
    pub width: u8,
    pub value: u32,
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

/// One citation per value group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Citations {
    pub bus: Citation,
    pub identity: Citation,
    pub supply: Citation,
}

/// One part.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Part {
    pub part: String,
    pub bus: Bus,
    pub identity: Identity,
    /// The supply range, (min, max) in millivolts.
    pub supply_millivolts: (u32, u32),
    /// The Renode model, as `platforms` names it, or `None`: unmodelled.
    pub renode: Option<String>,
    pub citations: Citations,
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
            for (group, c) in [
                ("bus", &p.citations.bus),
                ("identity", &p.citations.identity),
                ("supply", &p.citations.supply),
            ] {
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
    parts: Vec<serde_json::Value>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPart {
    part: String,
    bus: RawBus,
    identity: RawIdentity,
    supply_millivolts: RawSupply,
    renode: Option<String>,
    citations: RawCitations,
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
    write: String,
    read: u8,
    value: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSupply {
    min: u32,
    max: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCitations {
    bus: RawCitation,
    identity: RawCitation,
    supply: RawCitation,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCitation {
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

fn citation(raw: RawCitation, group: &str) -> Result<Citation, String> {
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

fn part(raw: RawPart) -> Result<Part, String> {
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
    let write = u8::try_from(hex(&raw.identity.write, "identity.write", 0xFF)?)
        .map_err(|e| e.to_string())?;
    let max = if width == 4 {
        u32::MAX
    } else {
        (1u32 << (8 * width)) - 1
    };
    let value = hex(&raw.identity.value, "identity.value", max)?;
    let RawSupply { min, max } = raw.supply_millivolts;
    if min == 0 || min > max || max > MAX_SUPPLY_MILLIVOLTS {
        return Err(format!(
            "`supply_millivolts` is {min} to {max}, not 1 <= min <= max <= {MAX_SUPPLY_MILLIVOLTS}"
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
    }
    Ok(Part {
        part: raw.part,
        bus,
        identity: Identity {
            write,
            width,
            value,
        },
        supply_millivolts: (min, max),
        renode: raw.renode,
        citations: Citations {
            bus: citation(raw.citations.bus, "bus")?,
            identity: citation(raw.citations.identity, "identity")?,
            supply: citation(raw.citations.supply, "supply")?,
        },
    })
}

/// Parses a parts table, refusing the first malformed entry by naming its
/// part and the field: an unknown key, a field of the wrong kind or out of
/// range, a model that is not known or does not attach to the part's bus, a
/// citation that is neither held nor pending, or a part named twice. It never
/// panics.
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
    let mut parts: Vec<Part> = Vec::with_capacity(raw.parts.len());
    for (i, entry) in raw.parts.into_iter().enumerate() {
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
        let raw: RawPart = serde_json::from_value(entry).map_err(|e| named(e.to_string()))?;
        let p = part(raw).map_err(named)?;
        if parts.iter().any(|q| q.part == p.part) {
            return Err(named("the part is in the table twice".to_owned()));
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
    Ok(table()?.get(part).map(|p| p.identity))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_embedded_table_parses_and_holds_the_four_parts() {
        let t = table().unwrap();
        let names: Vec<&str> = t.parts().iter().map(|p| p.part.as_str()).collect();
        assert_eq!(names, ["BME280", "ICM20948", "TMP117", "W25Q64JV"]);
        assert_eq!(identity("BME280").unwrap().unwrap().value, 0x60);
        assert_eq!(identity("TMP117").unwrap().unwrap().width, 2);
        assert_eq!(identity("bme280").unwrap(), None);
        assert_eq!(identity("W25Q128").unwrap(), None);
    }
}
