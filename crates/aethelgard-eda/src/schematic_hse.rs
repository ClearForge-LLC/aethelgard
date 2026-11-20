//! The HSE support group (`AET-WO-4240c` §1.3), drawn from the declaration
//! (`mcu.hse`) and the crystal's row in `power-parts.json`, never from the
//! symbol's pin names:
//!
//! - **a crystal** across `PH0-OSC_IN` and `PH1-OSC_OUT`, each MCU pin found
//!   by its package position in ST's pin table, on the emitter's own nets
//!   `<mcu>_OSC_IN` and `<mcu>_OSC_OUT`;
//! - its pads 1 and 3 on those nets and pads 2 and 4 on `GND`: ABM8AIG
//!   "REVISED 06-13-19" p. 3 (pads 2 and 4 are ground, the lid), as KiCad's
//!   `Crystal_GND24` symbol ("GND on pins 2 and 4") and
//!   `Crystal_SMD_3225-4Pin_3.2x2.5mm` footprint number them;
//! - one load capacitor from each oscillator net to `GND`, of the declared
//!   value (`load_caps_femtofarads`, `CL1` on `OSC_IN`, `CL2` on `OSC_OUT`),
//!   a C0G 0402 row of the power parts table picked by that value
//!   ([`load_capacitor`]).
//!
//! An external clock is refused (`HseSourceNotDrawn`): no declared connector
//! pin or test point carries it to PH0 yet (ruled Q4 (a)).
//!
//! Each part is anchored by the MCU pin it serves, so the placer puts it
//! there. A specification that declares no HSE draws nothing here, and no
//! byte of its board moves.
//!
//! The support gate (`crate::support`) re-reads the emitted netlist against
//! the declaration and the row; it never reads this module's choices
//! (`RUL-20`).

use std::collections::BTreeMap;

use aethelgard_ir::PinTable;
use aethelgard_ir::clock::Hse;
use aethelgard_ir::power_parts::{self, Dielectric, Kind, PowerPartsTable, Row, Tolerance};

use super::{Builder, Conn, EmitError, Part};

/// The crystal's pads, as the footprint numbers them: `(pad, role)`.
pub(crate) const CRYSTAL_PADS: [(&str, Role); 4] = [
    ("1", Role::In),
    ("2", Role::Ground),
    ("3", Role::Out),
    ("4", Role::Ground),
];

/// What a crystal pad is tied to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Role {
    In,
    Out,
    Ground,
}


/// `pin`'s package position in `table`, as the symbol numbers it.
fn position(table: &PinTable, pin: &str) -> Result<String, EmitError> {
    table
        .pins
        .get(pin)
        .map(|p| p.position.to_string())
        .ok_or_else(|| EmitError::Internal(format!("the pin table has no {pin}")))
}

/// Ties the oscillator pins to their nets in `conns` (the MCU's connections,
/// before its part is pushed), and returns the group to draw once the MCU is.
pub(super) fn tie(
    b: &mut Builder<'_>,
    mcu: &str,
    table: &PinTable,
    conns: &mut BTreeMap<String, Conn>,
) -> Result<Option<Group>, EmitError> {
    let c = match &b.spec.mcu.hse {
        None => return Ok(None),
        Some(Hse::External(_)) => return Err(EmitError::HseSourceNotDrawn),
        Some(Hse::Crystal(c)) => c,
    };
    let ptable = power_parts::table().map_err(|e| EmitError::Internal(e.to_string()))?;
    let row = match ptable.get(&c.part) {
        Some(r) if matches!(r.kind, Kind::Crystal(_)) => r,
        // belt-and-suspenders: `validate` refuses a crystal part that is not
        // a crystal row (`4240a`'s `HseRefusal`).
        _ => {
            return Err(EmitError::Internal(format!(
                "`{}` is not a crystal row of the power parts table",
                c.part
            )));
        }
    };
    let [osc_in, osc_out] = aethelgard_ir::names::oscillator_nets(mcu);
    let osc_in = b.own_net(osc_in)?;
    let osc_out = b.own_net(osc_out)?;
    let pin_in = position(table, "PH0")?;
    let pin_out = position(table, "PH1")?;
    for (pin, net) in [(&pin_in, &osc_in), (&pin_out, &osc_out)] {
        // belt-and-suspenders: `solve` reserves PH0 and PH1 beside a crystal,
        // and the support group leaves the row's `OSC_IN` and `OSC_OUT` untied.
        if conns.insert(pin.clone(), Conn::Net(net.clone())).is_some() {
            return Err(EmitError::Internal(format!(
                "the MCU's pin {pin} is already tied"
            )));
        }
    }
    Ok(Some(Group {
        part: row.part.as_str(),
        symbol: row.symbol.as_str(),
        footprint: row.footprint.as_str(),
        caps: [
            load_capacitor(ptable, c.load_caps_femtofarads[0])?,
            load_capacitor(ptable, c.load_caps_femtofarads[1])?,
        ],
        pins: [(pin_in, osc_in), (pin_out, osc_out)],
    }))
}

