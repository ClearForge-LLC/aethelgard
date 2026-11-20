#!/usr/bin/env python3
"""AET-WO-1211e §1.5: one mutant per rule and per guard this work order adds, each run against
the tests that should kill it, in a detached worktree copy (never the branch's own tree). Each
mutant's text must occur exactly once; the file is restored from memory after each run.
After spike/1211a/mutants/mutants.py. Usage: python3 -I mutants.py COPY_DIR TARGET_DIR > result.json"""
import json, os, subprocess, sys

copy, target = sys.argv[1], sys.argv[2]
PD = "crates/aethelgard-ir/src/pin_devices.rs"
PP = "crates/aethelgard-ir/src/power_parts.rs"
PR = "crates/aethelgard-ir/src/pin_parts.rs"
T = ["-p", "aethelgard-ir", "--test", "pin_devices", "--test", "power", "--test", "parts"]
M = [
 # A1
 ("A1.1 the floor passes everything", PD, "if v <= floor {", "if true {"),
 ("A1.2 the floor's edge", PD, "if v <= floor {", "if v < floor {"),
 ("A1.3 an unpinned pull judged on an FT pin", PD, "let Some(pin) = s.pin.as_deref() else {", "let Some(pin) = s.pin.as_deref().or(Some(\"PA1\")) else {"),
 ("A1.4 a reset-pulled pin", PD, "if MCU_RESET_PULL_UPS.contains(&pin) || MCU_RESET_PULL_DOWNS.contains(&pin) {", "if false {"),
 ("A1.5 PA14's reset pull-down", PD, "if MCU_RESET_PULL_UPS.contains(&pin) || MCU_RESET_PULL_DOWNS.contains(&pin) {", "if MCU_RESET_PULL_UPS.contains(&pin) {"),
 ("A1.6 the structure's limit", PD, "if v > limit {", "if false {"),
 ("A1.7 the limit's edge", PD, "if v > limit {", "if v >= limit {"),
 ("A1.8 VDD at its maximum", PD, "let vdd = i64::from(crate::minimum_millivolts(mcu));", "let vdd = i64::from(crate::maximum_millivolts(mcu));"),
 ("A1.9 FT without its supply term", PP, "Some(above) => self.max_mv.value.min(supply_mv + above),", "Some(_) => self.max_mv.value,"),
 ("A1.10 every structure read as FT", PP, "let name = structure.split('_').next().unwrap_or(structure);", "let name = \"FT\";"),
 ("A1.11 an unheld pin assumed FT", PD, "= io.tolerated(pin, vdd) else {", "= io.tolerated(pin, vdd).or(io.tolerated(\"PA1\", vdd)) else {"),
 ("A1.12 the resistor pull not judged", PD, "self.pull_tolerated(s, crate::maximum_millivolts(v), &at)?;", ""),
 # A2
 ("A2.1 VREF on GND", PD, "if to == crate::GND {", "if false {"),
 ("A2.2 the trip below full scale", PD, "if full_uv > i128::from(ref_min) * 1000 {", "if false {"),
 ("A2.3 the trip's edge", PD, "if full_uv > i128::from(ref_min) * 1000 {", "if full_uv >= i128::from(ref_min) * 1000 {"),
 ("A2.4 VREF at its maximum", PD, "let ref_min = crate::minimum_millivolts(v);", "let ref_min = crate::maximum_millivolts(v);"),
 ("A2.5 the sense pin filter", PD, "if a.device_pin != t.sense {", "if a.device_pin == t.sense {"),
 ("A2.6 the trip not judged", PD, "        self.trip()?;\n", ""),
 # bulk
 ("B.1 a bulk the row does not admit", PD, "            (None, None) => Ok(()),", "            (None, _) => Ok(()),"),
 ("B.2 a bulk left undeclared", PD, "            (None, None) => Ok(()),", "            (None | Some(_), None) => Ok(()),"),
 ("B.3 the rating", PD, "if cap.rated_millivolts.value <= i64::from(hi) {", "if false {"),
 ("B.4 the rating's edge", PD, "if cap.rated_millivolts.value <= i64::from(hi) {", "if cap.rated_millivolts.value < i64::from(hi) {"),
 ("B.5 the domain at its minimum", PD, "let hi = crate::maximum_millivolts(", "let hi = crate::minimum_millivolts("),
 ("B.6 any part a capacitor", PD, "pp.get(&b.part).map(|r| &r.kind)", "pp.get(\"GCM32EC71H106KA03\").map(|r| &r.kind)"),
 ("B.7 bulk not judged", PD, "        self.limits()?;\n        self.bulk()", "        self.limits()"),
 ("B.8 bulk's reference not counted", PD, "            out.push(b.reference.as_str());", "            let _ = b;"),
 ("B.9 bulk's source not checked", PD, "            crate::board::source(&b.source, &format!(\"{at}.bulk.source\"))?;", ""),
 # the rows' parsers
 ("P.1 a family twice", PP, "            || families.iter().any(|g| g.family == f.family)", ""),
 ("P.2 a lower-case family", PP, "            || !f.family.bytes().all(|b| b.is_ascii_uppercase())", ""),
 ("P.3 a supply term of zero", PP, "if f.above_supply_millivolts.is_some_and(|a| a <= 0) {", "if false {"),
 ("P.4 a pin not canonical", PP, "if !canonical || ", "if "),
 ("P.5 a structure of no held family", PP, "|| !families.iter().any(|f| f.family == family)", ""),
 ("P.6 page 0", PP, "|| p.page == 0 {", "{"),
 ("P.7 no pins", PP, "if pins.is_empty() {", "if false {"),
 ("P.8 a trip on the wrong pins", PR, "if let Some(t) = &raw.trip", "if let Some(t) = raw.trip.as_ref().filter(|_| false)"),
 ("P.9 a bulk_unstated on no supply", PR, "if let Some(b) = &raw.bulk_unstated", "if let Some(b) = raw.bulk_unstated.as_ref().filter(|_| false)"),
]
out = []
for mid, rel, old, new in M:
    path = os.path.join(copy, rel)
    text = open(path).read()
    n = text.count(old)
    if n != 1:
        out.append({"id": mid, "status": f"text found {n} times"})
        print(json.dumps(out[-1]), file=sys.stderr)
        continue
    open(path, "w").write(text.replace(old, new))
    try:
        r = subprocess.run(["cargo", "test", "-j", "4", *T], cwd=copy, capture_output=True, text=True,
                           env={**os.environ, "CARGO_TARGET_DIR": target})
        failed = [l.split()[1] for l in r.stdout.splitlines() if l.startswith("test ") and l.endswith("FAILED")]
        status = "killed" if r.returncode else "SURVIVED"
        if r.returncode and not failed:
            status = "killed (build)" if "error[" in r.stderr else "killed"
        out.append({"id": mid, "status": status, "by": failed[:4]})
    finally:
        open(path, "w").write(text)
    print(json.dumps(out[-1]), file=sys.stderr)
json.dump(out, sys.stdout, indent=1)
