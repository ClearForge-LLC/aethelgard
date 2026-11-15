//! The output speed table (`AET-WO-4300c` §1.1): DS12117 Rev 10 Table 149
//! (rev V silicon) and Table 52 (rev Y), which give the same value in every
//! cell, as data.
//!
//! The table is `data/output-speeds.json`, embedded in this crate and pinned
//! by its sha256 in a tier-1 test. One parser, [`parse_output_speeds`], reads
//! it; it never panics, and refuses a malformed table by naming the field or
//! the row (`INV-06`). It holds every `OSPEEDR` row at 10, 30 and 50 pF in
//! both VDD bands. Rows `10` and `11` are held with their note-5 condition
//! (the I/O compensation cell, which the twin does not model) and are never
//! selectable (`AET-WO-4300` D1).
//!
//! [`OutputSpeeds::choose`] is the check: the slowest selectable speed whose
//! Fmax meets a line's rate and whose rise and fall time meets the line's
//! declared rise time, at the band its supply's minimum selects and the row
//! its declared capacitance rounds up to. Above 50 pF, below 1.62 V, or when
//! only `10` or `11` would do, it refuses by name; it never speeds a line up
//! past `01`.

use std::collections::BTreeSet;
use std::sync::LazyLock;

use serde::Deserialize;

use crate::IrValidationError;
use crate::parts::{Citation, RawCitation, citation};

/// The table as embedded: `data/output-speeds.json`.
pub const OUTPUT_SPEEDS_JSON: &str = include_str!("../data/output-speeds.json");

/// The `schema_version` the parser reads.
pub const OUTPUT_SPEEDS_SCHEMA_VERSION: u32 = 1;

/// The loads the table gives, in picofarads, lightest first.
pub const LOADS_PICOFARADS: [u32; 3] = [10, 30, 50];

/// The two VDD bands, (min, max) in millivolts, lower first. The table prints
/// 2.7 V in both; a supply whose minimum is exactly 2.7 V takes the lower,
/// slower band.
pub const BANDS_MILLIVOLTS: [(u32, u32); 2] = [(1620, 2700), (2700, 3600)];

/// The largest Fmax or rise time a row may state; anything above is a typo.
const MAX_FMAX_HZ: u32 = 1_000_000_000;
const MAX_RISE_FALL_PS: u32 = 1_000_000;

/// One row: an `OSPEEDR` value, a VDD band and a load, with its Fmax and its
/// maximum rise and fall time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Row {
    /// `OSPEEDR`, 0 to 3 (`00` to `11`).
    pub ospeedr: u8,
    pub vdd_min_millivolts: u32,
    pub vdd_max_millivolts: u32,
    pub load_picofarads: u32,
    pub fmax_hz: u32,
    pub rise_fall_picoseconds: u32,
    /// Note 5: the row holds only with the compensation cell enabled, so it
    /// is never selectable.
    pub requires_compensation_cell: bool,
}

/// The parsed table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputSpeeds {
    /// The part the table is for, as a specification names it.
    pub mcu: String,
    pub rev_v: Citation,
    pub rev_y: Citation,
    /// Note 3's definition of Fmax, and notes 1 and 4.
    pub definition: String,
    /// Note 2: the pins a VBAT supply can feed, and their limit.
    pub vbat_pins: Vec<String>,
    pub vbat_fmax_hz: u32,
    /// Why I2C lines have no row (`AET-WO-4300c` D1), with its citation.
    pub open_drain: String,
    rows: Vec<Row>,
}

/// What [`OutputSpeeds::choose`] chose, and the row that decided it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Choice {
    pub ospeedr: u8,
    pub row: Row,
    /// The row's Fmax, or note 2's limit on a VBAT pin when lower.
    pub fmax_hz: u32,
}

