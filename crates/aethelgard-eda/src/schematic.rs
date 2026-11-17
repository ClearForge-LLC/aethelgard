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

// The power tree's drawing, a child of this module beside it in `src/`
// (`strict_fence` gives every file of `src/` a role).
#[path = "schematic_power.rs"]
mod power;
#[path = "schematic_support.rs"]
mod support;
use aethelgard_ir::power::BATTERY_NET;

/// KiCad 10's own schematic format version (ruled D7 on PR #40).
pub const VERSION: &str = "20260306";

const GRID: i64 = 1_270_000;
const STUB: i64 = 2 * GRID;
pub(crate) use aethelgard_ir::GND;
const RAIL_SYMBOL: &str = "power:+3V3";
const GND_SYMBOL: &str = "power:GND";
const FLAG_SYMBOL: &str = "power:PWR_FLAG";
pub(crate) const R: &str = "Device:R";
pub(crate) const C: &str = "Device:C";
const R_FOOTPRINT: &str = "Resistor_SMD:R_0402_1005Metric";
const C_FOOTPRINT: &str = "Capacitor_SMD:C_0402_1005Metric";
/// A test point (`AET-WO-1312`): KiCad's own symbol and a 1.0 mm pad, both
/// vendored. The pad is bare copper, so nothing is bought or placed for it.
pub(crate) const TEST_POINT: &str = "Connector:TestPoint";
const TEST_POINT_FOOTPRINT: &str = "TestPoint:TestPoint_Pad_D1.0mm";
/// The reset push-button (`AET-WO-1312`): KiCad's `SW_Push` symbol on C&K's
/// PTS810 footprint, both vendored. The part is C&K PTS810 SJM 250 SMTR LFS
/// (the footprint's own datasheet link, ckswitches.com pts810.pdf; listed
/// "Active" by digikey.sg, read by `AET-WO-3400`, FEEDBACK T8).
pub(crate) const SWITCH: &str = "Switch:SW_Push";
const SWITCH_FOOTPRINT: &str = "Button_Switch_SMD:SW_SPST_PTS810";
const SWITCH_VALUE: &str = "PTS810 SJM 250 SMTR LFS";

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EmitError {
    #[error(transparent)]
    Library(#[from] LibraryError),
    /// The specification fails [`Specification::validate`] (`AET-WO-1208`
    /// F3): the emitter draws only what `validate` passes, whoever calls it.
    #[error(transparent)]
    InvalidSpecification(aethelgard_ir::IrValidationError),
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
        allowed: String,
    },
    #[error("the solved topology has no pin for net `{0}`")]
    UnsolvedNet(String),
    #[error("net `{net}` is solved to pin `{pin}`, which the MCU's symbol does not have")]
    UnknownPin { net: String, pin: String },
    #[error("`{0}` names a net, rail or reference the emitter uses for itself; rename it")]
    ReservedName(String),
    #[error("`{0}` is used twice as a reference")]
    DuplicateReference(String),
    #[error("`{reference}` of the power tree cannot be drawn: {why}")]
    PowerNotDrawable { reference: String, why: String },
    #[error("the router cannot complete net `{net}`: {why}")]
    Unroutable { net: String, why: String },
    #[error("the router cannot complete the board: {0}")]
    BoardUnroutable(String),
    /// `AET-WO-1310` D1: a run of adjacent connected pads with no lane to the
    /// grid. Never left to chance.
    #[error("`{reference}` pads {} have no escape: no lane on which this run of adjacent connected pads reaches the grid at its width and clearance", pads.join(", "))]
    PadEscapeExhausted {
        reference: String,
        pads: Vec<String>,
    },
    /// `AET-WO-1313`: the declared outline bound cannot hold the parts at
    /// all, judged before any placement: one part's keep-out box, or the sum
    /// of every keep-out box, against the bound less its margins.
    #[error("board.max_outline_mm is {width_mm} x {height_mm} mm; {why}")]
    OutlineTooSmall {
        width_mm: u32,
        height_mm: u32,
        /// The part that does not fit, or `None` when the sum does not.
        reference: Option<String>,
        why: String,
    },
    /// `AET-WO-1313`: the placer packed the parts but its outline exceeds
    /// the declared bound; nothing is written. The outline reached and the
    /// shortfall are in micrometres.
    #[error("board.max_outline_mm is {width_mm} x {height_mm} mm; the placer's best outline is {} x {} mm, over by {} x {} mm", um(*reached_um_w), um(*reached_um_h), um(*over_um_w), um(*over_um_h))]
    OutlineUnreachable {
        width_mm: u32,
        height_mm: u32,
        reached_um_w: i64,
        reached_um_h: i64,
        over_um_w: i64,
        over_um_h: i64,
    },
    #[error("{0}")]
    Internal(String),
}

/// Micrometres as millimetres with three decimals, for a message.
fn um(v: i64) -> String {
    format!("{}.{:03}", v.div_euclid(1000), v.rem_euclid(1000))
}

impl EmitError {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Library(_) => "LibraryCorrupt",
            Self::InvalidSpecification(e) => e.kind(),
            Self::UnknownPart { .. } => "UnknownPart",
            Self::UnsupportedAddress { .. } => "UnsupportedAddress",
            Self::PowerNotDrawable { .. } => "PowerNotDrawable",
            Self::UnsolvedNet(_) => "UnsolvedNet",
            Self::UnknownPin { .. } => "UnknownPin",
            Self::ReservedName(_) => "ReservedName",
            Self::DuplicateReference(_) => "DuplicateReference",
            Self::Unroutable { .. } | Self::BoardUnroutable(_) => "Unroutable",
            Self::PadEscapeExhausted { .. } => "PadEscapeExhausted",
            Self::OutlineTooSmall { .. } => "OutlineTooSmall",
            Self::OutlineUnreachable { .. } => "OutlineUnreachable",
            Self::Internal(_) => "EmitterError",
        }
    }

    pub fn id(&self) -> Option<&str> {
        match self {
            Self::InvalidSpecification(e) => e.id(),
            Self::UnknownPart { reference, .. }
            | Self::UnsupportedAddress { reference, .. }
            | Self::PowerNotDrawable { reference, .. }
            | Self::PadEscapeExhausted { reference, .. } => Some(reference),
            Self::UnsolvedNet(id) | Self::ReservedName(id) | Self::DuplicateReference(id) => {
                Some(id)
            }
            Self::UnknownPin { net, .. } | Self::Unroutable { net, .. } => Some(net),
            Self::OutlineTooSmall { reference, .. } => reference.as_deref(),
            Self::Library(_)
            | Self::BoardUnroutable(_)
            | Self::OutlineUnreachable { .. }
            | Self::Internal(_) => None,
        }
    }
}

