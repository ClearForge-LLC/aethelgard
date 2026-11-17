//! Renamings of a specification that change names and nothing else
//! (`AET-WO-1313c` §2), from `AET-WO-1313b`'s `rename_perm.py` and
//! `rename_len.py`, for `outline_tier1` and `board_tier2`.

use std::collections::BTreeMap;

use serde_json::Value;

/// FNV-1a over `seed`, `kind` and `s`: a fixed, seeded order.
fn hash(seed: u64, kind: &str, s: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in seed
        .to_le_bytes()
        .iter()
        .chain(kind.as_bytes())
        .chain(b"/")
        .chain(s.as_bytes())
    {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// Each reference mapped to another of the same letters and length, by a
/// seeded permutation, so only the references' order changes.
fn permutation(refs: &[String], seed: u64) -> BTreeMap<String, String> {
    let mut groups: BTreeMap<(String, usize), Vec<String>> = BTreeMap::new();
    for r in refs {
        let letters: String = r.chars().take_while(char::is_ascii_alphabetic).collect();
        groups
            .entry((letters, r.len()))
            .or_default()
            .push(r.clone());
    }
    let mut map = BTreeMap::new();
    for g in groups.into_values() {
        let mut from = g.clone();
        from.sort();
        let mut to = g;
        to.sort_by_key(|r| (hash(seed, "p", r), r.clone()));
        map.extend(from.into_iter().zip(to));
    }
    map
}

fn each<'a>(v: &'a mut Value, key: &str) -> impl Iterator<Item = &'a mut Value> {
    v.get_mut(key)
        .and_then(Value::as_array_mut)
        .into_iter()
        .flatten()
}

fn rename_refs(v: &mut Value, keys: &[&str], map: &BTreeMap<String, String>) {
    for key in keys {
        for x in each(v, key) {
            let r = x["reference"].as_str().unwrap().to_owned();
            x["reference"] = Value::String(map[&r].clone());
        }
    }
}

fn refs(v: &Value, keys: &[&str]) -> Vec<String> {
    keys.iter()
        .flat_map(|k| v[*k].as_array().into_iter().flatten())
        .map(|x| x["reference"].as_str().unwrap().to_owned())
        .collect()
}

/// `rename_perm.py`: device, connector and test-point references permuted
/// among references of the same letters and length (a seeded permutation),
/// carried to the MCU's supply source; every bus and chip-select net given
/// a seeded four-digit prefix (a net's name is not on the silkscreen),
/// carried to net constraints, connector pins and test points. The parts and
/// their sizes are unchanged: only the order of their names moves.
pub fn order_only(spec: &Value, seed: u64) -> Value {
    let mut v = spec.clone();
    let devices = ["i2c_devices", "spi_devices"];
    let map = permutation(&refs(&v, &devices), seed);
    rename_refs(&mut v, &devices, &map);
    let jmap = permutation(&refs(&v, &["connectors"]), seed);
    rename_refs(&mut v, &["connectors"], &jmap);
    if let Some(c) = v["mcu"]["supply_source"]["connector"].as_str()
        && let Some(to) = jmap.get(c)
    {
        v["mcu"]["supply_source"]["connector"] = Value::String(to.clone());
    }
    let tmap = permutation(&refs(&v, &["test_points"]), seed);
    rename_refs(&mut v, &["test_points"], &tmap);
    let mut nets: BTreeMap<String, String> = BTreeMap::new();
    let mut prefix = |n: &str| -> String {
        nets.entry(n.to_owned())
            .or_insert_with(|| format!("{:04x}_{n}", hash(seed, "n", n) & 0xffff))
            .clone()
    };
    for (key, signals) in [
        ("i2c_buses", &["sda", "scl"][..]),
        ("spi_buses", &["sck", "mosi", "miso"][..]),
    ] {
        for b in each(&mut v, key) {
            for s in signals {
                let n = b[*s]["net"].as_str().unwrap().to_owned();
                b[*s]["net"] = Value::String(prefix(&n));
            }
        }
    }
    for d in each(&mut v, "spi_devices") {
        let n = d["cs"]["net"].as_str().unwrap().to_owned();
        d["cs"]["net"] = Value::String(prefix(&n));
    }
    let carry = |x: &mut Value, field: &str| {
        if let Some(to) = x[field].as_str().and_then(|n| nets.get(n)) {
            x[field] = Value::String(to.clone());
        }
    };
    for c in each(&mut v, "net_constraints") {
        carry(c, "net_name");
    }
    for c in each(&mut v, "connectors") {
        for p in each(c, "pins") {
            carry(p, "net");
        }
    }
    for t in each(&mut v, "test_points") {
        carry(t, "net");
    }
    v
}

/// `rename_len.py`, a size sample: [`order_only`] with seed `len`, then
/// every device, connector and test-point reference prefixed by `len`
/// characters and an underscore (`Q`, then `x`s), so each part's reference
/// text, and with it its keep-out box, grows by a chosen amount.
pub fn sized(spec: &Value, len: usize) -> Value {
    let mut v = order_only(spec, len as u64);
    if len == 0 {
        return v;
    }
    let pre: String = std::iter::once('Q')
        .chain(std::iter::repeat('x'))
        .take(len)
        .collect();
    for key in ["i2c_devices", "spi_devices", "connectors", "test_points"] {
        for x in each(&mut v, key) {
            let r = x["reference"].as_str().unwrap().to_owned();
            x["reference"] = Value::String(format!("{pre}_{r}"));
        }
    }
    if let Some(c) = v["mcu"]["supply_source"]["connector"].as_str() {
        let c = format!("{pre}_{c}");
        v["mcu"]["supply_source"]["connector"] = Value::String(c);
    }
    v
}
