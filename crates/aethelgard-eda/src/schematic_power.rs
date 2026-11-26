//! The power tree on the sheet (`AET-WO-4300d2` §1.2): each regulator with
//! its declared external parts, the translator of each crossing with its
//! pull-up and bypasses, each battery-sense divider with its reservoir, and
//! the power-in connector, all on the nets slice A's topology names.
//!
//! What each pin is tied to, and why (ERC's pin types as KiCad 10.0.7's
//! symbols give them):
//! - **A buck** (LMR33630, SNVSAN3F p. 19, Figure 9-1): VIN on the input,
//!   EN and PG as their `controls` declare them, FB on its own net
//!   between the divider's resistors, VCC (`power_out`) on its own net with
//!   CVCC, BOOT on its own net with CBOOT to SW, SW on its own net with the
//!   inductor, whose other end is the output domain. K5 holds by
//!   construction: RFBT's pin 1 is on the domain and RFBB's pin 2 on `GND`.
//! - **An LDO** (TLV755, SBVS320D p. 15): IN on the input, EN as declared, OUT
//!   (`power_out`) on the output domain, which therefore carries no
//!   `PWR_FLAG` (`write_sheet`).
//! - **A translator** (TXU0304, SCES935A p. 3): VCCA on side A, VCCB on side
//!   B, OE as declared, the three A-to-B channels in table order
//!   carrying SCK, MOSI and CS (so the pull-up sits on A3's net, ruled D9),
//!   the B-to-A channel carrying MISO.
//! - **The battery** is the rail [`BATTERY_NET`]; the power-in connector's
//!   positive pad (`2`, the footprint's `+` silkscreen) is on it, the other on
//!   `GND`.
//! - **Each hot-plug bulk capacitor** (`power.input.hot_plug`, `AET-WO-1315`
//!   §1.3) across the battery: its positive pin on [`BATTERY_NET`], the other
//!   on `GND`. Its row names KiCad's `Device:C_Polarized` and
//!   `Capacitor_SMD:CP_Elec_8x10`, vendored at 10.0.7; in both, `1` is the
//!   positive terminal (the symbol draws its `+` beside pin 1, at
//!   (-1.27, 2.286) with pin 1 at (0, 3.81); the footprint's silkscreen `+`
//!   sits at x = -5 mm, beside pad 1 at x = -3.25 mm, pad 2 at +3.25 mm). It
//!   is the last part of the first battery-fed regulator's sequence, behind
//!   the ceramics (SNVSAN3F p. 23, 9.2.2.6: the bulk capacitor in parallel
//!   with the ceramics), anchored to its input and ground pins; with no
//!   battery-fed regulator it is drawn by its nets alone. A row of any other
//!   kind (a TVS), or a part on the pack's side, is refused by name, never
//!   skipped and never drawn as something else (`INV-06`).
//!
//! **Control pins** (`AET-WO-1315` §1.4) are drawn as each `controls` entry
//! declares, never by a rule of the emitter's own: a strap `input` on the
//! regulator's input rail, `vcca` and `vccb` on the translator's A and B
//! rails, `open` as a no-connect; a wire as a label on its declared net, only
//! when a connector pin or test point the specification declares ends that
//! net. A control pin of the row with no entry, an entry for a pin the row
//! does not list, a strap the row does not allow or the part has no rail for,
//! a wire to a rail or to a net nothing else ends, are each refused by name
//! (`PowerNotDrawable`, `INV-06`).

use std::collections::{BTreeMap, BTreeSet};

use aethelgard_ir::power::{Control, HotPlug, Power, Resistor, Side};
use aethelgard_ir::power_parts::{
    self, BulkCapacitor, ControlRule, Direction, Kind, PowerPartsTable, Role, Row, Strap,
};

use aethelgard_ir::{SolvedTopology, SpiDevice};

use super::{Builder, Conn, EmitError, Part};

