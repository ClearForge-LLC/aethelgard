//! Off-board connectors (`AET-WO-4300f`, ruled D1 on PR #134): where a net
//! leaves the board.
//!
//! A specification declares each connector with its `part` (a row of
//! [`PARTS`]), its `standard` (a row of [`crate::ds009::PORTS`], or `null`
//! for an explicit map), and every signal pin with its net or `null`. The
//! mounting pads are not declared: the emitter puts them on `GND` (D3).
//!
//! A connector lives in the [`crate::Specification`] only. Nothing solves it
//! and the [`crate::SolvedTopology`] never carries it, so the twin, which
//! reads only the latter (`RUL-16`), cannot see one.
//!
//! **Off-board capacitance (D8).** A pin on an I2C or SPI line, and only
//! there (not a motor output's or a serial port's), declares the
//! capacitance beyond the board: the mated contact, the cable and the remote
//! pins, with the source of the number. Extraction adds it to the board's own
//! copper, so a line's bound stays a bound (`RUL-19` (1)).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::de::value::MapAccessDeserializer;
use serde::de::{MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{IrValidationError as E, Specification};

/// A connector series: the contact pitch and the mating plug.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Series {
    /// JST GH, 1.25 mm.
    JstGh,
    /// JST SH, 1.00 mm.
    JstSh,
}

/// Where a row's values were read: a document held by sha256 in
/// `~/.cache/aet-4300f/ds/` on Furnace, and the section.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cited {
    pub document: &'static str,
    pub sha256: &'static str,
    pub section: &'static str,
}

/// JST's GH catalogue.
pub const JST_GH: &str = "JST GH series catalogue, eGH.pdf";
pub const JST_GH_SHA256: &str = "b1dcb317b6b9a4fbbedd2dbf42c64c95306a252d23de8933b85fcf161240b722";
/// JST's SH catalogue.
pub const JST_SH: &str = "JST SH series catalogue, eSH.pdf";
pub const JST_SH_SHA256: &str = "ea3071ca5ee5a6069eba534fa39a10f34c9fb742ee42135a69ab4156bfa0f5de";

/// One connector part: a side-entry SMT header whose two mounting pads the
/// emitter puts on `GND`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Part {
    /// The manufacturer's part number, which is also its BOM value.
    pub name: &'static str,
    pub series: Series,
    /// Signal pins, numbered `1` to `pins`; the mounting pads are not counted.
    pub pins: u8,
    pub cited: Cited,
}

/// The connector parts (ruled D4): JST GH 4, 6 and 10 for DS-009's external
/// ports, and JST SH 6 for its debug port (Sentinel-02's SWD).
pub const PARTS: [Part; 4] = [
    Part {
        name: "SM04B-GHS-TB",
        series: Series::JstGh,
        pins: 4,
        cited: Cited {
            document: JST_GH,
            sha256: JST_GH_SHA256,
            section: "p. 3, Header, side entry type (SM04B-GHS-TB, 4 circuits)",
        },
    },
    Part {
        name: "SM06B-GHS-TB",
        series: Series::JstGh,
        pins: 6,
        cited: Cited {
            document: JST_GH,
            sha256: JST_GH_SHA256,
            section: "p. 3, Header, side entry type (SM06B-GHS-TB, 6 circuits)",
        },
    },
    Part {
        name: "SM10B-GHS-TB",
        series: Series::JstGh,
        pins: 10,
        cited: Cited {
            document: JST_GH,
            sha256: JST_GH_SHA256,
            section: "p. 3, Header, side entry type (SM10B-GHS-TB, 10 circuits)",
        },
    },
    Part {
        name: "SM06B-SRSS-TB",
        series: Series::JstSh,
        pins: 6,
        cited: Cited {
            document: JST_SH,
            sha256: JST_SH_SHA256,
            section: "p. 3, Header, side entry type (SM06B-SRSS-TB, 6 circuits)",
        },
    },
];

/// The row named `name`.
pub fn part(name: &str) -> Option<&'static Part> {
    PARTS.iter().find(|p| p.name == name)
}

/// How a through-hole pad joins a copper zone of its net (`AET-WO-1309` D1,
/// D2): the power-in row declares it, and slice B's `Stack::of` maps it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PadJoin {
    Solid,
    Relief,
}

