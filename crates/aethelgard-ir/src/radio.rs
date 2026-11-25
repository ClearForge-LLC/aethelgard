//! A radio module as a part (`AET-WO-4411`, Path A): a pre-certified
//! module on a declared serial port, its supply, enable and straps judged,
//! its antenna placed off the board's edge, and its firmware named as not
//! judged and its radio as not simulated.
//!
//! A module is a part with its own certified radio and firmware. Aethelgard
//! judges what the board does to it: its supply (the row's stated supply
//! requirement is counted on its rail, never its typical transmit peak,
//! ruled D5), its enable pull, each strap's level at reset against the
//! row's SPI-boot level, and the serial port it rides. The specification's
//! `radio_modules` list is private and read through
//! [`Specification::declared_radio_modules`] (`RUL-16`). [`validate`] is
//! `validate`'s step 27; nothing substitutes a default (`INV-06`).
//!
//! Every value of a row cites a held document by sha256
//! (`data/radio-parts.json`): the module datasheet, Espressif's Hardware
//! Design Guidelines for the antenna's clearance and placement (ruled §7/D3:
//! the module datasheet does not dimension the base-board keep-out), and
//! Espressif's product selector for the lifecycle (D8).

use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::pin_devices::PullResistor;
use crate::serial::SerialRole;
use crate::{GND, IrValidationError, Specification};

// ---------------------------------------------------------------- the key

/// One radio module (`radio_modules[]`). Every key is required.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RadioModule {
    pub reference: String,
    /// A row of the radio parts table.
    pub part: String,
    /// The label of a declared `serial_ports` entry, role `mavlink`: the
    /// module's U0TXD drives the port's RX net and its U0RXD hears the
    /// port's TX net.
    pub serial_port: String,
    /// The voltage domain on the row's supply pin: the MCU's own supply,
    /// since the module's levels are stated against it and no 5 V tolerance
    /// is stated.
    pub supply: String,
    /// The enable pin's RC delay.
    pub enable: Enable,
    /// Each strap pin of the row to its strap; every row strap is declared.
    #[serde(deserialize_with = "crate::unique_keys")]
    pub straps: BTreeMap<String, RadioStrap>,
    /// The capacitors from the supply to ground beside the module; at least
    /// one.
    pub decoupling: Vec<Capacitor>,
}

/// The enable pin's RC delay: a pull-up to the module's supply and a
/// capacitor to ground (the row cites the values the maker advises).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Enable {
    pub resistor: PullResistor,
    pub capacitor: Capacitor,
}

/// A capacitor by reference and power-parts row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Capacitor {
    pub reference: String,
    pub part: String,
}

/// A strap: left `open` (the module's own pull, if any, holds it), or a
/// resistor to the module's supply or to `GND`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum RadioStrap {
    Open,
    Resistor(PullResistor),
}

// ---------------------------------------------------------------- the table