use aethelgard_ir::power::BATTERY_NET;

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
/// A buck's loop rules, cited to its own datasheet: the LMR33630's above;
/// the LMR33620's (`AET-WO-4411`), whose SNVSAW1E 10.1 lists the same items.
struct BuckRules {
    cin_hf: &'static str,
    cin: &'static str,
    cboot: &'static str,
    cvcc: &'static str,
    fb: &'static str,
    sw: &'static str,
}
const SNVSAN3F_RULES: BuckRules = BuckRules {
    cin_hf: CIN_HF_RULE,
    cin: CIN_RULE,
    cboot: CBOOT_RULE,
    cvcc: CVCC_RULE,
    fb: FB_RULE,
    sw: SW_RULE,
};
const SNVSAW1E_RULES: BuckRules = BuckRules {
    cin_hf: "SNVSAW1E p. 32, 10.1 item 1 and p. 22, 9.2.2.6: the 220 nF input capacitor as close as possible to the VIN and GND terminals (2.0 mm, an engine choice: no distance is stated)",
    cin: "SNVSAW1E p. 32, 10.1 item 1: the bulk input capacitor close to the VIN and GND terminals, behind the high-frequency one (5.0 mm, an engine choice set after measuring the placer: no distance is stated)",
    cboot: "SNVSAW1E p. 32, 10.1 item 3: CBOOT close to the device, with short traces to the BOOT and SW pins (2.0 mm, an engine choice: no distance is stated)",
    cvcc: "SNVSAW1E p. 32, 10.1 item 2: the VCC bypass capacitor close to the VCC pin (2.0 mm, an engine choice: no distance is stated; its GND return is by the ground fill)",
    fb: "SNVSAW1E p. 32, 10.1 item 4: the feedback divider as close as possible to the FB pin (2.0 mm, an engine choice: no distance is stated)",
    sw: "SNVSAW1E p. 32, 10.1 item 9: the copper from the SW pin to the inductor as short as possible (4.0 mm, an engine choice set after measuring the placer: no distance is stated)",
};
const LDO_RULE: &str = "SBVS320D p. 21, 7.4.1: input and output capacitors as close as possible to the device (2.0 mm, an engine choice: no distance is stated)";
const BYPASS_RULE: &str = "SCES935A p. 25, 12.1: the bypass capacitors as close to the device as possible (2.0 mm, an engine choice: no distance is stated)";
/// A hot-plug bulk capacitor's positive pin and pad (see the module
/// documentation for where each vendored file marks it).
const BULK_POSITIVE: &str = "1";
const BULK_NEGATIVE: &str = "2";
const HOT_PLUG_NM: i64 = 10_000_000;
const HOT_PLUG_RULE: &str = "SNVSAN3F p. 23, 9.2.2.6: the bulk capacitor that damps the input's ring in parallel with the ceramic input capacitors, behind them (10.0 mm, an engine choice set after measuring the placer: no distance is stated)";
const RESERVOIR_RULE: &str = "AN2834 p. 45, 4.4.3: a large external capacitor at the ADC input pin (3.0 mm, an engine choice: no distance is stated)";

/// Anchors the part just drawn to `ic`'s pads.
pub(super) fn anchor(
    b: &mut Builder<'_>,
    ic: &str,
    pads: &[(&str, &str)],
    bound_nm: i64,
    rule: &'static str,
) {
    let pads = pads
        .iter()
        .map(|(own, theirs)| ((*own).to_owned(), (*theirs).to_owned()))
        .collect();
    b.anchor_last(ic, pads, Some((bound_nm, rule)), false);
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
    table.get(part).ok_or_else(|| {
        refuse(
            reference,
            format!("`{part}` is not in the power parts table"),
        )
    })
}

/// Picofarads the way schematics write them: `100n`, `10u`, `2u2`.
pub(super) fn farads(pf: u64) -> String {
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
/// resistor, with the row's own symbol and footprint, and `note` in the BOM
/// (a pin device's declared bulk's, `AET-WO-1211f`).
pub(super) fn two_pin(
    b: &mut Builder<'_>,
    reference: &str,
    part: &str,
    value: String,
    one: Conn,
    two: Conn,
    note: Option<String>,
) -> Result<(), EmitError> {
    let r = row(reference, part)?;
    b.parts.push(Part {
        lib_id: r.symbol.as_str(),
        reference: reference.to_owned(),
        value,
        footprint: r.footprint.clone(),
        conns: BTreeMap::from([("1".into(), one), ("2".into(), two)]),
        note,
        owner: None,
        offboard: BTreeMap::new(),
        seat: None,
        dnp: false,
        beside: None,
    });
    Ok(())
}

/// A declared capacitor, valued by its row's rated capacitance and named by
/// its part number.
pub(super) fn capacitor(
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
    two_pin(b, reference, part, value, one, two, None)
}

pub(super) fn resistor(
    b: &mut Builder<'_>,
    r: &Resistor,
    one: Conn,
    two: Conn,
) -> Result<(), EmitError> {
    let value = format!("{} {}", super::ohms(r.ohms), r.part);
    two_pin(b, &r.reference, &r.part, value, one, two, None)
}

/// A declared reservoir capacitor on an ADC input, by the MCU's pin at
/// package `position`: a battery sense's, and a pin device's analog output's
/// (`AET-WO-1211d`).
pub(super) fn reservoir(
    b: &mut Builder<'_>,
    mcu: &str,
    c: &aethelgard_ir::power::Reservoir,
    net: Conn,
    g: Conn,
    position: &str,
) -> Result<(), EmitError> {
    capacitor(b, &c.reference, &c.part, net, g)?;
    anchor(b, mcu, &[("1", position)], RESERVOIR_NM, RESERVOIR_RULE);
    Ok(())
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
        owner: None,
        offboard: BTreeMap::new(),
        seat: None,
        dnp: false,
        beside: None,
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
    b.rail(if name == aethelgard_ir::power::BATTERY {
        BATTERY_NET
    } else {
        name
    })
}

/// Each crossing's device: the reference of the one SPI device on its bus,
/// with its B-side nets, which the device is drawn on instead of the bus's.
///
/// `spi` is the specification's SPI devices, which `schematic` reads for this
/// module (`RUL-16`: `eda`'s one reader of declared devices).
pub(crate) fn behind(
    b: &mut Builder<'_>,
    p: &Power,
    spi: &[SpiDevice],
) -> Result<BTreeMap<String, Behind>, EmitError> {
    let mut out = BTreeMap::new();
    for c in &p.crossings {
        let device = device_on(&c.reference, &c.bus, spi)?;
        let mut nets = Behind::new();
        for (line, name) in lines(&c.reference, &c.part)? {
            let net = b.own_net(format!("{}_{name}", c.reference))?;
            nets.insert(line, net);
        }
        out.insert(device.reference.clone(), nets);
    }
    Ok(out)
}

/// The one SPI device on a crossing's bus. belt-and-suspenders: `validate`
/// refuses a crossing whose bus has not exactly one device
/// (`CrossingInvalid`), but `Drawing::of` and `decoupling_picofarads` draw
/// without it, so the emitter refuses one too (`AET-WO-1322`, `1208` F3).
/// [`behind`] and [`crossing`] both take the device from here.
fn device_on<'s>(
    reference: &str,
    bus: &str,
    spi: &'s [SpiDevice],
) -> Result<&'s SpiDevice, EmitError> {
    let devices: Vec<_> = spi.iter().filter(|d| d.bus == bus).collect();
    match devices.as_slice() {
        [device] => Ok(device),
        _ => Err(refuse(
            reference,
            format!(
                "bus `{bus}` has {} devices, and one translator carries one chip select",
                devices.len()
            ),
        )),
    }
}

