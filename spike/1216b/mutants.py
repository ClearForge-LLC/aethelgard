"""AET-WO-1216b: one mutant per guard this work order adds, each against the
suites it lives in. Each mutant is one exact text replacement in a copy of
the tree (never the worktree); a mutant is killed when its suites fail.

usage: python3 -I mutants.py <tree copy> <cargo target dir> -> JSON on stdout
"""
import json
import subprocess
import sys

IR = "crates/aethelgard-ir/src/off_domain.rs"
PP = "crates/aethelgard-ir/src/power_parts.rs"
PINP = "crates/aethelgard-ir/src/pin_parts.rs"
LIB = "crates/aethelgard-ir/src/lib.rs"
LED = "crates/aethelgard-cli/src/ledger.rs"
IR_TESTS = ["-p", "aethelgard-ir", "--test", "off_domain", "--test", "pin_devices"]
CLI_TESTS = ["-p", "aethelgard-cli", "--test", "ledger_tier1"]

MUTANTS = [
    ("judged ignores the lead", IR,
     "matches!(self.lead, Lead::Leads(_)) && matches!(self.bound, Bound::Stated { .. })",
     "matches!(self.bound, Bound::Stated { .. })", IR_TESTS),
    ("the limit admits equality as a refusal", IR,
     "if *millivolts > limit {", "if *millivolts >= limit {", IR_TESTS),
    ("step 25 not called", LIB,
     "pin_devices::validate(self)?;\n        // 25 (`AET-WO-1216b`): each path from off `mcu.supply` at VDD = 0\n        off_domain::validate(self)",
     "pin_devices::validate(self)", IR_TESTS),
    ("a rail with no regulator is exempt", IR,
     'return Lead::Leads(format!(\n                "no regulator in the drawn tree outputs',
     'return Lead::After(format!(\n                "no regulator in the drawn tree outputs', IR_TESTS),
    ("the battery is exempt", IR,
     'return Lead::Leads("the battery is the tree\'s root".to_owned());',
     'return Lead::After("the battery is the tree\'s root".to_owned());', IR_TESTS),
    ("downstream ignores connectors", IR,
     "return off_board(&chain[..at]).unwrap_or_else(",
     "return None.unwrap_or_else(", IR_TESTS),
    ("downstream guards the rail alone, not the rails between", IR,
     "return off_board(&chain[..at]).unwrap_or_else(",
     "return off_board(&chain[..1]).unwrap_or_else(", IR_TESTS),
    ("an upstream PG grants the exemption", IR,
     "let below = self.chain(r.output.as_str()).contains(&supply);\n            r.controls",
     "let below = true;\n            r.controls", IR_TESTS),
    ("an EN/PG-gated rail ignores off-board ends", IR,
     "Some((by, net)) => off_board(&[rail, net]).unwrap_or_else(",
     "Some((by, net)) => None.unwrap_or_else(", IR_TESTS),
    ("an EN/PG gate ignores an off-board end on its EN net (F1)", IR,
     "Some((by, net)) => off_board(&[rail, net]).unwrap_or_else(",
     "Some((by, net)) => off_board(&[rail]).unwrap_or_else(", IR_TESTS),
    ("a test point is no off-board end (F3)", IR,
     "        for t in spec.declared_test_points() {\n            off_board",
     "        for t in spec.declared_test_points().iter().take(0) {\n            off_board", IR_TESTS),
    ("a control pin judged by the rail it makes (F2)", IR,
     "                    vec![r.input.clone()],", "                    vec![r.output.clone()],", IR_TESTS),
    ("two supplies: only the first is judged", IR,
     "for rail in rails.iter().filter(|r| r.as_str() != self.supply) {",
     "for rail in rails.iter().take(1).filter(|r| r.as_str() != self.supply) {", IR_TESTS),
    ("two supplies: exempt if any is exempt", IR,
     "        let mut after = Vec::new();\n        for rail in rails.iter().filter(|r| r.as_str() != self.supply) {\n            match self.rail_lead(rail) {\n                Lead::Leads(why) => return Some(Lead::Leads(why)),\n                Lead::After(why) => after.push(why),",
     "        let mut after: Vec<String> = Vec::new();\n        for rail in rails.iter().filter(|r| r.as_str() != self.supply) {\n            match self.rail_lead(rail) {\n                Lead::Leads(why) => after.push(why),\n                Lead::After(why) => return Some(Lead::After(why)),", IR_TESTS),
    ("a pull on mcu.supply is listed", IR,
     "            lead: self.lead(&rails)?,",
     "            lead: self.lead(&rails).unwrap_or(Lead::Leads(String::new())),", IR_TESTS),
    ("BOOT0 takes any-other-pin's limit", PP,
     "Some(crate::BOOT0) => (a.boot0.value, &a.boot0.at),",
     "Some(crate::BOOT0) => (a.other.value, &a.other.at),", IR_TESTS),
    ("a held pin takes Table 13's family", PP,
     "Some(p) => match self.pins.get(p).and_then(|(s, _)| family(s)) {",
     "Some(p) => match self.pins.get(p).and_then(|(s, _)| self.family(s)) {", IR_TESTS),
    ("an unplaced signal takes the most, not the least", PP,
     ".min_by_key(|(v, _)| *v)\n            .unwrap_or((a.other.value, &a.other.at))",
     ".max_by_key(|(v, _)| *v)\n            .unwrap_or((a.other.value, &a.other.at))", IR_TESTS),
    ("the current output at R2's nominal", IR,
     "ceil_div(i128::from(*output_max_ua) * r_hi, 1000)",
     "ceil_div(i128::from(*output_max_ua) * i128::from(c.resistor.ohms), 1000)", IR_TESTS),
    ("the basis is always a maximum", IR,
     "basis: *output_max_basis,", "basis: Basis::Maximum,", IR_TESTS),
    ("a connector pin on any net is a path", IR,
     "let Some(net) = pin.net.as_deref().filter(|n| w.mcu_nets.contains(n)) else {",
     "let Some(net) = pin.net.as_deref() else {", IR_TESTS),
    ("a test point on any net is a path", IR,
     "        if w.mcu_nets.contains(t.net.as_str()) {\n            out.push(w.off_board(",
     "        if true {\n            out.push(w.off_board(", IR_TESTS),
    ("a foreign clock is not listed", IR,
     "if let Some(crate::clock::Hse::External(x)) = &spec.mcu.hse {",
     "if let Some(crate::clock::Hse::External(x)) = None::<&crate::clock::Hse> {", IR_TESTS),
    ("Table 10's families unchecked against Table 13's", PP,
     ".ne(families.iter().map(|f| &f.family))",
     ".ne(stress.iter().map(|f| &f.family))", IR_TESTS),
    ("an unknown basis reads as a maximum", PINP,
     '"maximum" => Basis::Maximum,', '"maximum" | "typical" => Basis::Maximum,', IR_TESTS),
    ("a stated path gets a twin-limit row", LED,
     "let Bound::Unbounded(why) = &p.bound else {\n            continue;\n        };",
     'let why = match &p.bound { Bound::Unbounded(w) => w.clone(), _ => String::new() };', CLI_TESTS),
    ("an unbounded row counted as judged", LED,
     "            class: Class::SiliconOnly,\n            judge: format!(\n                \"not judged: no bound",
     "            class: Class::Observed,\n            judge: format!(\n                \"not judged: no bound", CLI_TESTS),
    ("no unbounded row is written", LED,
     "        out.push(LedgerRow {\n            id: format!(\"OFFDOMAIN.{}\", p.at),",
     "        if false { out.push(LedgerRow {\n            id: format!(\"OFFDOMAIN.{}\", p.at),", CLI_TESTS),
]


