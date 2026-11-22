//! D4 review harness: drives schematic::emit and Drawing::of on inputs the
//! committed specifications do not cover; writes every result to $D4OUT.
//! Each case has a donor that solves; the case's own spec is drawn against the
//! donor's solved topology (with net renames applied), so `design` is reached
//! even where validate/solve would refuse the case.
use aethelgard_eda::schematic::{self, Drawing};
use aethelgard_ir::parts::{self, Lifecycle};
use aethelgard_ir::{SolvedTopology, Specification};
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
fn lifecycle(mut v: Value) -> Value {
    let t = parts::table().unwrap();
    let mut acc: Vec<Value> = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for k in ["i2c_devices", "spi_devices"] {
        for d in v[k].as_array().cloned().unwrap_or_default() {
            let p = d["part"].as_str().unwrap().to_owned();
            if let Some(row) = t.get(&p) {
                if row.lifecycle != Lifecycle::Active && seen.insert(p.clone()) {
                    acc.push(json!({"part": p, "status": row.lifecycle.name(), "statement": "d4 review"}));
                }
            }
        }
    }
    // keep entries for parts outside the device lists (power parts)
    for e in v["accept_lifecycle"].as_array().cloned().unwrap_or_default() {
        let p = e["part"].as_str().unwrap();
        let dev = ["i2c_devices", "spi_devices"].iter().any(|k| {
            v[*k].as_array().map_or(false, |a| a.iter().any(|d| d["part"] == p))
        });
        let is_dev_part = ["BME280","BMP581","TMP117","W25Q64JVxIQ","ICM20948","ICM42688P","ICP10111"].contains(&p);
        if !is_dev_part || dev {
            if seen.insert(p.to_owned()) { acc.push(e); }
        }
    }
    v["accept_lifecycle"] = json!(acc);
    v
}
fn to_1v8(v: &Value) -> Value {
    let mut v: Value = serde_json::from_str(&v.to_string().replace("VDD_3V3", "VDD_1V8")).unwrap();
    v["voltage_domains"][0]["nominal_millivolts"] = json!(1800);
    v["mcu"]["bor_level"] = json!(0);
    v
}
const ALL: [&str; 9] = [
    "BME280", "BMP581", "TMP117", "W25Q64JVxIQ", "ICM20948", "ICM42688P", "ICP10111", "FOO", "bme280",
];
fn addrs(part: &str) -> Vec<u64> {
    match part {
        "BME280" => vec![0x76, 0x77],
        "BMP581" => vec![0x46, 0x47],
        "TMP117" => vec![0x48, 0x49, 0x4A, 0x4B],
        "ICP10111" => vec![0x63],
        _ => vec![0x48],
    }
}
fn i2c(r: &str, p: &str, a: u64) -> Value {
    json!({"reference": r, "part": p, "supply": "VDD_3V3", "bus": "SENSORS", "address": a, "pin_capacitance": null})
}
fn spi(r: &str, p: &str, sup: &str, bus: &str, cs: &str) -> Value {
    json!({"reference": r, "part": p, "supply": sup, "bus": bus, "cs": {"net": cs, "pin": null}, "pin_capacitance": null})
}
/// A donor for `v`: I2C devices BMP581/BME280 at their addresses on the bus's
/// domain; SPI devices ICM20948 on 1V8 or the IMU bus, else W25Q64JVxIQ.
fn donor_of(v: &Value) -> Value {
    let mut d = v.clone();
    let buses = d["i2c_buses"].clone();
    if let Some(a) = d["i2c_devices"].as_array_mut() {
        let pool = [("BMP581", 0x46), ("BMP581", 0x47), ("BME280", 0x76), ("BME280", 0x77)];
        for (i, x) in a.iter_mut().enumerate() {
            x["part"] = json!(pool[i].0);
            x["address"] = json!(pool[i].1);
            if let Some(b) = buses.as_array().and_then(|b| b.iter().find(|b| b["id"] == x["bus"])) {
                x["supply"] = b["domain"].clone();
            }
        }
    }
    if let Some(a) = d["spi_devices"].as_array_mut() {
        for x in a.iter_mut() {
            let p = if x["supply"] == "VDD_1V8" || x["bus"] == "IMU" { "ICM20948" } else { "W25Q64JVxIQ" };
            x["part"] = json!(p);
            x["pin_capacitance"] = Value::Null;
        }
    }
    lifecycle(capped(d))
}
fn capped(mut v: Value) -> Value {
    if let Some(a) = v["spi_devices"].as_array_mut() {
        for x in a.iter_mut().filter(|x| x["part"] == "ICM20948" || x["part"] == "ICM42688P") {
            x["pin_capacitance"] = json!({"declared": {"femtofarads": 10000, "condition": "d4 review", "source": "d4 review"}});
        }
    }
    v
}
struct Case { name: String, donor: Value, case: Value, renames: Vec<(String, String)> }
fn c(name: String, case: Value) -> Case {
    let case = lifecycle(capped(case));
    Case { name, donor: donor_of(&case), case, renames: vec![] }
}
/// `case` with reference `r` in it; the donor spells it `U8`.
fn cref(name: String, case: Value, r: &str) -> Case {
    let case = lifecycle(capped(case));
    let donor = rename(&donor_of(&case), r, "U18");
    Case { name, donor, case, renames: vec![("U18".into(), r.into())] }
}

