//! AET-WO-1319 §5: inputs the 274 of `d4diff.rs` do not cover. Every one of
//! them declares a pin device, or names a pin device's part where a bus
//! device goes (and a bus device's part where a pin device goes). Driven
//! exactly as `d4diff.rs` drives its cases: validate, solve, draw
//! (`Drawing::of`, dumped part by part), and `emit` at library level when
//! validate and solve pass. A case `solve` refuses is drawn against a donor's
//! solved topology with names renamed, so `design` is reached anyway.
//! Writes every result under `$D4OUT`.
use aethelgard_eda::schematic::{self, Drawing};
use aethelgard_ir::Specification;
use serde_json::{Value, json};
use std::fmt::Write as _;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn load(rel: &str) -> Value {
    serde_json::from_slice(&std::fs::read(root().join(rel)).unwrap()).unwrap()
}
fn rename(v: &Value, from: &str, to: &str) -> Value {
    serde_json::from_str(&v.to_string().replace(&format!("\"{from}\""), &format!("\"{to}\""))).unwrap()
}

/// `pin_devices_drawn_tier1.rs`'s D2 drawn: each bridge input a GPIO output.
fn d2() -> Value {
    let mut v = load("crates/aethelgard-ir/tests/fixtures/d2-drv8833.json");
    v["board"].as_object_mut().unwrap().remove("drawn");
    let inputs = [("A_IN1", "AIN1"), ("A_IN2", "AIN2"), ("B_IN1", "BIN1"), ("B_IN2", "BIN2")];
    v["motor_outputs"]
        .as_array_mut()
        .unwrap()
        .retain(|m| !inputs.iter().any(|(net, _)| m["label"] == *net));
    let signals = v["pin_devices"][0]["signals"].as_array_mut().unwrap();
    for (net, pin) in inputs {
        signals.push(json!({"device_pin": pin, "net": net, "pin": null, "direction": "output", "initial": "low"}));
    }
    v
}
/// `pin_devices_drawn_tier1.rs`'s D1 drawn.
fn d1() -> Value {
    let mut v = load("crates/aethelgard-ir/tests/fixtures/d1-drv8876.json");
    v["board"].as_object_mut().unwrap().remove("drawn");
    v
}
fn res(r: &str, ohms: u64) -> Value {
    json!({"reference": r, "part": "CRCW0603_F_K", "ohms": ohms})
}

struct Case {
    name: String,
    case: Value,
    /// Drawn against this one's solved topology, renamed, when the case's own
    /// solve refuses (or always, when `renames` is not empty).
    donor: Option<Value>,
    renames: Vec<(String, String)>,
}
/// `case`, drawn against `donor`'s solved topology when its own solve
/// refuses (the IR refuses most of these before any drawing).
fn c(name: &str, donor: Value, case: Value) -> Case {
    Case { name: name.into(), case, donor: Some(donor), renames: vec![] }
}
/// `case` is `donor` with `from` spelled `to` everywhere.
fn r(name: &str, donor: Value, from: &str, to: &str) -> Case {
    Case {
        name: name.into(),
        case: rename(&donor, from, to),
        donor: Some(donor),
        renames: vec![(from.into(), to.into())],
    }
}