/// The connection `d`'s address strap takes (`AET-WO-1311` §1.5, `4200`
/// F12). The addresses are the parts table's (`parts.json`
/// `bus.i2c.addresses`, cited), in the order the row lists them; `targets`
/// are the strap's connections in that same order, the one layout fact the
/// emitter keeps. A row whose count of addresses is not the emitter's count
/// of straps is refused, never matched in part (`INV-06`).
fn strapped(d: &aethelgard_ir::I2cDevice, targets: &[Conn]) -> Result<Conn, EmitError> {
    let table = aethelgard_ir::parts::table().map_err(|e| EmitError::Internal(e.to_string()))?;
    let addresses = match table.get(&d.part).map(|p| &p.bus) {
        Some(aethelgard_ir::parts::Bus::I2c { addresses }) => addresses,
        _ => {
            return Err(EmitError::Internal(format!(
                "{} ({}): the parts table has no I2C row for it",
                d.reference, d.part
            )));
        }
    };
    if addresses.len() != targets.len() {
        return Err(EmitError::Internal(format!(
            "{}: the parts table lists {} addresses, and the emitter draws {} straps",
            d.part,
            addresses.len(),
            targets.len()
        )));
    }
    match addresses.iter().position(|a| *a == d.address) {
        Some(i) => Ok(targets[i].clone()),
        None => Err(EmitError::UnsupportedAddress {
            reference: d.reference.clone(),
            part: d.part.clone(),
            address: d.address,
            allowed: addresses
                .iter()
                .map(|a| format!("{a} ({a:#04X})"))
                .collect::<Vec<_>>()
                .join(", "),
        }),
    }
}

/// What `emit` produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Emission {
    pub schematic: String,
    pub bom: String,
    pub board: String,
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
pub(crate) enum Conn {
    Net(String),
    /// A rail, by domain id, or `GND`.
    Rail(String),
    NoConnect,
}

/// One symbol on the sheet and footprint on the board: the board emitter
/// reads the same connections the sheet is drawn from.
#[derive(Clone, Debug)]
pub(crate) struct Part {
    pub(crate) lib_id: &'static str,
    pub(crate) reference: String,
    pub(crate) value: String,
    pub(crate) footprint: String,
    /// By pin number; pins absent here get a no-connect.
    pub(crate) conns: BTreeMap<String, Conn>,
    pub(crate) note: Option<&'static str>,
    /// The IC this part belongs to, if any: one relation for a decoupling
    /// capacitor and for a part a datasheet asks to sit by an IC's pins.
    pub(crate) owner: Option<Owner>,
    /// A connector's declared off-board capacitance by pin number, with its
    /// source: what extraction adds to a line it carries (`AET-WO-4300f` D8).
    pub(crate) offboard: BTreeMap<String, (u64, String)>,
}

/// The IC a part belongs to (`AET-WO-4300d2` reduction finding 9: one
/// relation, where there were two fields): the placer puts the part beside
/// it, by its pins when the part is anchored.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Owner {
    pub(crate) ic: String,
    /// Where by the IC's pins it must sit (`AET-WO-4300d2` §1.3), which the
    /// placer targets and the DRC gate judges; `None` beside the IC by its
    /// shared net alone.
    pub(crate) anchor: Option<Anchor>,
    /// Whether it counts in its rail's decoupling, as a capacitor between the
    /// rail and `GND` drawn for this IC ([`decoupling_picofarads`]).
    pub(crate) decouples: bool,
}

/// Where an owned part must sit: each of its own pads with the IC pad it
/// must lie beside, and, for a power-tree part, the largest copper-edge gap
/// allowed with the rule that asks for it, which the loop gate judges
/// (`crate::loops`). The MCU's support parts carry none: the support gate
/// judges them from the MCU's row, not from here (`AET-WO-4300g`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Anchor {
    pub(crate) pads: Vec<(String, String)>,
    pub(crate) loop_rule: Option<(i64, &'static str)>,
}

/// Each connector part's symbol and footprint, both vendored (`AET-WO-4300f`
/// D3, D4). The symbol carries a mounting pin, `MP`, which both of the
/// footprint's `MP` pads take.
pub(crate) fn connector_library(part: &str) -> Option<(&'static str, &'static str)> {
    Some(match part {
        "SM04B-GHS-TB" => (
            "Connector_Generic_MountingPin:Conn_01x04_MountingPin",
            "Connector_JST:JST_GH_SM04B-GHS-TB_1x04-1MP_P1.25mm_Horizontal",
        ),
        "SM06B-GHS-TB" => (
            "Connector_Generic_MountingPin:Conn_01x06_MountingPin",
            "Connector_JST:JST_GH_SM06B-GHS-TB_1x06-1MP_P1.25mm_Horizontal",
        ),
        "SM10B-GHS-TB" => (
            "Connector_Generic_MountingPin:Conn_01x10_MountingPin",
            "Connector_JST:JST_GH_SM10B-GHS-TB_1x10-1MP_P1.25mm_Horizontal",
        ),
        "SM06B-SRSS-TB" => (
            "Connector_Generic_MountingPin:Conn_01x06_MountingPin",
            "Connector_JST:JST_SH_SM06B-SRSS-TB_1x06-1MP_P1.00mm_Horizontal",
        ),
        _ => return None,
    })
}