/// Why no selectable speed serves a line. Each names its figures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// The supply's minimum is outside both bands.
    SupplyOutsideTable { vdd_min_millivolts: u32 },
    /// The declared capacitance is above the table's heaviest row.
    LoadAboveTable { capacitance_femtofarads: u64 },
    /// `01`'s Fmax (or note 2's limit) is below the rate: only `10` or `11`,
    /// which need the compensation cell, or nothing, would do.
    TooFastForPin { rate_hz: u32, fmax_hz: u32 },
    /// `01`'s rise and fall time is slower than the line's declared rise.
    RiseFasterThanPin {
        rise_time_ns: u32,
        rise_fall_picoseconds: u32,
    },
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SupplyOutsideTable { vdd_min_millivolts } => write!(
                f,
                "its supply's minimum of {vdd_min_millivolts} mV is outside DS12117 Table 149's bands (1620 to 3600 mV)"
            ),
            Self::LoadAboveTable {
                capacitance_femtofarads,
            } => write!(
                f,
                "its declared {capacitance_femtofarads} fF is above DS12117 Table 149's heaviest row, 50 pF"
            ),
            Self::TooFastForPin { rate_hz, fmax_hz } => write!(
                f,
                "its rate of {rate_hz} Hz is above {fmax_hz} Hz, the fastest speed without the I/O compensation cell (`01`) allows at its row (DS12117 Table 149); speeds `10` and `11` are never selected"
            ),
            Self::RiseFasterThanPin {
                rise_time_ns,
                rise_fall_picoseconds,
            } => write!(
                f,
                "its declared rise time of {rise_time_ns} ns is below {rise_fall_picoseconds} ps, `01`'s maximum rise and fall time at its row (DS12117 Table 149)"
            ),
        }
    }
}

impl OutputSpeeds {
    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    /// The row for a speed, band and load; every combination is present.
    pub fn row(&self, ospeedr: u8, band: (u32, u32), load_picofarads: u32) -> Option<Row> {
        self.rows.iter().copied().find(|r| {
            r.ospeedr == ospeedr
                && (r.vdd_min_millivolts, r.vdd_max_millivolts) == band
                && r.load_picofarads == load_picofarads
        })
    }

    /// The band a supply's minimum selects: the lower one up to and
    /// including 2.7 V.
    pub fn band(vdd_min_millivolts: u32) -> Result<(u32, u32), Refusal> {
        match vdd_min_millivolts {
            1620..=2700 => Ok(BANDS_MILLIVOLTS[0]),
            2701..=3600 => Ok(BANDS_MILLIVOLTS[1]),
            _ => Err(Refusal::SupplyOutsideTable { vdd_min_millivolts }),
        }
    }

    /// The row a capacitance rounds up to, in picofarads.
    pub fn load(capacitance_femtofarads: u64) -> Result<u32, Refusal> {
        LOADS_PICOFARADS
            .iter()
            .copied()
            .find(|&pf| capacitance_femtofarads <= u64::from(pf) * 1000)
            .ok_or(Refusal::LoadAboveTable {
                capacitance_femtofarads,
            })
    }

