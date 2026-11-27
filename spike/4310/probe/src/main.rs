//! AET-WO-4310 (1), part (2): every powered part a specification declares,
//! its rail, its declared minimum operating supply (from the IR's own
//! tables, with the citation), and the reset's falling minimum
//! (`supervisor::reset_floor`), judged under three rail readings:
//! (A) parts on the MCU's supply against the MCU's reset;
//! (B) each part against its own rail's reset (only the MCU's rail has one);
//! (C) every part on any rail against the MCU's reset (`radio::not_judged`'s
//! comparison).
//! One TSV row per part; usage: probe-4310 SPEC...
use aethelgard_ir::Specification;
use aethelgard_ir::power_parts::Kind;

struct Part {
    reference: String,
    part: String,
    class: &'static str,
    rail: String,
    min: Option<(u32, String)>,
}

fn parts_of(spec: &Specification) -> Vec<Part> {
    let mut out = Vec::new();
    let pp = aethelgard_ir::power_parts::table().expect("power parts");
    let dev = aethelgard_ir::parts::table().expect("parts");
    let radio = aethelgard_ir::radio::table().expect("radio parts");
    // The MCU.
    let mcu = pp.get(&spec.mcu.part).and_then(|r| match &r.kind {
        Kind::Mcu(m) => Some((
            u32::try_from(m.vdd_millivolts.range.min).unwrap_or(0),
            pp.cite(&m.vdd_millivolts.at),
        )),
        _ => None,
    });
    out.push(Part {
        reference: "MCU".into(),
        part: spec.mcu.part.clone(),
        class: "mcu",
        rail: spec.mcu.supply.clone(),
        min: mcu,
    });
    // I2C and SPI devices, at their bus's speed.
    for d in spec.declared_i2c_devices() {
        let hz = spec
            .i2c_buses
            .iter()
            .find(|b| b.id == d.bus)
            .map(|b| b.speed_hz);
        out.push(Part {
            reference: d.reference.clone(),
            part: d.part.clone(),
            class: "i2c",
            rail: d.supply.clone(),
            min: dev.get(&d.part).and_then(|p| {
                p.supply_min_at(u64::from(hz.unwrap_or(u32::MAX)))
                    .map(|v| (v, "parts.json supply_millivolts".to_owned()))
            }),
        });
    }
    for d in spec.declared_spi_devices() {
        let hz = spec
            .spi_buses
            .iter()
            .find(|b| b.id == d.bus)
            .map(|b| b.speed_hz);
        out.push(Part {
            reference: d.reference.clone(),
            part: d.part.clone(),
            class: "spi",
            rail: d.supply.clone(),
            min: dev.get(&d.part).and_then(|p| {
                p.supply_min_at(u64::from(hz.unwrap_or(u32::MAX)))
                    .map(|v| (v, "parts.json supply_millivolts".to_owned()))
            }),
        });
    }
    // Radio modules.
    for m in spec.declared_radio_modules() {
        out.push(Part {
            reference: m.reference.clone(),
            part: m.part.clone(),
            class: "radio",
            rail: m.supply.clone(),
            min: radio.get(&m.part).map(|r| {
                (
                    r.supply.min_mv,
                    format!("{}, {}", r.supply.at.document, r.supply.at.section),
                )
            }),
        });
    }
    // A declared supervisor: powered from the MCU's supply.
    if let Some(s) = &spec.mcu.reset_supervisor {
        let min = pp.get(&s.part).and_then(|r| match &r.kind {
            Kind::Supervisor(v) => Some((
                u32::try_from(v.vdd_millivolts.range.min).unwrap_or(0),
                pp.cite(&v.vdd_millivolts.at),
            )),
            _ => None,
        });
        out.push(Part {
            reference: s.reference.clone(),
            part: s.part.clone(),
            class: "supervisor",
            rail: spec.mcu.supply.clone(),
            min,
        });
    }
    // Translators: each side is a powered part; no row holds an operating
    // minimum in a field (only timing-row bands), so `min` is None.
    if let Some(p) = &spec.power {
        for c in &p.crossings {
            for (side, rail) in [("A", &c.a_domain), ("B", &c.b_domain)] {
                out.push(Part {
                    reference: format!("{}.{side}", c.reference),
                    part: c.part.clone(),
                    class: "translator",
                    rail: rail.clone(),
                    min: None,
                });
            }
        }
        for c in &p.uart_crossings {
            for (side, rail) in [("A", &c.a_domain), ("B", &c.b_domain)] {
                out.push(Part {
                    reference: format!("{}.{side}", c.reference),
                    part: c.part.clone(),
                    class: "translator",
                    rail: rail.clone(),
                    min: None,
                });
            }
        }
    }
    // Pin devices (motor drivers): the pin-parts rows hold no supply minimum.
    for (reference, d) in spec
        .declared_pin_devices()
        .iter()
        .map(|d| (&d.reference, d))
    {
        out.push(Part {
            reference: reference.clone(),
            part: d.part.clone(),
            class: "pin_device",
            rail: "-".into(),
            min: None,
        });
    }
    out
}

fn main() {
    println!("spec\treset_mv\treset_src\tref\tpart\tclass\trail\tmin_mv\tmin_src\ttoday\tA\tB\tC");
    for path in std::env::args().skip(1) {
        let text = std::fs::read_to_string(&path).expect("read");
        let spec: Specification = match serde_json::from_str(&text) {
            Ok(s) => s,
            Err(e) => {
                println!("{path}\tparse-refused: {e}");
                continue;
            }
        };
        let (reset, src) = match aethelgard_ir::supervisor::reset_floor(&spec) {
            Ok(r) => r,
            Err(e) => {
                println!("{path}\treset-refused: {e:?}");
                continue;
            }
        };
        let today: Vec<String> = aethelgard_ir::supervisor::floors_above_reset(&spec)
            .map(|v| v.into_iter().map(|f| f.device).collect())
            .unwrap_or_default();
        let name = path.rsplit('/').next().unwrap_or(&path).to_owned();
        for p in parts_of(&spec) {
            let on_mcu = p.rail == spec.mcu.supply;
            let judge = |applies: bool| -> &'static str {
                if !applies {
                    return "not-judged";
                }
                match &p.min {
                    None => "NAMED(no-min)",
                    Some((v, _)) if *v > reset => "FLOOR",
                    Some(_) => "pass",
                }
            };
            let a = judge(on_mcu);
            let b = if on_mcu {
                judge(true)
            } else {
                "NAMED(no-reset-on-rail)"
            };
            let c = judge(true);
            let (mv, msrc) = p
                .min
                .as_ref()
                .map(|(v, s)| (v.to_string(), s.clone()))
                .unwrap_or(("-".into(), "-".into()));
            println!(
                "{name}\t{reset}\t{src}\t{}\t{}\t{}\t{}\t{mv}\t{msrc}\t{}\t{a}\t{b}\t{c}",
                p.reference,
                p.part,
                p.class,
                p.rail,
                if today.contains(&p.reference) {
                    "FLOOR"
                } else {
                    "-"
                }
            );
        }
    }
}
