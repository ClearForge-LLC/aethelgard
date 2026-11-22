#!/usr/bin/env python3
"""AET-WO-1217: one mutant per guard this work order adds or changes, each run against the IR suites
that hold it (`cargo test -p aethelgard-ir --lib --test pin_devices --test pin_devices_solve`). Each
mutant is applied in place and the file is restored from its saved bytes afterwards, so the tree is
unchanged at the end. A mutant is killed when the suites fail and they compile; a build failure is
counted apart. Usage: python3 -I mutants.py <repo> <out.json>"""
import json, os, subprocess, sys

repo, out = sys.argv[1], sys.argv[2]
PD = "crates/aethelgard-ir/src/pin_devices.rs"
SO = "crates/aethelgard-ir/src/solve.rs"
LIB = "crates/aethelgard-ir/src/lib.rs"
PP = "crates/aethelgard-ir/src/power_parts.rs"

MUTANTS = [
    ("F2.1 tie admitted on any current output", PD,
     "if to == crate::GND && self.row.trip.as_ref().is_some_and(|t| t.sense == *pin) =>",
     "if to == crate::GND =>"),
    ("F2.2 tie admitted to any rail", PD,
     "if to == crate::GND && self.row.trip.as_ref().is_some_and(|t| t.sense == *pin) =>",
     "if self.row.trip.as_ref().is_some_and(|t| t.sense == *pin) =>"),
    ("F2.3 the trip judged despite the tie", PD,
     "            return Ok(());\n        }\n        // The sense pin terminated by nothing",
     "        }\n        // The sense pin terminated by nothing"),
    ("F2.4 a tied and read sense not refused", PD,
     "            if let Some(k) = self.d.analog.iter().position(|a| a.device_pin == t.sense) {",
     "            if let Some(k) = self.d.analog.iter().position(|a| a.device_pin == t.sense && false) {"),
    ("F2.5 the tie skips the grounded reference", PD,
     "        if to == crate::GND {\n            return Err(self.err(\n                K::TripReferenceGrounded,",
     "        if to == crate::GND && !self.d.straps.contains_key(&t.sense) {\n            return Err(self.err(\n                K::TripReferenceGrounded,"),
    ("F8 the reservoir at VDDA's minimum", PD,
     "    rated_mv >= i64::from(crate::maximum_millivolts(vdda))",
     "    rated_mv >= i64::from(crate::minimum_millivolts(vdda))"),
    ("F9 the ADC rule before the trip", PD,
     "        self.trip()?;\n        self.analog()?;",
     "        self.analog()?;\n        self.trip()?;"),
    ("F10 the MCU row looked up with no pin device", PD,
     "    if spec.declared_pin_devices().is_empty() {\n        return Ok(());\n    }\n    let mcu = mcu_row(spec)?;",
     "    let mcu = mcu_row(spec)?;\n    if spec.declared_pin_devices().is_empty() {\n        return Ok(());\n    }"),
    ("F4.1 no digital hand pin refused on a pad", SO,
     "            if digital && pads.contains(&key) {",
     "            if false && digital && pads.contains(&key) {"),
    ("F4.2 an ADC hand pin refused on a pad", SO,
     "            if digital && pads.contains(&key) {",
     "            if pads.contains(&key) {"),
    ("F4.3 bus and serial candidates on a pad", SO,
     "                            !reserved.contains(&pin.as_str()) && !pads.contains(&pin.as_str())",
     "                            !reserved.contains(&pin.as_str())"),
    ("F4.4 timer channels on a pad", SO,
     ".filter(|(pin, _)| !reserved.contains(&pin.as_str()) && !pads.contains(&pin.as_str()))",
     ".filter(|(pin, _)| !reserved.contains(&pin.as_str()))"),
    ("F4.5 a chip select on a pad", SO,
     "                        !reserved.contains(p) && !pads.contains(p) && !search.used.contains_key(p)",
     "                        !reserved.contains(p) && !search.used.contains_key(p)"),
    ("F4.6 a device GPIO on a pad", PD,
     "                        !reserved.contains(p)\n                            && !pads.contains(p)\n",
     "                        !reserved.contains(p)\n"),
    ("F4.7 the topology's pad check", LIB,
     "            if pads.contains(&pin.as_str()) {\n                return Err(E::AnalogPadPin {",
     "            if false && pads.contains(&pin.as_str()) {\n                return Err(E::AnalogPadPin {"),
    ("F4.8 a pad not held, or twice", PP,
     "        if !pins.contains_key(&pin) || !pads.insert(pin.clone()) {",
     "        if !pads.insert(pin.clone()) && false {"),
]

def run():
    r = subprocess.run(["cargo", "test", "-q", "-p", "aethelgard-ir", "--lib", "--test", "pin_devices",
                        "--test", "pin_devices_solve"], cwd=repo, capture_output=True, text=True)
    text = r.stdout + r.stderr
    if r.returncode == 0:
        return "survived"
    if "error[E" in text or "could not compile" in text:
        return "build-failure"
    return "killed"

results = []
for name, rel, old, new in MUTANTS:
    path = os.path.join(repo, rel)
    saved = open(path, "rb").read()
    text = saved.decode()
    assert text.count(old) == 1, (name, text.count(old))
    try:
        open(path, "w").write(text.replace(old, new))
        verdict = run()
    finally:
        open(path, "wb").write(saved)
    results.append({"mutant": name, "file": rel, "verdict": verdict})
    print(f"{verdict:13} {name}", flush=True)
assert run() == "survived", "the unmutated tree must pass"
json.dump(results, open(out, "w"), indent=1)
print(sum(r["verdict"] == "killed" for r in results), "of", len(results), "killed")
