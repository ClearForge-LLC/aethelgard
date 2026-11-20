//! AET-WO-1317 §5 adversarial driver: spec-level removals of one or two
//! declared entries, and additions of one, placed through the public
//! `schematic::placement` only, so it runs unchanged on `main` and the branch.
//! Copy to crates/aethelgard-cli/examples/adv1317.rs; run from the copy's root.
#[allow(dead_code)]
#[path = "../tests/common/rename.rs"]
mod rename;

use aethelgard_eda::schematic::{self, EmitError};
use aethelgard_ir::Specification;
use serde_json::{Value, json};

const KEYS: [&str; 4] = ["i2c_devices", "spi_devices", "connectors", "test_points"];

fn fixture() -> Value {
    serde_json::from_str(&std::fs::read_to_string("crates/aethelgard-cli/tests/fixtures/sentinel-03.json").unwrap()).unwrap()
}

fn with_u2(mut v: Value) -> Value {
    v["i2c_devices"].as_array_mut().unwrap().insert(0, json!({"reference": "U2", "part": "BME280", "supply": "VDD_3V3", "bus": "SENSORS", "address": 118, "pin_capacitance": null}));
    let loads = v["power"]["loads"].as_array_mut().unwrap();
    let flash: Vec<usize> = (0..loads.len()).filter(|&i| loads[i]["source"].as_str().unwrap().contains("W25Q64JV rev J 9.4 ILI")).collect();
    for (i, r) in flash.into_iter().zip(["R7", "R8"]) {
        loads[i]["reference"] = json!(r);
    }
    let at = loads.iter().position(|l| l["reference"] == "R20").unwrap() + 1;
    loads.insert(at, json!({"domain": "VDD_3V3", "milliamps": 1, "reference": "U2", "source": "Bosch BST-BME280-DS001-24 rev 1.24 Table 1 p. 8: IDDP current during pressure measurement 714 uA (Max value at -40 C); rounded up"}));
    v
}

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
    let v = if u2 { with_u2(fixture()) } else { fixture() };
    match name {
        "base" => v,
        n if n.starts_with("turn-") => turned(v, &n[5..]),
        n if n.starts_with("size-") => rename::sized(&v, n[5..].parse().unwrap()),
        _ => panic!("{name}"),
    }
}

/// (class, w_mm, h_mm)
fn place(v: &Value) -> (String, f64, f64) {
    let Ok(spec) = serde_json::from_value::<Specification>(v.clone()) else { return ("parse".into(), 0.0, 0.0) };
    if spec.validate().is_err() {
        return ("validate".into(), 0.0, 0.0);
    }
    let table = aethelgard_svd_hal::pin_table(&spec.mcu.part).unwrap();
    let Ok(solved) = aethelgard_ir::solve(&spec, &table, &"0".repeat(64)) else { return ("solve".into(), 0.0, 0.0) };
    match schematic::placement(&spec, &solved, &table) {
        Ok(board) => {
            let r = &board[board.find("(gr_rect").unwrap()..];
            let num = |key: &str| -> (f64, f64) {
                let s = &r[r.find(key).unwrap() + key.len()..];
                let mut it = s.split([' ', ')', '\n', '\t']).filter(|t| !t.is_empty());
                (it.next().unwrap().parse().unwrap(), it.next().unwrap().parse().unwrap())
            };
            let ((x0, y0), (x1, y1)) = (num("(start "), num("(end "));
            ("fit".into(), x1 - x0, y1 - y0)
        }
        Err(EmitError::OutlineUnreachable { reached_um_w, reached_um_h, .. }) => {
            ("OutlineUnreachable".into(), reached_um_w as f64 / 1000.0, reached_um_h as f64 / 1000.0)
        }
        Err(e) => (e.kind().to_string(), 0.0, 0.0),
    }
}

fn remove(v: &Value, (key, i): (&str, usize)) -> (Value, String) {
    let mut r = v.clone();
    let gone = r[key].as_array_mut().unwrap().remove(i);
    let reference = gone["reference"].clone();
    if let Some(loads) = r["power"]["loads"].as_array_mut() {
        loads.retain(|l| l["reference"] != reference);
    }
    (r, reference.as_str().unwrap().to_owned())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mode = args[0].as_str();
    println!("variant\tknobs\tremoved\tclass\tw_mm\th_mm\tfull_class\tfull_w\tfull_h");
    for name in &args[1..] {
        let v = variant(name);
        let full = place(&v);
        let entries: Vec<(&str, usize)> = KEYS.iter().flat_map(|&k| (0..v[k].as_array().map_or(0, Vec::len)).map(move |i| (k, i))).collect();
        let mut cases: Vec<(String, Value)> = Vec::new();
        match mode {
            "pairs" => {
                for a in 0..entries.len() {
                    for b in a + 1..entries.len() {
                        // remove the later index first so the earlier stays valid
                        let (ka, ia) = entries[a];
                        let (kb, ib) = entries[b];
                        let (r1, n1) = remove(&v, (kb, ib));
                        let ia = if ka == kb && ia > ib { ia - 1 } else { ia };
                        let (r2, n2) = remove(&r1, (ka, ia));
                        cases.push((format!("{n2}+{n1}"), r2));
                    }
                }
            }
            // add one: U2 (where absent), and a second test point on each test point's net
            "addone" => {
                if !v["i2c_devices"].as_array().unwrap().iter().any(|d| d["part"] == "BME280") {
                    let mut r = v.clone();
                    let u2 = with_u2(fixture())["i2c_devices"][0].clone();
                    r["i2c_devices"].as_array_mut().unwrap().insert(0, u2);
                    cases.push(("+U2".into(), r));
                }
                for (k, tp) in v["test_points"].as_array().unwrap().iter().enumerate() {
                    let mut r = v.clone();
                    let mut t = tp.clone();
                    t["reference"] = json!(format!("TP9{k}"));
                    r["test_points"].as_array_mut().unwrap().push(t);
                    cases.push((format!("+TP9{k}({})", tp["net"].as_str().unwrap()), r));
                }
            }
            _ => panic!("mode"),
        }
        println!("{name}\t{mode}\t-\t{}\t{:.3}\t{:.3}\t{}\t{:.3}\t{:.3}", full.0, full.1, full.2, full.0, full.1, full.2);
        let threads = std::thread::available_parallelism().map_or(4, |n| n.get().min(6));
        let out: std::sync::Mutex<Vec<String>> = std::sync::Mutex::default();
        std::thread::scope(|s| {
            for chunk in cases.chunks(cases.len().div_ceil(threads).max(1)) {
                let (out, full) = (&out, &full);
                s.spawn(move || {
                    for (label, r) in chunk {
                        let c = place(r);
                        let c = if matches!(c.0.as_str(), "validate" | "solve" | "parse") { (format!("{}:", c.0), c.1, c.2) } else { c };
                        out.lock().unwrap().push(format!("{name}\t{mode}\t{label}\t{}\t{:.3}\t{:.3}\t{}\t{:.3}\t{:.3}", c.0, c.1, c.2, full.0, full.1, full.2));
                    }
                });
            }
        });
        let mut out = out.into_inner().unwrap();
        out.sort();
        for l in out {
            println!("{l}");
        }
    }
}
