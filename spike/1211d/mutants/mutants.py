#!/usr/bin/env python3
"""AET-WO-1211d mutants: one per guard this work order adds, each applied alone to a scratch copy of
HEAD, then the named test target run. A test failure kills the mutant. Usage:
mutants.py <worktree> <scratch dir> <result.json> [mutant prefix, to rerun one]"""
import json, subprocess, sys, os, shutil

ARM = "crates/aethelgard-eda/src/schematic_pin_device.rs"
SCH = "crates/aethelgard-eda/src/schematic.rs"
NAMES = "crates/aethelgard-ir/src/names.rs"
DRAWN = ("aethelgard-cli", "pin_devices_drawn_tier1")
MUTANTS = [
    ("W1 a strap left floating (a tie not drawn)", ARM,
     "            Strap::Tie(net) => {\n                let c = b.declared(net);\n                put(n, c, &mut conns)?;",
     "            Strap::Tie(net) => {\n                let c = b.declared(net);\n                let _ = (n, c);", DRAWN),
    ("W2 nFAULT's pull-up missing (declared resistors not drawn)", ARM,
     "    for (r, one, two) in resistors {", "    for (r, one, two) in resistors.into_iter().filter(|_| false) {", DRAWN),
    ("W3 a bridge output to no connector (the check removed)", ARM,
     "            if !ended.contains(&net.as_str()) {", "            if false && !ended.contains(&net.as_str()) {", DRAWN),
    ("G1 CVM2 skipped instead of refused", ARM,
     "        let Some(value) = r.value else {\n            return Err(EmitError::PassiveValueUnstated {",
     "        let Some(value) = r.value else {\n            continue;\n            #[allow(unreachable_code)]\n            return Err(EmitError::PassiveValueUnstated {", DRAWN),
    ("G2 an undeclared current output left open", ARM,
     "            PinKind::OpenDrainOutput { .. } => {",
     "            PinKind::OpenDrainOutput { .. } | PinKind::CurrentOutput { .. } => {", DRAWN),
    ("G3 a pin device with no arm drawn anyway", SCH,
     "        .find(|(_, d)| !pin_device::draws(&d.part))", "        .find(|_| false)",
     ("aethelgard-cli", "pin_device_refusals")),
    ("G4 own_nets without the pin devices' nets", NAMES,
     "    for d in spec.declared_pin_devices() {\n        if let Some(row) = parts.pin_part(&d.part) {",
     "    for d in spec.declared_pin_devices().iter().take(0) {\n        if let Some(row) = parts.pin_part(&d.part) {", DRAWN),
    ("G5 VINT on a label, not a rail", ARM,
     "            let c = if super::power_in(", "            let c = if false && super::power_in(", DRAWN),
    ("G6 the pin device's ADC input not on the MCU", SCH,
     "                .filter(|(net, _)| analog.contains(net))", "                .filter(|_| false)", DRAWN),
    ("G7 a row pin left floating (the completeness refusal removed)", ARM,
     "            _ => {\n                return Err(not_drawable(\n                    d,\n                    format!(\n                        \"its {} pin `{name}` is declared nowhere, and would float\",",
     "            _ => {\n                conns.insert(n, Conn::NoConnect);\n                continue;\n                #[allow(unreachable_code)]\n                return Err(not_drawable(\n                    d,\n                    format!(\n                        \"its {} pin `{name}` is declared nowhere, and would float\",", DRAWN),
]

def main():
    repo, scratch, out = sys.argv[1:4]
    shutil.rmtree(scratch, ignore_errors=True)
    os.makedirs(scratch)
    subprocess.run(f"git -C {repo} archive HEAD | tar -x -C {scratch}", shell=True, check=True)
    env = dict(os.environ, CARGO_TARGET_DIR=os.path.expanduser("~/.cache/aet-1211d/target-mut"))
    results = []
    only = sys.argv[4] if len(sys.argv) > 4 else ""
    for name, path, old, new, (pkg, test) in MUTANTS:
        if not name.startswith(only):
            continue
        p = os.path.join(scratch, path)
        orig = open(p).read()
        assert orig.count(old) == 1, f"{name}: the guard moved"
        open(p, "w").write(orig.replace(old, new))
        r = subprocess.run(["nice", "-n", "10", "cargo", "test", "-q", "-p", pkg, "--test", test],
                           cwd=scratch, env=env, capture_output=True, text=True)
        open(p, "w").write(orig)
        log = r.stdout + r.stderr
        compiled = "could not compile" not in log
        failing = [l.split()[0] for l in log.splitlines() if l.endswith("--- FAILED") or l.endswith(" FAILED")]
        killed = r.returncode != 0 and compiled
        results.append({"mutant": name, "target": f"{pkg} --test {test}", "killed": killed,
                        "compiled": compiled, "failing": failing})
        print(("KILLED  " if killed else ("NOCOMP  " if not compiled else "SURVIVED")), name, failing, flush=True)
    if only:
        old = [r for r in json.load(open(out)) if not r["mutant"].startswith(only)]
        results = sorted(old + results, key=lambda r: [m[0] for m in MUTANTS].index(r["mutant"]))
    json.dump(results, open(out, "w"), indent=1)

main()