def run(tree, target, args):
    r = subprocess.run(["nice", "-n", "10", "cargo", "test", "-q", "--target-dir", target, "-j", "6", *args],
                       cwd=tree, capture_output=True, text=True)
    if "error[E" in r.stderr or "error: could not compile" in r.stderr:
        return None
    return r.returncode


def main(tree, target):
    out = []
    for name, rel, old, new, suites in MUTANTS:
        path = f"{tree}/{rel}"
        src = open(path).read()
        if src.count(old) != 1:
            out.append({"mutant": name, "result": f"ANCHOR x{src.count(old)}"})
            continue
        mutated = src.replace(old, new)
        if name == "no unbounded row is written":
            # close the `if false {` block after the push's `});`
            i = mutated.index("if false { out.push(LedgerRow {")
            j = mutated.index("        });\n", i) + len("        });\n")
            mutated = mutated[:j] + "        }\n" + mutated[j:]
        open(path, "w").write(mutated)
        try:
            rc = run(tree, target, suites)
        finally:
            open(path, "w").write(src)
        out.append({"mutant": name, "result": "DID NOT COMPILE" if rc is None else "killed" if rc != 0 else "SURVIVED"})
        print(json.dumps(out[-1]), file=sys.stderr, flush=True)
    print(json.dumps(out, indent=1))


if __name__ == "__main__":
    main(*sys.argv[1:3])