/// The footprint of every load capacitor: 0402, `4240` §9's family.
const LOAD_FOOTPRINT: &str = "Capacitor_SMD:C_0402_1005Metric";

/// The load capacitor for a declared `ff`: the first row of the power parts
/// table, by part number, that is C0G, 0402, rated exactly `ff`, with a
/// stated tolerance of at most ±1 % or ±0.25 pF (`4240` §9's family,
/// `AET-WO-4240c` A2), or the refusal by name.
fn load_capacitor(table: &'static PowerPartsTable, ff: u64) -> Result<&'static Row, EmitError> {
    table
        .rows()
        .iter()
        .find(|r| {
            let Kind::Capacitor(k) = &r.kind else {
                return false;
            };
            let close = match &k.tolerance {
                Some(Tolerance::Permille(l)) => l.value <= 10,
                Some(Tolerance::Femtofarads(l)) => l.value <= 250,
                None => false,
            };
            k.dielectric == Dielectric::C0g
                && r.footprint == LOAD_FOOTPRINT
                && close
                && u64::try_from(k.rated_picofarads.value)
                    .ok()
                    .and_then(|pf| pf.checked_mul(1000))
                    == Some(ff)
        })
        .ok_or(EmitError::LoadCapacitorNoPart { femtofarads: ff })
}

/// The crystal and its capacitors, drawn once the MCU's part is pushed.
pub(super) struct Group {
    part: &'static str,
    symbol: &'static str,
    footprint: &'static str,
    caps: [&'static Row; 2],
    /// `(MCU pin, net)` for OSC_IN, then OSC_OUT.
    pins: [(String, String); 2],
}

impl Group {
    pub(super) fn draw(self, b: &mut Builder<'_>, mcu: &str) -> Result<(), EmitError> {
        let gnd = b.ground();
        let [(pin_in, osc_in), (pin_out, osc_out)] = self.pins;
        let reference = b.reference('Y')?;
        let conns = CRYSTAL_PADS
            .iter()
            .map(|&(pad, role)| {
                let to = match role {
                    Role::In => Conn::Net(osc_in.clone()),
                    Role::Out => Conn::Net(osc_out.clone()),
                    Role::Ground => gnd.clone(),
                };
                (pad.to_owned(), to)
            })
            .collect();
        b.parts.push(Part {
            lib_id: self.symbol,
            reference,
            value: self.part.to_owned(),
            footprint: self.footprint.to_owned(),
            conns,
            note: None,
            owner: None,
            offboard: BTreeMap::new(),
            seat: None,
        });
        b.anchor_last(
            mcu,
            vec![("1".into(), pin_in.clone()), ("3".into(), pin_out.clone())],
            None,
            false,
        );
        for (row, net, pin) in [
            (self.caps[0], osc_in, pin_in),
            (self.caps[1], osc_out, pin_out),
        ] {
            let Kind::Capacitor(k) = &row.kind else {
                return Err(EmitError::Internal(format!("`{}` is not a capacitor", row.part)));
            };
            let pf = u64::try_from(k.rated_picofarads.value)
                .map_err(|_| EmitError::Internal("a negative capacitance".into()))?;
            let reference = b.reference('C')?;
            b.parts.push(Part {
                lib_id: row.symbol.as_str(),
                reference,
                value: format!("{} {}", super::power::farads(pf), row.part),
                footprint: row.footprint.clone(),
                conns: BTreeMap::from([("1".into(), Conn::Net(net)), ("2".into(), gnd.clone())]),
                note: None,
                owner: None,
                offboard: BTreeMap::new(),
                seat: None,
            });
            b.anchor_last(mcu, vec![("1".into(), pin)], None, false);
        }
        Ok(())
    }
}
