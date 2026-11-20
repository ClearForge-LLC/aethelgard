# mutants.py: one mutant per guard AET-WO-4405 adds or changes, each a single
# exact-string edit, run against the suite it lives in; the file is restored
# from a copy taken first (never git checkout). A mutant that fails to compile
# is reported as NOCOMPILE, never as killed.
import subprocess, sys, os, shutil
R = "/home/forge/aethelgard-4405"
E = "crates/aethelgard-pincheck/src/elf.rs"
I = "crates/aethelgard-pincheck/src/identity.rs"
M = [
 ("M1 baud float taken as the baud", I, "let runs = map_baudrate(param.baud);", "let runs = u64::try_from(param.baud).unwrap_or(0);"),
 ("M2 read past AP_GROUPEND", E, "if i + 1 != entries.len() || !name.is_empty()", "if !name.is_empty()"),
 ("M3 GPS cap dropped", I, "if gps.len() >= GPS_MAX_RECEIVERS {", "if gps.len() >= usize::MAX {"),
 ("M4 GPS2 not counted", I, "runs == GPS || runs == GPS2", "runs == GPS"),
 ("M5 undeclared ports not counted", I, "    for q in params {", "    for q in params.iter().filter(|q| usize::from(q.port) < order.len()) {"),
 ("M6 flags ignored", E, "if flags != 0 {", "if flags == 0xffff {"),
 ("M7 type ignored", E, "if kind != want {", "if kind == 0xff {"),
 ("M8 duplicate row read", E, "if seen.iter().any(|s| s == name) {", "if seen.is_empty() && false {"),
 ("M9 unknown row read as pass-through", E, "        _ => None,\n    }\n}", "        _ => Some((Row::Pass, AP_PARAM_INT32)),\n    }\n}"),
 ("M10 a fractional default truncated", E, "value.is_finite() && value.fract() == 0.0 &&", "value.is_finite() &&"),
 ("M11 a default past its type saturated", E, "held(-128.0, 128.0)", "held(-1.0e9, 1.0e9)"),
 ("M11b a default below its type saturated (adversarial A9)", E, "value >= lo && value < hi", "value >= lo.min(-1.0e12) && value < hi"),
 ("M12 a partial port skipped", E, '            _ => {\n                return refuse(\n                    "VarInfoPartialPort",', '            _ => {\n                #[allow(unreachable_code)]\n                if n < 99 { continue; }\n                return refuse(\n                    "VarInfoPartialPort",'),
 ("M13 AP_GROUPEND's fields not held", E, " || !name.is_empty() || e[4..14] != [0; 10] || idx != 0xff", ""),
 ("M14 name length unbounded", E, ".take(NAME_MAX + 1)", ""),
 ("M16 the name's end not required flashed", E, "    loaded_offset(bytes, addr, len as u32 + 1)?;\n", ""),
 ("M18 missing declared port not named", I, '            out.push(\n                "MissingSerialParameters",', '            if false { out.push(\n                "MissingSerialParameters",'),
 ("M20 RCIN twice dropped", I, "            if let Some(first) = rcin.first() {", "            if let Some(first) = rcin.first().filter(|_| false) {"),
]
only = sys.argv[1:] 
env = dict(os.environ, PATH=os.path.expanduser("~/.cargo/bin") + ":" + os.environ["PATH"])
for name, f, old, new in M:
    if only and name.split()[0] not in only: continue
    p = os.path.join(R, f); keep = p + ".mutkeep"
    src = open(p).read()
    if src.count(old) != 1:
        print(f"{name}: SITE NOT UNIQUE ({src.count(old)})"); continue
    shutil.copy(p, keep)
    try:
        s2 = src.replace(old, new)
        if name.startswith("M18"):
            s2 = s2.replace('            continue;\n        };\n        // belt-and-suspenders: the generator', '            }\n            continue;\n        };\n        // belt-and-suspenders: the generator', 1)
        open(p, "w").write(s2)
        r = subprocess.run(["cargo", "test", "-q", "-p", "aethelgard-pincheck", "--test", "identity"], cwd=R, env=env, capture_output=True, text=True)
        out = r.stdout + r.stderr
        if "could not compile" in out or "error[E" in out:
            verdict = "NOCOMPILE"
        elif r.returncode != 0:
            failed = sorted({l.split()[0] for l in out.splitlines() if l.endswith(" --- FAILED")})
            verdict = "KILLED by " + ",".join(failed)
        else:
            verdict = "SURVIVED"
        print(f"{name}: {verdict}", flush=True)
    finally:
        shutil.move(keep, p)