/// A power-in connector: where the battery enters the board (`AET-WO-4300d`,
/// ruled D2 and D7). A specification names it in `power.input`, never in
/// `connectors`, so `4300f`'s emitter never meets one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PowerIn {
    pub name: &'static str,
    /// The rated current, mA, and the condition the maker states it under.
    pub rated_milliamps: u32,
    pub rating_condition: &'static str,
    pub cited: Cited,
    /// The operating temperature, °C, and where it was read.
    pub temperature_celsius: (i32, i32),
    pub temperature_cited: Cited,
    /// AMASS's other documents, which state other ratings: cited beside the
    /// judged one so the disagreement is in every verdict (ruled D-B6).
    pub disagreeing: [Cited; 2],
    /// Declared solid: a relieved through-hole pad on a supply path is
    /// refused until its spokes are priced (`1309` D2, ruled at 10:30 UTC).
    pub join: PadJoin,
}

/// AMASS's board-type table for the XT30 family, held as a screenshot of the
/// maker's page (the architect's fetch, 2026-11-15).
pub const AMASS_XT30_TABLE: &str =
    "AMASS XT30 2PIN board-type table (amassconnectors.com, screenshot)";
pub const AMASS_XT30_TABLE_SHA256: &str =
    "7fa569aa04e90c72a5a37333efd2bf057bf63aac6265d2485a47b073d3e0c7bc";
/// AMASS's own XT30PW-M specification V1.2, a copy hosted by a distributor
/// (store.comet.bg): the sheet names its maker and its exact part, so it is
/// the rating judged (ruled D-B6). AMASS's board-type table gives the XT30PW
/// family 20 A at a rise under 85 K, and AMASS's 2025V0 sheet gives 20 A at
/// 80 V for the XT30PW-M30, a different part number.
pub const AMASS_XT30PW_M_V12: &str =
    "AMASS specification XT30PW-M, version V1.2 (copy hosted by store.comet.bg)";
pub const AMASS_XT30PW_M_V12_SHA256: &str =
    "c57b5a373507b2cd0004dbd3e043448347b5ff8e95d6435d61a409c81cdcf185";
/// AMASS's own 2025V0 sheet from its site, for the XT30PW-M30.
pub const AMASS_XT30PW_M30_2025: &str =
    "AMASS product specification XT30PW-M30, version 2025V0 (china-amass.net)";
pub const AMASS_XT30PW_M30_2025_SHA256: &str =
    "475a4aa380f31dff1962c9efa578b63c966701d755eae6188739a76b46ad25ef";

pub const POWER_IN: [PowerIn; 1] = [PowerIn {
    name: "XT30PW-M",
    rated_milliamps: 15_000,
    rating_condition: "rated current, as AMASS's V1.2 sheet for the XT30PW-M states it (ruled D-B6); AMASS's board-type table states 20 A at a rise under 85 K for the XT30PW family, and its 2025V0 sheet 20 A at 80 V for the XT30PW-M30",
    cited: Cited {
        document: AMASS_XT30PW_M_V12,
        sha256: AMASS_XT30PW_M_V12_SHA256,
        section: "the one page: rated current 15A, rated voltage DC 500V",
    },
    temperature_celsius: (-20, 120),
    temperature_cited: Cited {
        document: AMASS_XT30PW_M_V12,
        sha256: AMASS_XT30PW_M_V12_SHA256,
        section: "the one page: operating temperature -20 C to 120 C",
    },
    disagreeing: [
        Cited {
            document: AMASS_XT30_TABLE,
            sha256: AMASS_XT30_TABLE_SHA256,
            section: "the XT30PW row: current rating 20A MAX (delta < 85 C), 500V DC",
        },
        Cited {
            document: AMASS_XT30PW_M30_2025,
            sha256: AMASS_XT30PW_M30_2025_SHA256,
            section: "current rating 20 A (16AWG, delta <= 85 K), rated voltage 80 V DC",
        },
    ],
    join: PadJoin::Solid,
}];

/// The power-in row named `name`.
pub fn power_in(name: &str) -> Option<&'static PowerIn> {
    POWER_IN.iter().find(|p| p.name == name)
}

/// One connector, as declared.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct Connector {
    pub reference: String,
    pub part: String,
    /// A row of [`crate::ds009::PORTS`] by its name, or `null`: an explicit
    /// map, judged by no standard. Required as a key (`INV-06`).
    #[serde(deserialize_with = "Option::deserialize")]
    pub standard: Option<String>,
    pub pins: Vec<ConnectorPin>,
}

