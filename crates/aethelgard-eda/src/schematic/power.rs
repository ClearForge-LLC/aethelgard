//! The power tree on the sheet (`AET-WO-4300d2` §1.2): each regulator with
//! its declared external parts, the translator of each crossing with its
//! pull-up and bypasses, each battery-sense divider with its reservoir, and
//! the power-in connector, all on the nets slice A's topology names.
//!
//! What each pin is tied to, and why (ERC's pin types as KiCad 10.0.7's
//! symbols give them):
//! - **A buck** (LMR33630, SNVSAN3F p. 19, Figure 9-1): VIN and EN on the
//!   input, PG open (an open-collector output nothing reads), FB on its own net
//!   between the divider's resistors, VCC (`power_out`) on its own net with
//!   CVCC, BOOT on its own net with CBOOT to SW, SW on its own net with the
//!   inductor, whose other end is the output domain. K5 holds by
//!   construction: RFBT's pin 1 is on the domain and RFBB's pin 2 on `GND`.
//! - **An LDO** (TLV755, SBVS320D p. 15): IN and EN on the input, OUT
//!   (`power_out`) on the output domain, which therefore carries no
//!   `PWR_FLAG` (`write_sheet`).
//! - **A translator** (TXU0304, SCES935A p. 3): VCCA on side A, VCCB on side
//!   B, OE on VCCA (always enabled), the three A-to-B channels in table order
//!   carrying SCK, MOSI and CS (so the pull-up sits on A3's net, ruled D9),
//!   the B-to-A channel carrying MISO.
//! - **The battery** is the rail [`BATTERY_NET`]; the power-in connector's
//!   positive pad (`2`, the footprint's `+` silkscreen) is on it, the other on
//!   `GND`.

use std::collections::BTreeMap;

use aethelgard_ir::power::{Power, Resistor};
use aethelgard_ir::power_parts::{self, Direction, Kind, Role, Row};

use aethelgard_ir::{PinTable, SolvedTopology};

use super::{Anchor, Builder, Conn, EmitError, Part};

pub(crate) use aethelgard_ir::power::BATTERY_NET;

/// The power-in connector's symbol and footprint, both vendored (`RUL-11`),
/// and which pad is which: the footprint's silkscreen puts `+` beside pad 2.
pub(crate) const POWER_IN_SYMBOL: &str = "Connector_Generic:Conn_01x02";
pub(crate) const POWER_IN_FOOTPRINT: &str =
    "Connector_AMASS:AMASS_XT30PW-M_1x02_P2.50mm_Horizontal";
const POWER_IN_POSITIVE: &str = "2";
const POWER_IN_NEGATIVE: &str = "1";

/// The largest copper-edge gap from a part's pad to its IC's pin, by rule.
/// Every bound is an engine choice: each datasheet asks for "as close as
/// possible" and states no distance (`AET-WO-4300d2` R4, the `D-B5`
/// precedent), so each rule's text says so.
const NEAR_NM: i64 = 2_000_000;
/// The bulk input capacitor sits behind the high-frequency one, which takes
/// the pins first (SNVSAN3F p. 23, 9.2.2.6), and the inductor beside CBOOT:
/// both bounds were set after the §1.4 measurement, not before (FEEDBACK D3).
const CIN_BULK_NM: i64 = 5_000_000;
const SW_NM: i64 = 4_000_000;
const RESERVOIR_NM: i64 = 3_000_000;
const CIN_HF_RULE: &str = "SNVSAN3F p. 33, 10.1 item 1 and p. 23, 9.2.2.6: the 220 nF input capacitor as close as possible to the VIN and GND terminals (2.0 mm, an engine choice: no distance is stated)";
const CIN_RULE: &str = "SNVSAN3F p. 33, 10.1 item 1: the bulk input capacitor close to the VIN and GND terminals, behind the high-frequency one (5.0 mm, an engine choice set after measuring the placer: no distance is stated)";
const CBOOT_RULE: &str = "SNVSAN3F p. 33, 10.1 item 3: CBOOT close to the device, with short traces to the BOOT and SW pins (2.0 mm, an engine choice: no distance is stated)";
const CVCC_RULE: &str = "SNVSAN3F p. 33, 10.1 item 2: the VCC bypass capacitor close to the VCC pin (2.0 mm, an engine choice: no distance is stated; its GND return is by the ground fill)";
const FB_RULE: &str = "SNVSAN3F p. 33, 10.1 item 4: the feedback divider as close as possible to the FB pin (2.0 mm, an engine choice: no distance is stated)";
const SW_RULE: &str = "SNVSAN3F p. 33, 10.1 item 9: the copper from the SW pin to the inductor as short as possible (4.0 mm, an engine choice set after measuring the placer: no distance is stated)";
const LDO_RULE: &str = "SBVS320D p. 21, 7.4.1: input and output capacitors as close as possible to the device (2.0 mm, an engine choice: no distance is stated)";
const BYPASS_RULE: &str = "SCES935A p. 25, 12.1: the bypass capacitors as close to the device as possible (2.0 mm, an engine choice: no distance is stated)";
const RESERVOIR_RULE: &str = "AN2834 p. 45, 4.4.3: a large external capacitor at the ADC input pin (3.0 mm, an engine choice: no distance is stated)";

