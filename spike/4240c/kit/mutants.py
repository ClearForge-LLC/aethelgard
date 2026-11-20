"""AET-WO-4240c mutants: one per guard this work order adds (CLAUDE.md global: sized to the diff).
Each applies one exact replacement to one file, runs the named tests, and restores the file from git.
Run from the repository root with every change committed (the restore is `git checkout -- FILE`).
Usage: python3 -I spike/4240c/kit/mutants.py TARGET_DIR OUT_LOG [M1,M2,...]"""
import subprocess, sys

TARGET, LOG = sys.argv[1], sys.argv[2]
HSE = ["-p", "aethelgard-cli", "--test", "hse_eda_tier1"]

M = [
    ("M1 router offers an oscillator net a via", "crates/aethelgard-eda/src/route.rs",
     "*free = board.stack.oscillator.iter().any(|o| o == net);", "*free = false;", HSE),
    ("M2 no oscillator nets on the stack", "crates/aethelgard-eda/src/stack.rs",
     "oscillator: aethelgard_ir::names::oscillator_nets(spec)\n", "oscillator: Vec::<String>::new()\n", HSE),
    ("M3 OscillatorNetVia never raised", "crates/aethelgard-eda/src/extract.rs",
     "(self.line.via_count > 0).then(", "(self.line.via_count > 1).then(", HSE),
    ("M4 Cs dropped from the report (A1): no pin condition", "crates/aethelgard-cli/src/extract.rs",
     "conditions.push(bound::OSC_PIN_CONDITION.to_owned());", "", HSE),
    ("M5 Cs dropped from the report (A1): no oscillator objects", "crates/aethelgard-cli/src/extract.rs",
     "let oscillator: Vec<Value> = oscillator.iter().map(oscillator_net).collect();",
     "let oscillator: Vec<Value> = Vec::new();", HSE),
    ("M6 the crystal gap taken as the largest", "crates/aethelgard-eda/src/extract.rs",
     ".map(move |b| crate::loops::gap_nm(a, b))\n            })\n            .min()",
     ".map(move |b| crate::loops::gap_nm(a, b))\n            })\n            .max()", HSE),
    ("M7 gate: floating crystal ground passes", "crates/aethelgard-eda/src/support.rs",
     "if n != Some(gnd) {", "if n.is_none() {", HSE),
    ("M8 gate: dielectric not judged", "crates/aethelgard-eda/src/support.rs",
     "row.filter(|k| k.dielectric == power_parts::Dielectric::C0g)", "row", HSE),
    ("M17 gate: every MCU pin left off the extra-part check (F1)", "crates/aethelgard-eda/src/support.rs",
     "!(r == mcu && p == pin) && r != *y", "r != mcu && r != *y", HSE),
    ("M18 gate: tolerance not judged (F2)", "crates/aethelgard-eda/src/support.rs",
     "        if !close {\n            fail(\n                \"hse_load_capacitor_tolerance\"",
     "        if false && !close {\n            fail(\n                \"hse_load_capacitor_tolerance\"", HSE),
    ("M9 gate: wrong-net capacitor passes (one cap per net not required)", "crates/aethelgard-eda/src/support.rs",
     "let [c] = caps.as_slice() else {", "let [c, ..] = caps.as_slice() else {", HSE),
    ("M10 gate: crystal on a board declaring none passes", "crates/aethelgard-eda/src/support.rs",
     '"hse_crystal_undeclared",', '"support_pin_off_net",', HSE),
    ("M11 emit: external clock drawn as nothing", "crates/aethelgard-eda/src/schematic_hse.rs",
     "Some(Hse::External(_)) => return Err(EmitError::HseSourceNotDrawn),", "Some(Hse::External(_)) => return Ok(None),", HSE),
    ("M12 emit: load capacitor of any dielectric", "crates/aethelgard-eda/src/schematic_hse.rs",
     "k.dielectric == Dielectric::C0g\n", "k.dielectric != Dielectric::X5r\n",
     ["-p", "aethelgard-eda", "--lib", "schematic::hse"]),
    ("M13 emit: crystal grounds on the oscillator net", "crates/aethelgard-eda/src/schematic_hse.rs",
     "(\"2\".to_owned(), gnd.clone()),", "(\"2\".to_owned(), Conn::Net(osc_in.clone())),", HSE),
    ("M14 support group ties OSC pins on every board", "crates/aethelgard-eda/src/schematic_support.rs",
     "if OSCILLATOR.contains(&group.as_str()) {", "if OSCILLATOR.contains(&group.as_str()) && b.spec.mcu.hse.is_some() {", HSE),
    ("M15 own_nets names the oscillator nets on every board", "crates/aethelgard-ir/src/names.rs",
     "    let Some(crate::clock::Hse::Crystal(_)) = &spec.mcu.hse else {\n        return Vec::new();\n    };\n", "", HSE),
    ("M16 private symbol fields reach the footprint", "crates/aethelgard-eda/src/pcb.rs",
     ".filter(|p| !library::private(p))", "", HSE),
]

ONLY = sys.argv[3].split(",") if len(sys.argv) > 3 else None
out = open(LOG, "w")
for name, path, old, new, args in M:
    if ONLY and name.split()[0] not in ONLY:
        continue
    src = open(path).read()
    if src.count(old) != 1:
        out.write(f"{name}: NOT APPLIED ({src.count(old)} matches)\n"); out.flush(); continue
    open(path, "w").write(src.replace(old, new))
    try:
        r = subprocess.run(["cargo", "test", "-j", "6", *args], env={**__import__("os").environ, "CARGO_TARGET_DIR": TARGET},
                           capture_output=True, text=True, timeout=3000)
        text = r.stdout + r.stderr
        failed = [l for l in text.splitlines() if l.startswith("test ") and l.endswith("FAILED")]
        verdict = "KILLED" if r.returncode != 0 else "SURVIVED"
        why = "; ".join(failed) or ("build error" if "error[" in text else "")
        out.write(f"{name}: {verdict} {why}\n"); out.flush()
    finally:
        subprocess.run(["git", "checkout", "--", path], check=True)
out.close()