/// Whether `spec`'s connectors declare any off-board capacitance, which the
/// extraction bound then adds ([`crate::extract::OFFBOARD`]). Here, so that
/// `eda`'s one reader of declared intent stays this module (`RUL-16`).
pub fn declares_offboard(spec: &Specification) -> bool {
    spec.declared_connectors()
        .iter()
        .any(|c| c.pins.iter().any(|p| p.offboard.is_some()))
}

/// Each device's declared pin capacitance (`AET-WO-4220b`, the `AET-WO-4220`
/// D10 ruling), by reference: I2C devices, then SPI devices, as the
/// specification declares them. Here for the reason [`declares_offboard`] is.
pub fn declared_pin_capacitance(
    spec: &Specification,
) -> Vec<(String, (String, aethelgard_ir::DeclaredPinCapacitance))> {
    spec.declared_i2c_devices()
        .iter()
        .map(|d| (&d.reference, &d.part, &d.pin_capacitance))
        .chain(
            spec.declared_spi_devices()
                .iter()
                .map(|d| (&d.reference, &d.part, &d.pin_capacitance)),
        )
        .filter_map(|(reference, part, c)| {
            c.as_ref()
                .map(|c| (reference.clone(), (part.clone(), c.declared.clone())))
        })
        .collect()
}

/// Whether `part` is an off-board connector: placed last, at the board edge,
/// and on a line counted as copper plus its declared budget, never at `Ci`.
pub(crate) fn is_connector(part: &Part) -> bool {
    part.lib_id.starts_with("Connector_")
}

/// Whether `part` is a power tree's level translator (`AET-WO-4300d2`).
pub(crate) fn is_translator(part: &Part) -> bool {
    part.lib_id.starts_with("Logic_LevelTranslator:")
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

/// The MCU's symbol, the one part every design has.
pub(crate) const MCU: &str = "MCU_ST_STM32H7:STM32H753ZITx";

use aethelgard_ir::names::VENDORED_PARTS as KNOWN_PARTS;
const STRAP: u32 = 10_000;

struct Builder<'a> {
    spec: &'a Specification,
    parts: Vec<Part>,
    used: Taken,
    next: BTreeMap<char, u32>,
    rails: BTreeSet<String>,
    names: Taken,
    /// `aethelgard_ir::names::own_nets` for the specification.
    own: Vec<String>,
}

/// Names claimed so far, keyed by their ASCII-folded spelling, each holding
/// the spelling that claimed it. KiCad's ERC refuses labels and references
/// that differ only in case (`AET-WO-1204`), so two spellings of one folded
/// name collide; the names themselves are emitted as written.
#[derive(Default)]
struct Taken(BTreeMap<String, String>);

impl Taken {
    fn of(names: impl IntoIterator<Item = String>) -> Self {
        Self(
            names
                .into_iter()
                .map(|n| (n.to_ascii_lowercase(), n))
                .collect(),
        )
    }

    /// The spelling that already holds `name`'s folded form, if any.
    fn holder(&self, name: &str) -> Option<&str> {
        self.0.get(&name.to_ascii_lowercase()).map(String::as_str)
    }

    /// Claims `name`; `Err(holder)` when its folded form is taken, by
    /// the same spelling or another.
    fn claim(&mut self, name: &str) -> Result<(), String> {
        match self.0.entry(name.to_ascii_lowercase()) {
            std::collections::btree_map::Entry::Occupied(e) => Err(e.get().clone()),
            std::collections::btree_map::Entry::Vacant(e) => {
                e.insert(name.to_owned());
                Ok(())
            }
        }
    }
}

/// The refusal for a claim that failed. Another spelling of the name names
/// both, as `pcb`'s pair check does (`a and b`): KiCad's ERC refuses the pair.
fn collision(name: &str, holder: String, refuse: fn(String) -> EmitError) -> EmitError {
    if holder == name {
        refuse(holder)
    } else {
        refuse(format!("{name} and {holder}"))
    }
}

impl Builder<'_> {
    /// The next free `U<n>`, `R<n>` or `C<n>`, skipping every reference the
    /// specification spells that way. A specification reference that differs
    /// from it only in case is refused (`DuplicateReference`), never skipped.
    fn reference(&mut self, prefix: char) -> Result<String, EmitError> {
        let n = self.next.entry(prefix).or_insert(0);
        loop {
            *n += 1;
            let r = format!("{prefix}{n}");
            match self.used.holder(&r) {
                Some(holder) if holder == r => {}
                Some(holder) => {
                    return Err(collision(
                        &r,
                        holder.to_owned(),
                        EmitError::DuplicateReference,
                    ));
                }
                None => {
                    self.used.claim(&r).expect("holder was None");
                    return Ok(r);
                }
            }
        }
    }

    /// An emitter-chosen net, refused if the specification already uses the
    /// name, in any case, for something else. Every such name is one
    /// `aethelgard_ir::names::own_nets` lists, the list `validate` refuses by
    /// (`AET-WO-1206`): a name it lacks is this emitter's fault, never drawn.
    fn own_net(&mut self, name: String) -> Result<String, EmitError> {
        if !self.own.contains(&name) {
            return Err(EmitError::Internal(format!(
                "the emitter names net `{name}`, which aethelgard_ir::names::own_nets does not list"
            )));
        }
        self.names
            .claim(&name)
            .map_err(|holder| collision(&name, holder, EmitError::ReservedName))?;
        Ok(name)
    }

    fn passive(
        &mut self,
        kind: char,
        value: String,
        one: Conn,
        two: Conn,
    ) -> Result<(), EmitError> {
        let reference = self.reference(kind)?;
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
            owner: None,
            offboard: BTreeMap::new(),
        });
        Ok(())
    }

    fn rail(&mut self, domain: &str) -> Conn {
        self.rails.insert(domain.to_owned());
        Conn::Rail(domain.to_owned())
    }

    fn ground(&mut self) -> Conn {
        self.rails.insert(GND.to_owned());
        Conn::Rail(GND.to_owned())
    }

    /// A net the specification declares, as a connector pin or test point
    /// names it: `GND` or a domain as its rail, any other as a label.
    fn declared(&mut self, net: &str) -> Conn {
        if net == GND {
            self.ground()
        } else if self.spec.voltage_domains.iter().any(|d| d.id == net) {
            self.rail(net)
        } else {
            Conn::Net(net.to_owned())
        }
    }

    /// A bidirectional pin strapped to a rail, through a resistor.
    fn strap(&mut self, reference: &str, pin: &str, rail: Conn) -> Result<Conn, EmitError> {
        let net = self.own_net(format!("{reference}_{pin}"))?;
        self.passive('R', ohms(STRAP), Conn::Net(net.clone()), rail)?;
        Ok(Conn::Net(net))
    }

    fn decouple(&mut self, ic: &str, rail: &str, value: &str) -> Result<(), EmitError> {
        let r = self.rail(rail);
        let g = self.ground();
        self.passive('C', value.to_owned(), r, g)?;
        if let Some(cap) = self.parts.last_mut() {
            cap.owner = Some(Owner {
                ic: ic.to_owned(),
                anchor: None,
                decouples: true,
            });
        }
        Ok(())
    }
}

