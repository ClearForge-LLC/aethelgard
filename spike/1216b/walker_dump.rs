//! AET-WO-1216b harness (never committed into a crate): the walker's paths
//! and step 25's verdict for every board in `$D1216B_LIST` ("name\tpath",
//! paths relative to `$D1216B_ROOT`), one TSV per board in `$D1216B_OUT`.
//! Dropped into a copy of the tree's `crates/aethelgard-ir/tests/`.
//! Columns: net class rails lead worst_mv basis pin at; then a last line
//! `#validate\t<ok | kind: message>`.

use std::fs;

use aethelgard_ir::Specification;
use aethelgard_ir::off_domain::{Bound, Lead, paths};

#[test]
fn dump() {
    let list = std::env::var("D1216B_LIST").expect("D1216B_LIST");
    let root = std::env::var("D1216B_ROOT").expect("D1216B_ROOT");
    let out = std::env::var("D1216B_OUT").expect("D1216B_OUT");
    fs::create_dir_all(&out).unwrap();
    for line in fs::read_to_string(list).unwrap().lines().filter(|l| !l.is_empty()) {
        let (name, path) = line.split_once('\t').unwrap();
        let text = fs::read_to_string(format!("{root}/{path}")).unwrap();
        let mut tsv = String::from("net\tclass\trails\tlead\tworst_mv\tbasis\tpin\tat\n");
        match serde_json::from_str::<Specification>(&text) {
            Err(e) => tsv.push_str(&format!("#parse\t{e}\n")),
            Ok(spec) => {
                match paths(&spec) {
                    Err(e) => tsv.push_str(&format!("#paths\t{}: {e}\n", e.kind())),
                    Ok(ps) => {
                        for p in ps {
                            let lead = match &p.lead {
                                Lead::Leads(w) => format!("yes: {w}"),
                                Lead::After(w) => format!("no: {w}"),
                            };
                            let (worst, basis) = match &p.bound {
                                Bound::Stated { millivolts, basis, .. } => {
                                    (millivolts.to_string(), basis.name().to_owned())
                                }
                                Bound::Unbounded(_) => ("not stated".to_owned(), "-".to_owned()),
                            };
                            tsv.push_str(&format!(
                                "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                                p.net,
                                p.class.name(),
                                if p.rails.is_empty() { "off-board".to_owned() } else { p.rails.join("/") },
                                lead,
                                worst,
                                basis,
                                p.pin.as_deref().unwrap_or("-"),
                                p.at
                            ));
                        }
                    }
                }
                let v = match spec.validate() {
                    Ok(()) => "ok".to_owned(),
                    Err(e) => format!("{}: {e}", e.kind()),
                };
                tsv.push_str(&format!("#validate\t{v}\n"));
            }
        }
        fs::write(format!("{out}/{name}.tsv"), tsv).unwrap();
    }
}