fn cases() -> Vec<Case> {
    let mut out = vec![c("d2", d2(), d2()), c("d1", d1(), d1())];
    // 1. D2's straps: a tie to each rail, a resistor, one left out.
    for tie in ["VDD_3V3", "VMOTOR", "GND"] {
        let mut v = d2();
        v["pin_devices"][0]["straps"]["AISEN"] = json!({"tie": tie});
        out.push(c(&format!("d2-aisen-tie-{tie}"), d2(), v));
    }
    let mut v = d2();
    v["pin_devices"][0]["straps"]["AISEN"] = json!({"resistor": res("R9", 200)});
    out.push(c("d2-aisen-resistor", d2(), v.clone()));
    v["pin_devices"][0]["straps"]["BISEN"] = json!({"resistor": res("R8", 200)});
    out.push(c("d2-both-isen-resistor", d2(), v));
    let mut v = d2();
    v["pin_devices"][0]["straps"].as_object_mut().unwrap().remove("BISEN");
    out.push(c("d2-bisen-missing", d2(), v));
    // 2. nFAULT: a declared resistor pull, the MCU's pull, left out.
    let mut v = d2();
    v["pin_devices"][0]["signals"][1]["pull"] = json!({"resistor": {"reference": "R9", "part": "CRCW0603_F_K", "ohms": 10000, "to": "VDD_3V3"}, "source": "1319 harness"});
    out.push(c("d2-nfault-resistor", d2(), v));
    let mut v = d2();
    v["pin_devices"][0]["signals"].as_array_mut().unwrap().remove(1);
    out.push(c("d2-nfault-undeclared", d2(), v));
    let mut v = d1();
    v["pin_devices"][0]["signals"].as_array_mut().unwrap().remove(1);
    out.push(c("d1-nfault-undeclared", d1(), v));
    // 3. D1's IPROPI: no reservoir, undeclared; its straps moved.
    let mut v = d1();
    v["pin_devices"][0]["analog"][0]["transfer"]["current_output"]["reservoir"] = Value::Null;
    out.push(c("d1-no-reservoir", d1(), v));
    let mut v = d1();
    v["pin_devices"][0]["analog"] = json!([]);
    out.push(c("d1-ipropi-undeclared", d1(), v));
    let mut v = d1();
    v["pin_devices"][0]["straps"]["IMODE"] = json!({"resistor": res("R9", 20000)});
    out.push(c("d1-imode-resistor", d1(), v));
    let mut v = d1();
    v["pin_devices"][0]["straps"]["VREF"] = json!({"tie": "GND"});
    out.push(c("d1-vref-gnd", d1(), v));
    let mut v = d1();
    v["pin_devices"][0]["straps"].as_object_mut().unwrap().remove("PMODE");
    out.push(c("d1-pmode-missing", d1(), v));
    // 4. Bridge outputs: on GND, on VMOTOR, on a domain, on no connector.
    for (tag, net) in [("gnd", "GND"), ("vm", "VMOTOR"), ("3v3", "VDD_3V3"), ("loose", "MA_LOOSE")] {
        let mut v = d2();
        v["pin_devices"][0]["bridges"][0]["outputs"]["AOUT1"] = json!(net);
        out.push(c(&format!("d2-aout1-{tag}"), d2(), v));
    }
    let mut v = d1();
    v["pin_devices"][0]["bridges"][0]["outputs"]["OUT2"] = json!("M1_LOOSE");
    out.push(c("d1-out2-loose", d1(), v));
    // 5. The VM rating's arithmetic: other VM domains.
    for (mv, tol) in [(7400, 37), (5000, 0), (12000, 100), (3300, 50), (10800, 125)] {
        let mut v = d2();
        v["voltage_domains"][1]["nominal_millivolts"] = json!(mv);
        v["voltage_domains"][1]["tolerance_permille"] = json!(tol);
        out.push(c(&format!("d2-vm-{mv}-{tol}"), d2(), v));
    }
    // 6. References that collide with the emitter's numbering, or differ in case.
    for re in ["U1", "U3", "R1", "C1", "C2", "u2", "J1", "J9"] {
        out.push(r(&format!("d2-ref-{re}"), d2(), "U2", re));
    }
    for re in ["U1", "R1", "R2", "C1", "C4"] {
        out.push(r(&format!("d1-ref-{re}"), d1(), "U2", re));
    }
    out.push(r("d1-res-R1", d1(), "R2", "R1"));
    out.push(r("d1-res-C2", d1(), "C1", "C2"));
    out.push(r("d1-pull-R4", d1(), "R3", "R4"));
    // 7. A net the specification spells as the pin device's own net.
    for (from, to) in [("MA_OUT1", "U2_VINT"), ("MA_OUT1", "U2_VCP"), ("MA_OUT1", "u2_vint"), ("DRV_NFAULT", "U2_VCP"), ("MB_OUT2", "U2_AISEN")] {
        out.push(r(&format!("d2-own-{from}-{to}"), d2(), from, to));
    }
    for (from, to) in [("M1_OUT1", "U2_VCP"), ("M1_OUT1", "U2_CPH"), ("M1_OUT2", "U2_CPL"), ("DRV_IPROPI", "U2_CPH")] {
        out.push(r(&format!("d1-own-{from}-{to}"), d1(), from, to));
    }
    let mut v = d1();
    v["pin_devices"][0]["straps"]["IMODE"] = json!({"resistor": res("R9", 20000)});
    out.push(r("d1-own-imode", v, "M1_OUT1", "U2_IMODE"));
    // 8. Two pin devices (a second DRV8833 as U5 on its own nets and connectors).
    let mut v = d2();
    let mut u5 = rename(&v["pin_devices"][0], "U2", "U5");
    for n in ["DRV_NSLEEP", "DRV_NFAULT", "A_IN1", "A_IN2", "B_IN1", "B_IN2", "MA_OUT1", "MA_OUT2", "MB_OUT1", "MB_OUT2"] {
        u5 = rename(&u5, n, &format!("{n}_B"));
    }
    u5["bridges"][0]["id"] = json!("MC");
    u5["bridges"][1]["id"] = json!("MD");
    v["pin_devices"].as_array_mut().unwrap().push(u5);
    let mut j7 = v["connectors"][4].clone();
    j7 = rename(&rename(&rename(&j7, "J5", "J7"), "MA_OUT1", "MA_OUT1_B"), "MA_OUT2", "MA_OUT2_B");
    let mut j8 = v["connectors"][5].clone();
    j8 = rename(&rename(&rename(&j8, "J6", "J8"), "MB_OUT1", "MB_OUT1_B"), "MB_OUT2", "MB_OUT2_B");
    v["connectors"].as_array_mut().unwrap().extend([j7, j8]);
    out.push(c("d2-two", d2(), v.clone()));
    let mut w = v.clone();
    w["pin_devices"].as_array_mut().unwrap().reverse();
    out.push(c("d2-two-reversed", d2(), w));
    // 9. Pin devices beside bus devices: Sentinel-01's buses and devices on D2.
    let m = load("examples/sentinel-01-minimal.json");
    let mut v = rename(&d2(), "U2", "U9");
    for k in ["i2c_buses", "spi_buses", "i2c_devices", "spi_devices", "net_constraints"] {
        v[k] = m[k].clone();
    }
    v["accept_lifecycle"] = m["accept_lifecycle"].clone();
    out.push(c("d2-with-bus-devices", d2(), v.clone()));
    let mut w = v.clone();
    w["i2c_devices"] = json!([{"reference": "U3", "part": "BME280", "supply": "VDD_3V3", "bus": m["i2c_buses"][0]["id"], "address": 0x77, "pin_capacitance": null},
        {"reference": "U5", "part": "TMP117", "supply": "VDD_3V3", "bus": m["i2c_buses"][0]["id"], "address": 0x4B, "pin_capacitance": null}]);
    out.push(c("d2-with-bus-devices-2", d2(), w));
    // 10. Kind crossing: a bus part as a pin device, a pin part (or a near
    // spelling) as a pin device, each drawn on D2's solved topology.
    for p in ["BME280", "W25Q64JVxIQ", "ICP10111", "drv8833pwpr", "DRV8833", "DRV8876PWPR", "FOO"] {
        out.push(r(&format!("d2-part-{p}"), d2(), "DRV8833PWPR", p));
    }
    out.push(r("d1-part-DRV8833PWPR", d1(), "DRV8876PWPR", "DRV8833PWPR"));
    // A pin part on the I2C bus and on the SPI bus: drawn against Sentinel-01
    // minimal's own solve, the device's part renamed.
    let i2c_part = m["i2c_devices"][0]["part"].as_str().unwrap().to_owned();
    let spi_part = m["spi_devices"][0]["part"].as_str().unwrap().to_owned();
    for p in ["DRV8833PWPR", "DRV8876PWPR"] {
        out.push(r(&format!("m-i2c-{p}"), m.clone(), &i2c_part, p));
        out.push(r(&format!("m-spi-{p}"), m.clone(), &spi_part, p));
    }
    let pw = load("crates/aethelgard-ir/tests/fixtures/sentinel-02-power.json");
    // 11. The adversarial reviewer's inputs (§5): which refusal wins.
    for (tag, base) in [("d1", d1()), ("d2", d2())] {
        let mut v = base.clone();
        v["pin_devices"][0]["supplies"]["VM"] = json!("VFOO");
        out.push(c(&format!("{tag}-vm-not-a-domain"), base.clone(), v));
    }
    let mut v = d2();
    v["pin_devices"][0]["straps"]["FOO"] = json!({"tie": "GND"});
    out.push(c("d2-strap-unknown-pin", d2(), v.clone()));
    v["pin_devices"][0]["bridges"][0]["outputs"]["AOUT1"] = json!("MA_LOOSE");
    out.push(c("d2-strap-unknown-pin-and-loose", d2(), v));
    let mut v = d2();
    v["pin_devices"][0]["signals"].as_array_mut().unwrap().push(json!({"device_pin": "AISEN", "net": "DRV_X", "pin": null, "direction": "output", "initial": "low"}));
    out.push(c("d2-aisen-two-connections", d2(), v));
    let mut v = d1();
    v["pin_devices"][0]["analog"][0]["net"] = json!("DRV_IPROPX");
    out.push(c("d1-analog-unsolved", d1(), v.clone()));
    v["pin_devices"][0]["bridges"][0]["outputs"]["OUT2"] = json!("M1_LOOSE");
    out.push(c("d1-analog-unsolved-and-loose", d1(), v));
    // Pin device, I2C and SPI devices on one board, a declared resistor
    // spelled like an emitter number in the other case.
    let mut mixed = rename(&d2(), "U2", "U9");
    for k in ["i2c_buses", "spi_buses", "i2c_devices", "spi_devices", "net_constraints"] {
        mixed[k] = m[k].clone();
    }
    mixed["accept_lifecycle"] = m["accept_lifecycle"].clone();
    mixed["i2c_devices"] = json!([{"reference": "U2", "part": "BMP581", "supply": "VDD_3V3", "bus": "SENSORS", "address": 0x47, "pin_capacitance": null},
        {"reference": "U3", "part": "TMP117", "supply": "VDD_3V3", "bus": "SENSORS", "address": 0x4A, "pin_capacitance": null}]);
    out.push(c("mixed", mixed.clone(), mixed.clone()));
    for re in ["r5", "c7", "R2", "C3"] {
        let mut v = mixed.clone();
        v["pin_devices"][0]["signals"][1]["pull"] = json!({"resistor": {"reference": re, "part": "CRCW0603_F_K", "ohms": 10000, "to": "VDD_3V3"}, "source": "1319 harness"});
        out.push(c(&format!("mixed-pull-{re}"), mixed.clone(), v));
    }
    // Two I2C buses: TMP117s on the second, ADD0 on its own lines.
    let mut v = m.clone();
    let mut aux = rename(&rename(&m["i2c_buses"][0], "I2C_SDA", "AUX_SDA"), "I2C_SCL", "AUX_SCL");
    aux["id"] = json!("AUX");
    v["i2c_buses"].as_array_mut().unwrap().push(aux);
    for (f, t) in [("I2C_SDA", "AUX_SDA"), ("I2C_SCL", "AUX_SCL")] {
        let k = v["net_constraints"].as_array().unwrap().iter().find(|n| n["net_name"] == f).unwrap().clone();
        v["net_constraints"].as_array_mut().unwrap().push(rename(&k, f, t));
    }
    v["i2c_devices"] = json!([{"reference": "U2", "part": "TMP117", "supply": "VDD_3V3", "bus": "AUX", "address": 0x4A, "pin_capacitance": null},
        {"reference": "U3", "part": "TMP117", "supply": "VDD_3V3", "bus": "AUX", "address": 0x4B, "pin_capacitance": null},
        {"reference": "U5", "part": "BME280", "supply": "VDD_3V3", "bus": "SENSORS", "address": 0x77, "pin_capacitance": null}]);
    out.push(c("m-two-i2c-buses", v.clone(), v));
    // BMP581 on the power fixture, its own net spelled by the specification,
    // crossed with an unsupported address.
    let mut v = pw.clone();
    v["i2c_devices"][0]["part"] = json!("BMP581");
    v["i2c_devices"][0]["address"] = json!(0x46);
    out.push(r("p-bmp581-own-sdo", v.clone(), "I2C_SDA", "U2_SDO"));
    let mut w = v.clone();
    w["i2c_devices"][0]["address"] = json!(0x10);
    let mut k = r("p-bmp581-own-sdo-badaddr", v, "I2C_SDA", "U2_SDO");
    k.case = rename(&w, "I2C_SDA", "U2_SDO");
    out.push(k);
    // And behind the crossing on the power fixture.
    let imu = pw["spi_devices"][1]["part"].as_str().unwrap().to_owned();
    out.push(r("p-imu-DRV8833PWPR", pw.clone(), &imu, "DRV8833PWPR"));
    out
}

