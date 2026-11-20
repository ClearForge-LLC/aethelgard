//! Pin devices drawn (`AET-WO-1211d`): a driver IC with no bus, read from its
//! parts row (`aethelgard_ir::pin_parts`, `parts.json` `pin_parts`) and its
//! declaration (`pin_devices`), with the passives its datasheet requires.
//!
//! What comes from where:
//! - **the row:** every pin's number and kind;
//! - **the declaration:** the supply's domain, each strap's tie or resistor,
//!   each signal's net and pull, each analog output's resistor and reservoir,
//!   each bridge's input and output nets;
//! - **this module's [`Arm`]:** the symbol, the footprint and the passives the
//!   datasheet requires but the row does not hold (`AET-WO-1211` Q6: they are
//!   this work order's), each with its page. A passive whose value the
//!   datasheet does not state is refused by name (`PassiveValueUnstated`),
//!   never invented (ruled S1, 2026-11-20).
//!
//! Every numbered pin of the row ends drawn on a net or a rail, or, for an
//! open-drain output the declaration leaves out, on a no-connect. Anything
//! else is refused (`PinDeviceNotDrawable`), so no pin is left floating by an
//! omission here. Each bridge output must reach a declared connector pin
//! (`BridgeOutputNotOnConnector`).
//!
//! Capacitor ratings: the emitter's passives carry a value and a footprint
//! only, so each value names the rating the datasheet asks for (`16V`, or
//! `>` the VM domain's maximum where it asks for "VM-rated"). No gate judges a rating
//! (ruled 18:18Z: a finding, folded into `4600` D4's proposal).

use std::collections::{BTreeMap, BTreeSet};

use aethelgard_ir::SolvedTopology;
use aethelgard_ir::pin_devices::{PinDevice, SignalPull, Strap, Transfer};
use aethelgard_ir::pin_parts::{PinKind, PinPart};

use super::{Builder, Conn, EmitError, Part};

/// A pin of the part: its row name.
type Pin = &'static str;

/// One end of a required passive: a row pin (its drawn connection), or
/// ground.
#[derive(Clone, Copy, Debug)]
enum End {
    Pin(Pin),
    Ground,
}

/// The voltage a capacitor must be rated for.
#[derive(Clone, Copy, Debug)]
enum Rating {
    /// A stated rating, as the datasheet writes it (`16V`).
    Volts(&'static str),
    /// "VM-rated": above the VM domain's maximum, from the specification,
    /// written `>12.6V`, a minimum and not a part's rating (SLVSAR1E §9.1
    /// p. 14: "higher than the operating voltage").
    Vm,
}

/// A passive the datasheet requires and the row does not hold.
#[derive(Clone, Copy, Debug)]
struct Required {
    /// The datasheet's designator, or a name for it where it gives none.
    name: &'static str,
    /// Its value as schematics write it; `None` where the datasheet states
    /// none, which is refused.
    value: Option<&'static str>,
    rating: Rating,
    /// The dielectric the datasheet names, if any.
    dielectric: Option<&'static str>,
    one: End,
    two: End,
    /// The datasheet's words for it, with the page.
    cite: &'static str,
}

/// A required passive with its two connections and the pads it is
/// anchored by (its own, the device's).
type Placed<'a> = (&'a Required, Conn, Conn, Vec<(String, String)>);

/// One part's arm: what the row does not hold.
struct Arm {
    part: &'static str,
    symbol: &'static str,
    footprint: &'static str,
    /// The symbol's thermal pad, which the row lists unnumbered.
    pad: &'static str,
    /// In draw order; a refusal comes after every other passive is drawn.
    passives: &'static [Required],
    /// The datasheet's placement rule for its passives.
    near: &'static str,
}

/// How close each required passive sits to its pins: an engine choice, as
/// the power tree's are (`schematic_power`'s `NEAR_NM`): each datasheet asks
/// for "as close as possible" and states no distance.
const NEAR_NM: i64 = 2_000_000;

