#!/usr/bin/env python3
"""AET-WO-1211a §1.7: one mutant per guard this work order adds, each run against the tests that
should kill it, in a detached worktree copy (never the branch's own tree). Each mutant's text must
occur exactly once; the original file is restored from memory after each run.
Usage: python3 -I mutants.py COPY_DIR TARGET_DIR > result.json"""
import json, os, subprocess, sys

copy, target = sys.argv[1], sys.argv[2]
IR = "crates/aethelgard-ir/src/"
T_IR = ["-p", "aethelgard-ir", "--test", "pin_devices", "--test", "pin_devices_solve", "--test", "motor",
        "--test", "validate", "--test", "sense_pin"]
T_CLI = ["-p", "aethelgard-cli", "--test", "pin_devices_solve", "--test", "pin_device_refusals"]
# (id, file, old, new, tests): the four the work order names first (W1-W4).
M = [
 ("W1 a pulsed bridge idle accepted (read as low)", IR + "pin_devices.rs",
  "(MotorProtocol::Pwm, Some(p)) => match p.idle {", "(MotorProtocol::Pwm, Some(_p)) => match Idle::Level(Level::Low) {", T_IR),
 ("W2 the bridge idle judged without nSLEEP", IR + "pin_devices.rs",
  "self.judge_state(mode, enable, idle,", "self.judge_state(mode, Level::High, idle,", T_IR),
 ("W3 resolution_ps defaults", IR + "motor.rs",
  "    pub resolution_ps: u64,\n}", "    #[serde(default = \"one_us\")]\n    pub resolution_ps: u64,\n}\n\nfn one_us() -> u64 {\n    1_000_000\n}", T_IR),
 ("W4 the solver chooses ADC2", IR + "pin_devices.rs",
  ".or_else(|| if hand { pick(\"ADC2\") } else { None })", ".or_else(|| pick(\"ADC2\"))", T_IR),
 ("G1 a reset pull-up passed", IR + "pin_devices.rs",
  "if MCU_RESET_PULL_UPS.contains(&pin.as_str()) {", "if false {", T_IR),
 ("G2 a channel input outside a bridge", IR + "pin_devices.rs",
  "c.inputs.iter().find(|i| !bridged.contains(i.as_str()))", "c.inputs.iter().find(|_| false)", T_IR),
 ("G3 AERR outside its supply", IR + "pin_devices.rs", "if !in_band {", "if false {", T_IR),
 ("G4 a device net named as a sense", IR + "pin_devices.rs",
  "nets(spec).find(|n| sense.contains(&n.to_lowercase()))", "nets(spec).find(|_| sense.is_empty() && false)", T_IR),
 ("G5 a sense off its ADC input", IR + "pin_devices.rs", ".is_none_or(|a| a.pin != s.pin)", ".is_none_or(|_| false)", T_IR),
 ("G6 an idle of the whole period", IR + "motor.rs", "if t == ticks {", "if false {", T_IR),
 ("G7 the solved CCR of idle low", IR + "lib.rs", "Some(motor::Idle::Level(motor::Level::Low)) => m.ccr == 0,",
  "Some(motor::Idle::Level(motor::Level::Low)) => true,", T_IR),
 ("G8 check_silicon's idle", IR + "motor.rs", "Idle::Level(Level::High) => (ccr == arr1,", "Idle::Level(Level::High) => (true,", T_CLI),
 ("G9 check_silicon's tick", IR + "motor.rs", "let tick_ok = psc1 * PS_PER_S", "let tick_ok = true || psc1 * PS_PER_S", T_IR),
 ("G10 the full scale at VREF+", IR + "pin_devices.rs", "if full_uv > i128::from(vref_min) * 1000 {", "if false {", T_IR),
 ("G11 the typical gain for the band edge", IR + "pin_devices.rs",
  "(Some(pc), _) => ua * (100 + i128::from(pc)) / 100,", "(Some(_), _) => ua,", T_IR),
 ("G12 RAIN", IR + "pin_devices.rs", "if r_hi > i128::from(rain.value) {", "if false {", T_IR),
 ("G13 a driving idle", IR + "pin_devices.rs", "if state.drives {", "if false {", T_IR),
 ("G14 the strap's mode", IR + "pin_devices.rs", "if got != Some(want) {", "if false {", T_IR),
 ("G15 the input rate", IR + "pin_devices.rs", "if rate > max {", "if false {", T_IR),
 ("G16 an undeclared input PWM limit", IR + "pin_devices.rs", "(None, Some(d)) => d.value,", "(None, _) => u32::MAX,", T_IR),
 ("G17 the MCU's levels", IR + "pin_devices.rs", "if voh < l.vih_min_mv ||", "if false && voh < l.vih_min_mv ||", T_IR),
 ("G18 an input with no pull from reset", IR + "pin_devices.rs",
  "if self.row.pins[pin].pull_down_ohms_typical.is_none() {", "if false {", T_IR),
 ("G19 a pull-up the part cannot sink", IR + "pin_devices.rs", "if ua > i128::from(sink_ma) * 1000 ||", "if false && ua > 0 ||", T_IR),
 ("G20 a device's gpio without its device", IR + "pin_devices.rs",
  "if t.pin_devices.is_empty() && !g.is_high_output() {", "if false {", T_IR),
 ("G21 the timer key's tick", IR + "solve.rs", "if rate(&first.key) == rate(&second.key) {", "if false {", T_IR),
 ("G22 an ADC2 channel named without its pin", IR + "pin_devices.rs", "Some(w) => !w.starts_with(\"ADC2_\"),", "Some(_) => true,", T_IR),
 ("G23 an IN/IN pair on two timers", IR + "solve.rs", "if first_timer != second_timer {", "if false {", T_CLI),
 ("G24 idle pulse 0", IR + "lib.rs", "if p.idle == motor::Idle::PulseUs(0) {", "if false {", T_IR),
 ("G25 a forbidden state", IR + "pin_devices.rs", "if mode.forbidden.contains(&state.name) {", "if false {", T_IR),
 ("G26 a bus part in pin_devices", IR + "pin_devices.rs", "None if table.get(&d.part).is_some() => {", "None if false => {", T_IR),
 ("G27 a busless part on a bus", IR + "lib.rs", "if let Some(e) = pin_devices::part_without_bus(reference, part, &path) {", "if let Some(e) = None::<IrValidationError> {", T_IR),
 ("R1 cli generate takes a pin device", "crates/aethelgard-cli/src/generate.rs",
  "if let Some((reference, d)) = topology.pin_devices.iter().next() {", "if let Some((reference, d)) = topology.pin_devices.iter().next().filter(|_| false) {", T_CLI),
 ("R2 cli generate takes idle high", "crates/aethelgard-cli/src/generate.rs",
  "topology.motor_outputs.iter().find(|(_, m)| high(m))", "topology.motor_outputs.iter().find(|(_, m)| high(m) && false)", T_CLI),
]
out = []
for mid, rel, old, new, tests in M:
    path = os.path.join(copy, rel)
    text = open(path).read()
    n = text.count(old)
    if n != 1:
        out.append({"id": mid, "status": f"text found {n} times"})
        continue
    open(path, "w").write(text.replace(old, new))
    try:
        r = subprocess.run(["cargo", "test", *tests], cwd=copy, capture_output=True, text=True,
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