/// Emits the schematic, the bill of materials and the placed board
/// ([`crate::pcb`]), all from one set of parts. `table` is the MCU's pin table
/// the topology was solved against: each solved pin is drawn on the symbol
/// pin whose number is the table's package position, so ST's data, not a
/// name match, decides it (KiCad names pins 28 and 29 `PC2_C` and `PC3_C`
/// where ST's table says `PC2` and `PC3`).
pub fn emit(
    spec: &Specification,
    solved: &SolvedTopology,
    table: &PinTable,
) -> Result<Emission, EmitError> {
    emit_moved(spec, solved, table, &[])
}

/// [`emit`], with each of `moves` placed at its own origin and rotation
/// instead of by the placer (`AET-WO-1308` §1.5: a part on any board edge).
pub fn emit_moved(
    spec: &Specification,
    solved: &SolvedTopology,
    table: &PinTable,
    moves: &[crate::pcb::Move],
) -> Result<Emission, EmitError> {
    spec.validate().map_err(EmitError::InvalidSpecification)?;
    let (parts, rails) = design(spec, solved, table)?;
    let stack = crate::stack::Stack::of(spec);
    let bound = spec.board.max_outline_mm;
    if let Some(b) = bound {
        crate::pcb::bound_holds(&parts, b)?;
    }
    Ok(Emission {
        schematic: write_sheet(&spec.project_name, &parts, &rails)?,
        // A test point is a bare pad: its symbol is out of the BOM, as its
        // footprint is (`AET-WO-1312`).
        bom: crate::bom::write(
            &parts
                .iter()
                .filter(|p| p.lib_id != TEST_POINT)
                .map(BomRow::from)
                .collect::<Vec<_>>(),
        ),
        board: crate::pcb::write_moved(&spec.project_name, &parts, &stack, moves, bound)?,
    })
}

/// `AET-WO-1313`: whether a declared outline bound can hold the parts the
/// emitter would draw at all, before any placement ([`crate::pcb::bound_holds`]).
/// A specification that declares no bound passes.
pub fn bound_holds(
    spec: &Specification,
    solved: &SolvedTopology,
    table: &PinTable,
) -> Result<(), EmitError> {
    let Some(b) = spec.board.max_outline_mm else {
        return Ok(());
    };
    let (parts, _) = design(spec, solved, table)?;
    crate::pcb::bound_holds(&parts, b)
}

/// The decoupling capacitance the emitter places on each rail, in pF, keyed
/// by domain: every part `design` marks as decoupling an IC, summed by the
/// rail it connects (`AET-WO-2301`, ruled D7: a droop's rail is computed with
/// the decoupling the board will carry, not a restated value). A value the
/// emitter writes that this cannot read is an error, never a guess.
pub fn decoupling_picofarads(
    spec: &Specification,
    solved: &SolvedTopology,
    table: &PinTable,
) -> Result<BTreeMap<String, u64>, EmitError> {
    let (parts, _) = design(spec, solved, table)?;
    let mut out: BTreeMap<String, u64> = BTreeMap::new();
    for p in parts
        .iter()
        .filter(|p| p.owner.as_ref().is_some_and(|o| o.decouples))
    {
        let pf = picofarads(&p.value).ok_or_else(|| {
            EmitError::Internal(format!(
                "{}'s value `{}` is not a capacitance this reads",
                p.reference, p.value
            ))
        })?;
        for c in p.conns.values() {
            if let Conn::Rail(r) = c
                && r != GND
            {
                let sum = out.entry(r.clone()).or_default();
                *sum = sum.checked_add(pf).ok_or_else(|| {
                    EmitError::Internal(format!("rail `{r}`'s decoupling overflows"))
                })?;
            }
        }
    }
    Ok(out)
}

/// A capacitor value as the emitter writes it, `100n` or `2u2`, in pF.
fn picofarads(value: &str) -> Option<u64> {
    let i = value.find(['p', 'n', 'u'])?;
    let (whole, rest) = value.split_at(i);
    let unit: u64 = match &rest[..1] {
        "p" => 1,
        "n" => 1_000,
        _ => 1_000_000,
    };
    let frac = &rest[1..];
    let digits = |d: &str| !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit());
    if !digits(whole) || !(frac.is_empty() || digits(frac)) || frac.len() > 3 {
        return None;
    }
    let scale = 10u64.pow(u32::try_from(frac.len()).ok()?);
    let n: u64 = format!("{whole}{frac}").parse().ok()?;
    let total = n.checked_mul(unit)?;
    total.is_multiple_of(scale).then_some(total / scale)
}