/// SLVSDS7B (DRV8876, Aug 2019 rev. Nov 2019). The symbol and footprint are
/// our own (`data/aethelgard/`, cited there): KiCad 10.0.7 has neither, and
/// no HTSSOP-16 variant of its matches PWP0016A's (3.3) by (3.3) mask
/// opening (pp. 45–47).
const DRV8876: Arm = Arm {
    part: "DRV8876PWPR",
    symbol: "Aethelgard_Driver:DRV8876PWP",
    footprint: "Aethelgard_Driver:TI_HTSSOP-16-1EP_4.4x5mm_P0.65mm_EP3.4x5mm_Mask3.3x3.3mm",
    pad: "17",
    passives: &[
        Required {
            name: "CVM1",
            value: Some("100n"),
            rating: Rating::Vm,
            dielectric: None,
            one: End::Pin("VM"),
            two: End::Ground,
            cite: "SLVSDS7B Table 1 p. 10: CVM1, VM to GND, \"0.1-µF, low ESR ceramic capacitor, VM-rated\"; p. 3, VM",
        },
        Required {
            name: "CVCP",
            value: Some("100n"),
            rating: Rating::Volts("16V"),
            dielectric: Some("X5R/X7R"),
            one: End::Pin("VCP"),
            two: End::Pin("VM"),
            cite: "SLVSDS7B Table 1 p. 10: CVCP, VCP to VM, \"X5R or X7R, 100-nF, 16-V ceramic capacitor\"; p. 3, VCP",
        },
        Required {
            name: "CFLY",
            value: Some("22n"),
            rating: Rating::Vm,
            dielectric: Some("X5R/X7R"),
            one: End::Pin("CPH"),
            two: End::Pin("CPL"),
            cite: "SLVSDS7B Table 1 p. 10: CFLY, CPH to CPL, \"X5R or X7R, 22-nF, VM-rated ceramic capacitor\"; p. 3, CPH",
        },
        Required {
            name: "CVM2",
            value: None,
            rating: Rating::Vm,
            dielectric: None,
            one: End::Pin("VM"),
            two: End::Ground,
            cite: "SLVSDS7B Table 1 p. 10: CVM2, VM to GND, \"Bulk Capacitance, VM-rated\", no value; p. 3: \"sufficient Bulk Capacitance rated for VM\"; §9.1 p. 31: \"system level testing is required to determine the appropriately sized bulk capacitor\"",
        },
    ],
    near: "SLVSDS7B §10.1 p. 32: \"The VM power supply and VCP, CPH, CPL charge pump capacitors should be placed as close to the device as possible\" (2.0 mm, an engine choice: no distance is stated)",
};

/// SLVSAR1E (DRV8833, Jan 2011 rev. Jul 2015). Our own symbol (ruled Q3:
/// AISEN and BISEN `passive`, since p. 10 grounds them directly when current
/// control is not needed), and KiCad 10.0.7's PWP0016C footprint without
/// thermal vias (PWP0016C note 10, p. 27: "Vias are optional"), vendored
/// byte for byte.
const DRV8833: Arm = Arm {
    part: "DRV8833PWPR",
    symbol: "Aethelgard_Driver:DRV8833PWP",
    footprint: "Package_SO:HTSSOP-16-1EP_4.4x5mm_P0.65mm_EP3.4x5mm_Mask2.46x2.31mm",
    pad: "17",
    passives: &[
        Required {
            name: "CVM",
            value: Some("10u"),
            rating: Rating::Vm,
            dielectric: None,
            one: End::Pin("VM"),
            two: End::Ground,
            cite: "SLVSAR1E p. 3, VM: \"A 10-µF (minimum) ceramic bypass capacitor to GND is recommended\"; §10.1 p. 15: \"a recommended value of 10-μF rated for VM\"",
        },
        Required {
            name: "CVCP",
            value: Some("10n"),
            rating: Rating::Volts("16V"),
            dielectric: Some("X7R"),
            one: End::Pin("VCP"),
            two: End::Pin("VM"),
            cite: "SLVSAR1E p. 3, VCP: \"Connect a 0.01-μF, 16-V (minimum) X7R ceramic capacitor to VM\"; §10.1 p. 15",
        },
        Required {
            name: "CVINT",
            value: Some("2u2"),
            rating: Rating::Volts("6.3V"),
            dielectric: None,
            one: End::Pin("VINT"),
            two: End::Ground,
            cite: "SLVSAR1E p. 3, VINT: \"Bypass to GND with 2.2-μF, 6.3-V ceramic capacitor\"; §10.1 p. 15",
        },
    ],
    near: "SLVSAR1E §10.1 p. 15: each capacitor \"as close to the pin(s) as possible\" (2.0 mm, an engine choice: no distance is stated)",
};

/// Every part this module draws: [`super::KNOWN_PARTS`] lists them.
const ARMS: [&Arm; 2] = [&DRV8876, &DRV8833];

