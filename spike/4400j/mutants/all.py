"""AET-WO-4400j: every guard's mutant on the branch's head, one at a time (inside the tools image, at /repo): the
mutation applied, its suite run, the file restored (and touched, so cargo rebuilds). KILLED, SURVIVED, or NOT
APPLIED (the text is not in the file exactly once). Usage: python3 -I all.py > result.txt"""
import os, subprocess, sys
RUN = "crates/aethelgard-cli/src/run.rs"
FOR = "crates/aethelgard-cli/src/renode/foreign.rs"
MAIN = "crates/aethelgard-cli/src/main.rs"
T1 = ["-p", "aethelgard-cli", "--test", "boot_tier1", "--"]
TU = ["-p", "aethelgard-cli", "--bin", "aethelgard", "--"]
TT = ["-p", "aethelgard-cli", "--test", "tools_tier1", "--"]
M = [
    # D1
    ("D1", RUN, "if HELD_STDIN.contains(&self.tool.name) {", "if false {", TT + ["stdin"]),
    ("D1", RUN, "if HELD_STDIN.contains(&self.tool.name) {", "if true {", TT + ["stdin"]),
    ("D1", RUN, 'pub const HELD_STDIN: [&str; 2] = ["renode", crate::analog::BRIDGE];',
     'pub const HELD_STDIN: [&str; 2] = ["renode", "renode"];', T1 + ["the_bridges_stdin"]),
    # D3
    ("D3", MAIN, "&& ns > renode::foreign::FOREIGN_MAX_VIRTUAL_NS =>",
     "&& ns > renode::foreign::FOREIGN_INPUT_MAX_VIRTUAL_NS =>", T1 + ["twelve"]),
    ("D3", MAIN, "&& input.is_none()", "&& true", T1 + ["twelve"]),
    ("D3", FOR, "        window: if input {", "        window: if false {", T1 + ["twelve"]),
    ("D3", MAIN, "    time_up_to(s, renode::foreign::FOREIGN_INPUT_MAX_VIRTUAL_NS)",
     "    time_up_to(s, renode::foreign::FOREIGN_INPUT_MAX_VIRTUAL_NS + 1_000_000)", T1 + ["twelve"]),
    # D4
    ("D4", FOR, "let ok = observed[i].iter().any(|&g| holds(g));", "let ok = holds(got);", T1 + ["four"]),
    ("D4", FOR, "if icsr.iter().all(|i| i & 0x1FF != 0) {", "if vectactive != 0 {", T1 + ["four"]),
    ("D4", FOR, "if begins.len() != END_READS || ends.len() != END_READS {",
     "if begins.len() < END_READS || ends.len() < END_READS {", T1 + ["four"]),
    ("D4", FOR, "    (mask & !fields, mask & fields)", "    (mask, 0)", TU + ["foreign"]),
    ("D4", FOR, '        (true, "CR2") => rc.1,', '        (_, "CR2") => rc.1,', TU + ["foreign"]),
    ("D4", FOR, "std::array::from_fn(|k| virtual_ns.saturating_sub((END_READS - 1 - k) as u64 * q))",
     "std::array::from_fn(|k| virtual_ns.saturating_sub((END_READS - 1 - k) as u64 * q) + q)", TU + ["foreign"]),
    # D2
    ("D2", FOR, ".map(|n| n.count - 1)", ".map(|n| n.count)", T1 + ["repeated_exempt"]),
    ("D2", FOR, "if let Some(name) = held_error {", "if let Some(name) = held_error.filter(|_| false) {", TU + ["summar", "planted", "warnings_are_never"]),
    ("D2", FOR, "        if self.queue.is_empty() {\n            out.extend(e);", "        if true {\n            out.extend(e);", TU + ["summar", "planted"]),
    ("D2", FOR, "            self.queue.push_back(Slot::Ready(e));\n            self.bound(out);", "            self.queue.push_back(Slot::Ready(e));", TU + ["queue_behind"]),
    ("D2", FOR, "        } else if !self.inside {", "        } else if true {", TU + ["summar", "planted"]),
    ("D2", FOR, "                if force {\n                    // Kept by force", "                if false {\n                    // Kept by force", TU + ["summar", "planted"]),
    ("D2", FOR, "                && !force\n            {", "\n            {", TU + ["summar", "planted"]),
    ("D2", FOR, "self.release_all(out);\n        let line = line.trim_end();", "let line = line.trim_end();", TU + ["summar", "planted"]),
    ("D2", FOR, "        if device_level {", "        if false {", TU + ["summar", "planted"]),
    ("D2", FOR, "            force = true;", "            force = false;", TU + ["summar", "planted"]),
    ("D2", FOR, "let owed = c.owed || matches!(c.cycle, Cycle::Errored(..));", "let owed = matches!(c.cycle, Cycle::Errored(..));", TU + ["summar", "planted"]),
    ("D2", FOR, "self.found_identity\n            && self", "self.found_identity\n            || self", TU + ["summar", "planted"]),
    ("D2", FOR, 'body.strip_prefix("[WARNING] adc")', 'body.strip_prefix("[WARNING] adc-never")', TU + ["summar", "planted"]),
    ("D2", FOR, "Some(Some(key)) if !device_level => Some(key),", "Some(Some(key)) => Some(key),", TU + ["warnings_are_never"]),
    ("D2", FOR, "if *found && p.matches(&own) =>", "if *found && p.matches(&own) && false =>", TU + ["summar"]),
    ("D2", FOR, "if !muxes.contains(&mux) {", "if false && !muxes.contains(&mux) {", T1 + ["each_repeating"]),
    ("D2", FOR, "            // No witness: the judge finds nothing, so nothing is summarised.\n            let Some(want) = identity.witness.as_ref() else {",
     "            // No witness: the judge finds nothing, so nothing is summarised.\n            let Some(want) = identity.witness.as_ref().filter(|_| false) else {", T1 + ["each_repeating"]),
    ("D2", FOR, "        let mut force = std::mem::take(&mut self.pin_next);", "        let mut force = false;", TU + ["planted"]),
    ("D2", FOR, "            Cycle::Stopped(s0) => {\n                self.pin_next = true;", "            Cycle::Stopped(s0) => {\n                self.pin_next = false;", TU + ["planted"]),
    ("D2", FOR, "                    self.pin_next = true;\n                    return self.emit(vec![Emit::Keep(line.to_owned())], out);", "                    return self.emit(vec![Emit::Keep(line.to_owned())], out);", TU + ["planted"]),
    # equivalent, named
    ("D4 equivalent", FOR, "    let due = events(d).into_iter().filter(|(t, _)| *t <= first_read);",
     "    let due = events(d).into_iter().filter(|(t, _)| *t < first_read);", TU + ["foreign"]),
]
for (dec, f, old, new, test) in M:
    src = open(f).read()
    if src.count(old) != 1:
        print(f"NOT APPLIED\t{dec}\t{f}\t{old[:70]!r}", flush=True); continue
    open(f, "w").write(src.replace(old, new))
    r = subprocess.run(["cargo", "test", "--offline", "--locked"] + test, capture_output=True, text=True)
    open(f, "w").write(src); os.utime(f)
    out = r.stdout + r.stderr
    res = "SURVIVED" if r.returncode == 0 else ("UNCOMPILED" if "error[E" in out else "KILLED")
    print(f"{res}\t{dec}\t{f}\t{old[:70]!r} -> {new[:50]!r}", flush=True)
