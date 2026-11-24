#!/usr/bin/env python3
"""AET-WO-4410: one mutant per guard this work order adds or changes, each run
against the test that should kill it. Each source edit is reverted in a
`finally`, byte for byte. Run from the repository root, with CARGO_TARGET_DIR
set, inside a capped scope. Writes one TSV row per mutant to stdout."""
import os
import subprocess
import sys

H = "crates/aethelgard-hwdef/src/lib.rs"
P = "crates/aethelgard-pincheck/src/lib.rs"
R = "crates/aethelgard-pincheck/src/header.rs"
C = "crates/aethelgard-cli/src/pinout.rs"
S = "crates/aethelgard-ir/src/solve.rs"
PP = "crates/aethelgard-ir/src/power_parts.rs"
I = "crates/aethelgard-pincheck/src/identity.rs"

# (id, file, old, new, cargo test args)
MUTANTS = [
    ("h-swd-lines", H,
     "    for (_, line) in SWD_LINES {\n        l.push(",
     "    for (_, line) in SWD_LINES.into_iter().take(1) {\n        l.push(",
     ["-p", "aethelgard-hwdef", "--test", "generate", "every_board_keeps_swd_on"]),
    ("h-swd-solved", H,
     "        if SWD_LINES.iter().any(|(swd, _)| swd == pin) {",
     "        if SWD_LINES.iter().any(|(swd, _)| swd == pin && false) {",
     ["-p", "aethelgard-hwdef", "--test", "generate", "a_net_solved_on_an_swd_pin_is_refused"]),
    ("h-swd-solved-gpio", H,
     "        .chain(t.gpio.iter().map(|(net, g)| (net, &g.pin)))\n        .chain(t.adc_inputs",
     "        .chain(t.adc_inputs",
     ["-p", "aethelgard-hwdef", "--test", "generate", "a_net_solved_on_an_swd_pin_is_refused"]),
    ("h-adc1", H,
     '.filter(|a| a.peripheral != "ADC1")',
     '.filter(|a| a.peripheral == "ADC2")',
     ["-p", "aethelgard-hwdef", "--test", "generate", "a_sense_solved_on_adc3_alone_is_refused_by_name"]),
    ("h-f2-af-prefix", H,
     "        if let Some(prefix) = AF_LABEL_PREFIXES.iter().find(|p| net.starts_with(*p)) {",
     "        if let Some(prefix) = AF_LABEL_PREFIXES.iter().find(|p| net.starts_with(*p) && false) {",
     ["-p", "aethelgard-hwdef", "--test", "generate", "a_chip_select_named_like_an_alternate_function_is_refused"]),
    ("h-f1-serial0", H,
     "and SERIAL0 when GCS looks up its MAVLink console",
     "and not SERIAL0",
     ["-p", "aethelgard-hwdef", "--test", "generate", "each_board_generates_its_committed_file"]),
    ("p-swd-check", P,
     "        if (mode, af) != pin.reset() {\n            let (want_mode, want_af) = pin.reset();",
     "        if (mode, af) != pin.reset() && mode == Mode::Alternate {\n            let (want_mode, want_af) = pin.reset();",
     ["-p", "aethelgard-pincheck", "--test", "check", "an_image_that_loses_swd_is_named"]),
    ("p-swd-af", P,
     "        if (mode, af) != pin.reset() {\n            let (want_mode, want_af) = pin.reset();",
     "        if mode != pin.reset().0 {\n            let (want_mode, want_af) = pin.reset();",
     ["-p", "aethelgard-pincheck", "--test", "check", "an_image_that_loses_swd_is_named"]),
    ("p-swd-named", P,
     "    if SWD.contains(&pin) {",
     "    if SWD.contains(&pin) && false {",
     ["-p", "aethelgard-pincheck", "--test", "check", "an_swd_pin_named_by_the_topology_or_the_allow_list_is_refused"]),
    ("p-swd-allow", P,
     "        .find(|pin| SWD.contains(&pin.to_string().as_str()))\n    {\n        return allow_refusal(\n            \"AllowNamesSwdPin\",",
     "        .find(|pin| SWD.contains(&pin.to_string().as_str()) && false)\n    {\n        return allow_refusal(\n            \"AllowNamesSwdPin\",",
     ["-p", "aethelgard-pincheck", "--test", "check", "an_swd_pin_named_by_the_topology_or_the_allow_list_is_refused"]),
    ("p-odr", P,
     "        if want != got {\n            drive.push(Finding {\n                check: \"forward\",\n                kind: \"WrongOutputLevel\",",
     "        if want != got && false {\n            drive.push(Finding {\n                check: \"forward\",\n                kind: \"WrongOutputLevel\",",
     ["-p", "aethelgard-pincheck", "--test", "check", "a_chip_select_low_from_its_first_write_is_named"]),
    ("p-odr-bit", P,
     "        (self.ports[pin.port].odr >> pin.n) & 1",
     "        (self.ports[pin.port].odr >> (pin.n + 1)) & 1",
     ["-p", "aethelgard-pincheck", "--test", "check", "a_chip_select_low_from_its_first_write_is_named"]),
    ("p-pmcr", P,
     "            mask: 1 << 25,",
     "            mask: 1 << 26,",
     ["-p", "aethelgard-pincheck", "--test", "check", "no_pmcr_writer_at_the_tag_touches_pc2so_or_pc3so"]),
    ("r-fn-value", R,
     "    pub fn value(&self, name: &str) -> Result<Option<String>, Refusal> {\n        refuse_function_like(&self.0, name)?;",
     "    pub fn value(&self, name: &str) -> Result<Option<String>, Refusal> {",
     ["-p", "aethelgard-pincheck", "--test", "identity", "a_function_like_define_of_a_name_the_judge_reads_is_refused"]),
    ("r-fn-definition", R,
     "fn definition(lines: &[(usize, State, String)], name: &str) -> Result<String, Refusal> {\n    refuse_function_like(lines, name)?;",
     "fn definition(lines: &[(usize, State, String)], name: &str) -> Result<String, Refusal> {",
     ["-p", "aethelgard-pincheck", "--test", "check", "a_function_like_register_macro_is_refused_by_name"]),
    ("r-fn-candidates", R,
     "    pub fn candidates(&self, name: &str) -> Result<Vec<String>, Refusal> {\n        refuse_function_like(&self.0, name)?;",
     "    pub fn candidates(&self, name: &str) -> Result<Vec<String>, Refusal> {",
     ["-p", "aethelgard-pincheck", "--test", "identity", "a_function_like_define_of_a_name_the_judge_reads_is_refused"]),
    ("r-fn-blank", R,
     "    define_tail(line, name).is_some_and(|after| after.starts_with('('))",
     "    define_tail(line, name).is_some_and(|after| after.trim_start().starts_with('('))",
     ["-p", "aethelgard-pincheck", "--test", "identity", "a_function_like_define_of_a_name_the_judge_reads_is_refused"]),
    ("c-order", C,
     "    aethelgard_pincheck::admit(&topology, &table).map_err(refused)?;\n\n    let elf_bytes = read(elf)?;",
     "    let elf_bytes = read(elf)?;\n    aethelgard_pincheck::admit(&topology, &table).map_err(refused)?;",
     ["-p", "aethelgard-cli", "--test", "pinout_tier1", "a_topology_refusal_is_reported_beside_an_unreadable_elf"]),
    # The IR half (after AET-WO-4240e merged).
    ("ir-adc1-first", S,
     '                .find(|(.., (p, _))| p == "ADC1")\n',
     '                .find(|(.., (p, _))| p == "ADC1" && false)\n',
     ["-p", "aethelgard-ir", "--test", "sense_pin", "the_sense_takes_an_adc1_pin_before_a_lower_adc3_only_pin"]),
    ("ir-sense-top", S,
     "                    top_ohms: s.top.ohms,",
     "                    top_ohms: s.bottom.ohms,",
     ["-p", "aethelgard-ir", "--test", "sense_pin", "the_sense_takes_an_adc1_pin_before_a_lower_adc3_only_pin"]),
    ("pp-osc-dup", PP,
     "        if oscillators.iter().any(|seen| seen.oscillator_hz == hz) {",
     "        if oscillators.iter().any(|seen| seen.oscillator_hz == hz) && false {",
     ["-p", "aethelgard-ir", "--test", "power", "a_malformed_mcu_ardupilot_column_is_refused"]),
    ("pp-osc-empty", PP,
     "    if m.oscillators.is_empty() {",
     "    if m.oscillators.is_empty() && false {",
     ["-p", "aethelgard-ir", "--test", "power", "a_malformed_mcu_ardupilot_column_is_refused"]),
    ("pp-osc-lookup", PP,
     "        self.oscillators.iter().find(|o| o.oscillator_hz == hz)",
     "        self.oscillators.first()",
     ["-p", "aethelgard-ir", "--test", "power", "the_mcu_row_holds_its_flash_and_ardupilot_s_names_and_kernels"]),
    ("h-osc-select", H,
     "        .oscillator(t.clock.hse_hz)",
     "        .oscillator(0)",
     ["-p", "aethelgard-cli", "--test", "hwdef_tier1", "a_16_mhz_board_generates_and_an_uncited_oscillator_is_refused"]),
    ("h-batt-scale", H,
     "                self.top + self.bottom,",
     "                self.top,",
     ["-p", "aethelgard-hwdef", "--test", "generate", "a_solved_sense_writes_the_battery_monitor"]),
    ("h-batt-type", H,
     '            "define HAL_BATT_MONITOR_DEFAULT 3",',
     '            "define HAL_BATT_MONITOR_DEFAULT 4",',
     ["-p", "aethelgard-hwdef", "--test", "generate", "a_solved_sense_writes_the_battery_monitor"]),
    ("h-batt-adcoff", H,
     '        Some(b) => b.lines(&mut l),',
     '        Some(b) => {\n            b.lines(&mut l);\n            l.push("define HAL_USE_ADC FALSE", "x");\n        }',
     ["-p", "aethelgard-hwdef", "--test", "generate", "a_solved_sense_writes_the_battery_monitor"]),
    ("h-divider-exact", H,
     "    if bottom == 0 || top + bottom > EXACT_IN_FLOAT {",
     "    if bottom == 0 || top + bottom >= EXACT_IN_FLOAT {",
     ["-p", "aethelgard-hwdef", "--test", "generate", "a_divider_with_no_exact_float_quotient_is_refused"]),
    ("h-divider-zero", H,
     "    if bottom == 0 || top + bottom > EXACT_IN_FLOAT {",
     "    if top + bottom > EXACT_IN_FLOAT {",
     ["-p", "aethelgard-hwdef", "--test", "generate", "a_divider_with_no_exact_float_quotient_is_refused"]),
    ("h-sense-twice", H,
     "    if let Some((second, _)) = nets.next() {",
     "    if let Some((second, _)) = nets.next().filter(|_| false) {",
     ["-p", "aethelgard-hwdef", "--test", "generate", "two_battery_senses_are_refused"]),
    ("p-batt-pin", I,
     "        Some(p) if p == channel => {}",
     "        Some(_) => {}",
     ["-p", "aethelgard-pincheck", "--test", "identity", "the_battery_monitor_is_judged_against_the_solved_sense"]),
    ("p-batt-scale", I,
     "        .is_some_and(|(n, d)| n.checked_mul(bottom) == d.checked_mul(top + bottom));",
     "        .is_some_and(|(n, d)| n.checked_mul(bottom) >= d.checked_mul(top + bottom));",
     ["-p", "aethelgard-pincheck", "--test", "identity", "the_battery_monitor_is_judged_against_the_solved_sense"]),
    ("p-batt-type", I,
     "    if monitor != Some(3) {",
     "    if monitor.is_none() {",
     ["-p", "aethelgard-pincheck", "--test", "identity", "the_battery_monitor_is_judged_against_the_solved_sense"]),
    ("p-batt-adcoff", I,
     '    if h.value("HAL_USE_ADC")?.as_deref() == Some("FALSE") {',
     '    if h.value("HAL_USE_ADC")?.as_deref() == Some("FALSE") && false {',
     ["-p", "aethelgard-pincheck", "--test", "identity", "the_battery_monitor_is_judged_against_the_solved_sense"]),
    ("p-batt-row-scale", I,
     "&& !a[2].contains('*')",
     "",
     ["-p", "aethelgard-pincheck", "--test", "identity", "the_battery_monitor_is_judged_against_the_solved_sense"]),
    ("p-batt-row-analog", I,
     "a.len() == 3 && a.get(1).and_then(|n| number(n)) == Some(channel) &&",
     "a.len() == 3 &&",
     ["-p", "aethelgard-pincheck", "--test", "identity", "the_battery_monitor_is_judged_against_the_solved_sense"]),
    ("p-batt-unexpected", I,
     "        if let Some(p) = pin {\n            out.push(\n                \"UnexpectedBattMonitor\",",
     "        if let Some(p) = pin.filter(|_| false) {\n            out.push(\n                \"UnexpectedBattMonitor\",",
     ["-p", "aethelgard-pincheck", "--test", "identity", "a_battery_monitor_with_no_adc1_sense_is_named"]),
    ("p-batt-adc1", I,
     '        .filter(|a| a.peripheral == "ADC1")\n        .and_then(|a| a.channel.strip_prefix("INP"))\n        .and_then(|n| n.parse::<u64>().ok());',
     '        .and_then(|a| a.channel.strip_prefix("INP"))\n        .and_then(|n| n.parse::<u64>().ok());',
     ["-p", "aethelgard-pincheck", "--test", "identity", "a_battery_monitor_with_no_adc1_sense_is_named"]),
    ("p-fraction-frac", I,
     "        Some((n, 10u128.pow(u32::try_from(frac.len()).ok()?)))",
     "        Some((n, 10u128.pow(u32::try_from(frac.len()).ok()? + 1)))",
     ["-p", "aethelgard-pincheck", "--test", "identity", "the_battery_monitor_is_judged_against_the_solved_sense"]),
    ("p-notanalog", P,
     "        if mode != Mode::Analog {",
     "        if mode == Mode::Output {",
     ["-p", "aethelgard-pincheck", "--test", "check", "a_battery_sense_pin_that_is_not_analog_is_named"]),
    ("p-sense-allow", P,
     "            .or_else(|| sensed.get(pin))\n",
     "",
     ["-p", "aethelgard-pincheck", "--test", "check", "a_battery_sense_pin_that_is_not_analog_is_named"]),
]


def main():
    only = set(sys.argv[1:])
    for mid, path, old, new, args in MUTANTS:
        if only and mid not in only:
            continue
        orig = open(path, encoding="utf-8").read()
        if orig.count(old) != 1:
            print(f"{mid}\tNOT-APPLIED\t{path}: the text occurs {orig.count(old)} times", flush=True)
            continue
        try:
            open(path, "w", encoding="utf-8").write(orig.replace(old, new))
            r = subprocess.run(["cargo", "test", "-q", "-j", "6", *args],
                               capture_output=True, text=True)
            out = r.stdout + r.stderr
            if "error[E" in out or "could not compile" in out:
                verdict = "COMPILE-ERROR"
            elif r.returncode != 0 and "FAILED" in out:
                verdict = "KILLED"
            elif r.returncode == 0:
                verdict = "SURVIVED"
            else:
                verdict = f"OTHER(rc={r.returncode})"
            print(f"{mid}\t{verdict}\t{' '.join(args)}", flush=True)
        finally:
            open(path, "w", encoding="utf-8").write(orig)


if __name__ == "__main__":
    main()
