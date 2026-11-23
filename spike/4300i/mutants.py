"""AET-WO-4300i: one mutant per guard this work order added, each run against the test that should kill it.
usage (from the repo root, work committed): python3 -I spike/4300i/mutants.py > spike/4300i/out/mutants.tsv
Each mutant is applied, `cargo test -p aethelgard-ir --test <t> <filter>` is run, and the file is restored with
`git checkout -- <file>`. A mutant that does not compile is reported as such, never as killed."""
import os, subprocess, sys
IR = "crates/aethelgard-ir/"
OB, SO, PD, LIB, SI, NA, PP = (IR + f for f in ("src/offboard.rs", "src/solve.rs", "src/pin_devices.rs",
                                                "src/lib.rs", "src/silicon.rs", "src/names.rs", "src/power_parts.rs"))
M = [
 ("M1", OB, "if r.to != crate::GND {", "if false && r.to != crate::GND {", "offboard", "a_pull_that_is_not_to_gnd"),
 ("M2", OB, "Some(power_parts::Kind::Resistor(_))", "Some(_)", "offboard", "a_pull_whose_part_is_not_a_resistor_row"),
 ("M3", OB, "if ohms < rpd.min || ohms > rpd.max {", "if ohms < rpd.min {", "offboard", "a_pull_outside_rpd"),
 ("M4", OB, "if ohms < rpd.min || ohms > rpd.max {", "if ohms <= rpd.min || ohms > rpd.max {", "offboard", "a_pull_outside_rpd"),
 ("M5", OB, "if ohms < rpd.min || ohms > rpd.max {", "if ohms < rpd.min || ohms >= rpd.max {", "offboard", "a_pull_outside_rpd"),
 ("M6", OB, "if !seen.insert(r.pin.as_str()) {", "if false && !seen.insert(r.pin.as_str()) {", "offboard", "a_pin_reserved_twice"),
 ("M7", OB, "if engine.contains(&r.pin.as_str()) {", "if false && engine.contains(&r.pin.as_str()) {", "offboard", "a_pin_the_engine_reserves"),
 ("M8", OB, "if spec.motor_outputs.is_empty() && hands.is_empty() {", "if spec.motor_outputs.is_empty() || hands.is_empty() {", "offboard", "an_mcu_with_no_row"),
 ("M9", OB, "if !is_ft(structure) {", "if false && !is_ft(structure) {", "offboard", "a_hand_tt_pin_at_a_connector"),
 ("M10", OB, "let Some((structure, at)) = io.pins.get(pin) else {", "let Some((structure, at)) = io.pins.get(pin).or_else(|| io.pins.get(\"PA0\")) else {", "offboard", "a_hand_pin_at_a_connector_with_no_held_structure"),
 ("M11", OB, ".filter(|(_, net, _)| facing.contains(net))", ".filter(|(_, net, _)| facing.contains(net) && false)", "offboard", "a_hand_tt_pin_at_a_connector"),
 ("M12", OB, "structure.split('_').next() == Some(\"FT\")", "structure.starts_with(\"FT_\")", "offboard", "a_hand_ft_pin_of_bare_structure"),
 ("M13", OB, "let (mut first, rest): (Vec<T>, Vec<T>) = items.into_iter().partition(|t| ft.contains(pin(t)));",
             "let (mut first, rest): (Vec<T>, Vec<T>) = items.into_iter().partition(|t| !ft.contains(pin(t)));", "offboard", "a_connector_facing_"),
 ("M14", OB, ".filter_map(|p| p.net.as_deref())", ".filter_map(|_| None::<&str>)", "offboard", "a_connector_facing_"),
 ("M15", LIB, "board::source(&m.pull.source, &format!(\"{at}.source\"))?;", "", "offboard", "a_blank_source_or_reference"),
 ("M16", LIB, "board::source(&r.reason, &format!(\"mcu.reserved_pins[{i}].reason\"))?;", "", "offboard", "a_blank_reason"),
 ("M17", NA, ".map(|m| m.pull.resistor.reference.as_str()),", ".map(|m| m.pull.resistor.reference.as_str()).filter(|_| false),", "offboard", "a_pull_reference_another_part_holds"),
 ("M18", PP, "if raw.min <= 0 || raw.min > raw.typical || raw.typical > raw.max {", "if raw.min > raw.typical || raw.typical > raw.max {", "offboard", "the_mcu_row_holds_rpd"),
 ("M19", PP, "if raw.min <= 0 || raw.min > raw.typical || raw.typical > raw.max {", "if raw.min <= 0 || raw.typical > raw.max {", "offboard", "the_mcu_row_holds_rpd"),
 ("M20", PP, "if raw.min <= 0 || raw.min > raw.typical || raw.typical > raw.max {", "if raw.min <= 0 || raw.min > raw.typical {", "offboard", "the_mcu_row_holds_rpd"),
 ("M21", SO, "    pins.extend(spec.mcu.reserved_pins.iter().map(|r| r.pin.as_str()));\n", "", "offboard", "never_"),
 ("M22", SO, "    spec.validate().map_err(SolveError::InvalidSpecification)?;\n    let reserved = all_reserved(spec);",
             "    spec.validate().map_err(SolveError::InvalidSpecification)?;\n    let reserved = reserved_pins(spec.mcu.hse.as_ref());", "offboard", "never_"),
 ("M23", SO, "        let reserved = all_reserved(spec);\n        let pads", "        let reserved = reserved_pins(spec.mcu.hse.as_ref());\n        let pads", "offboard", "never_"),
 ("M24", SO, "if !table.pins.contains_key(&r.pin) {", "if false && !table.pins.contains_key(&r.pin) {", "offboard", "a_reservation_of_an_unknown_pin"),
 ("M25", SO, "format!(\"`mcu.reserved_pins`: {}\", r.reason)", "ENGINE_RESERVED.to_owned()", "offboard", "a_hand_pin_on_a_declared_reservation"),
 ("M26", SO, ".ft_first(self.model.motors[m].label, options, |c| c.2);", ".ft_first(\"\", options, |c| c.2);", "offboard", "a_connector_facing_motor"),
 ("M27", SO, "self.model.ft_first(port.tx.net, pins(p, &port.tx), |c| c.0);", "self.model.ft_first(\"\", pins(p, &port.tx), |c| c.0);", "offboard", "a_connector_facing_port"),
 ("M28", SO, "self.model.ft_first(port.rx.net, pins(p, &port.rx), |c| c.0);", "self.model.ft_first(\"\", pins(p, &port.rx), |c| c.0);", "offboard", "a_connector_facing_port"),
 ("M29", SO, "model.ft_first(sig.net, model.pins_for", "model.ft_first(\"\", model.pins_for", "offboard", "a_connector_facing_bus_line"),
 ("M30", SO, "ft.order(cs.net, keys, |p| p)", "ft.order(\"\", keys, |p| p)", "offboard", "a_connector_facing_chip_select"),
 ("M31", PD, "ft.order(&a.net, free, |(n, _)| n);", "ft.order(\"\", free, |(n, _)| n);", "offboard", "a_connector_facing_adc_input"),
 ("M32", PD, ".order(\n                        &s.net,", ".order(\n                        \"\",", "offboard", "a_connector_facing_device_signal"),
 ("M34", LIB, "plain(&r.pin, || format!(\"mcu.reserved_pins[{i}].pin\"))?;", "", "offboard", "a_blank_reserved_pin"),
 ("M35", LIB, "identifier(IdentifierKind::Reference, &r.reference, || {\n                format!(\"{at}.resistor.reference\")\n            })?;", "", "offboard", "a_blank_source_or_reference"),
 ("M36", LIB, "plain(&r.part, || format!(\"{at}.resistor.part\"))?;", "", "offboard", "a_blank_source_or_reference"),
 ("M37", LIB, "plain(&r.to, || format!(\"{at}.resistor.to\"))?;", "", "offboard", "a_blank_source_or_reference"),
 ("M33", SI, "(Some(_), _) => 0,", "(Some(_), _) => PUPDR_PULL_DOWN,", "offboard", "a_motor_net_has_no_internal_pull"),
]
env = dict(os.environ, CARGO_TARGET_DIR=os.path.expanduser("~/.cache/aet-4300i/target"),
           PATH=os.path.expanduser("~/.cargo/bin") + ":" + os.environ["PATH"])
