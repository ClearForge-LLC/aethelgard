//! AET-WO-4240a §1.1 measurement: per committed specification, validate, solve and every
//! timing verdict on the solved topology; per stored topology, validate. One line each.
use aethelgard_ir::{SolvedTopology, Specification, motor, serial, silicon, vos};
use sha2::Digest;

fn main() {
    let mut args = std::env::args().skip(1);
    let kind = args.next().unwrap();
    for path in args {
        let text = std::fs::read_to_string(&path).unwrap();
        let mut out = serde_json::Map::new();
        out.insert("path".into(), path.clone().into());
        if kind == "spec" {
            let sha = format!("{:x}", sha2::Sha256::digest(text.as_bytes()));
            let spec: Result<Specification, _> = serde_json::from_str(&text);
            let spec = match spec {
                Ok(s) => s,
                Err(e) => {
                    out.insert("parse".into(), e.to_string().into());
                    println!("{}", serde_json::Value::Object(out));
                    continue;
                }
            };
            out.insert("validate".into(), format!("{:?}", spec.validate()).into());
            let table = match aethelgard_svd_hal::pin_table(&spec.mcu.part) { Ok(t) => t, Err(e) => { out.insert("pin_table".into(), format!("{e:?}").into()); println!("{}", serde_json::Value::Object(out)); continue; } };
            match aethelgard_ir::solve(&spec, &table, &sha) {
                Err(e) => {
                    out.insert("solve_err".into(), format!("{e:?}").into());
                }
                Ok(t) => {
                    out.insert("topology".into(), serde_json::to_value(&t).unwrap());
                    out.insert("topology_validate".into(), format!("{:?}", t.validate()).into());
                    let mut v = Vec::new();
                    for (net, m) in &t.motor_outputs {
                        let timer: u32 = t.assignments[net].peripheral[3..].parse().unwrap();
                        v.push(format!("{net}: {:?}", motor::check_silicon(&t.clock, timer, m)));
                    }
                    for (l, p) in &t.serial_ports {
                        v.push(format!("{l}: {:?}", serial::check_silicon(&t.clock, p)));
                    }
                    for (p, c) in &t.spi_controllers {
                        v.push(format!("{p}: {:?}", silicon::check_spi(p, c)));
                    }
                    for (p, c) in &t.i2c_timings {
                        v.push(format!("{p}: {:?}", silicon::check_i2c(p, c)));
                    }
                    v.push(format!("vos: {:?}", vos::judge_generated(&t)));
                    out.insert("verdicts".into(), v.into());
                }
            }
        } else {
            match serde_json::from_str::<SolvedTopology>(&text) {
                Err(e) => {
                    out.insert("parse".into(), e.to_string().into());
                }
                Ok(t) => {
                    out.insert("validate".into(), format!("{:?}", t.validate()).into());
                    out.insert("vos".into(), format!("{:?}", vos::judge_generated(&t)).into());
                }
            }
        }
        println!("{}", serde_json::Value::Object(out));
    }
}
