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
     "if b < m && word & o.bsrr_bit()", "if word & o.bsrr_bit()", CLI_BIN + ["an_output_is_witnessed"]),
    ("W2 output: the other level's bit allowed", "crates/aethelgard-cli/src/pinwitness.rs",
     "word & o.bsrr_bit() != 0 && word & o.bsrr_other() == 0", "word & o.bsrr_bit() != 0", CLI_BIN + ["an_output_is_witnessed"]),
    ("W3 input: judged against active, not the driven level", "crates/aethelgard-cli/src/pinwitness.rs",
     "Some(l) if (l == 1) == i.high =>", "Some(l) if (l == 1) == i.active_high =>", CLI_BIN + ["an_input_is_read_back"]),
    ("W4 ADC: no firmware read needed", "crates/aethelgard-cli/src/pinwitness.rs",
     "(0, _) | (_, None) =>", "(_, None) =>", CLI_BIN + ["an_adc_input_needs"]),
    ("W5 level: idle high settles at the anchor", "crates/aethelgard-cli/src/renode.rs",
     "m.observer.period_ns + aethelgard_twin::boot::QUANTUM_NS", "0", CLI_BIN + ["a_static_level_is_witnessed"]),
    ("W6 level: edges after settling ignored", "crates/aethelgard-cli/src/renode.rs",
     "w.status = if w.rises_after == 0 && w.falls_after == 0 && w.end_level == Some(high) {",
     "w.status = if w.end_level == Some(high) {", CLI_BIN + ["a_static_level_is_witnessed"]),
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
     "scope: Some(Scope::of(items.len() as u64, NOTHING_LISTED)),", "scope: Some(Scope::of(1, NOTHING_LISTED)),", CLI_BIN + ["the_per_item_gates"]),
    ("G2 an asserted input's item passes", "crates/aethelgard-cli/src/verify.rs",
     '"verdict": if ok(g) && !asserted { "pass" } else { "fail" },', '"verdict": if ok(g) { "pass" } else { "fail" },', CLI_BIN + ["the_per_item_gates"]),
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
