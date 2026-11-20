//! AET-WO-1317 spike driver (scratch copy only).
#[allow(dead_code)]
#[path = "../tests/common/rename.rs"]
mod rename;

use std::time::Instant;

use aethelgard_eda::spike1317::{self, Knobs};
use aethelgard_ir::{PinTable, SolvedTopology, Specification};
use serde_json::{Value, json};

const FIXTURE: &str = "crates/aethelgard-cli/tests/fixtures/sentinel-03.json";

fn fixture() -> Value {
    serde_json::from_str(&std::fs::read_to_string(FIXTURE).unwrap()).unwrap()
}

/// 03d89554's fixture hunk, reversed onto today's fixture.
fn with_u2(mut v: Value) -> Value {
    let u2 = json!({"reference": "U2", "part": "BME280", "supply": "VDD_3V3", "bus": "SENSORS", "address": 118, "pin_capacitance": null});
    v["i2c_devices"].as_array_mut().unwrap().insert(0, u2);
    let loads = v["power"]["loads"].as_array_mut().unwrap();
    let mut flash = 0;
    let mut at = None;
    for (k, l) in loads.iter_mut().enumerate() {
        if l["source"].as_str().unwrap().contains("W25Q64JV rev J 9.4 ILI") {
            l["reference"] = json!(if flash == 0 { "R7" } else { "R8" });
            flash += 1;
        }
        if l["reference"] == "R20" {
            at = Some(k + 1);
        }
    }
    assert_eq!(flash, 2);
    loads.insert(at.unwrap(), json!({"domain": "VDD_3V3", "milliamps": 1, "source": "Bosch BST-BME280-DS001-24 rev 1.24 Table 1 p. 8: IDDP current during pressure measurement 714 uA (Max value at -40 C); rounded up", "reference": "U2"}));
    v
}

/// outline_tier1's turned-IMU edit.
fn turned(mut v: Value, x: &str) -> Value {
    v["board"]["forward"] = json!("top");
    for key in ["spi_devices", "i2c_devices"] {
        for d in v[key].as_array_mut().unwrap() {
            if d["reference"] == "U5" {
                d["orientation"] = json!({"x": x, "z": "up"});
            }
        }
    }
    v
}

fn variant(name: &str) -> Value {
    let (u2, name) = match name.strip_prefix("u2-") {
        Some(rest) => (true, rest),
        None => (false, name),
    };
    let mut v = fixture();
    if u2 {
        v = with_u2(v);
    }
    match name {
        "base" => v,
        n if n.starts_with("turn-") => turned(v, &n[5..]),
        n if n.starts_with("size-") => rename::sized(&v, n[5..].parse().unwrap()),
        _ => panic!("variant {name}"),
    }
}

fn solved(v: &Value) -> (Specification, SolvedTopology, PinTable) {
    let spec: Specification = serde_json::from_value(v.clone()).unwrap();
    spec.validate().unwrap();
    let table = aethelgard_svd_hal::pin_table(&spec.mcu.part).unwrap();
    let solved = aethelgard_ir::solve(&spec, &table, &"0".repeat(64)).unwrap();
    (spec, solved, table)
}

fn mm(nm: i64) -> String {
    format!("{:.3}", nm as f64 / 1e6)
}

fn knobs(s: &str) -> Knobs {
    Knobs {
        no_k: s.contains('k'),
        no_m: s.contains('m'),
        no_s: s.contains('s'),
        order: None,
        salt: 0,
        rounds: None,
    }
}