/// Each anchored part's reference, with the IC it is anchored to
/// (`AET-WO-4300d2` §1.3), read from the design's ownership, never from a
/// board's layers: what a test needs to judge which references a board
/// prints (`AET-WO-1310` D2).
pub fn anchored(
    spec: &Specification,
    solved: &SolvedTopology,
    table: &PinTable,
) -> Result<BTreeMap<String, String>, EmitError> {
    let (parts, _) = design(spec, solved, table)?;
    Ok(parts
        .iter()
        .filter_map(|p| {
            let o = p.owner.as_ref().filter(|o| o.anchor.is_some())?;
            Some((p.reference.clone(), o.ic.clone()))
        })
        .collect())
}

/// Every part and what each of its pins connects to, and every rail.
pub(crate) fn design(
    spec: &Specification,
    solved: &SolvedTopology,
    table: &PinTable,
) -> Result<(Vec<Part>, BTreeSet<String>), EmitError> {
    if spec.mcu.part != "STM32H753ZITx" || table.part != spec.mcu.part {
        return Err(EmitError::UnknownPart {
            reference: "mcu".into(),
            part: spec.mcu.part.clone(),
            known: "STM32H753ZITx",
        });
    }
    let mut used = Taken::default();
    for r in spec
        .declared_i2c_devices()
        .iter()
        .map(|d| &d.reference)
        .chain(spec.declared_spi_devices().iter().map(|d| &d.reference))
        .chain(spec.declared_connectors().iter().map(|c| &c.reference))
        .chain(spec.power.iter().flat_map(aethelgard_ir::power::references))
        .chain(spec.declared_test_points().iter().map(|t| &t.reference))
    {
        // KiCad treats a reference starting with `#` as a virtual symbol (power,
        // flag) and drops it from the netlist and BOM, and the emitter names its
        // own power symbols and flags there.
        if r.starts_with('#') {
            return Err(EmitError::ReservedName(r.clone()));
        }
        used.claim(r)
            .map_err(|holder| collision(r, holder, EmitError::DuplicateReference))?;
    }
    // Every name the specification gives a net or a rail, serial and motor
    // nets included (`AET-WO-1207` §1.4, `1206` F1): an emitter net may not
    // take one. The one list `validate`'s step 20 reads.
    let spelled = aethelgard_ir::names::spelled_nets(spec);
    let mut names = Taken::of(spelled.into_iter().map(str::to_owned));
    names
        .claim(GND)
        .map_err(|holder| collision(GND, holder, EmitError::ReservedName))?;
    if spec.power.is_some() {
        names
            .claim(BATTERY_NET)
            .map_err(|holder| collision(BATTERY_NET, holder, EmitError::ReservedName))?;
    }
    let own =
        aethelgard_ir::names::own_nets(spec).map_err(|e| EmitError::Internal(e.to_string()))?;
    let mut b = Builder {
        spec,
        parts: Vec::new(),
        used,
        next: BTreeMap::new(),
        rails: BTreeSet::new(),
        names,
        own,
    };

    // The MCU.
    let mcu_ref = b.reference('U')?;
    let mcu_sym = library::symbol(MCU)?;
    let mcu_pins = pins(&mcu_sym).map_err(EmitError::Internal)?;
    // Its support pins and network, from its row (`AET-WO-4300g`); signal
    // pins are set below.
    let mut conns = support::draw(&mut b, &mcu_ref)?;
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
        // belt-and-suspenders: `solve` never assigns a reserved pin, and the
        // row's pins are power, reset, boot and debug pins.
        if conns.contains_key(&p.number) {
            return Err(EmitError::Internal(format!(
                "{net}: the MCU's pin {position} is already a support pin"
            )));
        }
        conns.insert(p.number.clone(), Conn::Net(net.clone()));
    }
    // belt-and-suspenders: `the_mcu_rows_support_pins_are_the_symbols` proves
    // the one MCU row's pins against the symbol; a row added later is refused
    // here at emit, not drawn on a pin the symbol lacks.
    if let Some((pin, _)) = conns
        .iter()
        .find(|(pin, _)| !mcu_pins.iter().any(|p| &p.number == *pin))
    {
        return Err(EmitError::Internal(format!(
            "the MCU's row names pin {pin}, which the symbol lacks"
        )));
    }
    b.parts.push(Part {
        lib_id: MCU,
        reference: mcu_ref.clone(),
        value: spec.mcu.part.clone(),
        footprint: mcu_sym.property("Footprint").unwrap_or("").to_owned(),
        conns,
        note: None,
        owner: None,
        offboard: BTreeMap::new(),
    });

    // The I2C buses' pull-ups, then their devices.
    for bus in &spec.i2c_buses {
        for net in [&bus.sda.net, &bus.scl.net] {
            if !solved.assignments.contains_key(net) {
                return Err(EmitError::UnsolvedNet(net.clone()));
            }
            let r = b.rail(&bus.domain);
            b.passive('R', ohms(bus.pull_up_ohms), r, Conn::Net(net.clone()))?;
        }
    }
    for d in spec.declared_i2c_devices() {
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
                // SDO to GND, then to VDDIO (BME280 datasheet §6.2).
                let target = strapped(d, &[gnd.clone(), rail.clone()])?;
                let sdo = b.strap(&d.reference, "SDO", target)?;
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
            // BST-BMP581-DS004-13 Table 28 and Fig. 26 (I2C), pp. 46 and 48:
            // VDDIO and VDD on the supply, each bypassed by 100 nF; the three
            // VSS to GND; SCK is SCL and SDI is SDA; SDO selects the address
            // LSB (low, then high: the parts table's order); CSB to VDDIO ("In I2C mode, the CSB
            // pin must be connected to VDDIO"); INT to GND, allowed "as long
            // as the IRQ pin is not activated" (Table 28 (a)), and the
            // bring-up leaves INT_CONFIG at its reset, `int_en` 0 (p. 52).
            "BMP581" => {
                let target = strapped(d, &[gnd.clone(), rail.clone()])?;
                let sdo = b.strap(&d.reference, "SDO", target)?;
                (
                    "Aethelgard_Sensor:BMP581",
                    [
                        ("1", rail.clone()),
                        ("10", rail.clone()),
                        ("3", gnd.clone()),
                        ("8", gnd.clone()),
                        ("9", gnd.clone()),
                        ("2", scl),
                        ("4", sda),
                        ("5", sdo),
                        ("6", rail.clone()),
                        ("7", gnd.clone()),
                    ]
                    .to_vec(),
                )
            }
            "TMP117" => {
                // ADD0 to GND, V+, SDA, then SCL (TMP117 datasheet Table 7-2).
                let add0 = strapped(d, &[gnd.clone(), rail.clone(), sda.clone(), scl.clone()])?;
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
        b.decouple(&d.reference, &d.supply, "100n")?;
        if d.part == "BMP581" {
            // Its second 100 nF: one on VDD and one on VDDIO (Fig. 26).
            b.decouple(&d.reference, &d.supply, "100n")?;
        }
    }
    // Each crossing's device is drawn on the translator's B side.
    let behind = match &spec.power {
        Some(p) => power::behind(&mut b, p, spec.declared_spi_devices())?,
        None => BTreeMap::new(),
    };
    for d in spec.declared_spi_devices() {
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
        let rail = b.rail(&d.supply);
        let gnd = b.ground();
        let net = |line: power::Line, own: &String| {
            Conn::Net(
                behind
                    .get(&d.reference)
                    .and_then(|m| m.get(&line))
                    .unwrap_or(own)
                    .clone(),
            )
        };
        match d.part.as_str() {
            "W25Q64JVxIQ" => {
                let wp = b.strap(&d.reference, "WP", rail.clone())?;
                let hold = b.strap(&d.reference, "HOLD", rail.clone())?;
                let conns = [
                    ("1", net(power::Line::Cs, &d.cs.net)),
                    ("2", net(power::Line::Miso, &bus.miso.net)),
                    ("3", wp),
                    ("4", gnd),
                    ("5", net(power::Line::Mosi, &bus.mosi.net)),
                    ("6", net(power::Line::Sck, &bus.sck.net)),
                    ("7", hold),
                    ("8", rail),
                ]
                .to_vec();
                device(&mut b, &d.reference, "Memory_Flash:W25Q32JVSS", conns)?;
                b.decouple(&d.reference, &d.supply, "100n")?;
            }
            // DS-000189 p. 20, 4.1 and 4.3: VDD and VDDIO on the supply, each
            // bypassed; REGOUT's 0.1 uF filter capacitor (C1); FSYNC and RESV
            // pin 20 to GND; RESV pin 19 and the auxiliary I2C master pins left
            // open. SPI: SCLK, SDI, SDO and nCS (`AET-WO-4300d2` D2 (a)).
            "ICM20948" => {
                let regout = b.own_net(format!("{}_REGOUT", d.reference))?;
                let conns = [
                    ("8", rail.clone()),
                    ("13", rail),
                    ("18", gnd.clone()),
                    ("20", gnd.clone()),
                    ("11", gnd.clone()),
                    ("10", Conn::Net(regout.clone())),
                    ("22", net(power::Line::Cs, &d.cs.net)),
                    ("23", net(power::Line::Sck, &bus.sck.net)),
                    ("24", net(power::Line::Mosi, &bus.mosi.net)),
                    ("9", net(power::Line::Miso, &bus.miso.net)),
                ]
                .to_vec();
                device(&mut b, &d.reference, "Sensor_Motion:ICM-20948", conns)?;
                b.passive('C', "100n".into(), Conn::Net(regout), gnd)?;
                b.decouple(&d.reference, &d.supply, "100n")?;
                b.decouple(&d.reference, &d.supply, "100n")?;
            }
            // DS-000347 Table 10 p. 20 and Fig. 7 p. 21 (4-wire SPI): VDD
            // bypassed by 0.1 uF and 2.2 uF, VDDIO by 10 nF; GND and RESV pin
            // 7 to GND; INT2/FSYNC to GND, "Connect to GND if FSYNC not
            // used"; RESV pins 2, 3, 10 and 11 and INT1 left open ("No
            // Connect or Connect to GND"; no interrupt is armed). AP_CS,
            // AP_SCLK, AP_SDI and AP_SDO are CS, SCK, MOSI and MISO.
            "ICM42688P" => {
                let conns = [
                    ("5", rail.clone()),
                    ("8", rail),
                    ("6", gnd.clone()),
                    ("7", gnd.clone()),
                    ("9", gnd),
                    ("12", net(power::Line::Cs, &d.cs.net)),
                    ("13", net(power::Line::Sck, &bus.sck.net)),
                    ("14", net(power::Line::Mosi, &bus.mosi.net)),
                    ("1", net(power::Line::Miso, &bus.miso.net)),
                ]
                .to_vec();
                device(&mut b, &d.reference, "Aethelgard_Sensor:ICM-42688-P", conns)?;
                b.decouple(&d.reference, &d.supply, "100n")?;
                b.decouple(&d.reference, &d.supply, "2u2")?;
                b.decouple(&d.reference, &d.supply, "10n")?;
            }
            _ => {
                return Err(EmitError::UnknownPart {
                    reference: d.reference.clone(),
                    part: d.part.clone(),
                    known: KNOWN_PARTS,
                });
            }
        }
    }
    if let Some(p) = &spec.power {
        power::draw(
            &mut b,
            p,
            spec.declared_spi_devices(),
            &behind,
            &mcu_ref,
            solved,
        )?;
    }
    connectors(&mut b)?;
    bench(&mut b, &mcu_ref)?;
    Ok((b.parts, b.rails))
}

