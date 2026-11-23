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
     "        if SWD.contains(&pin.as_str()) {",
     "        if SWD.contains(&pin.as_str()) && false {",
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
     "            .is_some_and(|after| after.starts_with('('))",
     "            .is_some_and(|after| after.trim_start().starts_with('('))",
     ["-p", "aethelgard-pincheck", "--test", "identity", "a_function_like_define_of_a_name_the_judge_reads_is_refused"]),
    ("c-order", C,
     "    aethelgard_pincheck::admit(&topology, &table).map_err(refused)?;\n\n    let elf_bytes = read(elf)?;",
     "    let elf_bytes = read(elf)?;\n    aethelgard_pincheck::admit(&topology, &table).map_err(refused)?;",
     ["-p", "aethelgard-cli", "--test", "pinout_tier1", "a_topology_refusal_is_reported_beside_an_unreadable_elf"]),
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