only = set(sys.argv[1:])
print("id\tfile\tmutant\ttest\tresult")
for mid, f, old, new, tbin, flt in M:
    if only and mid not in only:
        continue
    t = open(f).read()
    assert t.count(old) == 1, (mid, old)
    assert subprocess.run(["git", "diff", "--quiet", "--", f]).returncode == 0, f"{f} is dirty"
    open(f, "w").write(t.replace(old, new))
    try:
        r = subprocess.run(["nice", "-n", "10", "cargo", "test", "-q", "-p", "aethelgard-ir", "--test", tbin, flt],
                           env=env, capture_output=True, text=True)
        out = r.stdout + r.stderr
        if "could not compile" in out:
            res = "NO-COMPILE"
        elif r.returncode != 0 and "FAILED" in out:
            res = "killed"
        elif r.returncode == 0 and " 0 passed" in out:
            res = "NO-TEST-RAN"
        else:
            res = "SURVIVED" if r.returncode == 0 else f"rc{r.returncode}"
    finally:
        subprocess.run(["git", "checkout", "--", f], check=True)
    oneline = lambda s: s.strip().replace("\n", " ").replace("\t", " ")
    print(f"{mid}\t{f.replace(IR, '')}\t`{oneline(old)}` -> `{oneline(new)}`\t{tbin}::{flt}\t{res}", flush=True)
