//! Each radio module (`AET-WO-4411`), drawn after every other part so that
//! no other part's reference or schematic position moves: the module on
//! its supply and ground, its U0TXD on the port's RX net and its U0RXD on
//! the port's TX net (through a UART crossing, on the crossing's B-side
//! nets), its enable's RC delay, each strap's declared pull, and
//! its decoupling. A strap left `open` and every unused pin get a
//! no-connect. Each passive is anchored to the module pin it serves.

use std::collections::BTreeMap;

use aethelgard_ir::power::Resistor;
use aethelgard_ir::radio::{self, RadioStrap};

use super::power::{anchor, capacitor, resistor};
use super::{Builder, Conn, EmitError, Part};

/// How far a module's passive sits from the pin it serves.
const NEAR_NM: i64 = 8_000_000;
const NEAR_RULE: &str = "ESP32-C3-WROOM-02 datasheet v1.7 p. 33, Figure 9-1: the enable's RC delay and the decoupling at the module (8.0 mm, an engine choice: no distance is stated; pins 1 and 2 sit 1.1 and 2.6 mm below the board edge the antenna overhangs, and the edge's 2 mm margin keeps every part below that, so four parts share the room beside two pins)";

/// How far a strap's pull sits from its pin: farther, since a strap is a
/// static level at reset and the module's right side is the board's edge.
const STRAP_NM: i64 = 15_000_000;
const STRAP_RULE: &str = "ESP32-C3-WROOM-02 datasheet v1.7 p. 12, Table 4-1: a strap is read as a static level at reset, so its pull's distance carries no edge rate (15.0 mm, an engine choice: no distance is stated)";

fn not_drawable(reference: &str, why: impl Into<String>) -> EmitError {
    EmitError::PowerNotDrawable {
        reference: reference.to_owned(),
        why: why.into(),
    }
}

pub(super) fn draw(b: &mut Builder<'_>) -> Result<(), EmitError> {
    let spec = b.spec;
    let table = radio::table().map_err(EmitError::InvalidSpecification)?;
    for m in spec.declared_radio_modules() {
        let r = m.reference.as_str();
        // belt-and-suspenders: `validate` refuses both first (step 27 and
        // step 15's head); a library caller that draws without it is refused
        // here.
        let row = table
            .get(&m.part)
            .ok_or_else(|| not_drawable(r, format!("`{}` has no radio parts row", m.part)))?;
        let port = spec
            .serial_ports
            .iter()
            .find(|p| p.label == m.serial_port)
            .ok_or_else(|| not_drawable(r, format!("no serial port `{}`", m.serial_port)))?;
        // through a UART crossing, on its B side's nets, which the power
        // tree drew and claimed (`AET-WO-4411` (c1))
        let (txd, rxd) = match aethelgard_ir::power::uart_b_nets(spec, &port.label) {
            Some((tx_b, rx_b)) => (rx_b, tx_b),
            None => (port.rx.net.clone(), port.tx.net.clone()),
        };
        let supply = b.rail(&m.supply);
        let gnd = b.ground();
        let en = b.own_net(radio::enable_net(r))?;
        let mut conns = BTreeMap::from([
            (row.supply.number.clone(), supply.clone()),
            (row.enable.number.clone(), Conn::Net(en.clone())),
            (row.uart.tx_number.clone(), Conn::Net(txd)),
            (row.uart.rx_number.clone(), Conn::Net(rxd)),
        ]);
        for g in &row.ground_pins {
            conns.insert(g.clone(), gnd.clone());
        }
        let mut straps = Vec::new();
        for s in &row.straps {
            if let Some(RadioStrap::Resistor(p)) = m.straps.get(&s.pin) {
                let net = b.own_net(radio::strap_net(r, &s.pin))?;
                conns.insert(s.number.clone(), Conn::Net(net.clone()));
                straps.push((s.number.as_str(), p, net));
            }
        }
        b.parts.push(Part {
            lib_id: row.symbol.as_str(),
            reference: m.reference.clone(),
            value: row.part.clone(),
            footprint: row.footprint.clone(),
            conns,
            note: Some("firmware not judged; radio not simulated (AET-WO-4411)".to_owned()),
            owner: None,
            offboard: BTreeMap::new(),
            seat: None,
            dnp: false,
            beside: None,
        });
        let plain = |p: &aethelgard_ir::pin_devices::PullResistor| Resistor {
            reference: p.reference.clone(),
            part: p.part.clone(),
            ohms: p.ohms,
        };
        // The decoupling first, so it takes the spots nearest the supply pin.
        let vdd = row.supply.number.as_str();
        for c in &m.decoupling {
            capacitor(b, &c.reference, &c.part, supply.clone(), gnd.clone())?;
            anchor(b, r, &[("1", vdd)], NEAR_NM, NEAR_RULE);
        }
        let en_pin = row.enable.number.as_str();
        resistor(
            b,
            &plain(&m.enable.resistor),
            Conn::Net(en.clone()),
            supply.clone(),
        )?;
        anchor(b, r, &[("1", en_pin)], NEAR_NM, NEAR_RULE);
        capacitor(
            b,
            &m.enable.capacitor.reference,
            &m.enable.capacitor.part,
            Conn::Net(en),
            gnd.clone(),
        )?;
        anchor(b, r, &[("1", en_pin)], NEAR_NM, NEAR_RULE);
        for (pin, p, net) in straps {
            let to = if p.to == aethelgard_ir::GND {
                gnd.clone()
            } else {
                b.rail(&p.to)
            };
            resistor(b, &plain(p), Conn::Net(net), to)?;
            anchor(b, r, &[("1", pin)], STRAP_NM, STRAP_RULE);
        }
    }
    Ok(())
}