#[test]
fn d1319() {
    let dir = PathBuf::from(std::env::var("D4OUT").expect("D4OUT"));
    std::fs::create_dir_all(&dir).unwrap();
    let mut n = 0;
    for k in cases() {
        n += 1;
        let d = dir.join(&k.name);
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("spec.json"), k.case.to_string()).unwrap();
        let mut s = String::new();
        let spec: Specification = match serde_json::from_value(k.case.clone()) {
            Ok(x) => x,
            Err(e) => {
                writeln!(s, "parse: {e}").unwrap();
                std::fs::write(d.join("summary.txt"), s).unwrap();
                continue;
            }
        };
        let pins = aethelgard_svd_hal::pin_table(&spec.mcu.part).unwrap();
        let valid = spec.validate();
        writeln!(s, "validate: {:?}", valid.as_ref().map_err(|e| (e.kind(), e.to_string()))).unwrap();
        let own = aethelgard_ir::solve(&spec, &pins, &"0".repeat(64));
        writeln!(s, "solve(own): {}", match &own { Ok(_) => "ok".to_owned(), Err(e) => format!("{e}") }).unwrap();
        let solved = match (&own, &k.donor) {
            (Ok(x), _) if k.renames.is_empty() => x.clone(),
            (_, Some(donor)) => {
                let ds: Specification = serde_json::from_value(donor.clone()).unwrap();
                match aethelgard_ir::solve(&ds, &pins, &"0".repeat(64)) {
                    Ok(sd) => {
                        let mut t = serde_json::to_string(&sd).unwrap();
                        for (f, to) in &k.renames {
                            t = t.replace(&format!("\"{f}\""), &format!("\"{to}\""));
                        }
                        writeln!(s, "solve(donor): ok").unwrap();
                        serde_json::from_str(&t).unwrap()
                    }
                    Err(e) => {
                        writeln!(s, "solve(donor): {e}").unwrap();
                        std::fs::write(d.join("summary.txt"), s).unwrap();
                        continue;
                    }
                }
            }
            _ => {
                std::fs::write(d.join("summary.txt"), s).unwrap();
                continue;
            }
        };
        let drawn = match Drawing::of(&spec, &solved, &pins) {
            Ok(dr) => {
                writeln!(s, "draw: Ok").unwrap();
                writeln!(s, "drawn: {:?}", dr.drawn()).unwrap();
                writeln!(s, "nets: {:?}", dr.drawn_nets()).unwrap();
                writeln!(s, "anchored: {:?}", dr.anchored()).unwrap();
                std::fs::write(d.join("parts.dump"), dr.dump()).unwrap();
                true
            }
            Err(e) => {
                writeln!(s, "draw: Err {} | {e:?} | {e}", e.kind()).unwrap();
                false
            }
        };
        // Emitted whenever the drawing passed, on whichever topology it was
        // drawn against: the bytes of the board and the sheet, not only the parts.
        if drawn {
            match schematic::emit(&spec, &solved, &pins) {
                Ok(em) => {
                    writeln!(s, "emit: Ok ladder {:?} notes {:?}", em.ladder, em.notes).unwrap();
                    std::fs::write(d.join("out.kicad_sch"), em.schematic).unwrap();
                    std::fs::write(d.join("out.bom.csv"), em.bom).unwrap();
                    std::fs::write(d.join("out.kicad_pcb"), em.board).unwrap();
                }
                Err(e) => writeln!(s, "emit: Err {} | {e:?} | {e}", e.kind()).unwrap(),
            }
        }
        std::fs::write(d.join("summary.txt"), s).unwrap();
        eprintln!("done {}", k.name);
    }
    eprintln!("cases {n}");
}
