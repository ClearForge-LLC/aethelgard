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
//! **Off-board capacitance (D8).** A pin on an I2C or SPI line declares the
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
            section: "Header, side entry type (SM06B-SRSS-TB, 6 circuits)",
        },
    },
];

/// The row named `name`.
pub fn part(name: &str) -> Option<&'static Part> {
    PARTS.iter().find(|p| p.name == name)
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
    kinds
}

/// The connectors' step of `validate`, after the board: each connector by
/// document order, then its part, its pins, each pin's net and budget, then
/// its standard. References are checked with the devices' (step 8) and
/// identifiers with every other (step 2).
pub(crate) fn validate(spec: &Specification) -> Result<(), E> {
    let kinds = net_kinds(spec);
    for c in spec.declared_connectors() {
        let id = || c.reference.clone();
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
                Some(net) => Some(*kinds.get(net.as_str()).ok_or_else(|| {
                    E::ConnectorNetUnknown {
                        id: id(),
                        pin: p.pin.clone(),
                        net: net.clone(),
                    }
                })?),
            };
            let line = kind.is_some_and(NetKind::is_line);
            match &p.offboard {
                Some(_) if !line => {
                    return Err(E::ConnectorBudgetNotApplicable {
                        id: id(),
                        pin: p.pin.clone(),
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
