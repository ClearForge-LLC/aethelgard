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
use aethelgard_ir::power_parts::{self, Dielectric, Kind, PowerPartsTable, Row};

use super::{Builder, Conn, EmitError, Part};

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
    let [osc_in, osc_out]: [String; 2] =
        aethelgard_ir::names::oscillator_nets(b.spec)
            .try_into()
            .map_err(|_| EmitError::Internal("a crystal with no oscillator nets".into()))?;
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
            (
                load_capacitor(ptable, c.load_caps_femtofarads[0])?,
                c.load_caps_femtofarads[0],
            ),
            (
                load_capacitor(ptable, c.load_caps_femtofarads[1])?,
                c.load_caps_femtofarads[1],
            ),
        ],
        pins: [(pin_in, osc_in), (pin_out, osc_out)],
    }))
}

/// The footprint of every load capacitor: 0402, `4240` §9's package.
const LOAD_FOOTPRINT: &str = "Capacitor_SMD:C_0402_1005Metric";

/// The load capacitor for a declared `ff`: the first row of the power parts
/// table, by part number, that is C0G, 0402, rated exactly `ff`, with a
/// stated tolerance (`AET-WO-4240c` A2), or the refusal by name. No width is
/// required of the tolerance: `4240` §9's ±1 % or ±0.25 pF has no held
/// pullability source behind it (ruled 17:45Z), and the load is reported,
/// not judged.
fn load_capacitor(table: &'static PowerPartsTable, ff: u64) -> Result<&'static Row, EmitError> {
    table
        .rows()
        .iter()
        .find(|r| {
            let Kind::Capacitor(k) = &r.kind else {
                return false;
            };
            k.dielectric == Dielectric::C0g
                && r.footprint == LOAD_FOOTPRINT
                && k.tolerance.is_some()
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
    /// Each load capacitor's row and its declared value, fF.
    caps: [(&'static Row, u64); 2],
    /// `(MCU pin, net)` for OSC_IN, then OSC_OUT.
    pins: [(String, String); 2],
}

impl Group {
    pub(super) fn draw(self, b: &mut Builder<'_>, mcu: &str) -> Result<(), EmitError> {
        let gnd = b.ground();
        let [(pin_in, osc_in), (pin_out, osc_out)] = self.pins;
        let reference = b.reference('Y')?;
        // The footprint's pads: 1 and 3 the crystal, 2 and 4 ground.
        let conns = BTreeMap::from([
            ("1".to_owned(), Conn::Net(osc_in.clone())),
            ("2".to_owned(), gnd.clone()),
            ("3".to_owned(), Conn::Net(osc_out.clone())),
            ("4".to_owned(), gnd.clone()),
        ]);
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
            dnp: false,
            beside: None,
        });
        b.anchor_last(
            mcu,
            vec![("1".into(), pin_in.clone()), ("3".into(), pin_out.clone())],
            None,
            false,
        );
        for ((row, ff), net, pin) in [
            (self.caps[0], osc_in, pin_in),
            (self.caps[1], osc_out, pin_out),
        ] {
            let reference = b.reference('C')?;
            b.parts.push(Part {
                lib_id: row.symbol.as_str(),
                reference,
                value: format!("{} {}", super::power::farads(ff / 1000), row.part),
                footprint: row.footprint.clone(),
                conns: BTreeMap::from([("1".into(), Conn::Net(net)), ("2".into(), gnd.clone())]),
                note: None,
                owner: None,
                offboard: BTreeMap::new(),
                seat: None,
                dnp: false,
                beside: None,
            });
            b.anchor_last(mcu, vec![("1".into(), pin)], None, false);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const AT: &str = r#"{"document": "MURATA_GRM188R71C105KA12", "section": "test"}"#;

    /// A tolerance in the table's form.
    fn tol(form: &str, value: u32) -> Option<String> {
        Some(format!(r#"{{"{form}": {{"value": {value}, "at": {AT}}}}}"#))
    }

    /// The embedded table with `rows` added, each a 33 pF capacitor
    /// `(part, dielectric, footprint, tolerance)`: a value no committed row
    /// stocks, so only these rows can answer it.
    fn table(rows: &[(&str, &str, &str, Option<String>)]) -> &'static PowerPartsTable {
        let mut added = String::new();
        for (part, dielectric, footprint, tolerance) in rows {
            let tolerance = tolerance
                .as_ref()
                .map_or_else(String::new, |t| format!(r#", "tolerance": {t}"#));
            added.push_str(&format!(
                r#"{{"part": "{part}", "lifecycle": "unread", "symbol": "Device:C", "footprint": "{footprint}", "pins": {{}}, "temperature_celsius": {{"min": -55, "max": 125, "at": {AT}}}, "kind": {{"capacitor": {{"rated_picofarads": {{"value": 33, "at": {AT}}}, "rated_millivolts": {{"value": 50000, "at": {AT}}}, "dielectric": "{dielectric}", "dc_bias": null{tolerance}}}}}}},"#
            ));
        }
        let json = power_parts::POWER_PARTS_JSON.replacen(
            r#""parts": ["#,
            &format!(r#""parts": [{added}"#),
            1,
        );
        Box::leak(Box::new(power_parts::parse_power_parts(&json).unwrap()))
    }

    #[test]
    fn the_load_capacitor_is_c0g_0402_with_a_stated_tolerance() {
        let decoys = [
            ("A-X7R", "X7R", LOAD_FOOTPRINT, tol("permille", 10)),
            ("B-NO-TOLERANCE", "C0G", LOAD_FOOTPRINT, None),
            (
                "C-0603",
                "C0G",
                "Capacitor_SMD:C_0603_1608Metric",
                tol("permille", 10),
            ),
        ];
        assert!(matches!(
            load_capacitor(table(&decoys), 33_000),
            Err(EmitError::LoadCapacitorNoPart {
                femtofarads: 33_000
            })
        ));
        let mut rows = decoys.to_vec();
        rows.push((
            "E-QUARTER-PF",
            "C0G",
            LOAD_FOOTPRINT,
            tol("femtofarads", 250),
        ));
        rows.push(("D-FIVE-PERCENT", "C0G", LOAD_FOOTPRINT, tol("permille", 50)));
        // The first by part number, whatever its tolerance's width.
        assert_eq!(
            load_capacitor(table(&rows), 33_000).unwrap().part,
            "D-FIVE-PERCENT"
        );
    }
}