/// Each line a translator carries, with the name of its B-side pin.
/// belt-and-suspenders: `validate` refuses a crossing whose part is not a
/// translator with three A-to-B channels and one B-to-A (`CrossingInvalid`);
/// a library caller that draws without it is refused here (`AET-WO-1322`).
fn lines(reference: &str, part: &str) -> Result<Vec<(Line, String)>, EmitError> {
    let Kind::Translator(t) = &row(reference, part)?.kind else {
        return Err(refuse(reference, format!("`{part}` is not a translator")));
    };
    let Some([sck, mosi, cs, miso]) = t.crossing_lines() else {
        return Err(refuse(
            reference,
            format!("`{part}` has no three A-to-B channels and one B-to-A channel"),
        ));
    };
    Ok(vec![
        (Line::Sck, sck.to_owned()),
        (Line::Mosi, mosi.to_owned()),
        (Line::Cs, cs.to_owned()),
        (Line::Miso, miso.to_owned()),
    ])
}

/// Draws the whole tree. `behind` is [`behind`]'s map; the devices
/// themselves are drawn with the other SPI devices. `mcu` is the MCU's
/// reference, whose solved sense pins (`SolvedTopology.power.sense`, by
/// package position) are connected here. `ended` is every net a declared
/// connector pin or test point ends, which a control wire may join.
pub(crate) fn draw(
    b: &mut Builder<'_>,
    p: &Power,
    spi: &[SpiDevice],
    behind: &BTreeMap<String, Behind>,
    mcu: &str,
    solved: &SolvedTopology,
    ended: &BTreeSet<String>,
) -> Result<(), EmitError> {
    let table = power_parts::table().map_err(|e| EmitError::Internal(e.to_string()))?;
    let fed = p
        .regulators
        .iter()
        .position(|r| r.input == aethelgard_ir::power::BATTERY);
    for (i, r) in p.regulators.iter().enumerate() {
        regulator(b, r, ended)?;
        if fed == Some(i) {
            hot_plug(b, p, table, Some(r))?;
        }
    }
    if fed.is_none() {
        hot_plug(b, p, table, None)?;
    }
    for c in &p.crossings {
        crossing(b, c, spi, behind, ended)?;
    }
    for s in &p.sense {
        let pin = solved
            .power
            .as_ref()
            .and_then(|sp| sp.sense.get(&s.net))
            .ok_or_else(|| EmitError::UnsolvedNet(s.net.clone()))?;
        let position = pin.position.to_string();
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
            reservoir(b, mcu, c, net, g, &position)?;
        }
    }
    // `AET-WO-4300k` (10): the input's protection, from the connector to
    // `VBAT`: the fuse, then the P-FET (drain to the fuse, source to `VBAT`),
    // its gate pulled to `GND` and held by the zener, cathode on the source.
    // The sense divider and every regulator sit on `VBAT`, after it.
    let plus = match &p.input.protection {
        Some(x) => {
            let input = Conn::Net(aethelgard_ir::power::BATTERY_IN_NET.to_owned());
            let fused = Conn::Net(aethelgard_ir::power::BATTERY_FUSED_NET.to_owned());
            let gate = Conn::Net(format!("{}_GATE", x.fet.reference));
            two_pin(
                b,
                &x.fuse.reference,
                &x.fuse.part,
                x.fuse.part.clone(),
                input.clone(),
                fused.clone(),
                None,
            )?;
            let fet = row(&x.fet.reference, &x.fet.part)?;
            let mut conns = BTreeMap::new();
            for (role, conn) in [
                ("D", fused),
                ("G", gate.clone()),
                ("S", b.rail(BATTERY_NET)),
            ] {
                for n in pin(&x.fet.reference, fet, role)?.split(',') {
                    conns.insert(n.to_owned(), conn.clone());
                }
            }
            ic(b, &x.fet.reference, fet, conns)?;
            let source = b.rail(BATTERY_NET);
            two_pin(
                b,
                &x.zener.reference,
                &x.zener.part,
                x.zener.part.clone(),
                source,
                gate.clone(),
                None,
            )?;
            let g = b.ground();
            resistor(b, &x.gate_resistor, gate, g)?;
            input
        }
        None => b.rail(BATTERY_NET),
    };
    // The power-in connector, drawn like `4300f`'s connectors (it is placed
    // with them, at the board's edge).
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
        owner: None,
        offboard: BTreeMap::new(),
        seat: None,
        dnp: false,
        beside: None,
    });
    Ok(())
}