fn cases() -> Vec<Case> {
    let m = load("examples/sentinel-01-minimal.json");
    let mut m = m;
    m["spi_buses"][0]["speed_hz"] = json!(4000000);
    let mut m18 = to_1v8(&m);
    m18["i2c_devices"] = json!([i2c("U2","BMP581",0x46), i2c("U3","BME280",0x76)]);
    m18["i2c_devices"][0]["supply"] = json!("VDD_1V8");
    m18["i2c_devices"][1]["supply"] = json!("VDD_1V8");
    m18["spi_devices"] = json!([spi("U4","ICM42688P","VDD_1V8","FLASH","FLASH_CS")]);
    let p = load("crates/aethelgard-ir/tests/fixtures/sentinel-02-power.json");
    let mut out = Vec::new();
    // 1. each name on the I2C bus at each address (+ off-table), alone; 3V3 and 1V8 boards
    for (tag, base) in [("m", &m), ("m18", &m18)] {
        for part in ALL {
            let mut list = addrs(part);
            list.extend([0x10, 0x76, 0x4B, 0x63]);
            list.sort(); list.dedup();
            for a in list {
                let mut v = base.clone();
                let sup = v["voltage_domains"][0]["id"].clone();
                let mut d = i2c("U2", part, a); d["supply"] = sup;
                v["i2c_devices"] = json!([d]);
                out.push(c(format!("{tag}-i2c-{part}-{a:02x}"), v));
            }
        }
    }
    // 2. two (and four) of one part; reversed declaration order
    for part in ["BME280", "BMP581", "TMP117"] {
        let a = addrs(part);
        let mut v = m.clone();
        v["i2c_devices"] = json!([i2c("U2", part, a[0]), i2c("U3", part, a[1])]);
        out.push(c(format!("m-i2c-two-{part}"), v.clone()));
        v["i2c_devices"] = json!([i2c("U3", part, a[1]), i2c("U2", part, a[0])]);
        out.push(c(format!("m-i2c-two-rev-{part}"), v.clone()));
        v["i2c_devices"] = json!([i2c("U2", part, a[0]), i2c("U3", part, a[0])]);
        out.push(c(format!("m-i2c-two-same-addr-{part}"), v));
    }
    let mut v = m.clone();
    v["i2c_devices"] = json!([i2c("U2","TMP117",0x48), i2c("U3","TMP117",0x49), i2c("U5","TMP117",0x4A), i2c("U6","TMP117",0x4B)]);
    out.push(c("m-i2c-four-TMP117".into(), v));
    let mut v = m.clone();
    v["i2c_devices"] = json!([i2c("U2","BMP581",0x47), i2c("U3","BME280",0x77), i2c("U5","TMP117",0x4B), i2c("U6","BMP581",0x46)]);
    out.push(c("m-i2c-mixed".into(), v));
    // unknown part after a good one, and before: which refusal / what was drawn
    let mut v = m.clone();
    v["i2c_devices"] = json!([i2c("U2","BME280",0x77), i2c("U3","FOO",0x48), i2c("U5","TMP117",0x10)]);
    out.push(c("m-i2c-good-unknown-badaddr".into(), v));
    let mut v = m.clone();
    v["i2c_devices"] = json!([i2c("U5","TMP117",0x10), i2c("U3","FOO",0x48)]);
    out.push(c("m-i2c-badaddr-unknown".into(), v));
    // 3. references colliding with emitter-numbered ones
    for r in ["R1", "R3", "R4", "C1", "C2", "C3", "U1", "U2", "r3", "c1", "u1"] {
        for part in ["BME280", "BMP581", "TMP117"] {
            let mut v = m.clone();
            v["i2c_devices"] = json!([i2c(r, part, addrs(part)[1]), i2c("U3", "TMP117", 0x4A)]);
            if r == "U2" { v["i2c_devices"][1]["reference"] = json!("U9"); }
            out.push(cref(format!("m-ref-i2c-{part}-{r}"), v, r));
        }
        for part in ["W25Q64JVxIQ", "ICM42688P"] {
            let mut v = m.clone();
            v["spi_devices"] = json!([spi(r, part, "VDD_3V3", "FLASH", "FLASH_CS")]);
            out.push(cref(format!("m-ref-spi-{part}-{r}"), v, r));
        }
        let mut v = m18.clone();
        v["spi_devices"] = json!([spi(r, "ICM20948", "VDD_1V8", "FLASH", "FLASH_CS")]);
        out.push(cref(format!("m18-ref-spi-ICM20948-{r}"), v, r));
    }
    // 4. own-net collisions (renamed after solve), and refusal order
    for (part, a, net) in [("BME280",0x76,"U2_SDO"),("BME280",0x10,"U2_SDO"),("BMP581",0x47,"U2_SDO"),("BMP581",0x10,"u2_sdo"),("TMP117",0x10,"U2_SDO"),("TMP117",0x49,"U2_SDO"),("FOO",0x49,"U2_SDO")] {
        let mut v = m.clone();
        v["i2c_devices"] = json!([i2c("U2", part, a)]);
        let mut k = c(format!("m-own-{part}-{a:02x}-{net}"), v);
        k.case = rename(&k.case, "I2C_SDA", net);
        k.renames.push(("I2C_SDA".into(), net.into()));
        out.push(k);
    }
    for (tag, base, sup) in [("m", &m, "VDD_3V3"), ("m18", &m18, "VDD_1V8")] {
        for (part, net) in [("W25Q64JVxIQ","U4_WP"),("W25Q64JVxIQ","U4_HOLD"),("W25Q64JVxIQ","u4_hold"),("ICM20948","U4_REGOUT"),("ICM20948","U4_wp"),("ICM42688P","U4_REGOUT"),("FOO","U4_WP")] {
            let mut v = base.clone();
            v["spi_devices"] = json!([spi("U4", part, sup, "FLASH", "FLASH_CS")]);
            let mut k = c(format!("{tag}-own-{part}-{net}"), v);
            k.case = rename(&k.case, "SPI_MOSI", net);
            k.renames.push(("SPI_MOSI".into(), net.into()));
            out.push(k);
        }
    }
    // 5. each name on the SPI bus; two of each; mixed orders; 3V3 and 1V8
    for (tag, base, sup) in [("m", &m, "VDD_3V3"), ("m18", &m18, "VDD_1V8")] {
        for part in ALL {
            let mut v = base.clone();
            v["spi_devices"] = json!([spi("U4", part, sup, "FLASH", "FLASH_CS")]);
            out.push(c(format!("{tag}-spi-{part}"), v.clone()));
            v["spi_devices"] = json!([spi("U4", part, sup, "FLASH", "FLASH_CS"), spi("U5", part, sup, "FLASH", "FLASH_CS2")]);
            out.push(c(format!("{tag}-spi-two-{part}"), v));
        }
        let mut v = base.clone();
        v["spi_devices"] = json!([spi("U5","ICM42688P",sup,"FLASH","FLASH_CS2"), spi("U4","ICM20948",sup,"FLASH","FLASH_CS"), spi("U6","W25Q64JVxIQ",sup,"FLASH","FLASH_CS3")]);
        out.push(c(format!("{tag}-spi-mixed"), v));
        let mut v = base.clone();
        v["spi_devices"] = json!([spi("U5","W25Q64JVxIQ",sup,"FLASH","FLASH_CS2"), spi("U4","FOO",sup,"FLASH","FLASH_CS"), spi("U6","BME280",sup,"FLASH","FLASH_CS3")]);
        out.push(c(format!("{tag}-spi-good-unknown-i2c"), v));
    }
    // 6. behind the crossing (power fixture): U5 on the IMU bus, each name; U4 each name
    for part in ALL {
        let mut v = p.clone();
        v["spi_devices"][1]["part"] = json!(part);
        out.push(c(format!("p-imu-{part}"), v));
        let mut v = p.clone();
        v["spi_devices"][0]["part"] = json!(part);
        out.push(c(format!("p-flash-{part}"), v));
        let mut v = p.clone();
        v["spi_devices"][0]["part"] = json!(part);
        v["spi_devices"][1]["part"] = json!(part);
        out.push(c(format!("p-both-{part}"), v));
    }
    // CS and bus nets renamed to collide with REGOUT/WP behind the crossing
    for (part, from, to) in [("ICM20948","IMU_MOSI","U5_REGOUT"),("W25Q64JVxIQ","IMU_CS","U5_WP"),("ICM20948","IMU_CS","U5_REGOUT")] {
        let mut v = p.clone();
        v["spi_devices"][1]["part"] = json!(part);
        let mut k = c(format!("p-own-{part}-{to}-{from}"), v);
        k.case = rename(&k.case, from, to);
        k.renames.push((from.into(), to.into()));
        out.push(k);
    }
    // I2C parts at every address on the power board (U2 swept, U3 TMP117 kept)
    for part in ["BME280", "BMP581", "TMP117", "ICP10111"] {
        for a in addrs(part) {
            let mut v = p.clone();
            v["i2c_devices"][0]["part"] = json!(part);
            v["i2c_devices"][0]["address"] = json!(a);
            if part == "TMP117" {
                v["i2c_devices"][1]["address"] = json!(if a == 0x48 { 0x49 } else { 0x48 });
            }
            out.push(c(format!("p-i2c-{part}-{a:02x}"), v));
        }
        let mut v = p.clone();
        v["i2c_devices"][0]["part"] = json!(part);
        v["i2c_devices"][0]["supply"] = json!("VDD_1V8");
        v["i2c_devices"][0]["address"] = json!(addrs(part)[0]);
        out.push(c(format!("p-i2c-1v8-{part}"), v));
    }
    out
}

