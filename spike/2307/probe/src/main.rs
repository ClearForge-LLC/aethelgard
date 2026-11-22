// AET-WO-2307 §1.1, scratch: what `error.entries` should hold for one specification, derived from
// the ledger alone and not from the refusal's code. Reads the specification, solves it, builds the
// ledger, marks it with `ledger::acknowledge`, then groups every row left unacknowledged by
// (register, class). A group whose (register, class) the specification already acknowledges is
// that entry with the missing fields added. Every reason is "" (the architect's ruling on F1: a
// reason written for some fields cannot vouch for fields it never saw).
// Usage: probe2307 <spec.json>  -> one JSON object on stdout.
use sha2::Digest;
use std::collections::{BTreeMap, BTreeSet};

fn main() {
    let path = std::env::args().nth(1).expect("usage: probe2307 <spec.json>");
    let bytes = std::fs::read(&path).unwrap();
    let spec: aethelgard_ir::Specification = serde_json::from_slice(&bytes).unwrap();
    let table = aethelgard_svd_hal::pin_table(&spec.mcu.part).unwrap();
    let sha256: String = sha2::Sha256::digest(&bytes).iter().map(|b| format!("{b:02x}")).collect();
    let topology = aethelgard_ir::solve(&spec, &table, &sha256).unwrap();
    let mut ledger = aethelgard_cli::ledger::build(&topology, &table).unwrap();
    let missing = aethelgard_cli::ledger::acknowledge(&mut ledger, &spec);
    let mut groups: BTreeMap<(String, &'static str), BTreeSet<String>> = BTreeMap::new();
    for r in ledger.rows.iter().filter(|r| r.acknowledged == Some(false)) {
        let class = r.class.unwitnessed().unwrap().name();
        groups.entry((r.register.clone(), class)).or_default().insert(r.field.clone());
    }
    let registers: BTreeSet<&str> = groups.keys().map(|(r, _)| r.as_str()).collect();
    let mut completed = 0;
    let entries: Vec<serde_json::Value> = groups
        .iter()
        .map(|((register, class), fields)| {
            let existing = spec
                .acknowledge_unwitnessed
                .iter()
                .find(|a| &a.register == register && a.class == *class);
            let mut all = fields.clone();
            if let Some(a) = existing {
                completed += 1;
                all.extend(a.fields.iter().cloned());
            }
            serde_json::json!({"register": register, "class": class, "fields": all, "reason": ""})
        })
        .collect();
    let out = serde_json::json!({
        "rows": missing.len(),
        "first": missing.first(),
        "registers": registers.len(),
        "pairs": groups.len(),
        "completed": completed,
        "new": groups.len() - completed,
        "missing_fields": groups.values().map(BTreeSet::len).sum::<usize>(),
        "entries": entries,
    });
    println!("{}", serde_json::to_string(&out).unwrap());
}