/// Bench access (`AET-WO-1312`), after every other part so that no other
/// part's reference moves: BOOT0's pull-down when a connector pin or test
/// point carries `BOOT0` (anchored by the boot pin), the reset switch from
/// `NRST` to `GND`, then each test point in document order, on its net as a
/// connector pin would be.
fn bench(b: &mut Builder<'_>, mcu: &str) -> Result<(), EmitError> {
    let spec = b.spec;
    if aethelgard_ir::bench::carries_boot0(spec) {
        let pin = support::row(&spec.mcu.part)?
            .group("BOOT0")
            .first()
            .cloned()
            .ok_or_else(|| EmitError::Internal("the MCU's row has no BOOT0 pin".into()))?;
        let gnd = b.ground();
        b.passive(
            'R',
            ohms(aethelgard_ir::bench::BOOT0_PULL_DOWN_OHMS),
            Conn::Net(aethelgard_ir::BOOT0.to_owned()),
            gnd,
        )?;
        if let Some(r) = b.parts.last_mut() {
            r.owner = Some(Owner {
                ic: mcu.to_owned(),
                anchor: Some(Anchor {
                    pads: vec![("1".into(), pin)],
                    loop_rule: None,
                }),
                decouples: false,
            });
        }
    }
    if let Some(reference) = aethelgard_ir::bench::switch_reference(spec) {
        b.used
            .claim(&reference)
            .map_err(|holder| collision(&reference, holder, EmitError::DuplicateReference))?;
        let gnd = b.ground();
        b.parts.push(Part {
            lib_id: SWITCH,
            reference,
            value: SWITCH_VALUE.to_owned(),
            footprint: SWITCH_FOOTPRINT.to_owned(),
            conns: BTreeMap::from([
                ("1".into(), Conn::Net(aethelgard_ir::NRST.to_owned())),
                ("2".into(), gnd),
            ]),
            note: None,
            owner: None,
            offboard: BTreeMap::new(),
        });
    }
    for t in spec.declared_test_points() {
        let conn = b.declared(&t.net);
        b.parts.push(Part {
            lib_id: TEST_POINT,
            reference: t.reference.clone(),
            value: "TestPoint".to_owned(),
            footprint: TEST_POINT_FOOTPRINT.to_owned(),
            conns: BTreeMap::from([("1".into(), conn)]),
            note: None,
            owner: None,
            offboard: BTreeMap::new(),
        });
    }
    Ok(())
}

