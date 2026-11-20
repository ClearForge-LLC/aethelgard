# mutants.py [ID...]: one mutant per guard AET-WO-4406 adds or changes, each a
# single exact-string edit, run against the pincheck suites (every test
# binary of the crate); the file is restored from a copy taken first (never
# git checkout). A mutant that fails to compile is NOCOMPILE, never killed.
import os, shutil, subprocess, sys

R = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
E = "crates/aethelgard-pincheck/src/elf.rs"
I = "crates/aethelgard-pincheck/src/identity.rs"
M = [
    # The work order's mutants (§1.5).
    ("W1 drop load_defaults_file", E, "        (\n            LOAD_DEFAULTS_FILE,", "        (\n            &b\"_W1\"[..],"),
    ("W2 drop load_embedded_param_defaults", E, "        (\n            LOAD_EMBEDDED_PARAM_DEFAULTS,", "        (\n            &b\"_W2\"[..],"),
    ("W3 drop param_defaults_data's length", E, "        if length != 0 {", "        if length != 0 && false {"),
    ("W4 drop ROMFS defaults.parm", E, "            if name == ROMFS_DEFAULTS {", "            if name == \"_W4\" {"),
    ("W5 reload_defaults_file taken as a signature", E, "    ] {\n        if !named(symbol)?.is_empty() {",
     "        (b\"_ZN8AP_Param20reload_defaults_fileEb\", \"W5\"),\n    ] {\n        if !named(symbol)?.is_empty() {"),
    ("W6 the cap held at 2 again", I, "    let receivers = elf::gps_receivers(elf_bytes)?;", "    let receivers = elf::gps_receivers(elf_bytes)?.map(|_| 2);"),
    # The rest this work order adds.
    ("W7 the guard not called by read", E, "    if !overrides.is_empty() {", "    if !overrides.is_empty() && false {"),
    ("W8 no AP_GPS read as cap 0", I, "receivers.filter(|_| runs == GPS || runs == GPS2)", "receivers.or(Some(0)).filter(|_| runs == GPS || runs == GPS2)"),
    ("W9 GpsNotCompiled dropped", I, "    if receivers.is_none() {", "    if receivers.is_none() && false {"),
    ("W10 the row offset not held", E, "if let Some(at) = at.filter(|at| *at != i64::from(offset)) {", "if let Some(at) = at.filter(|_| false) {"),
    ("W11 a receiver's flags not held", E, " || r.flags != AP_PARAM_FLAG_NESTED_OFFSET {", " {"),
    ("W12 a receiver's name not held", E, "if r.name != format!(\"{next}_\") || ", "if "),
    ("W13 group rows not told from scalars", E, "        if r.kind != AP_PARAM_GROUP {\n            continue;\n        }\n", ""),
    ("W14 an absent AP_GPS::var_info read", E, "GPS_VAR_INFO_SYMBOL)?.is_empty() {\n        return Ok(None);", "GPS_VAR_INFO_SYMBOL)?.is_empty() && false {\n        return Ok(None);"),
    ("W15 param_defaults_data's magic not held", E, "if data[..16] != PARAM_DEFAULTS_MAGIC || u32::from(max) != room", "if u32::from(max) != room"),
    ("W16 its max_length not held", E, " || u32::from(max) != room || length > max {", " || length > max {"),
    ("W17 its length within max_length not held", E, " || length > max {", " {"),
    ("W18 a ROMFS name unbounded", E, "                ROMFS_NAME_MAX,", "                usize::MAX - 1,"),
    ("W19 a zero-size ROMFS read as empty", E, "(size == 0 || size % ROMFS_ENTRY_LEN != 0)", "(size % ROMFS_ENTRY_LEN != 0)"),
    ("W20 an absent ROMFS read", E, "    if !named(ROMFS_FILES)?.is_empty() {", "    if true {"),
]
only = sys.argv[1:]
env = dict(os.environ, PATH=os.path.expanduser("~/.cargo/bin") + ":" + os.environ["PATH"])
for name, f, old, new in M:
    if only and name.split()[0] not in only:
        continue
    p = os.path.join(R, f)
    keep = p + ".mutkeep"
    src = open(p).read()
    if src.count(old) != 1:
        print(f"{name}: SITE NOT UNIQUE ({src.count(old)})", flush=True)
        continue
    shutil.copy(p, keep)
    try:
        open(p, "w").write(src.replace(old, new))
        r = subprocess.run(["cargo", "test", "-p", "aethelgard-pincheck", "--no-fail-fast"], cwd=R, env=env,
                           capture_output=True, text=True)
        out = r.stdout + r.stderr
        if "could not compile" in out or "error[E" in out:
            verdict = "NOCOMPILE"
        elif r.returncode != 0:
            failed = sorted({l.split()[1] for l in out.splitlines() if l.startswith("test ") and l.endswith(" ... FAILED")})
            verdict = "KILLED by " + ",".join(failed)
        else:
            verdict = "SURVIVED"
        print(f"{name}: {verdict}", flush=True)
    finally:
        shutil.move(keep, p)