/// FALLBACK="salt:1-8" or "rounds:0-6" or "salt:1-8,rounds:0-6"
fn alts() -> (Vec<Knobs>, Vec<String>) {
    let mut ks = Vec::new();
    let mut ns = Vec::new();
    let Ok(spec) = std::env::var("FALLBACK") else { return (ks, ns) };
    for part in spec.split(',') {
        let (what, range) = part.split_once(':').unwrap();
        let (a, b) = range.split_once('-').unwrap();
        for v in a.parse::<u64>().unwrap()..=b.parse::<u64>().unwrap() {
            let mut k = knobs("");
            match what {
                "salt" => k.salt = v,
                "rounds" => k.rounds = Some(v as usize),
                _ => panic!(),
            }
            ks.push(k);
            ns.push(format!("{what}{v}"));
        }
    }
    (ks, ns)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args[0].as_str() {
        // outcome of each variant, full board
        "outcome" => {
            for name in &args[1..] {
                let v = variant(name);
                let (spec, sol, table) = solved(&v);
                let c = spike1317::run(&spec, &sol, &table, &[], false, &Knobs::default());
                println!("{name}\t{}\t{}\t{}\t{}", c.class, mm(c.w_nm), mm(c.h_nm), c.message);
            }
        }
        // trace <variant> <remove,comma> <knobs> : prints identities + trace
        "trace" => {
            let v = variant(&args[1]);
            let (spec, sol, table) = solved(&v);
            let remove: Vec<String> = args.get(2).map(|s| s.split(',').filter(|x| !x.is_empty() && *x != "-").map(String::from).collect()).unwrap_or_default();
            let mut k = knobs(args.get(3).map_or("", |s| s.as_str()));
            if let Some(path) = args.get(4) {
                k.order = Some(std::fs::read_to_string(path).unwrap().split_whitespace().map(String::from).collect());
            }
            for (r, id) in spike1317::identities(&spec, &sol, &table) {
                println!("id {r} {id}");
            }
            let c = spike1317::run(&spec, &sol, &table, &remove, true, &k);
            for l in &c.trace {
                println!("{l}");
            }
            println!("result {} {} {} {}", c.class, mm(c.w_nm), mm(c.h_nm), c.message);
        }
        // sweep <knobs> <variant...> : one TSV line per removal
        "sweep" => {
            let k = knobs(&args[1]);
            println!("variant\tknobs\tremoved\tclass\tw_mm\th_mm\tfull_class\tfull_w\tfull_h\tms");
            let closure = std::env::var("CLOSURE").is_ok();
            for name in &args[2..] {
                let v = variant(name);
                let (spec, sol, table) = solved(&v);
                let t0 = Instant::now();
                let full = spike1317::run(&spec, &sol, &table, &[], false, &k);
                let ms = t0.elapsed().as_millis();
                println!("{name}\t{}\t-\t{}\t{}\t{}\t{}\t{}\t{}\t{ms}", &args[1], full.class, mm(full.w_nm), mm(full.h_nm), full.class, mm(full.w_nm), mm(full.h_nm));
                for g in spike1317::removals(&spec, &sol, &table, closure) {
                    let t0 = Instant::now();
                    let c = spike1317::run(&spec, &sol, &table, &g, false, &k);
                    let ms = t0.elapsed().as_millis();
                    println!("{name}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{ms}", &args[1], g.join(","), c.class, mm(c.w_nm), mm(c.h_nm), full.class, mm(full.w_nm), mm(full.h_nm));
                }
            }
        }
        // specsweep <knobs> <variant...>: remove each spec entry
        "specsweep" => {
            let k = knobs(&args[1]);
            println!("variant\tknobs\tremoved\tclass\tw_mm\th_mm\tfull_class\tfull_w\tfull_h\tms");
            for name in &args[2..] {
                let v = variant(name);
                let (spec, sol, table) = solved(&v);
                let (ks, ns) = alts();
                let (full, _) = spike1317::run_fallback(&spec, &sol, &table, &[], &k, &ks, &ns);
                println!("{name}\t{}\t-\t{}\t{}\t{}\t{}\t{}\t{}\t0", &args[1], full.class, mm(full.w_nm), mm(full.h_nm), full.class, mm(full.w_nm), mm(full.h_nm));
                for key in ["i2c_devices", "spi_devices", "connectors", "test_points"] {
                    let n = v[key].as_array().map_or(0, Vec::len);
                    for idx in 0..n {
                        let mut r = v.clone();
                        let gone = r[key].as_array_mut().unwrap().remove(idx);
                        let reference = gone["reference"].as_str().unwrap().to_owned();
                        if let Some(loads) = r["power"]["loads"].as_array_mut() {
                            loads.retain(|l| l["reference"] != reference.as_str());
                        }
                        let t0 = Instant::now();
                        let (class, w, h) = match serde_json::from_value::<Specification>(r.clone()) {
                            Err(e) => (format!("parse:{e}"), 0, 0),
                            Ok(spec) => match spec.validate() {
                                Err(e) => (format!("validate:{}", e.to_string().chars().take(60).collect::<String>()), 0, 0),
                                Ok(()) => {
                                    let table = aethelgard_svd_hal::pin_table(&spec.mcu.part).unwrap();
                                    match aethelgard_ir::solve(&spec, &table, &"0".repeat(64)) {
                                        Err(e) => (format!("solve:{}", format!("{e:?}").chars().take(60).collect::<String>()), 0, 0),
                                        Ok(sol) => {
                                            let (ks, ns) = alts();
                                            let (c, _) = spike1317::run_fallback(&spec, &sol, &table, &[], &k, &ks, &ns);
                                            (c.class, c.w_nm, c.h_nm)
                                        }
                                    }
                                }
                            },
                        };
                        let ms = t0.elapsed().as_millis();
                        println!("{name}\t{}\t{key}:{reference}\t{class}\t{}\t{}\t{}\t{}\t{}\t{ms}", &args[1], mm(w), mm(h), full.class, mm(full.w_nm), mm(full.h_nm));
                    }
                }
            }
        }
        // fbsweep <variant...>: parts sweep (CLOSURE env) with the fallback
        "fbsweep" => {
            let (ks, ns) = alts();
            let closure = std::env::var("CLOSURE").is_ok();
            let k = knobs("");
            println!("variant\tknobs\tremoved\tclass\tw_mm\th_mm\tfull_class\tfull_w\tfull_h\talt");
            for name in &args[1..] {
                let v = variant(name);
                let (spec, sol, table) = solved(&v);
                let (full, fa) = spike1317::run_fallback(&spec, &sol, &table, &[], &k, &ks, &ns);
                println!("{name}\tfb\t-\t{}\t{}\t{}\t{}\t{}\t{}\t{fa}", full.class, mm(full.w_nm), mm(full.h_nm), full.class, mm(full.w_nm), mm(full.h_nm));
                for g in spike1317::removals(&spec, &sol, &table, closure) {
                    let (c, a) = spike1317::run_fallback(&spec, &sol, &table, &g, &k, &ks, &ns);
                    println!("{name}\tfb\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{a}", g.join(","), c.class, mm(c.w_nm), mm(c.h_nm), full.class, mm(full.w_nm), mm(full.h_nm));
                }
            }
        }
        _ => panic!("mode"),
    }
}