/// Each declared connector, after every other part: a pin's net is drawn as
/// the rail it names (`GND` or a domain), or a label, or a no-connect when it
/// is `null`; the mounting pin goes to `GND` (`AET-WO-4300f` D3).
fn connectors(b: &mut Builder<'_>) -> Result<(), EmitError> {
    let spec = b.spec;
    for c in spec.declared_connectors() {
        // belt-and-suspenders: `validate` refuses a part not in the IR's
        // table (`UnknownConnectorPart`), and a tier-1 test maps every row.
        let (lib_id, footprint) = connector_library(&c.part).ok_or_else(|| {
            EmitError::Internal(format!("{}: no library entry for {}", c.reference, c.part))
        })?;
        let mut conns = BTreeMap::from([("MP".to_owned(), b.ground())]);
        let mut offboard = BTreeMap::new();
        for p in &c.pins {
            let conn = match p.net.as_deref() {
                None => Conn::NoConnect,
                Some(n) => b.declared(n),
            };
            conns.insert(p.pin.clone(), conn);
            if let Some(o) = &p.offboard {
                offboard.insert(p.pin.clone(), (o.capacitance_femtofarads, o.source.clone()));
            }
        }
        b.parts.push(Part {
            lib_id,
            reference: c.reference.clone(),
            value: c.part.clone(),
            footprint: footprint.to_owned(),
            conns,
            note: None,
            owner: None,
            offboard,
        });
    }
    Ok(())
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
            // The ordering code in full: package `SS` (SOIC-8 208-mil, the
            // footprint), then `IQ`, whose JEDEC identity the twin reads
            // (`AET-WO-4300c` F11).
            "W25Q64JVSSIQ".to_owned(),
            Some(
                "symbol Memory_Flash:W25Q32JVSS stands in for W25Q64JVSSIQ: same family, same SOIC-8 208-mil pinout (AET-WO-1300 D2)",
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
        owner: None,
        offboard: BTreeMap::new(),
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
            // KiCad's test point symbol is in the BOM and its pad footprint is
            // not; parity refuses the pair, and a bare pad is bought by no one
            // (`AET-WO-1312`).
            let val = if lib_id == TEST_POINT && flag == "in_bom" {
                "no"
            } else {
                val
            };
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
    // A rail a `power_out` pin drives (an LDO's output) is driven already:
    // a `PWR_FLAG` there would be a second driver, which ERC refuses, so its
    // flag and the power symbol the flag sits on are left out (`AET-WO-4300d2`
    // D12; the parts' own power symbols still draw the rail).
    let mut driven = BTreeSet::new();
    for part in parts {
        for p in pins(&sheet.syms[part.lib_id]).map_err(EmitError::Internal)? {
            if p.etype == "power_out"
                && let Some(Conn::Rail(r)) = part.conns.get(&p.number)
            {
                driven.insert(r.clone());
            }
        }
    }
    // Flags: one row at the top, a power symbol and a PWR_FLAG on one point.
    let mut flags = 0;
    for rail in rails.iter().filter(|r| !driven.contains(*r)) {
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
    #[test]
    fn capacitor_values_read_as_the_emitter_writes_them() {
        use super::picofarads;
        assert_eq!(picofarads("100n"), Some(100_000));
        assert_eq!(picofarads("2u2"), Some(2_200_000));
        assert_eq!(picofarads("10p"), Some(10));
        for bad in ["", "n", "100", "1x", "1n5n", "1.5n", "1u2345"] {
            assert_eq!(picofarads(bad), None, "{bad}");
        }
    }

    use super::*;

    /// `AET-WO-1311` §1.5 (`4200` F12): each address takes the strap the
    /// datasheet gives it, by the parts table's order, pinned here so a
    /// reordered row fails; a count that disagrees, or an address the row
    /// does not offer, is refused.
    #[test]
    fn a_strap_follows_the_parts_tables_addresses() {
        let device = |part: &str, address: u8| aethelgard_ir::I2cDevice {
            reference: "U9".into(),
            part: part.into(),
            supply: "VDD_3V3".into(),
            bus: "B".into(),
            address,
            pin_capacitance: None,
        };
        let to = |n: &str| Conn::Net(n.into());
        let bme = [to("GND"), to("VDDIO")];
        assert_eq!(strapped(&device("BME280", 0x76), &bme).unwrap(), to("GND"));
        assert_eq!(
            strapped(&device("BME280", 0x77), &bme).unwrap(),
            to("VDDIO")
        );
        let tmp = [to("GND"), to("V+"), to("SDA"), to("SCL")];
        for (address, want) in [(0x48, "GND"), (0x49, "V+"), (0x4A, "SDA"), (0x4B, "SCL")] {
            assert_eq!(
                strapped(&device("TMP117", address), &tmp).unwrap(),
                to(want)
            );
        }
        let e = strapped(&device("TMP117", 0x48), &tmp[..3]).unwrap_err();
        assert!(
            e.to_string()
                .contains("lists 4 addresses, and the emitter draws 3"),
            "{e}"
        );
        let e = strapped(&device("BME280", 0x48), &bme).unwrap_err();
        assert_eq!(e.kind(), "UnsupportedAddress");
        assert!(e.to_string().contains("118 (0x76), 119 (0x77)"), "{e}");
    }

    /// No I2C address of any parts-table row is written in this file: the
    /// table is the one list (`AET-WO-1311` §1.5).
    #[test]
    fn the_emitter_keeps_no_copy_of_an_address() {
        let code = include_str!("schematic.rs");
        let code = &code[..code.find("#[cfg(test)]").unwrap()];
        let table = aethelgard_ir::parts::table().unwrap();
        let mut checked = 0;
        for part in table.parts() {
            if let aethelgard_ir::parts::Bus::I2c { addresses } = &part.bus {
                for a in addresses {
                    for text in [format!("{a:#04x}"), format!("{a:#04X}"), format!("{a} (")] {
                        assert!(!code.contains(&text), "{}: {text}", part.part);
                    }
                    checked += 1;
                }
            }
        }
        assert!(checked >= 6, "{checked}");
    }

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

    /// The MCU row's support pins (ST's pin data) and KiCad's symbol agree,
    /// pin for pin, and the row covers every supply, ground, reset, boot and
    /// regulator pin the symbol has (`AET-WO-4300g`).
    #[test]
    fn the_mcu_rows_support_pins_are_the_symbols() {
        let sym = library::symbol("MCU_ST_STM32H7:STM32H753ZITx").unwrap();
        let pins = pins(&sym).unwrap();
        assert_eq!(pins.len(), 144);
        let row = support::row("STM32H753ZITx").unwrap();
        let mut covered = BTreeSet::new();
        for (group, numbers) in &row.pins {
            let name = match group.as_str() {
                "VDD33USB" => "VDD33_USB",
                "SWDIO" => "PA13",
                "SWCLK" => "PA14",
                g => g,
            };
            for n in numbers {
                let p = pins.iter().find(|p| &p.number == n).unwrap();
                assert_eq!(p.name, name, "pin {n} of {group}");
                covered.insert(n.clone());
            }
        }
        let special = |n: &str| {
            [
                "VDD", "VSS", "VBAT", "VREF", "VCAP", "NRST", "BOOT0", "PDR_ON",
            ]
            .iter()
            .any(|s| n.starts_with(s))
        };
        for p in pins.iter().filter(|p| special(&p.name)) {
            assert!(covered.contains(&p.number), "{} ({})", p.number, p.name);
        }
        let vcap: Vec<&str> = row
            .group("VCAP")
            .iter()
            .map(|n| pins.iter().find(|p| &p.number == n).unwrap().etype.as_str())
            .collect();
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

    #[test]
    fn the_connector_table_and_the_vendored_files_agree() {
        use aethelgard_ir::connectors::{PARTS, Series};
        for row in PARTS.iter() {
            let (sym_id, fp_id) = connector_library(row.name)
                .unwrap_or_else(|| panic!("{} has no connector_library mapping", row.name));
            let mut want: Vec<String> = (1..=row.pins).map(|i| i.to_string()).collect();
            want.push("MP".into());
            want.sort();
            let mut got: Vec<String> = pins(&library::symbol(sym_id).unwrap())
                .unwrap()
                .into_iter()
                .map(|p| p.number)
                .collect();
            got.sort();
            assert_eq!(
                got, want,
                "{}: symbol pins differ from 1..N plus MP",
                row.name
            );
            let fp = library::footprint(fp_id).unwrap();
            let mut pads: Vec<String> = fp
                .children("pad")
                .map(|p| p.arg(1).unwrap_or("").to_owned())
                .collect();
            pads.sort();
            let mut want_pads = want.clone();
            want_pads.push("MP".into());
            want_pads.sort();
            assert_eq!(
                pads, want_pads,
                "{}: footprint pads differ from 1..N plus two MP",
                row.name
            );
            let pitch = match row.series {
                Series::JstGh => "P1.25mm",
                Series::JstSh => "P1.00mm",
            };
            assert!(
                fp_id.contains(pitch),
                "{}: footprint name lacks the series pitch {pitch}",
                row.name
            );
            assert!(
                fp_id.contains(row.name),
                "{}: footprint name lacks the part name",
                row.name
            );
            assert!(
                is_connector(&Part {
                    lib_id: sym_id,
                    reference: "J1".into(),
                    value: row.name.into(),
                    footprint: fp_id.into(),
                    conns: Default::default(),
                    note: None,
                    owner: None,
                    offboard: Default::default(),
                }),
                "{}: its symbol is not recognised as a connector",
                row.name
            );
        }
        assert!(
            connector_library("SM05B-GHS-TB").is_none(),
            "a name not in PARTS has a mapping"
        );
        assert!(
            connector_library("").is_none(),
            "the empty name has a mapping"
        );
    }
}
