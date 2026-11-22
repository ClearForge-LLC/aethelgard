"""AET-WO-1211c §1.5: one mutant per witness, fault kind, gate and refusal this work order adds, each
applied alone in a scratch export of HEAD (never the worktree), and the test meant to kill it run.
A mutant whose build fails is `invalid`, never `killed`. Restored by rewriting the original bytes
(a fresh mtime, so cargo rebuilds; the rsync-mtime lesson). usage: python3 -I mutants.py [ids...]"""
import os, subprocess, sys
HOME = os.path.expanduser("~")
SRC, TARGET = f"{HOME}/.cache/aet-1211c/mut/src", f"{HOME}/.cache/aet-1211c/mut/target"
REPO = os.path.abspath(os.path.join(os.path.dirname(__file__), "../../.."))
CLI_BIN = ["-p", "aethelgard-cli", "--bin", "aethelgard", "--"]
M = [
    # witnesses
    ("W1 output: BSRR after MODER passes", "crates/aethelgard-cli/src/pinwitness.rs",
     'w.status = if w.bsrr.is_some() {\n                        "mismatch"', 'w.status = if w.bsrr.is_some() {\n                        "witnessed"', CLI_BIN + ["an_output_is_witnessed"]),
    ("W2 output: the other level's bit allowed", "crates/aethelgard-cli/src/pinwitness.rs",
     "let alone = last & o.bsrr_bit() != 0 && last & o.bsrr_other() == 0;", "let alone = last & o.bsrr_bit() != 0;", CLI_BIN + ["an_output_is_witnessed"]),
    ("W3 input: judged against active, not the driven level", "crates/aethelgard-cli/src/pinwitness.rs",
     "Some(l) if (l == 1) == i.high =>", "Some(l) if (l == 1) == i.active_high =>", CLI_BIN + ["an_input_is_read_back"]),
    ("W5 level: idle high settles at the anchor", "crates/aethelgard-cli/src/renode.rs",
     "m.observer.period_ns + aethelgard_twin::boot::QUANTUM_NS", "0", CLI_BIN + ["a_static_level_is_witnessed"]),
    ("W6 level: edges after settling ignored", "crates/aethelgard-cli/src/renode.rs",
     "if window && w.rises_after == 0 && w.falls_after == 0 && w.end_level == Some(high) {",
     "if window && w.end_level == Some(high) {", CLI_BIN + ["a_static_level_is_witnessed"]),
    ("W7 stimuli not re-applied after a reset", "crates/aethelgard-cli/src/renode.rs",
     "            c.extend(crate::pinwitness::stimuli(self.pins));\n", "", CLI_BIN + ["a_pin_device_s_stimuli"]),
    ("W8 image: an input imaged as an output", "crates/aethelgard-twin/src/boot.rs",
     "aethelgard_ir::pin_devices::Direction::Input => MODE_INPUT,", "aethelgard_ir::pin_devices::Direction::Input => MODE_OUTPUT,",
     ["-p", "aethelgard-twin", "--test", "spi_platform", "--", "a_topology_with_a_pin_device"]),
    # fault kinds
    ("F1 pin_held's level byte swapped on decode", "crates/aethelgard-chaos/src/record.rs",
     "                0 => Level::Low,\n                1 => Level::High,", "                1 => Level::Low,\n                0 => Level::High,",
     ["-p", "aethelgard-chaos", "--test", "pin_devices"]),
    ("F2 analog_value's bound one high", "crates/aethelgard-chaos/src/campaign.rs",
     "Ok(millivolts) if millivolts <= ANALOG_MAX_MILLIVOLTS =>", "Ok(millivolts) if millivolts <= ANALOG_MAX_MILLIVOLTS + 1 =>",
     ["-p", "aethelgard-chaos", "--test", "pin_devices"]),
    ("F3 replay: an asserted input needs no finding", "crates/aethelgard-chaos/src/campaign.rs",
     '"witnessed" if !*output && *level == *active => {', '"witnessed" if false => {',
     ["-p", "aethelgard-chaos", "--test", "pin_devices"]),
    ("F4 replay: kind 25's idle-high high time", "crates/aethelgard-chaos/src/campaign.rs",
     "record::MotorIdle::High => (ccr == arr1, 1_000_000_000_000 / rate.max(1)),", "record::MotorIdle::High => (ccr == arr1, 0),",
     ["-p", "aethelgard-chaos", "--test", "motor", "replay_re_derives_the_static_check_from"]),
    # gates
    ("G1 per-item gate counts the input once", "crates/aethelgard-cli/src/verify.rs",
     '                .filter(|i| i["verdict"] != "not_judged")\n                .count() as u64,', "                .take(1)\n                .count() as u64,", CLI_BIN + ["the_per_item_gates"]),
    ("G2 an asserted input passes the gate", "crates/aethelgard-cli/src/verify.rs",
     '&& findings.iter().all(|f| f["severity"] != "error")', '', CLI_BIN + ["the_per_item_gates"]),
    ("G3 a bridge passes on one witnessed input", "crates/aethelgard-cli/src/verify.rs",
     '.all(|i| i["status"] == "witnessed");', '.any(|i| i["status"] == "witnessed");', CLI_BIN + ["the_per_item_gates"]),
    # refusals and coverage
    ("R1 PinHeldIsOutput named Unknown", "crates/aethelgard-twin/src/pins.rs",
     "true => TwinError::PinHeldIsOutput { net: net.clone() },", "true => TwinError::PinHeldUnknown { net: net.clone() },",
     ["-p", "aethelgard-twin", "--test", "pins"]),
    ("R2 an unmeasured resolution runs", "crates/aethelgard-twin/src/pins.rs",
     "if bits != MEASURED_BITS {", "if bits != MEASURED_BITS && false {", ["-p", "aethelgard-twin", "--test", "pins"]),
    ("R3 IdleHighNotSimulated never fires", "crates/aethelgard-cli/src/generate.rs",
     "> aethelgard_twin::boot::MAX_VIRTUAL_NS", "> 10 * aethelgard_twin::boot::MAX_VIRTUAL_NS",
     ["-p", "aethelgard-cli", "--test", "pin_device_refusals"]),
    ("R4 the motor rail ignores a declared load", "crates/aethelgard-cli/src/verify.rs",
     "        && !loaded)", "        )", CLI_BIN + ["the_motor_rail_is"]),
    ("R5 an input is an unmet target again (D5)", "crates/aethelgard-cli/src/verify.rs",
     "        for i in &plan.inputs {\n            c.no_device.push(", "        for i in &plan.inputs {\n            c.unmet.push(",
     CLI_BIN + ["a_pin_device_s_adc_input_is_a_target"]),
    ("R6 a hold at the pull's level changes the run", "crates/aethelgard-cli/src/verify.rs",
     "(i.high != (*level == aethelgard_chaos::record::Level::High)).then_some((None, None))",
     "Some((None, None))", CLI_BIN + ["a_held_input_seeds_nothing"]),
    ("R7 ledger: a pin device's BSRR silicon-only again", "crates/aethelgard-cli/src/ledger.rs",
     "row.class = Class::Observed;", "row.class = Class::SiliconOnly;",
     ["-p", "aethelgard-cli", "--test", "ledger_tier1", "a_pin_device_s_input"]),
    # the adversarial pass's fixes (F1-F10)
    ("A1 replay: a level witness's idle unjoined (F1)", "crates/aethelgard-chaos/src/campaign.rs",
     '_ => return Err("a level witness\'s output does not idle at a static level"),', '_ => false,',
     ["-p", "aethelgard-chaos", "--test", "pin_devices", "a_level_witness_on_a_pulse_idle_output_is_refused"]),
    ("A2 replay: settled instant unjoined (F1)", "crates/aethelgard-chaos/src/campaign.rs",
     "if *settle_ns != settle {", "if false {", ["-p", "aethelgard-chaos", "--test", "pin_devices", "a_level_witness_s_level_and_settled"]),
    ("A3 replay: edges past the idle unjoined (F1)", "crates/aethelgard-chaos/src/campaign.rs",
     'if status == "witnessed" && !(still && window) {', 'if status == "witnessed" && !window {',
     ["-p", "aethelgard-chaos", "--test", "pin_devices", "a_live_pwm_forged"]),
    ("A4 replay: a pin device's pin not its own (F5)", "crates/aethelgard-chaos/src/campaign.rs",
     "        if !taken.insert(pin) {", "        if !taken.insert(pin) && false {",
     ["-p", "aethelgard-chaos", "--test", "pin_devices", "a_pin_device_s_pin_is_its_own"]),
    ("A5 replay: an ADC mismatch at the declared code (F6)", "crates/aethelgard-chaos/src/campaign.rs",
     'if status == "mismatch" && code.is_none_or(|c| c == *declared) {', 'if false {',
     ["-p", "aethelgard-chaos", "--test", "pin_devices", "a_mismatch_reads_other"]),
    ("A6 output: an ODR write before MODER ignored (F2)", "crates/aethelgard-cli/src/pinwitness.rs",
     'w.status = if alone && !odr {', 'w.status = if alone {', CLI_BIN + ["an_output_is_witnessed"]),
        ("A8 ADC: no conversion needed (F3)", "crates/aethelgard-cli/src/pinwitness.rs",
     "match (started, word) {", "match (true, word) {", CLI_BIN + ["an_adc_input_needs_a_conversion"]),
    ("A9 gpio gate: an input judged again (F4)", "crates/aethelgard-cli/src/verify.rs",
     'if g["direction"] == "input" {', 'if g["direction"] == "never" {', CLI_BIN + ["the_per_item_gates"]),
    ("A10 plan: a declared direction unjoined (F7)", "crates/aethelgard-twin/src/pins.rs",
     "if s.output != (solved.direction == aethelgard_ir::pin_devices::Direction::Output) {", "if false {",
     ["-p", "aethelgard-twin", "--test", "pins"]),
    ("A11 level: a short window passes (F9)", "crates/aethelgard-cli/src/renode.rs",
     "let window = settle.saturating_add(m.observer.period_ns) <= end_ns;", "let window = true;", CLI_BIN + ["a_static_level_is_witnessed"]),
    ("A12 a refused hold is a baseline (F10)", "crates/aethelgard-cli/src/verify.rs",
     "            else {\n                return Some((None, None));\n            };\n            (i.high", "            else {\n                return None;\n            };\n            (i.high",
     CLI_BIN + ["a_held_input_seeds_nothing"]),
]
want = set(sys.argv[1:])
subprocess.run(["rm", "-rf", SRC], check=True); os.makedirs(SRC)
subprocess.run(f"git -C {REPO} archive HEAD | tar -x -C {SRC}", shell=True, check=True)
env = dict(os.environ, CARGO_TARGET_DIR=TARGET, PATH=f"{HOME}/.cargo/bin:" + os.environ["PATH"])
rows = []
for mid, path, old, new, test in M:
    if want and mid.split()[0] not in want: continue
    f = os.path.join(SRC, path); orig = open(f).read()
    if orig.count(old) != 1:
        rows.append((mid, f"site-not-unique ({orig.count(old)})")); print(rows[-1], flush=True); continue
    open(f, "w").write(orig.replace(old, new))
    r = subprocess.run(["nice", "-n", "10", "cargo", "test", "-q"] + test, cwd=SRC, env=env, capture_output=True, text=True)
    out = r.stdout + r.stderr
    if "could not compile" in out or "error[E" in out: res = "invalid (does not build)"
    elif r.returncode == 0: res = "SURVIVED"
    else: res = "killed"
    open(f, "w").write(orig)
    rows.append((mid, res)); print(rows[-1], flush=True)
print("\n".join(f"{m}\t{r}" for m, r in rows))
print(f"killed {sum(r == 'killed' for _, r in rows)} of {len(rows)}")