/// `AET-WO-4300k` (9), Path B: a reset supervisor on `NRST` (its row's
/// `RESET`), on the MCU's supply (`VDD`) and `GND`; its pull-up from `NRST`
/// to the supply; its bypass from `VDD` to `GND` (SBVS157E p. 6, note 2).
pub(super) fn supervisor(
    b: &mut Builder<'_>,
    s: &aethelgard_ir::supervisor::ResetSupervisor,
    supply: &str,
) -> Result<(), EmitError> {
    let r = row(&s.reference, &s.part)?;
    let nrst = Conn::Net(aethelgard_ir::NRST.to_owned());
    let vdd = b.rail(supply);
    let g = b.ground();
    let conns = BTreeMap::from([
        (pin(&s.reference, r, "GND")?, g),
        (pin(&s.reference, r, "RESET")?, nrst.clone()),
        (pin(&s.reference, r, "VDD")?, vdd),
    ]);
    ic(b, &s.reference, r, conns)?;
    let up = b.rail(supply);
    resistor(b, &s.pull_up, nrst, up)?;
    let vdd = b.rail(supply);
    let g = b.ground();
    capacitor(b, &s.bypass.reference, &s.bypass.part, vdd, g)
}

/// A declared hot-plug part's row in `table`, when it is a bulk capacitor on
/// the board's side of the connector: the only kind the emitter draws.
fn bulk<'t>(
    table: &'t PowerPartsTable,
    h: &HotPlug,
) -> Result<(&'t Row, &'t BulkCapacitor), EmitError> {
    let r = table.get(&h.part).ok_or_else(|| {
        refuse(
            &h.reference,
            format!("`{}` is not in the power parts table", h.part),
        )
    })?;
    let Kind::BulkCapacitor(c) = &r.kind else {
        return Err(refuse(
            &h.reference,
            format!(
                "`{}` is a {} row, and the emitter draws only a bulk capacitor on the battery input",
                h.part,
                r.kind.name()
            ),
        ));
    };
    if h.side == Side::Pack {
        return Err(refuse(
            &h.reference,
            format!(
                "`{}` is declared on the pack's side of the connector, not on the board",
                h.part
            ),
        ));
    }
    Ok((r, c))
}

/// Each declared hot-plug part across the battery, anchored to `fed`'s input
/// and ground pins when a regulator is fed from the battery. Every part is
/// judged drawable before any is drawn.
fn hot_plug(
    b: &mut Builder<'_>,
    p: &Power,
    table: &'static PowerPartsTable,
    fed: Option<&aethelgard_ir::power::Regulator>,
) -> Result<(), EmitError> {
    let rows = p
        .input
        .hot_plug
        .iter()
        .map(|h| bulk(table, h))
        .collect::<Result<Vec<_>, _>>()?;
    let pads = match fed {
        Some(reg) => {
            let r = row(&reg.reference, &reg.part)?;
            let (vin, gnd) = match &r.kind {
                Kind::Buck(_) => ("VIN", "PGND"),
                _ => ("IN", "GND"),
            };
            Some((
                reg.reference.as_str(),
                pin(&reg.reference, r, vin)?,
                pin(&reg.reference, r, gnd)?,
            ))
        }
        None => None,
    };
    for (h, (r, c)) in p.input.hot_plug.iter().zip(rows) {
        let pf = u64::try_from(c.rated_picofarads.value)
            .map_err(|_| refuse(&h.reference, "a negative capacitance"))?;
        let plus = b.rail(BATTERY_NET);
        let minus = b.ground();
        b.parts.push(Part {
            lib_id: r.symbol.as_str(),
            reference: h.reference.clone(),
            value: format!("{} {}", farads(pf), h.part),
            footprint: r.footprint.clone(),
            conns: BTreeMap::from([(BULK_POSITIVE.into(), plus), (BULK_NEGATIVE.into(), minus)]),
            note: None,
            owner: None,
            offboard: BTreeMap::new(),
            seat: None,
            dnp: false,
            beside: None,
        });
        if let Some((u, vin, gnd)) = &pads {
            anchor(
                b,
                u,
                &[(BULK_POSITIVE, vin), (BULK_NEGATIVE, gnd)],
                HOT_PLUG_NM,
                HOT_PLUG_RULE,
            );
        }
    }
    Ok(())
}