fn solve(v: &Value) -> Result<(Specification, SolvedTopology), String> {
    let spec: Specification = serde_json::from_value(v.clone()).map_err(|e| format!("parse {e}"))?;
    let pins = aethelgard_svd_hal::pin_table(&spec.mcu.part).unwrap();
    let s = aethelgard_ir::solve(&spec, &pins, &"0".repeat(64)).map_err(|e| format!("solve {e}"))?;
    Ok((spec, s))
}

#[test]
fn d4diff() {
    let dir = PathBuf::from(std::env::var("D4OUT").expect("D4OUT"));
    let only_draw = std::env::var("D4DRAWONLY").is_ok();
    std::fs::create_dir_all(&dir).unwrap();
    for k in cases() {
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
        writeln!(s, "solve(own): {}", if own.is_ok() { "ok".to_owned() } else { format!("{}", own.as_ref().err().unwrap()) }).unwrap();
        let solved = match &own {
            Ok(x) if k.renames.is_empty() => x.clone(),
            _ => match solve(&k.donor) {
                Ok((_, sd)) => {
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
            },
        };
        match Drawing::of(&spec, &solved, &pins) {
            Ok(dr) => {
                writeln!(s, "draw: Ok").unwrap();
                writeln!(s, "drawn: {:?}", dr.drawn()).unwrap();
                writeln!(s, "nets: {:?}", dr.drawn_nets()).unwrap();
                writeln!(s, "anchored: {:?}", dr.anchored()).unwrap();
                std::fs::write(d.join("parts.dump"), dr.dump()).unwrap();
            }
            Err(e) => writeln!(s, "draw: Err {} | {e:?} | {e}", e.kind()).unwrap(),
        }
        if valid.is_ok() && own.is_ok() && !only_draw {
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
}
