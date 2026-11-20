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
 # A1 (Table 13, ruled D-A1)
 ("A1.1 a pull to another rail judged as mcu.supply's", PD, "if to.id == mcu.id {", "if true {"),
 ("A1.2 the same-rail exemption dropped", PD, "if to.id == mcu.id {", "if false {"),
 ("A1.3 the floor passes everything", PD, "if v <= floor {", "if true {"),
 ("A1.4 the floor's edge", PD, "if v <= floor {", "if v < floor {"),
 ("A1.5 an unpinned pull judged on an FT pin", PD, "let Some(pin) = s.pin.as_deref() else {", "let Some(pin) = s.pin.as_deref().or(Some(\"PA1\")) else {"),
 ("A1.6 a reset-pulled pin", PD, "&& (MCU_RESET_PULL_UPS.contains(&pin) || MCU_RESET_PULL_DOWNS.contains(&pin))", "&& false"),
 ("A1.7 PA14's reset pull-down", PD, "&& (MCU_RESET_PULL_UPS.contains(&pin) || MCU_RESET_PULL_DOWNS.contains(&pin))", "&& MCU_RESET_PULL_UPS.contains(&pin)"),
 ("A1.8 note 3's threshold raised", PD, "let pulls_off = vdd + io.pulls_off_above_supply.value;", "let pulls_off = vdd + io.pulls_off_above_supply.value + 1000;"),
 ("A1.9 the structure's limit", PD, "if v > limit {", "if false {"),
 ("A1.10 the limit's edge", PD, "if v > limit {", "if v >= limit {"),
 ("A1.11 VDD at its maximum", PD, "let vdd = i64::from(crate::minimum_millivolts(mcu));", "let vdd = i64::from(crate::maximum_millivolts(mcu));"),
 ("A1.12 the pull at its domain's minimum", PD, "let v = i64::from(crate::maximum_millivolts(to));", "let v = i64::from(crate::minimum_millivolts(to));"),
 ("A1.13 no supply term", PP, "let above = self.above_supply_mv.map_or(i64::MAX, |a| supply_mv + a);", "let above = i64::MAX;"),
 ("A1.14 no cap", PP, "above.min(self.max_mv.unwrap_or(i64::MAX))", "above"),
 ("A1.15 every structure read as FT", PP, "let name = structure.split('_').next().unwrap_or(structure);", "let name = \"FT\";"),
 ("A1.16 an unheld pin assumed FT", PD, "= io.tolerated(pin, vdd) else {", "= io.tolerated(pin, vdd).or(io.tolerated(\"PA1\", vdd)) else {"),
 ("A1.17 the resistor pull not judged", PD, "self.pull_tolerated(s, v, &at)?;", ""),
 # A2
 ("A2.1 VREF on GND", PD, "if to == crate::GND {", "if false {"),
 ("A2.2 a sense no input terminates", PD, "if !self.d.analog.iter().any(|a| a.device_pin == t.sense) {", "if false {"),
 ("A2.3 the trip below full scale", PD, "if full_uv > i128::from(ref_min) * 1000 {", "if false {"),
 ("A2.4 the trip's edge", PD, "if full_uv > i128::from(ref_min) * 1000 {", "if full_uv >= i128::from(ref_min) * 1000 {"),
 ("A2.5 VREF at its maximum", PD, "let ref_min = crate::minimum_millivolts(v);", "let ref_min = crate::maximum_millivolts(v);"),
 ("A2.6 the sense pin filter", PD, "if a.device_pin != t.sense {", "if a.device_pin == t.sense {"),
 ("A2.7 the trip not judged", PD, "        self.trip()?;\n", ""),
 # bulk
 ("B.1 a bulk the row does not admit", PD, "            (None, None) => Ok(()),", "            (None, _) => Ok(()),"),
 ("B.2 a bulk left undeclared", PD, "            (None, None) => Ok(()),", "            (None | Some(_), None) => Ok(()),"),
 ("B.3 the rating", PD, "if rated.value <= i64::from(hi) {", "if false {"),
 ("B.4 the rating's edge", PD, "if rated.value <= i64::from(hi) {", "if rated.value < i64::from(hi) {"),
 ("B.5 the domain at its minimum", PD, "let hi = crate::maximum_millivolts(", "let hi = crate::minimum_millivolts("),
 ("B.6 any row a capacitor", PD, "match pp.get(&b.part).map(|r| &r.kind) {", "match pp.get(\"GCM32EC71H106KA03\").map(|r| &r.kind) {"),
 ("B.7 an electrolytic refused", PD, "                    Some(crate::power_parts::Kind::BulkCapacitor(c)) => &c.rated_millivolts,\n", ""),
 ("B.8 bulk not judged", PD, "        self.limits()?;\n        self.bulk()", "        self.limits()"),
 ("B.9 bulk's reference not counted", PD, "            out.push(b.reference.as_str());", "            let _ = b;"),
 ("B.10 bulk's source not checked", PD, "            crate::board::source(&b.source, &format!(\"{at}.bulk.source\"))?;", ""),
 # the rows' parsers
 ("P.1 a family twice", PP, "            || families.iter().any(|g| g.family == f.family)", ""),
 ("P.2 a lower-case family", PP, "            || !f.family.bytes().all(|b| b.is_ascii_uppercase())", ""),
 ("P.3 a family with no term", PP, "if terms.iter().all(Option::is_none) || ", "if "),
 ("P.4 a term of zero", PP, " || terms.iter().flatten().any(|v| *v <= 0) {", " {"),
 ("P.5 a pin not canonical", PP, "if !canonical || ", "if "),
 ("P.6 a structure of no held family", PP, "|| !families.iter().any(|f| f.family == family)", ""),
 ("P.7 page 0", PP, "|| p.page == 0 {", "{"),
 ("P.8 no pins", PP, "if pins.is_empty() {", "if false {"),
 ("P.9 a trip on the wrong pins", PR, "if let Some(t) = &raw.trip", "if let Some(t) = raw.trip.as_ref().filter(|_| false)"),
 ("P.10 a bulk_unstated on no supply", PR, "if let Some(b) = &raw.bulk_unstated", "if let Some(b) = raw.bulk_unstated.as_ref().filter(|_| false)"),
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
