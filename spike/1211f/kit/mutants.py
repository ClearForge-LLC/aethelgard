#!/usr/bin/env python3
"""AET-WO-1211f §1.5: one mutant per new line of the arm. Each is applied to a copy of the tree
(never the worktree, never by git checkout), the tests that hold the arm are run, and the mutant is
killed only by a failing test, not by a build break. Usage:
mutants.py <tree copy> <cargo target dir> <out.json> [name prefix]"""
import json, subprocess, sys
tree, target, out = sys.argv[1:4]
only = sys.argv[4] if len(sys.argv) > 4 else None  # run the mutants whose name starts with this
A = "crates/aethelgard-eda/src/schematic_arms.rs"
S = "crates/aethelgard-eda/src/schematic.rs"
M = [
    ("M1 answers: any pin", A, "if p == u.supply)", "if true)"),
    ("M2 answers: any second end", A, "(End::Pin(p), End::Ground) if p", "(End::Pin(p), _) if p"),
    ("M3 the lift never answers", A, ".map(|_| (bulk, parts_row.document.as_str()))", ".and(None::<(&aethelgard_ir::pin_devices::Bulk, &str)>)"),
    ("M4 reference emitter-numbered", A, "    b.parts.push(Part {\n        lib_id: row.symbol.as_str(),\n        reference: bulk.reference.clone(),", "    let numbered = b.reference('C')?;\n    b.parts.push(Part {\n        lib_id: row.symbol.as_str(),\n        reference: numbered,"),
    ("M5 capacitance", A, "super::power::farads(pf)", "super::power::farads(pf / 10)"),
    ("M6 rating", A, "volts(mv), bulk.part)", "volts(mv / 2), bulk.part)"),
    ("M7 footprint", A, "footprint: row.footprint.clone(),\n        conns: BTreeMap::from([(\"1\".into(), one)", "footprint: \"Capacitor_SMD:C_0402_1005Metric\".into(),\n        conns: BTreeMap::from([(\"1\".into(), one)"),
    ("M8 symbol", A, "lib_id: row.symbol.as_str(),\n        reference: bulk.reference", "lib_id: \"Device:C\",\n        reference: bulk.reference"),
    ("M9 ends swapped", A, "conns: BTreeMap::from([(\"1\".into(), one), (\"2\".into(), two)]),\n        note: Some(", "conns: BTreeMap::from([(\"1\".into(), two), (\"2\".into(), one)]),\n        note: Some("),
    ("M10 no note", A, "note: Some(format!(\n            \"{name} declared", "note: None.or(Some(format!(\n            \"{name} declared"),
    ("M11 note without the source", A, "states no value: {}\",\n            bulk.source", "states no value: {}\",\n            bulk.part"),
    ("M12 not anchored", A, "                    b.anchor_last(&d.reference, pads, Some((NEAR_NM, near)), false);\n                    bulk_drawn = true;", "                    let _ = (&pads, near);\n                    bulk_drawn = true;"),
    ("M13 drawn not recorded", A, "                    bulk_drawn = true;\n", "                    bulk_drawn |= false;\n"),
    ("M14 dropped bulk not refused", A, "        && !bulk_drawn\n", "        && !bulk_drawn\n        && false\n"),
    ("M15 electrolytic row refused", A, "        Some(Kind::BulkCapacitor(c)) => (&c.rated_picofarads, &c.rated_millivolts),\n", ""),
    ("M16 bulk reference not claimed", A, "        if let Some(bulk) = &d.bulk {\n            out.push(&bulk.reference);\n        }\n", ""),
    ("M17 BOM note dropped", S, "note: p.note.clone().unwrap_or_default(),", "note: String::new(),"),
]
res = []
for name, f, old, new in M:
    if only and not name.startswith(only):
        continue
    p = f"{tree}/{f}"
    src = open(p).read()
    assert src.count(old) == 1, f"{name}: anchor count {src.count(old)}"
    if name == "M10 no note":  # a None note: drop the Some( ... ) wholly
        i = src.index(old); j = src.index("),\n", src.index("bulk.source", i)) + 3
        mutated = src[:i] + "note: None,\n" + src[j:]
    else:
        mutated = src.replace(old, new)
    open(p, "w").write(mutated)
    try:
        cmds = [["cargo", "test", "-q", "-p", "aethelgard-eda", "--lib", "arms", "--target-dir", target, "-j", "6"],
                ["cargo", "test", "-q", "-p", "aethelgard-cli", "--test", "pin_devices_drawn_tier1", "--target-dir", target, "-j", "6"]]
        verdict, log = "survived", ""
        for c in cmds:
            r = subprocess.run(c, cwd=tree, capture_output=True, text=True)
            log += r.stdout[-1500:] + r.stderr[-1500:]
            if r.returncode != 0:
                verdict = "build-break" if ("error[E" in r.stderr or "could not compile" in r.stderr) else "killed"
                break
        failing = sorted({l.split()[0] for l in log.splitlines() if l.startswith("    ") and "::" not in l and l.strip() and not l.strip().startswith("left") and l.strip().replace("_", "").isalnum()})
        res.append({"mutant": name, "verdict": verdict, "failing": failing})
        print(name, verdict, failing, flush=True)
    finally:
        open(p, "w").write(src)
json.dump(res, open(out, "w"), indent=1)
print(sum(r["verdict"] == "killed" for r in res), "of", len(res), "killed")
