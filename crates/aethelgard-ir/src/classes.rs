//! The specification's `net_classes` and `diff_pairs` (`AET-WO-1308` §1.1 and
//! §1.3, readings F2 to F4 in its FEEDBACK). Both keys are required, as every
//! key is (`INV-06`); an empty array declares none. Units are micrometres.
//!
//! ```json
//! "net_classes": [ { "name": "power", "track_width_um": 250,
//!                    "via": { "diameter_um": 700, "drill_um": 300 } | null,
//!                    "nets": ["VDD_3V3", "GND"] } ],
//! "diff_pairs":  [ { "name": "USB", "nets": ["SPI_SCK", "SPI_MOSI"],
//!                    "gap_um": 300, "width_um": 200 } ]
//! ```
//!
//! `via` is a required key whose value may be `null` (the router then uses the
//! rules' via). A pair's `nets` are `[positive, negative]`.
//!
//! **What step 13a of `Specification::validate` refuses** (`RUL-03`, `RUL-14`,
//! `INV-06`), in this order, classes first and then pairs, each in file order:
//! - a name that is not an identifier (`RUL-14`), or that repeats another
//!   class's or pair's name (`DuplicateClassName`; classes and pairs share one
//!   namespace);
//! - a class with no nets (`ClassNoNets`);
//! - a pair that does not name exactly two distinct nets (`PairNets`), or one
//!   that names a voltage domain or `GND` (`PairOnRail`): a rail, a plane or
//!   ground is not a coupled conductor (F4);
//! - a net that is not a declared signal net, voltage domain or `GND`
//!   (`ClassUnknownNet`, naming the class or pair and the net);
//! - a net in two classes, which includes a pair, since a pair is its nets'
//!   class and declares a width (`NetInTwoClasses`, naming the net and both
//!   names; F4);
//! - a `track_width_um` or pair `width_um` under
//!   `max(FLOOR_UM, board.rules.track_width_um)` (`ClassBelowMinimum`), or
//!   above `MAX_THICKNESS_UM` (`ClassAboveMaximum`);
//! - a via whose drill is under `board.rules.via_drill_um`, whose diameter is
//!   over `MAX_THICKNESS_UM`, or whose ring `(diameter - drill) / 2` is under
//!   the rules' own `(via_diameter_um - via_drill_um) / 2` (`ViaRingUnderRules`;
//!   F3). The ring is compared as `diameter - drill` in integers, and a
//!   diameter at or under the drill has no ring;
//! - a pair gap under `board.rules.clearance_um`, or over `MAX_THICKNESS_UM`.
//!
//! Every class's clearance is the declared `board.rules.clearance_um`, so no
//! clearance field exists (F2).

use std::collections::{BTreeMap, BTreeSet};