/// Whether this module has an arm for `part`.
pub(super) fn draws(part: &str) -> bool {
    ARMS.iter().any(|a| a.part == part)
}

fn not_drawable(d: &PinDevice, why: impl Into<String>) -> EmitError {
    EmitError::PinDeviceNotDrawable {
        reference: d.reference.clone(),
        part: d.part.clone(),
        why: why.into(),
    }
}

/// `mv` as volts the way a capacitor's rating is written: `12.6V`.
fn volts(mv: u64) -> String {
    let (whole, frac) = (mv / 1000, mv % 1000);
    if frac == 0 {
        return format!("{whole}V");
    }
    let mut digits = format!("{frac:03}");
    while digits.ends_with('0') {
        digits.pop();
    }
    format!("{whole}.{digits}V")
}

/// The references the pin devices bring: each device's, then its declared
/// passives' (a pull, a strap resistor, an analog output's resistor and
/// reservoir), claimed before any emitter-numbered one. `devices` is the
/// specification's, which `schematic` reads for this module (`RUL-16`:
/// `eda`'s one reader of declared devices, as for the power tree).
pub(super) fn references(devices: &[PinDevice]) -> Vec<&String> {
    let mut out = Vec::new();
    for d in devices {
        out.push(&d.reference);
        for s in d.straps.values() {
            if let Strap::Resistor(r) = s {
                out.push(&r.reference);
            }
        }
        for s in &d.signals {
            if let Some(SignalPull {
                resistor: Some(r), ..
            }) = &s.pull
            {
                out.push(&r.reference);
            }
        }
        for a in &d.analog {
            let Transfer::CurrentOutput(c) = &a.transfer;
            out.push(&c.resistor.reference);
            if let Some(r) = &c.reservoir {
                out.push(&r.reference);
            }
        }
    }
    out
}

/// Draws each of `devices` (the specification's, read by `schematic`), in
/// document order. `on_connectors` is every net a declared connector pin
/// carries, where a bridge output must end; `mcu` is the MCU's reference, by
/// whose ADC pins an analog output's reservoir sits.
pub(super) fn draw(
    b: &mut Builder<'_>,
    devices: &[PinDevice],
    on_connectors: &BTreeSet<String>,
    solved: &SolvedTopology,
    table: &aethelgard_ir::PinTable,
    mcu: &str,
) -> Result<(), EmitError> {
    let parts = aethelgard_ir::parts::table().map_err(EmitError::InvalidSpecification)?;
    for d in devices {
        // belt-and-suspenders: `design` refuses a part with no arm first
        // (`PinDeviceNotDrawn`), and `validate` a part with no row.
        let arm =
            ARMS.iter()
                .find(|a| a.part == d.part)
                .ok_or_else(|| EmitError::PinDeviceNotDrawn {
                    reference: d.reference.clone(),
                    part: d.part.clone(),
                })?;
        let row = parts
            .pin_part(&d.part)
            .ok_or_else(|| not_drawable(d, "no parts row"))?;
        let around = Around {
            solved,
            table,
            mcu,
            on_connectors,
        };
        device(b, d, arm, row, &around)?;
    }
    Ok(())
}

/// The symbol pin of row pin `name`.
fn number(d: &PinDevice, arm: &Arm, row: &PinPart, name: &str) -> Result<String, EmitError> {
    let p = row
        .pins
        .get(name)
        .ok_or_else(|| not_drawable(d, format!("its row names no pin `{name}`")))?;
    Ok(p.number
        .map_or_else(|| arm.pad.to_owned(), |n| n.to_string()))
}

/// What a device is drawn against, beside its row and declaration.
struct Around<'a> {
    solved: &'a SolvedTopology,
    table: &'a aethelgard_ir::PinTable,
    mcu: &'a str,
    on_connectors: &'a BTreeSet<String>,
}

