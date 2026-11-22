"""AET-WO-1606 mutants: one per guard this work order adds or changes, each against the suite it
lives in, run inside the tools image (~/.cache/aet-1606/sh/img.sh, the worktree mounted read-only).

usage (on the host, from the worktree): python3 -I spike/1606/sh/mut.py [name...]

Each mutant edits one file in place, runs its suite, then writes the saved bytes back; the file's
sha256 is checked equal to the original after every mutant (no git checkout).
"""
import hashlib
import os
import subprocess
import sys

IMG = os.path.expanduser("~/.cache/aet-1606/sh/img.sh")
BRIDGE = "cargo test --offline --manifest-path bridge/Cargo.toml"
CLI_LIB = "cargo test --offline -p aethelgard-cli --bin aethelgard"
DOCTOR = "cargo test --offline -p aethelgard-cli --test doctor_tier1"

# name, file, old, new, suite command (a test filter where it narrows the run)
MUTANTS = [
    ("record-constant", "bridge/src/main.rs",
     '''let mut v = json!({ "net": net, "method": "interpolated", "ngspice": ngspice });''',
     '''let mut v = json!({ "net": net, "method": "interpolated", "ngspice": "44.2" });''',
     BRIDGE + " --bin aethelgard-bridge"),
    ("pin-dropped", "bridge/src/solver.rs",
     "    if b.release != NGSPICE {", "    if b.release != b.release {",
     BRIDGE + " --bin aethelgard-bridge"),
    ("banner-first-of-many", "bridge/src/solver.rs",
     "    let [line] = lines.as_slice() else {", "    let [line, ..] = lines.as_slice() else {",
     BRIDGE + " --bin aethelgard-bridge"),
    ("banner-release-unchecked", "bridge/src/solver.rs",
     "                .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))",
     "                .all(|_| true)",
     BRIDGE + " --bin aethelgard-bridge"),
    ("version-names-no-banner", "bridge/src/main.rs",
     "    if emit_text(&identity(Some(spice.banner()))).is_err() {",
     "    if emit_text(&identity(None)).is_err() {",
     BRIDGE + " --test edges version_names"),
    ("cli-identity-pin-dropped", "crates/aethelgard-cli/src/analog.rs",
     "            if release == NGSPICE && banner ==", "            if banner ==",
     CLI_LIB + " a_bridge_that_does_not_name"),
    ("cli-identity-banner-unchecked", "crates/aethelgard-cli/src/analog.rs",
     '''            if release == NGSPICE && banner == format!("** ngspice-{release} shared library") =>''',
     '''            if release == NGSPICE && !banner.is_empty() =>''',
     CLI_LIB + " a_bridge_that_does_not_name"),
    ("cli-ran-default", "crates/aethelgard-cli/src/analog.rs",
     "        self.ngspice.clone().ok_or_else(|| {",
     "        Some(self.ngspice.clone().unwrap_or_else(|| NGSPICE.to_owned())).ok_or_else(|| {",
     CLI_LIB + " a_bridge_that_does_not_name"),
    ("cli-answers-constant", "crates/aethelgard-cli/src/analog.rs",
     '''        && o.get("ngspice").and_then(Value::as_str) == Some(ngspice);''',
     '''        && o.get("ngspice").and_then(Value::as_str) == Some(NGSPICE);''',
     CLI_LIB + " every_answer_but_one"),
    ("cli-rail-constant", "crates/aethelgard-cli/src/analog.rs",
     '''            && text("ngspice") == Some(ngspice)''',
     '''            && text("ngspice") == Some(NGSPICE)''',
     CLI_LIB + " a_rail_answer_is"),
    ("cli-judge-constant", "crates/aethelgard-cli/src/analog.rs",
     "            ngspice: ngspice.to_owned(),", "            ngspice: NGSPICE.to_owned(),",
     CLI_LIB + " an_unfinished_rise"),
    ("doctor-accepts-bare", "crates/aethelgard-cli/src/doctor.rs",
     "        (0, Some(ngspice)) if rest.is_empty() => Ok((named.version, ngspice)),",
     "        (0, n) if rest.is_empty() => Ok((named.version, n.unwrap_or_else(|| \"44.2\".to_owned()))),",
     DOCTOR + " a_bridge_whose_version"),
    ("doctor-accepts-more", "crates/aethelgard-cli/src/doctor.rs",
     "        (0, Some(ngspice)) if rest.is_empty() => Ok((named.version, ngspice)),",
     "        (0, Some(ngspice)) => Ok((named.version, ngspice)),",
     DOCTOR + " a_bridge_whose_version"),
    ("rustc-first-of-many", "crates/aethelgard-cli/src/firmware.rs",
     "    let [answer] = answers.as_slice() else {", "    let [answer, ..] = answers.as_slice() else {",
     CLI_LIB + " the_rustc_is_the_one"),
    ("rustc-unsuccessful-read", "crates/aethelgard-cli/src/firmware.rs",
     "        .filter(|o| o.get(\"success\") == Some(&serde_json::Value::Bool(true)))\n", "",
     CLI_LIB + " the_rustc_is_the_one"),
    ("rustc-field-repeated", "crates/aethelgard-cli/src/firmware.rs",
     "            [one] if !one.is_empty()", "            [one, ..] if !one.is_empty()",
     CLI_LIB + " the_rustc_is_the_one"),
]


def sha(path):
    return hashlib.sha256(open(path, "rb").read()).hexdigest()


def main(only):
    results = []
    for name, path, old, new, suite in MUTANTS:
        if only and name not in only:
            continue
        original = open(path, "rb").read()
        before = sha(path)
        text = original.decode()
        assert text.count(old) == 1, (name, text.count(old))
        open(path, "w").write(text.replace(old, new))
        try:
            cmd = [IMG, "sh", "-c", f"cd /repo && {suite} 2>&1 | tail -40"]
            env = dict(os.environ, REPO=os.getcwd(), TARGET="/cache/target-mut", MEM="8G")
            out = subprocess.run(cmd, env=env, capture_output=True, text=True).stdout
        finally:
            open(path, "wb").write(original)
        assert sha(path) == before, f"{path} not restored after {name}"
        failed = "test result: FAILED" in out or "error[E" in out or "panicked" in out
        compiled_error = "error[E" in out or "error: could not compile" in out
        verdict = "KILLED (compile)" if compiled_error else ("KILLED" if failed else "SURVIVED")
        line = f"{name}: {verdict}"
        print(line, flush=True)
        if verdict == "SURVIVED":
            print(out[-1500:], flush=True)
        results.append(line)
    return 1 if any("SURVIVED" in r for r in results) else 0


if __name__ == "__main__":
    sys.exit(main(set(sys.argv[1:])))