/// Each control pin of `r`'s row, by its role, on what `declared` says
/// (`AET-WO-1315` §1.4). `strap` is the rail each strap names on this part,
/// `None` where the part has none.
fn controls(
    b: &mut Builder<'_>,
    reference: &str,
    r: &Row,
    declared: &[Control],
    ended: &BTreeSet<String>,
    strap: impl Fn(Strap) -> Option<Conn>,
) -> Result<Vec<(String, Conn)>, EmitError> {
    let rules: &[ControlRule] = match &r.kind {
        Kind::Buck(k) => &k.controls,
        Kind::Ldo(k) => &k.controls,
        Kind::Translator(k) => &k.controls,
        _ => {
            return Err(refuse(
                reference,
                format!("`{}` has no control pins", r.part),
            ));
        }
    };
    for (i, c) in declared.iter().enumerate() {
        if declared[..i].iter().any(|d| d.pin == c.pin) || !rules.iter().any(|x| x.pin == c.pin) {
            return Err(refuse(
                reference,
                format!(
                    "`controls` names `{}` twice, or a pin `{}`'s row lists as no control pin",
                    c.pin, r.part
                ),
            ));
        }
    }
    let spec = b.spec;
    let rail = |net: &str| {
        [aethelgard_ir::GND, BATTERY_NET]
            .into_iter()
            .chain(spec.voltage_domains.iter().map(|d| d.id.as_str()))
            .any(|x| x.eq_ignore_ascii_case(net))
    };
    let regulators = spec.power.as_ref().map_or(&[][..], |p| &p.regulators[..]);
    // `AET-WO-4411` (iii): this part's PG, when another's EN follows it
    let follower = regulators.iter().any(|g| {
        g.controls
            .iter()
            .any(|c| c.after.as_ref().is_some_and(|a| a.regulator == reference))
    });
    let mut out = Vec::new();
    for rule in rules {
        let pin = &rule.pin;
        if pin == "PG" && follower && !declared.iter().any(|c| &c.pin == pin) {
            out.push((pin.clone(), Conn::Net(format!("{reference}_PG"))));
            continue;
        }
        let Some(c) = declared.iter().find(|c| &c.pin == pin) else {
            return Err(refuse(
                reference,
                format!(
                    "{pin} is declared neither strapped nor wired, and the emitter ties no control pin by a rule of its own"
                ),
            ));
        };
        let conn = match (&c.strap, &c.wire) {
            (None, None) if let Some(a) = &c.after => {
                // the open-drain PG's pull-up, to the leader's output
                let lead = regulators
                    .iter()
                    .find(|g| g.reference == a.regulator)
                    .ok_or_else(|| refuse(reference, format!("{pin} follows `{}`, no declared regulator", a.regulator)))?;
                let pg = b.own_net(format!("{}_PG", a.regulator))?;
                let up = b.rail(&lead.output);
                resistor(b, &a.pull_up, up, Conn::Net(pg.clone()))?;
                Conn::Net(pg)
            }
            (Some(s), None) => rule
                .straps
                .contains(s)
                .then(|| strap(*s))
                .flatten()
                .ok_or_else(|| {
                    refuse(
                        reference,
                        format!(
                            "{pin} is strapped `{}`, which its row does not allow or this part has no rail for",
                            s.name()
                        ),
                    )
                })?,
            // belt-and-suspenders: `validate` refuses a wire on a pin its row
            // allows none (`ControlPinUnwired`), and every held row's control
            // pins are wirable.
            (None, Some(w)) if !rule.wirable => {
                return Err(refuse(
                    reference,
                    format!("{pin} is wired to `{}`, and its row allows no wire", w.net),
                ));
            }
            (None, Some(w)) if rail(&w.net) => {
                return Err(refuse(
                    reference,
                    format!(
                        "{pin} is wired to `{}`, a rail: a tie to a rail is a strap",
                        w.net
                    ),
                ));
            }
            (None, Some(w)) if !ended.contains(&w.net) => {
                return Err(refuse(
                    reference,
                    format!(
                        "{pin} is wired to `{}`, which no connector pin or test point the specification declares ends: drawn, it would be a floating pin with a name",
                        w.net
                    ),
                ));
            }
            (None, Some(w)) => Conn::Net(w.net.clone()),
            _ => {
                return Err(refuse(
                    reference,
                    format!("{pin} is declared both strapped and wired, or neither"),
                ));
            }
        };
        out.push((pin.clone(), conn));
    }
    Ok(out)
}