fn device(
    b: &mut Builder<'_>,
    d: &PinDevice,
    arm: &Arm,
    row: &PinPart,
    around: &Around<'_>,
) -> Result<(), EmitError> {
    let Around {
        solved,
        table,
        mcu,
        on_connectors,
    } = *around;
    let spec = b.spec;
    let gnd = b.ground();
    let mut conns: BTreeMap<String, Conn> = BTreeMap::new();
    let put = |pin: String, c: Conn, conns: &mut BTreeMap<String, Conn>| {
        // A GPIO bridge input is declared twice, as a signal and as the
        // bridge's input, on one net (`AET-WO-1211` Q7). belt-and-suspenders:
        // `validate` binds each device pin to one net, so any other second
        // connection is refused here, never drawn.
        match conns.insert(pin.clone(), c.clone()) {
            Some(old) if old != c => Err(not_drawable(
                d,
                format!("pin {pin} is drawn on two connections"),
            )),
            _ => Ok(()),
        }
    };
    // Supplies and grounds.
    for (pin, domain) in &d.supplies {
        let c = b.rail(domain);
        put(number(d, arm, row, pin)?, c, &mut conns)?;
    }
    for (name, p) in &row.pins {
        if p.kind == PinKind::Ground {
            let g = b.ground();
            put(number(d, arm, row, name)?, g, &mut conns)?;
        }
    }
    // Straps: a tie straight to its rail (a strap through 10 kΩ would select
    // another level: SLVSDS7B p. 5's IMODE bands start at 18.6 kΩ), or the
    // declared resistor from the pin's own net to GND.
    let mut resistors: Vec<(aethelgard_ir::power::Resistor, Conn, Conn)> = Vec::new();
    for (pin, s) in &d.straps {
        let n = number(d, arm, row, pin)?;
        match s {
            Strap::Tie(net) => {
                let c = b.declared(net);
                put(n, c, &mut conns)?;
            }
            Strap::Resistor(r) => {
                let net = b.own_net(format!("{}_{pin}", d.reference))?;
                put(n, Conn::Net(net.clone()), &mut conns)?;
                resistors.push((r.clone(), Conn::Net(net), gnd.clone()));
            }
        }
    }
    // Signals, then analog outputs, then bridges: each on its declared net.
    for s in &d.signals {
        put(
            number(d, arm, row, &s.device_pin)?,
            Conn::Net(s.net.clone()),
            &mut conns,
        )?;
        if let Some(SignalPull {
            resistor: Some(r), ..
        }) = &s.pull
        {
            let to = b.declared(&r.to);
            resistors.push((
                aethelgard_ir::power::Resistor {
                    reference: r.reference.clone(),
                    part: r.part.clone(),
                    ohms: r.ohms,
                },
                to,
                Conn::Net(s.net.clone()),
            ));
        }
    }
    let mut analog = Vec::new();
    for a in &d.analog {
        put(
            number(d, arm, row, &a.device_pin)?,
            Conn::Net(a.net.clone()),
            &mut conns,
        )?;
        let Transfer::CurrentOutput(c) = &a.transfer;
        let pin = solved
            .adc_inputs
            .get(&a.net)
            .ok_or_else(|| EmitError::UnsolvedNet(a.net.clone()))?;
        let position = table
            .pins
            .get(&pin.pin)
            .ok_or_else(|| EmitError::UnknownPin {
                net: a.net.clone(),
                pin: pin.pin.clone(),
            })?
            .position
            .to_string();
        analog.push((c, Conn::Net(a.net.clone()), position));
    }
    for bridge in &d.bridges {
        for (pin, net) in &bridge.inputs {
            put(
                number(d, arm, row, pin)?,
                Conn::Net(net.clone()),
                &mut conns,
            )?;
        }
        for (pin, net) in &bridge.outputs {
            // A motor terminal is a net of its own: on `GND` or a domain it
            // would short the bridge to a rail (`validate` refuses a domain's
            // name; `GND` reached the drawing, adversarial pass #3).
            if net == aethelgard_ir::GND || spec.voltage_domains.iter().any(|v| &v.id == net) {
                return Err(not_drawable(
                    d,
                    format!(
                        "its bridge output `{pin}` names `{net}`, a rail: a motor terminal is a net of its own"
                    ),
                ));
            }
            if !on_connectors.contains(net) {
                return Err(EmitError::BridgeOutputNotOnConnector {
                    reference: d.reference.clone(),
                    pin: pin.clone(),
                    net: net.clone(),
                });
            }
            put(
                number(d, arm, row, pin)?,
                Conn::Net(net.clone()),
                &mut conns,
            )?;
        }
    }
    // The pins only a passive of the arm's joins: each on its own net. A
    // `power_in` pin there (the DRV8833's VINT, a regulator the part
    // drives itself) takes its net as a rail, so it carries a `PWR_FLAG`.
    for (name, p) in &row.pins {
        if p.kind == PinKind::Passive {
            let net = b.own_net(format!("{}_{name}", d.reference))?;
            let n = number(d, arm, row, name)?;
            let c = if super::power_in(arm.symbol, &n)? {
                b.rail(&net)
            } else {
                Conn::Net(net)
            };
            put(n, c, &mut conns)?;
        }
    }
    // Every numbered pin is drawn; an open-drain output left undeclared is
    // a no-connect (its pull-up is the declaration's), and anything else is
    // refused rather than left floating.
    for (name, p) in &row.pins {
        let n = number(d, arm, row, name)?;
        if conns.contains_key(&n) {
            continue;
        }
        match p.kind {
            PinKind::OpenDrainOutput { .. } => {
                conns.insert(n, Conn::NoConnect);
            }
            PinKind::CurrentOutput { .. } => {
                return Err(not_drawable(
                    d,
                    format!(
                        "`{name}` is declared neither as an analog output nor tied: SLVSDS7B p. 13 asks for RIPROPI to GND, or IPROPI tied to GND with VREF above GND, and a pin device cannot declare that tie"
                    ),
                ));
            }
            _ => {
                return Err(not_drawable(
                    d,
                    format!(
                        "its {} pin `{name}` is declared nowhere, and would float",
                        p.kind.name()
                    ),
                ));
            }
        }
    }
    let at = |name: &str| -> Result<Conn, EmitError> {
        let n = number(d, arm, row, name)?;
        conns
            .get(&n)
            .cloned()
            .ok_or_else(|| not_drawable(d, format!("`{name}` is not drawn")))
    };
    let ends: Vec<Placed<'_>> = {
        let mut out = Vec::new();
        for r in arm.passives {
            let mut pads = Vec::new();
            let mut end = |e: End, own: &str| -> Result<Conn, EmitError> {
                match e {
                    End::Ground => Ok(gnd.clone()),
                    End::Pin(name) => {
                        // Anchored by its first pin end only: CFLY anchored
                        // to both CPH and CPL (adjacent pins, 0.65 mm apart)
                        // left `U2_CPH` unroutable (measured on D1,
                        // `spike/1211d/RESULTS.md`).
                        if pads.is_empty() {
                            pads.push((own.to_owned(), number(d, arm, row, name)?));
                        }
                        at(name)
                    }
                }
            };
            let one = end(r.one, "1")?;
            let two = end(r.two, "2")?;
            out.push((r, one, two, pads));
        }
        out
    };
    let vm = d
        .supplies
        .get("VM")
        .and_then(|domain| spec.voltage_domains.iter().find(|v| &v.id == domain))
        .map(|v| u64::from(v.nominal_millivolts) * (1000 + u64::from(v.tolerance_permille)) / 1000);
    b.parts.push(Part {
        lib_id: arm.symbol,
        reference: d.reference.clone(),
        value: d.part.clone(),
        footprint: arm.footprint.to_owned(),
        conns,
        note: None,
        owner: None,
        offboard: BTreeMap::new(),
        seat: None,
    });
    // The declared resistors (straps, pulls), then each analog output's
    // resistor to GND and its reservoir by the MCU's ADC pin: all before the
    // arm's own passives, so that a passive refused there
    // (`PassiveValueUnstated`) is refused with every other part drawn.
    for (r, one, two) in resistors {
        super::power::resistor(b, &r, one, two)?;
    }
    for (c, net, position) in analog {
        super::power::resistor(b, &c.resistor, net.clone(), gnd.clone())?;
        if let Some(res) = &c.reservoir {
            super::power::reservoir(b, mcu, res, net, gnd.clone(), &position)?;
        }
    }
    // The arm's passives, each anchored to the pins it joins, in its order.
    for (r, one, two, pads) in ends {
        let Some(value) = r.value else {
            return Err(EmitError::PassiveValueUnstated {
                reference: d.reference.clone(),
                part: d.part.clone(),
                passive: r.name.to_owned(),
                cite: r.cite.to_owned(),
            });
        };
        let rating = match r.rating {
            Rating::Volts(v) => v.to_owned(),
            Rating::Vm => format!(
                ">{}",
                volts(vm.ok_or_else(|| not_drawable(d, "its VM names no domain"))?)
            ),
        };
        let text = match r.dielectric {
            Some(x) => format!("{value} {rating} {x}"),
            None => format!("{value} {rating}"),
        };
        b.passive('C', text, one, two)?;
        b.anchor_last(&d.reference, pads, Some((NEAR_NM, arm.near)), false);
    }
    Ok(())
}