/// Anchors the part just drawn to `ic`'s pads.
fn anchor(b: &mut Builder<'_>, ic: &str, pads: &[(&str, &str)], bound_nm: i64, rule: &'static str) {
    if let Some(p) = b.parts.last_mut() {
        p.anchor = Some(Anchor {
            ic: ic.to_owned(),
            pads: pads
                .iter()
                .map(|(own, theirs)| ((*own).to_owned(), (*theirs).to_owned()))
                .collect(),
            bound_nm,
            rule,
        });
    }
}

/// Each SPI line a crossing carries, by the channel that carries it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Line {
    Sck,
    Mosi,
    Cs,
    Miso,
}

/// A device's nets behind a crossing: each line's B-side net.
pub(crate) type Behind = BTreeMap<Line, String>;

fn refuse(reference: &str, why: impl Into<String>) -> EmitError {
    EmitError::PowerNotDrawable {
        reference: reference.to_owned(),
        why: why.into(),
    }
}

fn row(reference: &str, part: &str) -> Result<&'static Row, EmitError> {
    let table = power_parts::table().map_err(|e| EmitError::Internal(e.to_string()))?;
    // belt-and-suspenders: `validate` refuses a part the table lacks
    // (`PowerPartUnknown`) before any emitter runs.
    table
        .get(part)
        .ok_or_else(|| refuse(reference, format!("`{part}` is not in the power parts table")))
}

/// Picofarads the way schematics write them: `100n`, `10u`, `2u2`.
fn farads(pf: u64) -> String {
    si(pf, ['p', 'n', 'u'])
}

/// Nanohenries the same way: `8u2`.
fn henries(nh: u64) -> String {
    si(nh, ['n', 'u', 'm'])
}