/// A citation: a held document by key, and where in it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct At {
    pub document: String,
    pub section: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Document {
    pub title: String,
    pub revision: String,
    pub sha256: String,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Status {
    pub status: crate::parts::Lifecycle,
    pub at: At,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Supply {
    pub pin: String,
    pub number: String,
    pub min_mv: u32,
    pub max_mv: u32,
    pub at: At,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cited {
    pub value: u32,
    pub at: At,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnablePin {
    pub pin: String,
    pub number: String,
    /// VIH_nRST, the reset release voltage, in thousandths of the supply.
    pub release_permille: u32,
    /// tSTBL: how long the supply is stable before EN rises.
    pub stable_us: u32,
    pub at: At,
}

/// A strap's internal pull at reset, as the row states it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Internal {
    Floating,
    PullUp,
    PullDown,
}

/// The level SPI boot needs on a strap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Need {
    High,
    Low,
    Any,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StrapRow {
    pub pin: String,
    pub number: String,
    pub internal: Internal,
    pub spi_boot: Need,
    pub at: At,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Uart {
    pub tx: String,
    pub tx_number: String,
    pub rx: String,
    pub rx_number: String,
    pub at: At,
    /// VIH's maximum over the module's own supply, mV: how far above its
    /// rail a separately supplied driver may go (`AET-WO-4411`).
    pub vih_over_supply_mv: Cited,
}

/// A rectangle in the footprint's own coordinates, in µm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AreaUm {
    pub x0: i64,
    pub y0: i64,
    pub x1: i64,
    pub y1: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Antenna {
    /// The antenna area, at the module's top edge (smaller y).
    pub area_um: AreaUm,
    pub area_at: At,
    /// How far, left, right and beyond, no board may be.
    pub clearance_um: i64,
    pub clearance_at: At,
    /// At most how far the body's feed (right) side is from the board's
    /// right edge.
    pub feed_edge_max_um: i64,
    pub feed_at: At,
    pub placement_at: At,
}

/// One module of the radio parts table.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RadioPart {
    pub part: String,
    /// The family's u.FL variant, named beside it (ruled D1); not drawn.
    pub u_fl_variant: String,
    pub lifecycle: Status,
    pub symbol: String,
    pub footprint: String,
    pub supply: Supply,
    pub supply_current_min_ma: Cited,
    pub input_leakage_na: Cited,
    pub decoupling_min_pf: Cited,
    pub tx_peak_ma_typical: Cited,
    pub ground_pins: Vec<String>,
    /// The ground pad's thermal vias, as a power row's rule is written
    /// (`AET-WO-1309`): the engine emits them inside the pad.
    pub thermal_vias: crate::power_parts::ThermalRule,
    pub enable: EnablePin,
    pub straps: Vec<StrapRow>,
    pub uart: Uart,
    /// Where the ROM's output on UART0 at every boot is stated.
    pub rom_output: At,
    pub antenna: Antenna,
}

impl RadioPart {
    pub fn strap(&self, pin: &str) -> Option<&StrapRow> {
        self.straps.iter().find(|s| s.pin == pin)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RadioTable {
    pub schema_version: u32,
    pub documents: BTreeMap<String, Document>,
    pub modules: Vec<RadioPart>,
}

impl RadioTable {
    pub fn get(&self, part: &str) -> Option<&RadioPart> {
        self.modules.iter().find(|m| m.part == part)
    }

    /// `<title> <revision>, sha256 <hash>` for a citation.
    pub fn cite(&self, at: &At) -> String {
        match self.documents.get(&at.document) {
            Some(d) => format!(
                "{} {}, {} (sha256 {})",
                d.title, d.revision, at.section, d.sha256
            ),
            None => at.section.clone(),
        }
    }
}

/// The copper layers a radio module's footprint holds its keepout on.
pub const RADIO_COPPER_LAYERS: u8 = 4;

/// The radio parts table's schema version.
pub const RADIO_PARTS_SCHEMA_VERSION: u32 = 1;

const TABLE: &str = include_str!("../data/radio-parts.json");

/// The radio parts table, parsed and checked once; a malformed table is
/// refused by name (`RadioTableMalformed`), never a panic.
pub fn table() -> Result<&'static RadioTable, IrValidationError> {
    static CELL: OnceLock<Result<RadioTable, String>> = OnceLock::new();
    match CELL.get_or_init(|| parse(TABLE)) {
        Ok(t) => Ok(t),
        Err(why) => Err(refuse(
            Kind::RadioTableMalformed,
            "radio-parts.json",
            "radio-parts.json",
            why.clone(),
        )),
    }
}

/// Parses and checks a radio parts table: its version, each citation's
/// document held by a lowercase sha256, each part once, and each row's pins
/// distinct (a strap shared with a UART or enable pin is refused here).
pub fn parse(text: &str) -> Result<RadioTable, String> {
    let t: RadioTable = serde_json::from_str(text).map_err(|e| e.to_string())?;
    if t.schema_version != RADIO_PARTS_SCHEMA_VERSION {
        return Err(format!(
            "schema_version {} is not {RADIO_PARTS_SCHEMA_VERSION}",
            t.schema_version
        ));
    }
    for (key, d) in &t.documents {
        if d.sha256.len() != 64
            || !d
                .sha256
                .bytes()
                .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
        {
            return Err(format!(
                "document `{key}`: sha256 is not 64 lowercase hex digits"
            ));
        }
    }
    let mut parts = BTreeSet::new();
    for m in &t.modules {
        if !parts.insert(m.part.as_str()) {
            return Err(format!("part `{}` twice", m.part));
        }
        let ats = [
            &m.lifecycle.at,
            &m.supply.at,
            &m.supply_current_min_ma.at,
            &m.input_leakage_na.at,
            &m.decoupling_min_pf.at,
            &m.antenna.feed_at,
            &m.tx_peak_ma_typical.at,
            &m.enable.at,
            &m.uart.at,
            &m.rom_output,
            &m.antenna.area_at,
            &m.antenna.clearance_at,
            &m.antenna.placement_at,
        ];
        for at in ats.into_iter().chain(m.straps.iter().map(|s| &s.at)) {
            if !t.documents.contains_key(&at.document) {
                return Err(format!(
                    "`{}` cites `{}`, which is not held",
                    m.part, at.document
                ));
            }
        }
        let mut pins = BTreeSet::new();
        if !m.ground_pins.contains(&m.thermal_vias.pad) {
            return Err(format!(
                "`{}`: the thermal vias' pad is no ground pin",
                m.part
            ));
        }
        let numbers = m
            .ground_pins
            .iter()
            .chain([
                &m.supply.number,
                &m.enable.number,
                &m.uart.tx_number,
                &m.uart.rx_number,
            ])
            .chain(m.straps.iter().map(|s| &s.number));
        for n in numbers {
            if !pins.insert(n.as_str()) {
                return Err(format!("`{}`: pin {n} has two roles", m.part));
            }
        }
        let a = m.antenna.area_um;
        if a.x0 >= a.x1 || a.y0 >= a.y1 || m.antenna.clearance_um <= 0 {
            return Err(format!(
                "`{}`: the antenna area or clearance is empty",
                m.part
            ));
        }
        if m.supply.min_mv >= m.supply.max_mv {
            return Err(format!("`{}`: the supply range is empty", m.part));
        }
    }
    Ok(t)
}

// ---------------------------------------------------------------- refusals

/// Each refusal of a radio module is its own kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    RadioTableMalformed,
    RadioPartUnknown,
    RadioPortUndeclared,
    RadioPortRole,
    RadioPortShared,
    RadioSupplyUnknown,
    RadioSupplyOutOfRange,
    RadioEnableNotPulled,
    RadioStrapUndeclared,
    RadioStrapUnknown,
    RadioStrapUndriven,
    RadioStrapWrongLevel,
    RadioStrapPullTo,
    RadioDecouplingUndeclared,
    RadioPassivePartUnknown,
    RadioBoardLayers,
    RadioModuleTwice,
    RadioSupplyUnsourced,
    RadioPullTooWeak,
    RadioEnableTooFast,
    RadioDecouplingShort,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Self::RadioTableMalformed => "RadioTableMalformed",
            Self::RadioPartUnknown => "RadioPartUnknown",
            Self::RadioPortUndeclared => "RadioPortUndeclared",
            Self::RadioPortRole => "RadioPortRole",
            Self::RadioPortShared => "RadioPortShared",
            Self::RadioSupplyUnknown => "RadioSupplyUnknown",
            Self::RadioSupplyOutOfRange => "RadioSupplyOutOfRange",
            Self::RadioEnableNotPulled => "RadioEnableNotPulled",
            Self::RadioStrapUndeclared => "RadioStrapUndeclared",
            Self::RadioStrapUnknown => "RadioStrapUnknown",
            Self::RadioStrapUndriven => "RadioStrapUndriven",
            Self::RadioStrapWrongLevel => "RadioStrapWrongLevel",
            Self::RadioStrapPullTo => "RadioStrapPullTo",
            Self::RadioDecouplingUndeclared => "RadioDecouplingUndeclared",
            Self::RadioPassivePartUnknown => "RadioPassivePartUnknown",
            Self::RadioBoardLayers => "RadioBoardLayers",
            Self::RadioModuleTwice => "RadioModuleTwice",
            Self::RadioSupplyUnsourced => "RadioSupplyUnsourced",
            Self::RadioPullTooWeak => "RadioPullTooWeak",
            Self::RadioEnableTooFast => "RadioEnableTooFast",
            Self::RadioDecouplingShort => "RadioDecouplingShort",
        }
    }
}

/// A radio module refused: its kind, what it names, where, and why.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("`{path}`: {message}")]
pub struct RadioRefusal {
    pub kind: Kind,
    pub id: String,
    pub path: String,
    pub message: String,
}

fn refuse(
    kind: Kind,
    id: impl Into<String>,
    path: impl Into<String>,
    message: impl Into<String>,
) -> IrValidationError {
    IrValidationError::Radio(RadioRefusal {
        kind,
        id: id.into(),
        path: path.into(),
        message: message.into(),
    })
}

// ---------------------------------------------------------------- names

/// Every reference a radio module declares: its own, its enable resistor
/// and capacitor, its strap resistors (in pin order), then its decoupling,
/// in document order.
pub(crate) fn references(spec: &Specification) -> Vec<&str> {
    let mut out = Vec::new();
    for m in spec.declared_radio_modules() {
        out.push(m.reference.as_str());
        out.push(m.enable.resistor.reference.as_str());
        out.push(m.enable.capacitor.reference.as_str());
        for s in m.straps.values() {
            if let RadioStrap::Resistor(r) = s {
                out.push(r.reference.as_str());
            }
        }
        out.extend(m.decoupling.iter().map(|c| c.reference.as_str()));
    }
    out
}

/// The nets the emitter names for each module: `<ref>_EN`, and
/// `<ref>_<pin>` for each strap with a resistor.
pub fn own_nets(spec: &Specification) -> Vec<String> {
    let mut out = Vec::new();
    for m in spec.declared_radio_modules() {
        out.push(enable_net(&m.reference));
        for (pin, s) in &m.straps {
            if matches!(s, RadioStrap::Resistor(_)) {
                out.push(strap_net(&m.reference, pin));
            }
        }
    }
    out
}

/// The enable pin's net.
pub fn enable_net(reference: &str) -> String {
    format!("{reference}_EN")
}

/// A strap pin's net.
pub fn strap_net(reference: &str, pin: &str) -> String {
    format!("{reference}_{pin}")
}

/// Step 2 for each radio module: its reference, part, port, supply and
/// passives as identifiers or plain values.
pub(crate) fn identifiers(spec: &Specification) -> Result<(), IrValidationError> {
    use crate::{IdentifierKind, identifier, plain};
    let resistor = |r: &PullResistor, at: &str| {
        identifier(IdentifierKind::Reference, &r.reference, || {
            format!("{at}.reference")
        })?;
        plain(&r.part, || format!("{at}.part"))?;
        plain(&r.to, || format!("{at}.to"))
    };
    let capacitor = |c: &Capacitor, at: &str| {
        identifier(IdentifierKind::Reference, &c.reference, || {
            format!("{at}.reference")
        })?;
        plain(&c.part, || format!("{at}.part"))
    };
    for (i, m) in spec.declared_radio_modules().iter().enumerate() {
        let at = format!("radio_modules[{i}]");
        identifier(IdentifierKind::Reference, &m.reference, || {
            format!("{at}.reference")
        })?;
        plain(&m.part, || format!("{at}.part"))?;
        plain(&m.serial_port, || format!("{at}.serial_port"))?;
        plain(&m.supply, || format!("{at}.supply"))?;
        resistor(&m.enable.resistor, &format!("{at}.enable.resistor"))?;
        capacitor(&m.enable.capacitor, &format!("{at}.enable.capacitor"))?;
        for (pin, s) in &m.straps {
            plain(pin, || format!("{at}.straps"))?;
            if let RadioStrap::Resistor(r) = s {
                resistor(r, &format!("{at}.straps.{pin}.resistor"))?;
            }
        }
        for (k, c) in m.decoupling.iter().enumerate() {
            capacitor(c, &format!("{at}.decoupling[{k}]"))?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------- load

/// The current the radio modules on `domain` require of their supply: each
/// row's stated supply requirement (ruled D5), never its typical transmit
/// peak, as `power::loads` counts it.
pub fn load_ma(spec: &Specification, domain: &str) -> Result<u64, IrValidationError> {
    let t = table()?;
    let mut sum = 0;
    for m in spec.declared_radio_modules() {
        if m.supply == domain
            && let Some(row) = t.get(&m.part)
        {
            sum += u64::from(row.supply_current_min_ma.value);
        }
    }
    Ok(sum)
}

// ---------------------------------------------------------------- step 27

/// The level a strap reads at reset: high, low, or none held.
fn level(strap: &RadioStrap, internal: Internal, supply: &str) -> Option<bool> {
    match strap {
        RadioStrap::Resistor(r) if r.to == supply => Some(true),
        RadioStrap::Resistor(r) if r.to == GND => Some(false),
        RadioStrap::Resistor(_) => None,
        RadioStrap::Open => match internal {
            Internal::PullUp => Some(true),
            Internal::PullDown => Some(false),
            Internal::Floating => None,
        },
    }
}

/// Step 15's head (`AET-WO-4411`): each radio module's port, judged before
/// the connectors' step, which exempts a port a module rides from ending at
/// a connector: the port is declared, a `mavlink` port, one module's only,
/// and on no connector pin.
pub fn ports(spec: &Specification) -> Result<(), IrValidationError> {
    // One module per board: its antenna overhangs the one top-right corner
    // (ruled D2, HDG-C3 Figure 19 position 3).
    if let [_, second, ..] = spec.declared_radio_modules() {
        return Err(refuse(
            Kind::RadioModuleTwice,
            &second.reference,
            "radio_modules[1]",
            "a second radio module: its antenna overhangs the board's one top-right corner, which holds one",
        ));
    }
    let mut ports = BTreeSet::new();
    let connector_nets: BTreeSet<&str> = spec
        .declared_connectors()
        .iter()
        .flat_map(|c| c.pins.iter().filter_map(|p| p.net.as_deref()))
        .collect();
    for (i, m) in spec.declared_radio_modules().iter().enumerate() {
        let at = format!("radio_modules[{i}]");
        // The port.
        let port = spec
            .serial_ports
            .iter()
            .find(|p| p.label == m.serial_port)
            .ok_or_else(|| {
                refuse(
                    Kind::RadioPortUndeclared,
                    &m.serial_port,
                    format!("{at}.serial_port"),
                    format!("no serial port is labelled `{}`", m.serial_port),
                )
            })?;
        if port.role != SerialRole::Mavlink {
            return Err(refuse(
                Kind::RadioPortRole,
                &m.serial_port,
                format!("{at}.serial_port"),
                format!(
                    "port `{}` is a {} port; a radio module's link is a mavlink port",
                    port.label,
                    port.role.name()
                ),
            ));
        }
        if !ports.insert(port.label.as_str()) {
            return Err(refuse(
                Kind::RadioPortShared,
                &m.serial_port,
                format!("{at}.serial_port"),
                format!("port `{}` already has a radio module", port.label),
            ));
        }
        if let Some(net) = [&port.tx.net, &port.rx.net]
            .into_iter()
            .find(|n| connector_nets.contains(n.as_str()))
        {
            return Err(refuse(
                Kind::RadioPortShared,
                &m.serial_port,
                format!("{at}.serial_port"),
                format!("`{net}` also leaves on a connector pin: two drivers on one line"),
            ));
        }
    }
    Ok(())
}

/// Step 27 (`AET-WO-4411`): each radio module, in document order: its row
/// (its port was judged at step 15's head, [`ports`]), its supply, its enable, its straps, its passives.
pub fn validate(spec: &Specification) -> Result<(), IrValidationError> {
    if spec.declared_radio_modules().is_empty() {
        return Ok(());
    }
    let t = table()?;
    let passives = crate::power_parts::table()?;
    // The footprint holds one keepout per copper layer of a 4-layer board
    // (ruled D4): a zone on a layer the board lacks is refused, and one the
    // emitter dropped would differ from the library's footprint.
    if spec.board.copper_layers != RADIO_COPPER_LAYERS {
        return Err(refuse(
            Kind::RadioBoardLayers,
            spec.declared_radio_modules()[0].reference.as_str(),
            "board.copper_layers",
            format!(
                "a radio module's footprint holds its antenna keepout on each of {RADIO_COPPER_LAYERS} copper layers, and the board has {}",
                spec.board.copper_layers
            ),
        ));
    }
    for (i, m) in spec.declared_radio_modules().iter().enumerate() {
        let at = format!("radio_modules[{i}]");
        let r = m.reference.as_str();
        let row = t.get(&m.part).ok_or_else(|| {
            refuse(
                Kind::RadioPartUnknown,
                &m.part,
                format!("{at}.part"),
                format!("`{}` has no row in the radio parts table", m.part),
            )
        })?;
        // The supply.
        let domain = spec
            .voltage_domains
            .iter()
            .find(|d| d.id == m.supply)
            .ok_or_else(|| {
                refuse(
                    Kind::RadioSupplyUnknown,
                    &m.supply,
                    format!("{at}.supply"),
                    format!("`{}` is no declared voltage domain", m.supply),
                )
            })?;
        let (lo, hi) = crate::power::range(domain);
        if lo < u64::from(row.supply.min_mv) || hi > u64::from(row.supply.max_mv) {
            return Err(refuse(
                Kind::RadioSupplyOutOfRange,
                &m.supply,
                format!("{at}.supply"),
                format!(
                    "`{}` spans {lo} to {hi} mV, outside the module's {} to {} mV ({})",
                    m.supply,
                    row.supply.min_mv,
                    row.supply.max_mv,
                    t.cite(&row.supply.at)
                ),
            ));
        }
        // A module on a rail of its own is judged with the power tree
        // (`power::radio_rails`, `AET-WO-4411`, ruled 2026-11-25T21:17Z).
        // The row's supply requirement is judged on a declared regulator's
        // output (D5); with none, nothing judges it (`INV-06`).
        if !spec
            .power
            .as_ref()
            .is_some_and(|p| p.regulators.iter().any(|g| g.output == m.supply))
        {
            return Err(refuse(
                Kind::RadioSupplyUnsourced,
                &m.supply,
                format!("{at}.supply"),
                format!(
                    "no declared regulator outputs `{}`, so the module's {} mA supply requirement is judged by nothing ({})",
                    m.supply,
                    row.supply_current_min_ma.value,
                    t.cite(&row.supply_current_min_ma.at)
                ),
            ));
        }
        // The enable.
        if m.enable.resistor.to != m.supply {
            return Err(refuse(
                Kind::RadioEnableNotPulled,
                r,
                format!("{at}.enable.resistor.to"),
                format!(
                    "the enable pull goes to `{}`, not the module's supply `{}` ({})",
                    m.enable.resistor.to,
                    m.supply,
                    t.cite(&row.enable.at)
                ),
            ));
        }
        // The straps.
        for s in &row.straps {
            if !m.straps.contains_key(&s.pin) {
                return Err(refuse(
                    Kind::RadioStrapUndeclared,
                    r,
                    format!("{at}.straps.{}", s.pin),
                    format!("strap `{}` is not declared (`open` or a resistor)", s.pin),
                ));
            }
        }
        for (pin, strap) in &m.straps {
            let path = format!("{at}.straps.{pin}");
            let s = row.strap(pin).ok_or_else(|| {
                refuse(
                    Kind::RadioStrapUnknown,
                    r,
                    path.clone(),
                    format!("`{}` has no strap `{pin}`", m.part),
                )
            })?;
            if let RadioStrap::Resistor(p) = strap
                && p.to != m.supply
                && p.to != GND
            {
                return Err(refuse(
                    Kind::RadioStrapPullTo,
                    r,
                    format!("{path}.resistor.to"),
                    format!(
                        "strap `{pin}` is pulled to `{}`: only the module's supply `{}` or `GND` sets a level it is stated for",
                        p.to, m.supply
                    ),
                ));
            }
            let want = match s.spi_boot {
                Need::High => true,
                Need::Low => false,
                Need::Any => continue,
            };
            match level(strap, s.internal, &m.supply) {
                None => {
                    return Err(refuse(
                        Kind::RadioStrapUndriven,
                        r,
                        path,
                        format!(
                            "strap `{pin}` floats at reset, and the row holds it {} for SPI boot ({})",
                            if want { "high" } else { "low" },
                            t.cite(&s.at)
                        ),
                    ));
                }
                Some(got) if got != want => {
                    return Err(refuse(
                        Kind::RadioStrapWrongLevel,
                        r,
                        path,
                        format!(
                            "strap `{pin}` reads {} at reset, and the row holds it {} for SPI boot ({})",
                            if got { "high" } else { "low" },
                            if want { "high" } else { "low" },
                            t.cite(&s.at)
                        ),
                    ));
                }
                Some(_) => {}
            }
        }
        // The passives.
        if m.decoupling.is_empty() {
            return Err(refuse(
                Kind::RadioDecouplingUndeclared,
                r,
                format!("{at}.decoupling"),
                "no capacitor from the supply to ground is declared",
            ));
        }
        let pulls: Vec<(&PullResistor, String)> =
            std::iter::once((&m.enable.resistor, format!("{at}.enable.resistor")))
                .chain(m.straps.iter().filter_map(|(pin, s)| match s {
                    RadioStrap::Resistor(p) => Some((p, format!("{at}.straps.{pin}.resistor"))),
                    RadioStrap::Open => None,
                }))
                .collect();
        for (p, path) in &pulls {
            if !matches!(
                passives.get(&p.part).map(|row| &row.kind),
                Some(crate::power_parts::Kind::Resistor(_))
            ) {
                return Err(refuse(
                    Kind::RadioPassivePartUnknown,
                    &p.part,
                    format!("{path}.part"),
                    format!("`{}` is no resistor row of the power parts table", p.part),
                ));
            }
        }
        let capacitors = std::iter::once((&m.enable.capacitor, format!("{at}.enable.capacitor")))
            .chain(
                m.decoupling
                    .iter()
                    .enumerate()
                    .map(|(k, c)| (c, format!("{at}.decoupling[{k}]"))),
            );
        let cap = |part: &str| match passives.get(part).map(|row| &row.kind) {
            Some(crate::power_parts::Kind::Capacitor(c)) => Some(c),
            _ => None,
        };
        for (c, path) in capacitors {
            if cap(&c.part).is_none() {
                return Err(refuse(
                    Kind::RadioPassivePartUnknown,
                    &c.part,
                    format!("{path}.part"),
                    format!("`{}` is no capacitor row of the power parts table", c.part),
                ));
            }
        }
        // Each pull holds its level against the pin's input current: the
        // drop across it at the row's leakage stays inside the quarter of
        // the supply between VIH (0.75 x VDD) or VIH_nRST and the rail, and
        // VIL (0.25 x VDD) and ground (p. 21, Table 6-3), at the domain's
        // lowest voltage.
        // ohms <= (VDD/4) / I: mV * 1e6 / (4 * nA) ohms.
        let max_ohms = u128::from(lo) * 1_000_000 / (4 * u128::from(row.input_leakage_na.value));
        for (p, path) in &pulls {
            if u128::from(p.ohms) > max_ohms {
                return Err(refuse(
                    Kind::RadioPullTooWeak,
                    &p.reference,
                    format!("{path}.ohms"),
                    format!(
                        "{} ohm cannot hold its level: at {} nA its drop passes a quarter of {lo} mV above {max_ohms} ohm ({})",
                        p.ohms,
                        row.input_leakage_na.value,
                        t.cite(&row.input_leakage_na.at)
                    ),
                ));
            }
        }
        // The enable's RC keeps EN below VIH_nRST for tSTBL from the supply's
        // rise: R x C x ln(1 / (1 - 0.75)) >= tSTBL, on the capacitance the
        // specification's declared DC-bias derating retains (p. 15, p. 21).
        if let Some(c) = cap(&m.enable.capacitor.part) {
            let retained = spec
                .power
                .as_ref()
                .and_then(|p| {
                    p.conditions
                        .dc_bias_derating
                        .iter()
                        .find(|d| d.dielectrics.contains(&c.dielectric))
                })
                .map(|d| d.retained_permille);
            let pf = u128::try_from(c.rated_picofarads.value).unwrap_or(0);
            // tau (ohm pF) * ln 4 (1.386, rounded down) >= tSTBL (us * 1e6 ohm pF).
            let need = u128::from(row.enable.stable_us) * 1_000_000 * 1000;
            let have = retained
                .map(|r| u128::from(m.enable.resistor.ohms) * pf * u128::from(r) / 1000 * 1386);
            if have.is_none_or(|h| h < need) {
                return Err(refuse(
                    Kind::RadioEnableTooFast,
                    r,
                    format!("{at}.enable"),
                    match have {
                        None => format!(
                            "`{}` has no declared DC-bias derating for its dielectric, so its capacitance, and the enable's delay, is unbounded",
                            m.enable.capacitor.part
                        ),
                        Some(h) => format!(
                            "the enable's RC reaches the reset release voltage in {} ns, under the {} us the supply must be stable first ({})",
                            h / 1_000_000,
                            row.enable.stable_us,
                            t.cite(&row.enable.at)
                        ),
                    },
                ));
            }
        }
        let decoupled: u128 = m
            .decoupling
            .iter()
            .filter_map(|c| cap(&c.part))
            .map(|c| u128::try_from(c.rated_picofarads.value).unwrap_or(0))
            .sum();
        if decoupled < u128::from(row.decoupling_min_pf.value) {
            return Err(refuse(
                Kind::RadioDecouplingShort,
                r,
                format!("{at}.decoupling"),
                format!(
                    "{decoupled} pF rated from the supply to ground, under the {} pF at the module ({})",
                    row.decoupling_min_pf.value,
                    t.cite(&row.decoupling_min_pf.at)
                ),
            ));
        }
    }
    Ok(())
}

/// The verdict's line for each module (ruled: the firmware named as not
/// judged, the radio as not simulated), in document order.
pub fn not_judged(spec: &Specification) -> Vec<String> {
    // Q-SUP, ruled (a) 2026-11-25T00:00Z: the supervisor floor rule covers I2C
    // and SPI devices; a module whose VDD minimum is above the reset's falling
    // minimum is named here, not judged.
    let reset = crate::supervisor::reset_floor(spec).ok();
    spec.declared_radio_modules()
        .iter()
        .map(|m| {
            let mut line = format!(
                "{} {}: firmware not judged; radio not simulated; its ROM prints to UART0, on port `{}`, at every boot, unjudged",
                m.reference, m.part, m.serial_port
            );
            let row = table().ok().and_then(|t| t.get(&m.part));
            if let (Some(row), Some((reset_mv, why))) = (row, &reset)
                && row.supply.min_mv > *reset_mv
            {
                line.push_str(&format!(
                    "; its VDD minimum of {} mV ({}, {}) is above the reset's falling minimum of {reset_mv} mV ({why}), so it may run out of its range before the reset holds, unjudged",
                    row.supply.min_mv, row.supply.at.document, row.supply.at.section
                ));
            }
            line
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_held_table_parses() {
        let t = table().unwrap();
        let row = t.get("ESP32-C3-WROOM-02-N4").unwrap();
        assert_eq!(row.supply_current_min_ma.value, 500);
        assert_eq!(row.tx_peak_ma_typical.value, 345);
        assert_eq!(row.lifecycle.status, crate::parts::Lifecycle::Active);
    }

    #[test]
    fn a_strap_sharing_a_uart_pin_is_refused_in_the_table() {
        let bad = TABLE.replace("\"number\": \"16\"", "\"number\": \"12\"");
        assert!(parse(&bad).unwrap_err().contains("pin 12 has two roles"));
    }

    #[test]
    fn a_citation_of_an_unheld_document_is_refused() {
        let bad = TABLE.replacen(
            "\"document\": \"ESP32-C3-HDG\"",
            "\"document\": \"NOPE\"",
            1,
        );
        assert!(parse(&bad).unwrap_err().contains("which is not held"));
    }
}