fn regulator(
    b: &mut Builder<'_>,
    reg: &aethelgard_ir::power::Regulator,
    ended: &BTreeSet<String>,
) -> Result<(), EmitError> {
    let r = row(&reg.reference, &reg.part)?;
    let u = &reg.reference;
    let input = supply(b, &reg.input);
    let output = b.rail(&reg.output);
    let g = b.ground();
    let of = |role: Role| reg.external.iter().filter(move |e| e.role == role);
    let not_regulator = || refuse(u, format!("`{}` is not a regulator", reg.part));
    let (Kind::Buck(_) | Kind::Ldo(_)) = &r.kind else {
        return Err(not_regulator());
    };
    let mut conns = BTreeMap::new();
    let tied = controls(b, u, r, &reg.controls, ended, |s| match s {
        Strap::Input => Some(input.clone()),
        Strap::Open => Some(Conn::NoConnect),
        Strap::Vcca | Strap::Vccb => None,
    })?;
    for (role, conn) in tied {
        conns.insert(pin(u, r, &role)?, conn);
    }
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
                ("FB", Conn::Net(fb.clone())),
                ("VCC", Conn::Net(vcc.clone())),
                ("BOOT", Conn::Net(boot.clone())),
                ("SW", Conn::Net(sw.clone())),
            ] {
                conns.insert(pin(u, r, role)?, conn);
            }
            ic(b, u, r, conns)?;
            // `AET-WO-4411` (a), ruled 2026-11-25T21:17Z: the regulator whose
            // output is a radio module's supply belongs to the module, and
            // its output capacitors to it, so the placer sets the block
            // beside the module (`pcb::radio_group`).
            let powers = b
                .spec
                .declared_radio_modules()
                .iter()
                .find(|m| m.supply == reg.output)
                .map(|m| m.reference.clone());
            let own = |b: &mut Builder<'_>, ic: &str| {
                if let Some(p) = b.parts.last_mut() {
                    p.owner = Some(crate::schematic::Owner {
                        ic: ic.to_owned(),
                        anchor: None,
                        decouples: false,
                    });
                }
            };
            if let Some(m) = &powers {
                own(b, m);
            }
            let rules = match &r.kind {
                Kind::Buck(k) if k.fsw_hz.at.document == "SNVSAW1E" => &SNVSAW1E_RULES,
                _ => &SNVSAN3F_RULES,
            };
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
                anchor(b, u, &[("1", &vin), ("2", &pgnd)], NEAR_NM, rules.cin_hf);
            }
            for e in of(Role::Cboot) {
                capacitor(
                    b,
                    &e.reference,
                    &e.part,
                    Conn::Net(boot.clone()),
                    Conn::Net(sw.clone()),
                )?;
                anchor(b, u, &[("1", &bootp), ("2", &swp)], NEAR_NM, rules.cboot);
            }
            for e in of(Role::Inductor) {
                let Kind::Inductor(l) = &row(&e.reference, &e.part)?.kind else {
                    return Err(refuse(
                        &e.reference,
                        format!("`{}` is not an inductor", e.part),
                    ));
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
                    None,
                )?;
                anchor(b, u, &[("1", &swp)], SW_NM, rules.sw);
            }
            for e in of(Role::Cvcc) {
                capacitor(b, &e.reference, &e.part, Conn::Net(vcc.clone()), g.clone())?;
                anchor(b, u, &[("1", &vccp)], NEAR_NM, rules.cvcc);
            }
            // belt-and-suspenders: `validate` refuses a buck with no divider
            // (`ExternalPartMissing`).
            let d = reg
                .feedback
                .as_ref()
                .ok_or_else(|| refuse(u, "a buck with no feedback divider"))?;
            resistor(b, &d.top, output.clone(), Conn::Net(fb.clone()))?;
            anchor(b, u, &[("2", &fbp)], NEAR_NM, rules.fb);
            resistor(b, &d.bottom, Conn::Net(fb), g.clone())?;
            anchor(b, u, &[("1", &fbp)], NEAR_NM, rules.fb);
            for e in of(Role::Cin) {
                capacitor(b, &e.reference, &e.part, input.clone(), g.clone())?;
                anchor(b, u, &[("1", &vin), ("2", &pgnd)], CIN_BULK_NM, rules.cin);
            }
            for e in of(Role::Cout) {
                capacitor(b, &e.reference, &e.part, output.clone(), g.clone())?;
                if powers.is_some() {
                    own(b, u);
                }
            }
        }
        Kind::Ldo(_) => {
            for (role, conn) in [
                ("IN", input.clone()),
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
        _ => return Err(not_regulator()),
    }
    Ok(())
}

