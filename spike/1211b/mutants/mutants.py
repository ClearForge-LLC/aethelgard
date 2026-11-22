#!/usr/bin/env python3
"""AET-WO-1211b §1.5: one mutant per guard this work order adds or changes, each run against the
tests that should kill it, in a detached worktree copy (never the branch's own tree). Each mutant's
text must occur exactly once; the original file is restored after each run. A mutant whose killer
is a Kani proof is judged by the image runner instead (kani-mutants.sh), not here.
Usage: python3 -I mutants.py COPY_DIR TARGET_DIR [ID...] > result.json"""
import json, os, subprocess, sys

copy, target, only = sys.argv[1], sys.argv[2], sys.argv[3:]
B = "crates/aethelgard-svd-hal/src/board.rs"
L = "crates/aethelgard-cli/src/ledger.rs"
G = "crates/aethelgard-cli/src/generate.rs"
S = "crates/aethelgard-ir/src/silicon.rs"
T_HAL = ["-p", "aethelgard-svd-hal", "--lib", "--test", "motor_outputs"]
T_CLI = ["-p", "aethelgard-cli", "--test", "pin_device_refusals", "--test", "ledger_tier1"]
T_HAL2 = ["-p", "aethelgard-svd-hal", "--test", "pin_devices"]
M = [
 ("WO2 the idle written after CEN", B,
  """    writes.push(("EGR", tf.egr, 1 << tf.ug));
    writes.push(("CR1", tf.cr1, (1 << tf.arpe) | (1 << tf.cen)));
    writes""",
  """    writes.push(("EGR", tf.egr, 1 << tf.ug));
    writes.push(("CR1", tf.cr1, (1 << tf.arpe) | (1 << tf.cen)));
    let ccr: Vec<_> = writes.iter().filter(|w| w.0.starts_with("CCR")).copied().collect();
    writes.retain(|w| !w.0.starts_with("CCR"));
    writes.extend(ccr);
    writes""", T_HAL),
 ("WO1 a GPIO initial level dropped (always set)", B,
  "            Some(Level::Low) => 1u32 << pf.br,\n            _ => 1u32 << pf.bs,",
  "            Some(Level::Low) => 1u32 << pf.bs,\n            _ => 1u32 << pf.bs,", T_HAL),
 ("G1 the motor pull ignores an idle high", B,
  "        Some(motor::Idle::Level(motor::Level::High)) => MOTOR_PULL_HIGH,", "        Some(motor::Idle::Level(motor::Level::High)) => MOTOR_PULL,", T_HAL),
 ("G2 an input's declared pull ignored", B, "        Some(McuPull::Up) => 0b01,", "        Some(McuPull::Up) => 0b00,", T_HAL),
 ("G3 the oracle's input pull ignored", S,
  "        (None, Some(g)) if g.direction == Direction::Input => match g.pull {", "        (None, Some(g)) if false && g.direction == Direction::Input => match g.pull {", T_HAL),
 ("G4 the oracle's idle-high word", B, "        motor::Idle::Level(Level::High) => arr.checked_add(1),", "        motor::Idle::Level(Level::High) => Some(arr),", T_HAL),
 ("G5 an input takes a speed write", B, "        if !p.role.mode_only() {\n            field(w, g.ospeedr", "        if true {\n            field(w, g.ospeedr", T_HAL),
 ("G6 a power.sense input generated", B, "        .filter(move |(net, _)| !sense.contains(net))", "        .filter(move |(_net, _)| !sense.is_empty() || true)", T_CLI),
 ("G7 simulate takes a pin device", G,
  "    if let Some((reference, d)) = topology.pin_devices.iter().next() {", "    if let Some((reference, d)) = topology.pin_devices.iter().next().filter(|_| false) {", T_CLI),
 ("G8 simulate takes an idle high", G,
  "topology.motor_outputs.iter().find(|(_, m)| high(m))", "topology.motor_outputs.iter().find(|(_, m)| high(m) && false)", T_CLI),
 ("G9 a .repl for a pin device", G, "        .pin_devices\n        .is_empty()\n        .then(", "        .pin_devices\n        .len()\n        .ge(&0)\n        .then(", T_CLI),
 ("G10 a declared input's pull not judged", L, "                if row.value != Some(rule) {\n                    return Err(LedgerError::RuleDisagrees {\n                        id: row.id.clone(),\n                        value: row.value,\n                        rule,\n                        name: \"implied_pupd\",",
  "                if false {\n                    return Err(LedgerError::RuleDisagrees {\n                        id: row.id.clone(),\n                        value: row.value,\n                        rule,\n                        name: \"implied_pupd\",", T_CLI),
 ("G11 an idle-high pull judged as a pull-down", L, "        let (rule, name, judge) = if high {", "        let (rule, name, judge) = if false {", T_CLI),

 # Stage 2: the bridge, the ADC and the ledger (after S1, ruled 2026-11-20T19:06Z).
 ("WO3 a bridge's enable before its CEN (configured with the pins)", "crates/aethelgard-svd-hal/src/board/firmware.rs",
  "        .filter(|(pin, _)| plan.enable_after(pin).is_none())", "        .filter(|(pin, _)| plan.enable_after(pin).is_none() || true)", T_HAL2),
 ("WO3b the image's enable rule dropped", B, "    let e = plan.enable_after(pin)?;", "    let e = plan.enable_after(pin).filter(|_| false)?;", T_HAL2),
 ("WO4 an unclassified ledger field (PCSEL)", L, "        (\"PCSEL\", &[\"PCSEL\"][..]),", "        (\"PCSEL\", &[\"PCSELX\"][..]),", T_CLI),
 ("B1 a driving idle passed", B, "            if state.drives || mode.forbidden.contains(&state.name) {", "            if false && (state.drives || mode.forbidden.contains(&state.name)) {", T_HAL2),
 ("B2 a waking enable not waited", B, "            let wake_us = if initial == e.active {", "            let wake_us = if false && initial == e.active {", T_HAL2),
 ("A1 the 12-bit code of revision V alone", "crates/aethelgard-svd-hal/src/board/adc.rs", "        12 => Some(0b010),", "        12 => Some(0b110),", T_HAL2),
 ("A2 the calibration bound at the stated minimum", "crates/aethelgard-svd-hal/src/board/adc.rs",
  "pub(super) const CALIBRATION_POLLS: u32 = 2 * 165_010 * 8;", "pub(super) const CALIBRATION_POLLS: u32 = 165_010 * 8;", T_HAL2),
]
out = []
for (mid, f, old, new, tests) in M:
    if only and mid.split()[0] not in only:
        continue
    path = os.path.join(copy, f)
    text = open(path).read()
    n = text.count(old)
    if n != 1:
        out.append({"id": mid, "error": f"old text occurs {n} times"}); continue
    open(path, "w").write(text.replace(old, new))
    try:
        r = subprocess.run(["cargo", "test", "-q", "-j", "6", *tests, "--", "--test-threads", "6"], cwd=copy,
                           env={**os.environ, "CARGO_TARGET_DIR": target}, capture_output=True, text=True,
                           timeout=1800)
        tail = (r.stdout + r.stderr)[-600:]
        failed = [l for l in (r.stdout + r.stderr).splitlines() if l.startswith("---- ") or " FAILED" in l]
        out.append({"id": mid, "killed": r.returncode != 0, "by": failed[:6], "tail": tail if r.returncode == 0 else ""})
    finally:
        open(path, "w").write(text)
    print(json.dumps(out[-1]), file=sys.stderr)
print(json.dumps(out, indent=1))