use serde::de::value::MapAccessDeserializer;
use serde::de::{MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

use crate::board::{Board, FLOOR_UM, MAX_THICKNESS_UM};
use crate::{IdentifierKind, IrValidationError, check_identifier};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct NetClass {
    pub name: String,
    pub track_width_um: u32,
    /// The class's via; `null` takes the rules' via. Required as a key.
    #[serde(deserialize_with = "Option::deserialize")]
    pub via: Option<ClassVia>,
    pub nets: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct ClassVia {
    pub diameter_um: u32,
    pub drill_um: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct DiffPair {
    pub name: String,
    /// `[positive, negative]`.
    pub nets: Vec<String>,
    pub gap_um: u32,
    pub width_um: u32,
}

/// Half a micrometre is a ring: 125 or 12.5, never rounded.
fn ring_text(twice: i64) -> String {
    if twice % 2 == 0 {
        (twice / 2).to_string()
    } else {
        format!("{}.5", twice / 2)
    }
}

/// Step 13a. `nets` are the declared signal nets and `domains` the voltage
/// domain ids; `GND` is always a net.
pub fn validate(
    classes: &[NetClass],
    pairs: &[DiffPair],
    board: &Board,
    domains: &BTreeSet<&str>,
    nets: &BTreeSet<&str>,
) -> Result<(), IrValidationError> {
    use IrValidationError as E;
    let rules = &board.rules;
    let min_width = FLOOR_UM.max(rules.track_width_um);
    let min_ring2 = i64::from(rules.via_diameter_um) - i64::from(rules.via_drill_um);
    let mut names = BTreeSet::new();
    let mut owner: BTreeMap<&str, &str> = BTreeMap::new();

    let name = |value: &str, path: String| {
        check_identifier(IdentifierKind::Plain, value)
            .map_err(|rule| E::InvalidIdentifier { path, rule })
    };
    let width = |value: u32, path: String| {
        if value < min_width {
            return Err(E::ClassBelowMinimum {
                path,
                value,
                minimum: min_width,
                of: "the larger of the engine floor and board.rules.track_width_um",
            });
        }
        if value > MAX_THICKNESS_UM {
            return Err(E::ClassAboveMaximum {
                path,
                value,
                maximum: MAX_THICKNESS_UM,
            });
        }
        Ok(())
    };

    for (i, c) in classes.iter().enumerate() {
        let at = format!("net_classes[{i}]");
        name(&c.name, format!("{at}.name"))?;
        if !names.insert(c.name.as_str()) {
            return Err(E::DuplicateClassName { id: c.name.clone() });
        }
        if c.nets.is_empty() {
            return Err(E::ClassNoNets { id: c.name.clone() });
        }
        for net in &c.nets {
            known(&c.name, net, domains, nets)?;
            claim(&mut owner, net, &c.name)?;
        }
        width(c.track_width_um, format!("{at}.track_width_um"))?;
        if let Some(v) = &c.via {
            let drill = format!("{at}.via.drill_um");
            if v.drill_um < rules.via_drill_um {
                return Err(E::ClassBelowMinimum {
                    path: drill,
                    value: v.drill_um,
                    minimum: rules.via_drill_um,
                    of: "board.rules.via_drill_um",
                });
            }
            if v.diameter_um > MAX_THICKNESS_UM {
                return Err(E::ClassAboveMaximum {
                    path: format!("{at}.via.diameter_um"),
                    value: v.diameter_um,
                    maximum: MAX_THICKNESS_UM,
                });
            }
            let ring2 = i64::from(v.diameter_um) - i64::from(v.drill_um);
            if ring2 < min_ring2 {
                return Err(E::ViaRingUnderRules {
                    id: c.name.clone(),
                    ring_um: ring_text(ring2.max(0)),
                    minimum_um: ring_text(min_ring2),
                });
            }
        }
    }

    for (i, p) in pairs.iter().enumerate() {
        let at = format!("diff_pairs[{i}]");
        name(&p.name, format!("{at}.name"))?;
        if !names.insert(p.name.as_str()) {
            return Err(E::DuplicateClassName { id: p.name.clone() });
        }
        let [pos, neg] = p.nets.as_slice() else {
            return Err(E::PairNets {
                id: p.name.clone(),
                rule: "a pair names exactly two nets, [positive, negative]",
            });
        };
        if pos == neg {
            return Err(E::PairNets {
                id: p.name.clone(),
                rule: "a pair's two nets are distinct",
            });
        }
        for net in [pos, neg] {
            if net == "GND" || domains.contains(net.as_str()) {
                return Err(E::PairOnRail {
                    id: p.name.clone(),
                    net: net.clone(),
                });
            }
        }
        for net in [pos, neg] {
            known(&p.name, net, domains, nets)?;
            claim(&mut owner, net, &p.name)?;
        }
        width(p.width_um, format!("{at}.width_um"))?;
        if p.gap_um < rules.clearance_um {
            return Err(E::ClassBelowMinimum {
                path: format!("{at}.gap_um"),
                value: p.gap_um,
                minimum: rules.clearance_um,
                of: "board.rules.clearance_um",
            });
        }
        if p.gap_um > MAX_THICKNESS_UM {
            return Err(E::ClassAboveMaximum {
                path: format!("{at}.gap_um"),
                value: p.gap_um,
                maximum: MAX_THICKNESS_UM,
            });
        }
    }
    Ok(())
}

fn known(
    owner: &str,
    net: &str,
    domains: &BTreeSet<&str>,
    nets: &BTreeSet<&str>,
) -> Result<(), IrValidationError> {
    if net.contains(['\'', '?']) {
        // The gate names a class's nets inside KiCad rule expressions
        // (`aethelgard_eda::rules`): `'` ends the quote, and `==` reads `?` as
        // a wildcard, so a class on `SPI_SC?` would bind `SPI_SCK` too
        // (`AET-WO-1308` adversarial pass, measured). RUL-14 already refuses
        // `*`. Such a net is refused, never escaped by guess.
        Err(IrValidationError::ClassNetNotLiteral {
            id: net.to_owned(),
            class: owner.to_owned(),
        })
    } else if net == "GND" || domains.contains(net) || nets.contains(net) {
        Ok(())
    } else {
        Err(IrValidationError::ClassUnknownNet {
            id: net.to_owned(),
            class: owner.to_owned(),
        })
    }
}

fn claim<'a>(
    owner: &mut BTreeMap<&'a str, &'a str>,
    net: &'a str,
    by: &'a str,
) -> Result<(), IrValidationError> {
    match owner.insert(net, by) {
        Some(first) => Err(IrValidationError::NetInTwoClasses {
            id: net.to_owned(),
            first: first.to_owned(),
            second: by.to_owned(),
        }),
        None => Ok(()),
    }
}

crate::object_only!(NetClass, ClassVia, DiffPair);