/// One signal pin: its number, its net or `null` (drawn as a no-connect), and
/// its off-board capacitance or `null`. Both are required as keys.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct ConnectorPin {
    pub pin: String,
    #[serde(deserialize_with = "Option::deserialize")]
    pub net: Option<String>,
    #[serde(deserialize_with = "Option::deserialize")]
    pub offboard: Option<OffBoard>,
}

/// The capacitance a line carries beyond the board through this pin: the
/// mated contact, the cable and the remote pins, at most, with where the
/// number comes from (a document, or a condition the design states).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct OffBoard {
    pub capacitance_femtofarads: u64,
    pub source: String,
}

crate::object_only!(Connector, ConnectorPin, OffBoard);

/// What a net is, for a connector pin: what the specification declares it as.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NetKind<'s> {
    Ground,
    /// A voltage domain and its nominal, in millivolts.
    Domain(u32),
    I2cScl(&'s str),
    I2cSda(&'s str),
    /// An SPI bus's clock, data or chip-select line.
    Spi,
    /// A motor output's net (`AET-WO-4300a` §1.8): it leaves the board at
    /// a connector, on no bus, so it declares no off-board capacitance.
    Motor,
    /// One of the MCU's debug nets (`AET-WO-4300g`).
    Debug,
    /// A serial port's TX or RX net, with the port's label
    /// (`AET-WO-4300b`): on no bus, so, like a motor output's, it declares
    /// no off-board capacitance (D8 covers I2C and SPI lines only).
    SerialTx(&'s str),
    SerialRx(&'s str),
}

impl NetKind<'_> {
    /// An I2C or SPI line: a pin on one declares its off-board budget (D8).
    fn is_line(self) -> bool {
        matches!(self, Self::I2cScl(_) | Self::I2cSda(_) | Self::Spi)
    }
}

/// Every net a connector pin may name, by what it is. A connector cannot
/// invent a net: it joins nets the specification already declares.
fn net_kinds(spec: &Specification) -> BTreeMap<&str, NetKind<'_>> {
    let mut kinds = BTreeMap::from([(crate::GND, NetKind::Ground)]);
    for n in crate::DEBUG_NETS {
        kinds.insert(n, NetKind::Debug);
    }
    for d in &spec.voltage_domains {
        kinds.insert(d.id.as_str(), NetKind::Domain(d.nominal_millivolts));
    }
    for b in &spec.i2c_buses {
        kinds.insert(b.scl.net.as_str(), NetKind::I2cScl(&b.id));
        kinds.insert(b.sda.net.as_str(), NetKind::I2cSda(&b.id));
    }
    for b in &spec.spi_buses {
        for s in [&b.sck, &b.mosi, &b.miso] {
            kinds.insert(s.net.as_str(), NetKind::Spi);
        }
    }
    for d in spec.declared_spi_devices() {
        kinds.insert(d.cs.net.as_str(), NetKind::Spi);
    }
    for m in &spec.motor_outputs {
        kinds.insert(m.label.as_str(), NetKind::Motor);
    }
    for p in &spec.serial_ports {
        kinds.insert(p.tx.net.as_str(), NetKind::SerialTx(&p.label));
        kinds.insert(p.rx.net.as_str(), NetKind::SerialRx(&p.label));
    }
    kinds
}

/// Every net a connector pin, or a test point (`AET-WO-1312`), may name.
pub(crate) fn nameable(spec: &Specification) -> std::collections::BTreeSet<&str> {
    net_kinds(spec).into_keys().collect()
}

/// The largest off-board budget one connector pin may declare: 1 µF, three
/// orders above UM10204's 550 pF Fast-mode Plus bus limit, so no real line is
/// refused, and small enough that no board's sum of terms overflows the
/// bound's integers (`AET-WO-4300f` A4: `u64::MAX` reached `extract`'s
/// `Overflow`).
pub const MAX_OFFBOARD_FEMTOFARADS: u64 = 1_000_000_000;

/// Step 15 of `validate`, after the parts (step 14): each connector by
/// document order, then its part, its pins, each pin's net and budget, then
/// its standard; then each serial port's TX and RX, each on some connector's
/// pin. References are checked with the devices' (step 8) and identifiers
/// with every other (step 2).
pub(crate) fn validate(spec: &Specification) -> Result<(), E> {
    let kinds = net_kinds(spec);
    for c in spec.declared_connectors() {
        let id = || c.reference.clone();
        // `AET-WO-4300f` A3: the emitter numbers its own parts `U<n>`, `R<n>`
        // and `C<n>`, skipping a reference the specification spells so; a
        // connector there renumbers the MCU, or in another case is refused
        // only at `emit`.
        let mut chars = c.reference.chars();
        if chars.next().is_some_and(|f| "UuRrCc".contains(f))
            && !chars.as_str().is_empty()
            && chars.all(|d| d.is_ascii_digit())
        {
            return Err(E::ConnectorReferenceReserved { id: id() });
        }
        let part = part(&c.part).ok_or_else(|| E::UnknownConnectorPart {
            id: id(),
            part: c.part.clone(),
        })?;
        let mut seen = BTreeSet::new();
        for p in &c.pins {
            let number = p.pin.parse::<u8>().ok().filter(|n| p.pin == n.to_string());
            if !number.is_some_and(|n| (1..=part.pins).contains(&n)) {
                return Err(E::ConnectorPinUnknown {
                    id: id(),
                    pin: p.pin.clone(),
                    pins: part.pins,
                });
            }
            if !seen.insert(p.pin.as_str()) {
                return Err(E::ConnectorPinRepeated {
                    id: id(),
                    pin: p.pin.clone(),
                });
            }
        }
        if let Some(missing) = (1..=part.pins)
            .map(|n| n.to_string())
            .find(|n| !seen.contains(n.as_str()))
        {
            return Err(E::ConnectorPinMissing {
                id: id(),
                pin: missing,
            });
        }
        for p in &c.pins {
            let kind = match &p.net {
                None => None,
                Some(net) => {
                    Some(
                        *kinds
                            .get(net.as_str())
                            .ok_or_else(|| E::ConnectorNetUnknown {
                                id: id(),
                                pin: p.pin.clone(),
                                net: net.clone(),
                            })?,
                    )
                }
            };
            let line = kind.is_some_and(NetKind::is_line);
            match &p.offboard {
                Some(_) if !line => {
                    return Err(E::ConnectorBudgetNotApplicable {
                        id: id(),
                        pin: p.pin.clone(),
                    });
                }
                Some(o) if o.capacitance_femtofarads > MAX_OFFBOARD_FEMTOFARADS => {
                    return Err(E::ConnectorBudgetTooLarge {
                        id: id(),
                        pin: p.pin.clone(),
                        femtofarads: o.capacitance_femtofarads,
                        max: MAX_OFFBOARD_FEMTOFARADS,
                    });
                }
                Some(o) if o.capacitance_femtofarads == 0 => {
                    return Err(E::ConnectorBudgetMissing {
                        id: id(),
                        pin: p.pin.clone(),
                    });
                }
                None if line => {
                    return Err(E::ConnectorBudgetMissing {
                        id: id(),
                        pin: p.pin.clone(),
                    });
                }
                _ => {}
            }
        }
        if let Some(standard) = &c.standard {
            crate::ds009::judge(spec, c, part, standard, &kinds)?;
        }
    }
    // Each serial port's TX and RX end at a declared connector pin (the
    // architect's ruling 1 on `AET-WO-4300b`; adversarial F9).
    for p in &spec.serial_ports {
        for (line, net) in [("TX", &p.tx.net), ("RX", &p.rx.net)] {
            let at_pin = spec
                .declared_connectors()
                .iter()
                .flat_map(|c| &c.pins)
                .any(|pin| pin.net.as_ref() == Some(net));
            if !at_pin {
                return Err(E::SerialPortNotAtConnector {
                    label: p.label.clone(),
                    line,
                    net: net.clone(),
                });
            }
        }
    }
    Ok(())
}

/// Step 2 for the connectors: each reference, part, pin, net and source.
/// `standard` names a table row, which may hold a space, so it is checked
/// against the table in the connectors' own step, not as an identifier.
pub(crate) fn identifiers(spec: &Specification) -> Result<(), E> {
    for (i, c) in spec.declared_connectors().iter().enumerate() {
        let at = format!("connectors[{i}]");
        crate::identifier(crate::IdentifierKind::Reference, &c.reference, || {
            format!("{at}.reference")
        })?;
        crate::plain(&c.part, || format!("{at}.part"))?;
        for (j, p) in c.pins.iter().enumerate() {
            let at = format!("{at}.pins[{j}]");
            crate::plain(&p.pin, || format!("{at}.pin"))?;
            crate::optional_identifier(&p.net, || format!("{at}.net"))?;
            if let Some(o) = &p.offboard {
                crate::board::source(&o.source, &format!("{at}.offboard.source"))?;
            }
        }
    }
    Ok(())
}

/// Step 17 (`AET-WO-4300g` §1.4): one declared connector carries `SWDIO`,
/// `SWCLK`, `GND` and the MCU's supply (DS-009's debug ports carry those and
/// no NRST, so NRST is not required). Its standard, if any, was judged in
/// step 15.
pub(crate) fn programming_path(spec: &Specification) -> Result<(), E> {
    programming_connector(spec)
        .map(|_| ())
        .ok_or_else(|| E::NoProgrammingPath {
            supply: spec.mcu.supply.clone(),
        })
}

/// The connector that programs the part, by document order: the first that
/// carries `SWDIO`, `SWCLK`, `GND` and the supply, with whether it also
/// carries `NRST`.
pub fn programming_connector(spec: &Specification) -> Option<(&Connector, bool)> {
    let supply = spec.mcu.supply.as_str();
    spec.declared_connectors()
        .iter()
        .find(|c| {
            [crate::SWDIO, crate::SWCLK, crate::GND, supply]
                .iter()
                .all(|want| c.pins.iter().any(|p| p.net.as_deref() == Some(*want)))
        })
        .map(|c| {
            let nrst = c.pins.iter().any(|p| p.net.as_deref() == Some(crate::NRST));
            (c, nrst)
        })
}

/// Step 18 (`AET-WO-4300g` §1.4): the MCU's supply is driven by exactly one
/// declared source. With `mcu.supply_source` `null`, a regulator of `power`
/// must output the supply. With it set, it names a declared connector's
/// listed pin on the supply, on a connector other than the programming one
/// (whose supply pin is the debugger's target-voltage sense, adversarial A3),
/// and no regulator drives the supply too.
pub(crate) fn supply_source(spec: &Specification) -> Result<(), E> {
    let supply = spec.mcu.supply.as_str();
    let refuse = |why: String| E::SupplyHasNoSource {
        supply: supply.to_owned(),
        why,
    };
    let regulator = spec
        .power
        .iter()
        .flat_map(|p| &p.regulators)
        .find(|r| r.output == supply);
    match (&spec.mcu.supply_source, regulator) {
        (None, Some(_)) => Ok(()),
        (None, None) => Err(refuse(
            "`mcu.supply_source` is null and no regulator of `power` outputs it; name the connector pin it enters by, or declare its regulator".to_owned(),
        )),
        (Some(_), Some(r)) => Err(E::SupplyHasTwoSources {
            supply: supply.to_owned(),
            regulator: r.reference.clone(),
        }),
        (Some(s), None) => {
            let c = spec
                .declared_connectors()
                .iter()
                .find(|c| c.reference == s.connector)
                .ok_or_else(|| {
                    refuse(format!(
                        "`mcu.supply_source` names connector `{}`, which is not declared",
                        s.connector
                    ))
                })?;
            if programming_connector(spec).is_some_and(|(p, _)| p.reference == c.reference) {
                return Err(refuse(format!(
                    "`mcu.supply_source` names `{}`, the connector that programs the part: its supply pin is the debugger's target-voltage sense, not a source; name the connector the supply enters by",
                    s.connector
                )));
            }
            match c.pins.iter().find(|p| p.pin == s.pin) {
                Some(p) if p.net.as_deref() == Some(supply) => Ok(()),
                Some(p) => Err(refuse(format!(
                    "`mcu.supply_source` names `{}` pin `{}`, which is on {} rather than the supply",
                    s.connector,
                    s.pin,
                    p.net
                        .as_deref()
                        .map_or_else(|| "no net".to_owned(), |n| format!("`{n}`"))
                ))),
                None => Err(refuse(format!(
                    "`mcu.supply_source` names `{}` pin `{}`, which the connector does not list",
                    s.connector, s.pin
                ))),
            }
        }
    }
}