    /// The slowest selectable speed (`00`, then `01`) for a line on `pin`
    /// toggled at `rate_hz`, supplied at no less than `vdd_min_millivolts`,
    /// loaded by at most `capacitance_femtofarads`, whose edges must rise
    /// within `rise_time_ns`.
    pub fn choose(
        &self,
        pin: &str,
        rate_hz: u32,
        vdd_min_millivolts: u32,
        capacitance_femtofarads: u64,
        rise_time_ns: u32,
    ) -> Result<Choice, Refusal> {
        let band = Self::band(vdd_min_millivolts)?;
        let load = Self::load(capacitance_femtofarads)?;
        let vbat = self.vbat_pins.iter().any(|p| p == pin);
        let mut last = None;
        for ospeedr in [0u8, 1] {
            let row = self
                .row(ospeedr, band, load)
                .expect("the parser holds every speed, band and load");
            let fmax_hz = if vbat {
                row.fmax_hz.min(self.vbat_fmax_hz)
            } else {
                row.fmax_hz
            };
            let rise_ok = u64::from(rise_time_ns) * 1000 >= u64::from(row.rise_fall_picoseconds);
            if fmax_hz >= rate_hz && rise_ok {
                return Ok(Choice {
                    ospeedr,
                    row,
                    fmax_hz,
                });
            }
            last = Some((row, fmax_hz));
        }
        let (row, fmax_hz) = last.expect("two speeds were tried");
        if fmax_hz < rate_hz {
            Err(Refusal::TooFastForPin { rate_hz, fmax_hz })
        } else {
            Err(Refusal::RiseFasterThanPin {
                rise_time_ns,
                rise_fall_picoseconds: row.rise_fall_picoseconds,
            })
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawTable {
    schema_version: u32,
    mcu: String,
    citations: RawCitations,
    definition: String,
    vbat_pins: RawVbat,
    open_drain: String,
    rows: Vec<RawRow>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCitations {
    rev_v: RawCitation,
    rev_y: RawCitation,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawVbat {
    pins: Vec<String>,
    fmax_hz: u32,
    note: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRow {
    ospeedr: String,
    vdd_min_millivolts: u32,
    vdd_max_millivolts: u32,
    load_picofarads: u32,
    fmax_hz: u32,
    rise_fall_picoseconds: u32,
    requires_compensation_cell: bool,
}

fn refused(reason: impl Into<String>) -> IrValidationError {
    IrValidationError::OutputSpeedTable {
        reason: reason.into(),
    }
}

fn prose(value: &str, field: &str) -> Result<(), IrValidationError> {
    if value.is_empty() || value.len() > 1024 || !value.bytes().all(|b| (0x20..=0x7E).contains(&b))
    {
        return Err(refused(format!(
            "`{field}` is not 1 to 1024 bytes of printable ASCII"
        )));
    }
    Ok(())
}

fn row(i: usize, raw: &RawRow) -> Result<Row, IrValidationError> {
    let named = |reason: String| refused(format!("row {i}: {reason}"));
    let ospeedr = match raw.ospeedr.as_str() {
        "00" => 0,
        "01" => 1,
        "10" => 2,
        "11" => 3,
        other => {
            return Err(named(format!(
                "`ospeedr` is `{other}`, not `00`, `01`, `10` or `11`"
            )));
        }
    };
    let band = (raw.vdd_min_millivolts, raw.vdd_max_millivolts);
    if !BANDS_MILLIVOLTS.contains(&band) {
        return Err(named(format!(
            "the band {} to {} mV is not 1620 to 2700 or 2700 to 3600",
            band.0, band.1
        )));
    }
    if !LOADS_PICOFARADS.contains(&raw.load_picofarads) {
        return Err(named(format!(
            "`load_picofarads` is {}, not 10, 30 or 50",
            raw.load_picofarads
        )));
    }
    if !(1..=MAX_FMAX_HZ).contains(&raw.fmax_hz) {
        return Err(named(format!(
            "`fmax_hz` is {}, not 1 to {MAX_FMAX_HZ}",
            raw.fmax_hz
        )));
    }
    if !(1..=MAX_RISE_FALL_PS).contains(&raw.rise_fall_picoseconds) {
        return Err(named(format!(
            "`rise_fall_picoseconds` is {}, not 1 to {MAX_RISE_FALL_PS}",
            raw.rise_fall_picoseconds
        )));
    }
    // Note 5 governs speeds `10` and `11` and nothing else: a row `10` made
    // selectable, or a row `01` made unselectable, is a table that lies.
    if raw.requires_compensation_cell != (ospeedr >= 2) {
        return Err(named(format!(
            "speed `{}` has `requires_compensation_cell` {}; note 5 holds for `10` and `11` only",
            raw.ospeedr, raw.requires_compensation_cell
        )));
    }
    Ok(Row {
        ospeedr,
        vdd_min_millivolts: band.0,
        vdd_max_millivolts: band.1,
        load_picofarads: raw.load_picofarads,
        fmax_hz: raw.fmax_hz,
        rise_fall_picoseconds: raw.rise_fall_picoseconds,
        requires_compensation_cell: raw.requires_compensation_cell,
    })
}

/// Parses an output speed table, refusing by name: an unknown or missing
/// key, a citation neither held nor pending, a malformed or repeated row, a
/// speed, band and load with no row, a row `10` or `11` without note 5 (or
/// `00`/`01` with it), or a heavier load given a higher Fmax or a faster edge
/// than a lighter one at the same speed and band. It never panics.
pub fn parse_output_speeds(json: &str) -> Result<OutputSpeeds, IrValidationError> {
    let raw: RawTable = serde_json::from_str(json).map_err(|e| refused(e.to_string()))?;
    if raw.schema_version != OUTPUT_SPEEDS_SCHEMA_VERSION {
        return Err(refused(format!(
            "schema_version is {}, not {OUTPUT_SPEEDS_SCHEMA_VERSION}",
            raw.schema_version
        )));
    }
    crate::check_identifier(crate::IdentifierKind::Plain, &raw.mcu)
        .map_err(|rule| refused(format!("`mcu` is not an identifier: {rule}")))?;
    let rev_v = citation(raw.citations.rev_v, "rev_v").map_err(refused)?;
    let rev_y = citation(raw.citations.rev_y, "rev_y").map_err(refused)?;
    prose(&raw.definition, "definition")?;
    prose(&raw.open_drain, "open_drain")?;
    prose(&raw.vbat_pins.note, "vbat_pins.note")?;
    if raw.vbat_pins.pins.is_empty() {
        return Err(refused("`vbat_pins.pins` is empty"));
    }
    let mut seen = BTreeSet::new();
    for pin in &raw.vbat_pins.pins {
        if crate::check_identifier(crate::IdentifierKind::Plain, pin).is_err()
            || !seen.insert(pin.as_str())
        {
            return Err(refused(format!(
                "`vbat_pins.pins` holds `{pin}`, malformed or twice"
            )));
        }
    }
    if raw.vbat_pins.fmax_hz == 0 {
        return Err(refused("`vbat_pins.fmax_hz` is 0"));
    }
    let mut rows: Vec<Row> = Vec::with_capacity(raw.rows.len());
    for (i, r) in raw.rows.iter().enumerate() {
        let r = row(i, r)?;
        let key = |x: &Row| (x.ospeedr, x.vdd_min_millivolts, x.load_picofarads);
        if rows.iter().any(|q| key(q) == key(&r)) {
            return Err(refused(format!(
                "row {i}: speed {:02b}, {} mV band, {} pF is given twice",
                r.ospeedr, r.vdd_min_millivolts, r.load_picofarads
            )));
        }
        rows.push(r);
    }
    let table = OutputSpeeds {
        mcu: raw.mcu,
        rev_v,
        rev_y,
        definition: raw.definition,
        vbat_pins: raw.vbat_pins.pins,
        vbat_fmax_hz: raw.vbat_pins.fmax_hz,
        open_drain: raw.open_drain,
        rows,
    };
    for ospeedr in 0..4u8 {
        for band in BANDS_MILLIVOLTS {
            let mut lighter: Option<Row> = None;
            for load in LOADS_PICOFARADS {
                let Some(r) = table.row(ospeedr, band, load) else {
                    return Err(refused(format!(
                        "no row for speed {ospeedr:02b}, {} to {} mV, {load} pF",
                        band.0, band.1
                    )));
                };
                if let Some(l) = lighter
                    && (r.fmax_hz > l.fmax_hz || r.rise_fall_picoseconds < l.rise_fall_picoseconds)
                {
                    return Err(refused(format!(
                        "speed {ospeedr:02b}, {} to {} mV: {load} pF is given a higher Fmax or a faster edge than {} pF",
                        band.0, band.1, l.load_picofarads
                    )));
                }
                lighter = Some(r);
            }
        }
    }
    Ok(table)
}

static TABLE: LazyLock<Result<OutputSpeeds, IrValidationError>> =
    LazyLock::new(|| parse_output_speeds(OUTPUT_SPEEDS_JSON));

/// The embedded table, parsed once. A malformed table is an error here, which
/// every caller returns, never a panic.
pub fn table() -> Result<&'static OutputSpeeds, IrValidationError> {
    TABLE.as_ref().map_err(Clone::clone)
}
