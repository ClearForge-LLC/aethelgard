#!/usr/bin/env python3
"""AET-WO-1218: one mutant per guard this work order adds or changes, each run against the suites
that hold it. Each mutant is applied in place and the file is restored from its saved bytes
afterwards, so the tree is unchanged at the end. A mutant is killed when its suites fail and they
compile; a build failure is counted apart. Strings match the `cargo fmt`ed code.
Usage: python3 -I mutants.py <repo> <out.json>"""
import json, os, subprocess, sys

repo, out = sys.argv[1], sys.argv[2]
BENCH = "crates/aethelgard-ir/src/bench.rs"
NAMES = "crates/aethelgard-ir/src/names.rs"
PD = "crates/aethelgard-ir/src/pin_devices.rs"
POWER = "crates/aethelgard-ir/src/power.rs"
SO = "crates/aethelgard-ir/src/solve.rs"
PC = "crates/aethelgard-pincheck/src/lib.rs"

IR_BENCH = [["-p", "aethelgard-ir", "--test", "bench"]]
CLI_EMIT = [["-p", "aethelgard-cli", "--test", "emit_tier1", "a_pin_device_beside"]]
PINCHECK = [["-p", "aethelgard-pincheck", "--test", "check"],
            ["-p", "aethelgard-cli", "--test", "pinout_tier1"]]
POWER_T = [["-p", "aethelgard-ir", "--test", "power"]]
PD_T = [["-p", "aethelgard-ir", "--test", "pin_devices"]]
SOLVE_T = [["-p", "aethelgard-ir", "--test", "solve"]]
SERIAL_T = [["-p", "aethelgard-ir", "--test", "serial"]]

MUTANTS = [
    ("1.1 step 15a slices the test points off the end again", BENCH,
     "    let mut by_key: BTreeMap<String, &str> = crate::names::references_but_test_points(spec)\n        .into_iter()\n        .map(|r| (reference_key(r), r))\n        .collect();",
     "    let all = crate::names::references(spec);\n    let mut by_key: BTreeMap<String, &str> = all[..all.len() - spec.declared_test_points().len()]\n        .iter()\n        .map(|r| (reference_key(r), *r))\n        .collect();",
     IR_BENCH + CLI_EMIT),
    ("1.2 every other reference includes the test points", NAMES,
     "    declared(spec, false)\n}",
     "    declared(spec, true)\n}",
     IR_BENCH + CLI_EMIT),
    ("1.3 every other reference leaves the pin devices out", NAMES,
     "        .chain(test_points.iter().map(|t| t.reference.as_str()))\n        .chain(crate::pin_devices::references(spec))",
     "        .chain(test_points.iter().map(|t| t.reference.as_str()))\n        .chain(crate::pin_devices::references(spec).into_iter().filter(|_| !test_points.is_empty()))",
     IR_BENCH),
    ("2 check pinout judges a topology naming another MCU", PC,
     "    if topology.mcu != package.part {",
     "    if false && topology.mcu != package.part {",
     PINCHECK),
    ("3 the sense reservoir at VDDA's minimum", POWER,
     "            && !crate::pin_devices::reservoir_rated(cap.rated_millivolts.value, vdda)",
     "            && cap.rated_millivolts.value < i64::from(crate::minimum_millivolts(vdda))",
     POWER_T),
    ("R4.1 a bus held by hand names no withheld pad", SO,
     "                        \"{inst}: every pin for `{name}` is held by hand ({}){note}\",",
     "                        \"{inst}: every pin for `{name}` is held by hand ({})\",",
     SOLVE_T),
    ("R4.2 a bus with no pin names no withheld pad", SO,
     "                        format!(\"{inst}: the table has no unreserved pin for `{name}`{note}\")",
     "                        format!(\"{inst}: the table has no unreserved pin for `{name}`\")",
     SOLVE_T),
    ("R4.3 a serial port held by hand names no withheld pad", SO,
     "                        \"{p}: every pin for `{name}` is held by hand ({}){note}\",",
     "                        \"{p}: every pin for `{name}` is held by hand ({})\",",
     SERIAL_T),
    ("R4.4 the withheld pads not collected", SO,
     "                                && pads.contains(&pin.as_str())\n                                && p.alternate_functions.contains_key(&wanted)",
     "                                && pads.contains(&pin.as_str())\n                                && p.alternate_functions.contains_key(&wanted)\n                                && false",
     SOLVE_T + SERIAL_T),
    ("R5.1 a pin device's reservoir looked up only when RAIN needs it", PD,
     "            let reservoir = match &c.reservoir {\n                None => None,",
     "            let reservoir = match &c.reservoir {\n                _ if r_hi <= i128::from(rain.value) => None,\n                None => None,",
     PD_T),
    ("R5.2 the sense reservoir looked up only when RAIN needs it", POWER,
     "        let reservoir = match &s.reservoir {\n            None => None,",
     "        let reservoir = match &s.reservoir {\n            _ if rth <= i128::from(rain.value) => None,\n            None => None,",
     POWER_T),
]

def run(suites):
    for args in suites:
        r = subprocess.run(["nice", "-n", "10", "cargo", "test", "-q", *args], cwd=repo,
                           capture_output=True, text=True)
        text = r.stdout + r.stderr
        if r.returncode != 0:
            if "error[E" in text or "could not compile" in text:
                return "build-failure"
            return "killed"
    return "survived"

results = []
for name, rel, old, new, suites in MUTANTS:
    path = os.path.join(repo, rel)
    saved = open(path, "rb").read()
    text = saved.decode()
    assert text.count(old) == 1, (name, text.count(old))
    try:
        open(path, "w").write(text.replace(old, new))
        verdict = run(suites)
    finally:
        open(path, "wb").write(saved)
    results.append({"mutant": name, "file": rel, "verdict": verdict})
    print(f"{verdict:13} {name}", flush=True)
assert run(sorted({tuple(s) for m in MUTANTS for s in m[4]})) == "survived", "the unmutated tree must pass"
json.dump(results, open(out, "w"), indent=1)
print(sum(r["verdict"] == "killed" for r in results), "of", len(results), "killed")