fn crossing(
    b: &mut Builder<'_>,
    c: &aethelgard_ir::power::Crossing,
    spi: &[SpiDevice],
    behind: &BTreeMap<String, Behind>,
    ended: &BTreeSet<String>,
) -> Result<(), EmitError> {
    let spec = b.spec;
    let r = row(&c.reference, &c.part)?;
    let device = device_on(&c.reference, &c.bus, spi)?;
    // `design` drew every SPI device on its declared bus before the power
    // tree, refusing one whose bus is not declared, so this finds it.
    let bus = spec
        .spi_buses
        .iter()
        .find(|x| x.id == c.bus)
        .ok_or_else(|| EmitError::Internal(format!("{}: no SPI bus `{}`", c.reference, c.bus)))?;
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
        ("GND", g.clone()),
    ] {
        conns.insert(pin(&c.reference, r, role)?, conn);
    }
    let tied = controls(b, &c.reference, r, &c.controls, ended, |s| match s {
        Strap::Vcca => Some(a.clone()),
        Strap::Vccb => Some(bside.clone()),
        Strap::Open => Some(Conn::NoConnect),
        Strap::Input => None,
    })?;
    for (role, conn) in tied {
        conns.insert(pin(&c.reference, r, &role)?, conn);
    }
    let a_net = |line: Line| match line {
        Line::Sck => bus.sck.net.clone(),
        Line::Mosi => bus.mosi.net.clone(),
        Line::Miso => bus.miso.net.clone(),
        Line::Cs => device.cs.net.clone(),
    };
    // [`behind`] read every crossing's lines first, refusing a row that is
    // not a translator.
    let Kind::Translator(t) = &r.kind else {
        return Err(EmitError::Internal(format!(
            "{}: `{}` is not a translator",
            c.reference, c.part
        )));
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
    resistor(
        b,
        &c.cs_pull_up,
        a.clone(),
        Conn::Net(device.cs.net.clone()),
    )?;
    let (vcca, vccb) = (pin(&c.reference, r, "VCCA")?, pin(&c.reference, r, "VCCB")?);
    for e in &c.external {
        let (rail, at) = match e.role {
            Role::BypassA => (a.clone(), &vcca),
            Role::BypassB => (bside.clone(), &vccb),
            _ => {
                return Err(refuse(
                    &e.reference,
                    "a translator part that is not a bypass",
                ));
            }
        };
        capacitor(b, &e.reference, &e.part, rail, g.clone())?;
        anchor(b, &c.reference, &[("1", at)], NEAR_NM, BYPASS_RULE);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{BULK_NEGATIVE, BULK_POSITIVE, bulk, device_on, farads, henries, lines};
    use aethelgard_ir::power::{HotPlug, Side};
    use aethelgard_ir::power_parts::{POWER_PARTS_JSON, PowerPartsTable, parse_power_parts};

    /// The embedded table with one test-only TVS row, which no held row is
    /// (`AET-WO-4300d3` tested its clamp the same way).
    fn with_tvs() -> PowerPartsTable {
        let at = r#"{"document": "NICHICON_UCZ", "section": "test only"}"#;
        let tvs = format!(
            r#"{{"part": "TVS_TEST", "lifecycle": "unread", "symbol": "Device:D_TVS", "footprint": "Diode_SMD:D_SMB", "pins": {{}}, "temperature_celsius": {{"min": -65, "max": 150, "at": {at}}}, "kind": {{"tvs": {{"standoff_millivolts": {{"value": 27000, "at": {at}}}, "clamp_max_millivolts": {{"value": 37000, "at": {at}}}, "clamp_condition": "test only"}}}}}},"#
        );
        let json = POWER_PARTS_JSON.replacen(r#""parts": ["#, &format!(r#""parts": [{tvs}"#), 1);
        assert_ne!(json, POWER_PARTS_JSON);
        parse_power_parts(&json).unwrap()
    }

    fn hot(part: &str, side: Side) -> HotPlug {
        HotPlug {
            reference: "C1".into(),
            part: part.into(),
            side,
        }
    }

    /// `AET-WO-1315` §1.3: the emitter draws a bulk capacitor on the board's
    /// side and refuses anything else by name: a TVS row, a ceramic, a part
    /// on the pack's side, a part the table lacks.
    #[test]
    fn only_a_bulk_capacitor_on_the_board_side_is_drawable() {
        let t = with_tvs();
        let (row, c) = bulk(&t, &hot("UCZ1H470MCL1GS", Side::Board)).unwrap();
        assert_eq!(row.symbol, "Device:C_Polarized");
        assert_eq!(c.rated_picofarads.value, 47_000_000);
        for (part, side, why) in [
            ("TVS_TEST", Side::Board, "`TVS_TEST` is a tvs row"),
            ("GCM188R71H104KA57", Side::Board, "is a capacitor row"),
            ("UCZ1H470MCL1GS", Side::Pack, "on the pack's side"),
            ("NOT_A_PART", Side::Board, "is not in the power parts table"),
        ] {
            match bulk(&t, &hot(part, side)) {
                Err(super::EmitError::PowerNotDrawable { reference, why: w }) => {
                    assert_eq!(reference, "C1");
                    assert!(w.contains(why), "{part}: {w}");
                }
                other => panic!("{part}: {other:?}"),
            }
        }
    }

    /// The positive terminal is read from the vendored files themselves: the
    /// symbol's `+` (two crossing strokes) is nearer pin [`BULK_POSITIVE`]
    /// than pin [`BULK_NEGATIVE`], and so is the footprint's silkscreen `+`
    /// to its pad.
    #[test]
    fn the_bulk_capacitors_positive_terminal_is_where_the_vendored_files_mark_it() {
        use crate::sexpr::Sx;
        fn xy(s: &Sx, i: usize) -> (f64, f64) {
            (
                s.arg(i).unwrap().parse().unwrap(),
                s.arg(i + 1).unwrap().parse().unwrap(),
            )
        }
        fn walk<'a>(s: &'a Sx, out: &mut Vec<&'a Sx>) {
            out.push(s);
            for c in s.items() {
                walk(c, out);
            }
        }
        let d2 = |a: (f64, f64), b: (f64, f64)| (a.0 - b.0).powi(2) + (a.1 - b.1).powi(2);
        let nearer = |mark: (f64, f64), terminals: &[(String, (f64, f64))]| {
            let near = |n: &str| d2(mark, terminals.iter().find(|t| t.0 == n).unwrap().1);
            assert_eq!(terminals.len(), 2);
            near(BULK_POSITIVE) < near(BULK_NEGATIVE)
        };
        // The symbol: its pins, and the `+` as the crossing of a horizontal
        // and a vertical stroke, each under 1.5 mm.
        let sym = crate::library::symbol("Device:C_Polarized").unwrap();
        let mut all = Vec::new();
        walk(&sym, &mut all);
        let pins: Vec<(String, (f64, f64))> = all
            .iter()
            .filter(|s| s.head() == Some("pin"))
            .map(|p| {
                (
                    p.child("number").unwrap().arg(1).unwrap().to_owned(),
                    xy(p.child("at").unwrap(), 1),
                )
            })
            .collect();
        let strokes: Vec<((f64, f64), (f64, f64))> = all
            .iter()
            .filter(|s| s.head() == Some("polyline"))
            .filter_map(|s| {
                let pts: Vec<_> = s.child("pts")?.children("xy").map(|p| xy(p, 1)).collect();
                let [a, b] = pts.as_slice() else { return None };
                (d2(*a, *b) < 1.5f64.powi(2)).then_some((*a, *b))
            })
            .collect();
        let mid = |s: &((f64, f64), (f64, f64))| ((s.0.0 + s.1.0) / 2.0, (s.0.1 + s.1.1) / 2.0);
        let h = strokes.iter().find(|s| s.0.1 == s.1.1).unwrap();
        let v = strokes.iter().find(|s| s.0.0 == s.1.0).unwrap();
        assert!(d2(mid(h), mid(v)) < 1e-6, "{h:?} {v:?}");
        assert!(nearer(mid(h), &pins), "symbol {pins:?} {h:?}");
        // The footprint: its pads, and the silkscreen `+` the same way.
        let fp = crate::library::footprint("Capacitor_SMD:CP_Elec_8x10").unwrap();
        let pads: Vec<(String, (f64, f64))> = fp
            .children("pad")
            .map(|p| (p.arg(1).unwrap().to_owned(), xy(p.child("at").unwrap(), 1)))
            .collect();
        let silk: Vec<((f64, f64), (f64, f64))> = fp
            .children("fp_line")
            .filter(|l| l.child("layer").and_then(|x| x.arg(1)) == Some("F.SilkS"))
            .map(|l| {
                (
                    xy(l.child("start").unwrap(), 1),
                    xy(l.child("end").unwrap(), 1),
                )
            })
            .filter(|(a, b)| d2(*a, *b) < 1.5f64.powi(2))
            .collect();
        let h = silk.iter().find(|s| s.0.1 == s.1.1).unwrap();
        let v = silk.iter().find(|s| s.0.0 == s.1.0).unwrap();
        assert!(d2(mid(h), mid(v)) < 1e-6, "{h:?} {v:?}");
        assert!(nearer(mid(h), &pads), "footprint {pads:?} {h:?}");
    }

    /// `AET-WO-1322` (1): the crossing's re-checks of what `validate`
    /// refuses (`CrossingInvalid`), reached directly, as `Drawing::of`
    /// reaches them: a bus with no device, and a part that is not a
    /// translator.
    #[test]
    fn a_crossing_is_refused_without_one_device_or_a_translator() {
        let on = |reference: &str, bus: &str| aethelgard_ir::SpiDevice {
            reference: reference.into(),
            part: "BMP581".into(),
            supply: "VDD_3V3".into(),
            bus: bus.into(),
            cs: aethelgard_ir::Signal {
                net: format!("{reference}_CS"),
                pin: None,
            },
            pin_capacitance: None,
            orientation: None,
        };
        let one = [on("U2", "spi1"), on("U3", "spi2")];
        assert_eq!(device_on("U9", "spi1", &one).unwrap().reference, "U2");
        let two = [on("U2", "spi1"), on("U3", "spi1")];
        for (got, want) in [
            (
                device_on("U9", "spi1", &two).map(|_| ()),
                "bus `spi1` has 2 devices",
            ),
            (
                device_on("U9", "spi1", &[]).map(|_| ()),
                "bus `spi1` has 0 devices",
            ),
            (
                lines("U9", "GCM188R71H104KA57").map(|_| ()),
                "`GCM188R71H104KA57` is not a translator",
            ),
        ] {
            match got {
                Err(super::EmitError::PowerNotDrawable { reference, why }) => {
                    assert_eq!(reference, "U9");
                    assert!(why.contains(want), "{why}");
                }
                other => panic!("{want}: {other:?}"),
            }
        }
    }

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