/// `v` in its smallest unit, written with the largest of the three that
/// keeps a whole part, the unit letter standing for the decimal point.
fn si(v: u64, units: [char; 3]) -> String {
    let (div, unit) = match v {
        1_000_000.. => (1_000_000, units[2]),
        1_000.. => (1_000, units[1]),
        _ => return format!("{v}{}", units[0]),
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

/// A declared two-pin part of the table: a capacitor, an inductor or a
/// resistor, with the row's own symbol and footprint.
fn two_pin(
    b: &mut Builder<'_>,
    reference: &str,
    part: &str,
    value: String,
    one: Conn,
    two: Conn,
) -> Result<(), EmitError> {
    let r = row(reference, part)?;
    b.parts.push(Part {
        lib_id: r.symbol.as_str(),
        reference: reference.to_owned(),
        value,
        footprint: r.footprint.clone(),
        conns: BTreeMap::from([("1".into(), one), ("2".into(), two)]),
        note: None,
        decouples: None,
        anchor: None,
        offboard: BTreeMap::new(),
    });
    Ok(())
}

/// A declared capacitor, valued by its row's rated capacitance and named by
/// its part number.
fn capacitor(
    b: &mut Builder<'_>,
    reference: &str,
    part: &str,
    one: Conn,
    two: Conn,
) -> Result<(), EmitError> {
    let Kind::Capacitor(c) = &row(reference, part)?.kind else {
        return Err(refuse(reference, format!("`{part}` is not a capacitor")));
    };
    let pf = u64::try_from(c.rated_picofarads.value)
        .map_err(|_| refuse(reference, "a negative capacitance"))?;
    let value = format!("{} {part}", farads(pf));
    two_pin(b, reference, part, value, one, two)
}

fn resistor(b: &mut Builder<'_>, r: &Resistor, one: Conn, two: Conn) -> Result<(), EmitError> {
    let value = format!("{} {}", super::ohms(r.ohms), r.part);
    two_pin(b, &r.reference, &r.part, value, one, two)
}

/// An IC of the table, with each pin role's connection.
fn ic(
    b: &mut Builder<'_>,
    reference: &str,
    r: &'static Row,
    conns: BTreeMap<String, Conn>,
) -> Result<(), EmitError> {
    b.parts.push(Part {
        lib_id: r.symbol.as_str(),
        reference: reference.to_owned(),
        value: r.part.clone(),
        footprint: r.footprint.clone(),
        conns,
        note: None,
        decouples: None,
        anchor: None,
        offboard: BTreeMap::new(),
    });
    Ok(())
}

/// The pin number of `role` in `r`'s row (`power-parts.json` `pins`).
fn pin(reference: &str, r: &Row, role: &str) -> Result<String, EmitError> {
    r.pins
        .get(role)
        .cloned()
        .ok_or_else(|| refuse(reference, format!("`{}` names no pin `{role}`", r.part)))
}

/// A domain or the battery, as the rail it is drawn on.
fn supply(b: &mut Builder<'_>, name: &str) -> Conn {
    if name == aethelgard_ir::power::BATTERY {
        b.rail(BATTERY_NET)
    } else {
        b.rail(name)
    }
}

/// Every reference the power tree declares, for the emitter's reference
/// claims (`design`).
pub(crate) fn references(p: &Power) -> Vec<&String> {
    let mut out = vec![&p.input.reference];
    for r in &p.regulators {
        out.push(&r.reference);
        if let Some(d) = &r.feedback {
            out.extend([&d.top.reference, &d.bottom.reference]);
        }
        out.extend(r.external.iter().map(|e| &e.reference));
    }
    for s in &p.sense {
        out.extend([&s.top.reference, &s.bottom.reference]);
        out.extend(s.reservoir.iter().map(|c| &c.reference));
    }
    for c in &p.crossings {
        out.extend([&c.reference, &c.cs_pull_up.reference]);
        out.extend(c.external.iter().map(|e| &e.reference));
    }
    out
}

/// The nets the specification spells inside its power tree.
pub(crate) fn spelled(p: &Power) -> Vec<String> {
    p.sense.iter().map(|s| s.net.clone()).collect()
}

/// Each crossing's device: the reference of the one SPI device on its bus,
/// with its B-side nets, which the device is drawn on instead of the bus's.
pub(crate) fn behind(b: &mut Builder<'_>, p: &Power) -> Result<BTreeMap<String, Behind>, EmitError> {
    let spec = b.spec;
    let mut out = BTreeMap::new();
    for c in &p.crossings {
        let devices: Vec<_> = spec
            .declared_spi_devices()
            .iter()
            .filter(|d| d.bus == c.bus)
            .collect();
        let [device] = devices.as_slice() else {
            return Err(refuse(
                &c.reference,
                format!(
                    "bus `{}` has {} devices, and one translator carries one chip select",
                    c.bus,
                    devices.len()
                ),
            ));
        };
        let mut nets = Behind::new();
        for (line, name) in lines(&c.reference, &c.part)? {
            let net = b.own_net(format!("{}_{name}", c.reference))?;
            nets.insert(line, net);
        }
        out.insert(device.reference.clone(), nets);
    }
    Ok(out)
}

/// Each line a translator carries, with the name of its B-side pin.
fn lines(reference: &str, part: &str) -> Result<Vec<(Line, String)>, EmitError> {
    let Kind::Translator(t) = &row(reference, part)?.kind else {
        return Err(refuse(reference, format!("`{part}` is not a translator")));
    };
    let a: Vec<_> = t
        .channels
        .iter()
        .filter(|c| c.direction == Direction::AToB)
        .collect();
    let back: Vec<_> = t
        .channels
        .iter()
        .filter(|c| c.direction == Direction::BToA)
        .collect();
    let ([sck, mosi, cs, ..], [miso, ..]) = (a.as_slice(), back.as_slice()) else {
        return Err(refuse(
            reference,
            format!("`{part}` has no three A-to-B channels and one B-to-A channel"),
        ));
    };
    Ok(vec![
        (Line::Sck, sck.output.clone()),
        (Line::Mosi, mosi.output.clone()),
        (Line::Cs, cs.output.clone()),
        (Line::Miso, miso.input.clone()),
    ])
}

/// Draws the whole tree. `behind` is [`behind`]'s map; the devices
/// themselves are drawn with the other SPI devices. `mcu` is the MCU's
/// reference, whose solved sense pins (`SolvedTopology.power.sense`, by the
/// table's package position) are connected here.
pub(crate) fn draw(
    b: &mut Builder<'_>,
    p: &Power,
    behind: &BTreeMap<String, Behind>,
    mcu: &str,
    solved: &SolvedTopology,
    table: &PinTable,
) -> Result<(), EmitError> {
    for r in &p.regulators {
        regulator(b, r)?;
    }
    for c in &p.crossings {
        crossing(b, c, behind)?;
    }
    for s in &p.sense {
        let pin = solved
            .power
            .as_ref()
            .and_then(|sp| sp.sense.get(&s.net))
            .ok_or_else(|| EmitError::UnsolvedNet(s.net.clone()))?;
        let position = table
            .pins
            .get(&pin.pin)
            .ok_or_else(|| EmitError::UnknownPin {
                net: s.net.clone(),
                pin: pin.pin.clone(),
            })?
            .position
            .to_string();
        let m = b
            .parts
            .iter_mut()
            .find(|x| x.reference == mcu)
            .ok_or_else(|| EmitError::Internal("the MCU is not drawn".into()))?;
        if m.conns.contains_key(&position) {
            return Err(EmitError::Internal(format!(
                "{}: the MCU's pin {position} is already connected",
                s.net
            )));
        }
        m.conns.insert(position.clone(), Conn::Net(s.net.clone()));
        let top = b.rail(BATTERY_NET);
        let net = Conn::Net(s.net.clone());
        resistor(b, &s.top, top, net.clone())?;
        let g = b.ground();
        resistor(b, &s.bottom, net.clone(), g)?;
        if let Some(c) = &s.reservoir {
            let g = b.ground();
            capacitor(b, &c.reference, &c.part, net, g)?;
            anchor(b, mcu, &[("1", &position)], RESERVOIR_NM, RESERVOIR_RULE);
        }
    }
    // The power-in connector, drawn like `4300f`'s connectors (it is placed
    // with them, at the board's edge).
    let plus = b.rail(BATTERY_NET);
    let minus = b.ground();
    b.parts.push(Part {
        lib_id: POWER_IN_SYMBOL,
        reference: p.input.reference.clone(),
        value: p.input.part.clone(),
        footprint: POWER_IN_FOOTPRINT.to_owned(),
        conns: BTreeMap::from([
            (POWER_IN_POSITIVE.into(), plus),
            (POWER_IN_NEGATIVE.into(), minus),
        ]),
        note: None,
        decouples: None,
        anchor: None,
        offboard: BTreeMap::new(),
    });
    Ok(())
}

fn regulator(b: &mut Builder<'_>, reg: &aethelgard_ir::power::Regulator) -> Result<(), EmitError> {
    let r = row(&reg.reference, &reg.part)?;
    let u = &reg.reference;
    let input = supply(b, &reg.input);
    let output = b.rail(&reg.output);
    let g = b.ground();
    let of = |role: Role| reg.external.iter().filter(move |e| e.role == role);
    let mut conns = BTreeMap::new();
    match &r.kind {
        Kind::Buck(_) => {
            let fb = b.own_net(format!("{u}_FB"))?;
            let vcc = b.own_net(format!("{u}_VCC"))?;
            let boot = b.own_net(format!("{u}_BOOT"))?;
            let sw = b.own_net(format!("{u}_SW"))?;
            for (role, conn) in [
                ("PGND", g.clone()),
                ("EP", g.clone()),
                ("VIN", input.clone()),
                ("EN", input.clone()),
                ("PG", Conn::NoConnect),
                ("FB", Conn::Net(fb.clone())),
                ("VCC", Conn::Net(vcc.clone())),
                ("BOOT", Conn::Net(boot.clone())),
                ("SW", Conn::Net(sw.clone())),
            ] {
                conns.insert(pin(u, r, role)?, conn);
            }
            ic(b, u, r, conns)?;
            let [vin, pgnd, fbp, vccp, bootp, swp] =
                ["VIN", "PGND", "FB", "VCC", "BOOT", "SW"].map(|role| pin(u, r, role));
            let (vin, pgnd, fbp, vccp, bootp, swp) = (vin?, pgnd?, fbp?, vccp?, bootp?, swp?);
            // Drawn, and so placed (`pcb::place`), in this order around the
            // pins: the high-frequency input capacitor, CBOOT, the inductor
            // (beside SW before CVCC and the divider fill that side), CVCC,
            // the divider, then the bulk input capacitor behind the
            // high-frequency one.
            for e in of(Role::CinHf) {
                capacitor(b, &e.reference, &e.part, input.clone(), g.clone())?;
                anchor(b, u, &[("1", &vin), ("2", &pgnd)], NEAR_NM, CIN_HF_RULE);
            }
            for e in of(Role::Cboot) {
                capacitor(
                    b,
                    &e.reference,
                    &e.part,
                    Conn::Net(boot.clone()),
                    Conn::Net(sw.clone()),
                )?;
                anchor(b, u, &[("1", &bootp), ("2", &swp)], NEAR_NM, CBOOT_RULE);
            }
            for e in of(Role::Inductor) {
                let Kind::Inductor(l) = &row(&e.reference, &e.part)?.kind else {
                    return Err(refuse(&e.reference, format!("`{}` is not an inductor", e.part)));
                };
                // The nominal: the row's ± tolerance is symmetric.
                let n = &l.nanohenries.range;
                let nominal = u64::try_from((n.min + n.max) / 2)
                    .map_err(|_| refuse(&e.reference, "a negative inductance"))?;
                let value = format!("{} {}", henries(nominal), e.part);
                two_pin(
                    b,
                    &e.reference,
                    &e.part,
                    value,
                    Conn::Net(sw.clone()),
                    output.clone(),
                )?;
                anchor(b, u, &[("1", &swp)], SW_NM, SW_RULE);
            }
            for e in of(Role::Cvcc) {
                capacitor(b, &e.reference, &e.part, Conn::Net(vcc.clone()), g.clone())?;
                anchor(b, u, &[("1", &vccp)], NEAR_NM, CVCC_RULE);
            }
            // belt-and-suspenders: `validate` refuses a buck with no divider
            // (`ExternalPartMissing`).
            let d = reg
                .feedback
                .as_ref()
                .ok_or_else(|| refuse(u, "a buck with no feedback divider"))?;
            resistor(b, &d.top, output.clone(), Conn::Net(fb.clone()))?;
            anchor(b, u, &[("2", &fbp)], NEAR_NM, FB_RULE);
            resistor(b, &d.bottom, Conn::Net(fb), g.clone())?;
            anchor(b, u, &[("1", &fbp)], NEAR_NM, FB_RULE);
            for e in of(Role::Cin) {
                capacitor(b, &e.reference, &e.part, input.clone(), g.clone())?;
                anchor(b, u, &[("1", &vin), ("2", &pgnd)], CIN_BULK_NM, CIN_RULE);
            }
            for e in of(Role::Cout) {
                capacitor(b, &e.reference, &e.part, output.clone(), g.clone())?;
            }
        }
        Kind::Ldo(_) => {
            for (role, conn) in [
                ("IN", input.clone()),
                ("EN", input.clone()),
                ("GND", g.clone()),
                ("NC", Conn::NoConnect),
                ("OUT", output.clone()),
            ] {
                conns.insert(pin(u, r, role)?, conn);
            }
            ic(b, u, r, conns)?;
            let (inp, outp) = (pin(u, r, "IN")?, pin(u, r, "OUT")?);
            for e in of(Role::Cin) {
                capacitor(b, &e.reference, &e.part, input.clone(), g.clone())?;
                anchor(b, u, &[("1", &inp)], NEAR_NM, LDO_RULE);
            }
            for e in of(Role::Cout) {
                capacitor(b, &e.reference, &e.part, output.clone(), g.clone())?;
                anchor(b, u, &[("1", &outp)], NEAR_NM, LDO_RULE);
            }
        }
        _ => return Err(refuse(u, format!("`{}` is not a regulator", reg.part))),
    }
    Ok(())
}

fn crossing(
    b: &mut Builder<'_>,
    c: &aethelgard_ir::power::Crossing,
    behind: &BTreeMap<String, Behind>,
) -> Result<(), EmitError> {
    let spec = b.spec;
    let r = row(&c.reference, &c.part)?;
    let bus = spec
        .spi_buses
        .iter()
        .find(|x| x.id == c.bus)
        .ok_or_else(|| refuse(&c.reference, format!("no SPI bus `{}`", c.bus)))?;
    let device = spec
        .declared_spi_devices()
        .iter()
        .find(|d| d.bus == c.bus)
        .ok_or_else(|| refuse(&c.reference, format!("bus `{}` has no device", c.bus)))?;
    let b_nets = behind
        .get(&device.reference)
        .ok_or_else(|| EmitError::Internal(format!("{}: no B-side nets", c.reference)))?;
    let a = b.rail(&c.a_domain);
    let bside = b.rail(&c.b_domain);
    let g = b.ground();
    let mut conns = BTreeMap::new();
    for (role, conn) in [
        ("VCCA", a.clone()),
        ("VCCB", bside.clone()),
        ("OE", a.clone()),
        ("GND", g.clone()),
    ] {
        conns.insert(pin(&c.reference, r, role)?, conn);
    }
    let a_net = |line: Line| match line {
        Line::Sck => bus.sck.net.clone(),
        Line::Mosi => bus.mosi.net.clone(),
        Line::Miso => bus.miso.net.clone(),
        Line::Cs => device.cs.net.clone(),
    };
    let Kind::Translator(t) = &r.kind else {
        return Err(refuse(&c.reference, format!("`{}` is not a translator", c.part)));
    };
    let sym = crate::library::symbol(r.symbol.as_str())?;
    let pins = super::pins(&sym).map_err(EmitError::Internal)?;
    let number = |name: &str| {
        pins.iter()
            .find(|p| p.name == name)
            .map(|p| p.number.clone())
            .ok_or_else(|| refuse(&c.reference, format!("`{}` has no pin `{name}`", c.part)))
    };
    for (line, b_name) in lines(&c.reference, &c.part)? {
        let ch = t
            .channels
            .iter()
            .find(|ch| ch.output == b_name || ch.input == b_name)
            .ok_or_else(|| EmitError::Internal(format!("{}: channel {b_name}", c.reference)))?;
        let (a_pin, b_pin) = match ch.direction {
            Direction::AToB => (&ch.input, &ch.output),
            Direction::BToA => (&ch.output, &ch.input),
        };
        conns.insert(number(a_pin)?, Conn::Net(a_net(line)));
        conns.insert(number(b_pin)?, Conn::Net(b_nets[&line].clone()));
    }
    ic(b, &c.reference, r, conns)?;
    resistor(b, &c.cs_pull_up, a.clone(), Conn::Net(device.cs.net.clone()))?;
    let (vcca, vccb) = (pin(&c.reference, r, "VCCA")?, pin(&c.reference, r, "VCCB")?);
    for e in &c.external {
        let (rail, at) = match e.role {
            Role::BypassA => (a.clone(), &vcca),
            Role::BypassB => (bside.clone(), &vccb),
            _ => return Err(refuse(&e.reference, "a translator part that is not a bypass")),
        };
        capacitor(b, &e.reference, &e.part, rail, g.clone())?;
        anchor(b, &c.reference, &[("1", at)], NEAR_NM, BYPASS_RULE);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{farads, henries};

    #[test]
    fn farads_are_written_as_schematics_write_them() {
        for (pf, s) in [
            (100_000, "100n"),
            (220_000, "220n"),
            (1_000_000, "1u"),
            (2_200_000, "2u2"),
            (10_000_000, "10u"),
            (22_000_000, "22u"),
            (8_200_000, "8u2"),
            (470, "470p"),
        ] {
            assert_eq!(farads(pf), s);
        }
        assert_eq!(henries(8_200), "8u2");
    }
}
