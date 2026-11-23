#!/usr/bin/env python3
"""AET-WO-2312 mutants, sized to the diff: one per guard this WO adds or changes, each run against the suites it
lives in (chaos's and evolve's tests). Each mutant is an exact, single replacement; the file is restored from a copy
after each. A mutant is killed when the run fails and the failing test is named (not a build error).
Usage (from the worktree): python3 -I spike/2312/sh/mutants.py OUT.txt   (CARGO_TARGET_DIR from the environment)."""
import os, shutil, subprocess, sys
C = 'crates/aethelgard-chaos/src/campaign.rs'
E = 'crates/aethelgard-evolve/src/lib.rs'
M = [
 ('M1a kind dropped: register_mismatch', C, 'add("register_mismatch");', '{}'),
 ('M1b kind dropped: timeouts', C, 'add(if spi { "spi_timeout" } else { "i2c_timeout" });', '{}'),
 ('M1c kind dropped: read mismatches', C, '                        add(kind);\n                    }\n', '                        let _ = kind;\n                    }\n'),
 ('M1d kind dropped: record_invalid', C, '''add(if spi {
                        "spi_record_invalid"
                    } else {
                        "i2c_record_invalid"
                    });''', '{}'),
 ('M1e kind dropped: cpu_halted', C, 'add(HALTED_FINDING);', '{}'),
 ('M1f kind dropped: locked evidence', C, 'locked.push(i);', '{}'),
 ('M2 == weakened to >= (Equal)', C, 'Bound::Equal => n == shown as u64,', 'Bound::Equal => n >= shown as u64,'),
 ('M2b AtMostOnce unbounded', C, 'Bound::AtMostOnce => n <= 1,', 'Bound::AtMostOnce => true,'),
 ('M2c Reach becomes Equal for invalid', C, '''    for kind in ["i2c_record_invalid", "spi_record_invalid"] {
        if let Some(e) = out.get_mut(kind) {
            e.1 = Bound::Reach(e.0.len() as u64);''', '''    for kind in ["i2c_record_invalid", "spi_record_invalid"] {
        if let Some(e) = out.get_mut(kind) {
            e.1 = Bound::Equal;'''),
 ('M2d info severity counted', C, 'if n != failing || !s.bound.holds(n, s.records.len())', 'if !s.bound.holds(n, s.records.len())'),
 ('M5 boot gate dropped (value/validity before the boot word)', C, '                } else if !booted {\n                } else if status == "invalid" {', '                } else if status == "invalid" {'),
 ('M6 unread lower bound dropped', C, '                s.unwitnessed += 1;\n', ''),
 ('M7 unread no-format step counted as unwitnessed', C, '''            if expected.is_empty() {
                s.unavailable += 1;''', '''            if expected.is_empty() {
                s.unwitnessed += 1;'''),
 ('M8 locked evidence by any name', C, 'if name.starts_with("RCC.")\n', 'if !name.is_empty()\n'),
 ('M9 check_shown not called', C, '    check_shown(records, findings)?;\n', ''),
 ('M13 status set dropped', C, '        return Err("a read\'s status is not one the judge writes for its bus");', '        let _ = ();'),
 ('M14 joint bound dropped', C, '            return Err(why);\n        }\n    }\n    Ok(())', '            let _ = why;\n        }\n    }\n    Ok(())'),
 ('M15 equal_fits always', C, '        } else if observed.as_ref() != Some(expected) && equal_fits(&records[w]) {', '        } else if observed.as_ref() != Some(expected) {'),
 ('M16 [ERROR] line ignored', C, '            .is_some_and(|o| o.iter().any(|l| l.starts_with(LOG_ERROR)));', '            .is_some_and(|o| o.iter().any(|l| l.starts_with(LOG_ERROR) && false));'),
 ('M17 witness steps judged before the boot word', C, '    if !booted(records) {\n        return out;\n    }\n', ''),
 ('M18 no-format joint dropped', C, '            s.joint_unavailable += 1;\n', ''),
 ('M10 (4) bound removed', E, 'if records > MAX_ATTRIBUTE_RECORDS {', 'if records > usize::MAX - 1 {'),
 ('M11 attribute exactness dropped', E, '&& (s.bound == Bound::AtMostOnce || s.records.len() as u64 == count) =>', '=>'),
 ('M12 attribute converse loop dropped', E, '''    for s in &shown {
        for &i in &s.records {''', '''    for s in shown.iter().take(0) {
        for &i in &s.records {'''),
]
only = sys.argv[2:]
out = open(sys.argv[1], 'w')
for f in (C, E):
    os.utime(f)
env = dict(os.environ)
for name, path, old, new in M:
    if only and not any(name.startswith(o + ' ') for o in only):
        continue
    src = open(path).read()
    assert src.count(old) == 1, (name, src.count(old))
    shutil.copy(path, path + '.orig')
    try:
        open(path, 'w').write(src.replace(old, new))
        p = subprocess.run(['cargo', 'test', '-q', '-p', 'aethelgard-chaos', '-p', 'aethelgard-evolve', '--no-fail-fast'],
                           capture_output=True, text=True, env=env)
        log = p.stdout + p.stderr
        failed = sorted({l.split()[0] for l in log.splitlines() if l.startswith('    ') and l.strip() and ' ' not in l.strip()})
        build = 'error[' in log or 'could not compile' in log and not failed
        verdict = 'BUILD-ERROR' if build else ('KILLED' if p.returncode != 0 and failed else ('SURVIVED' if p.returncode == 0 else 'FAILED-UNNAMED'))
        out.write(f'{name}: {verdict} {failed}\n'); out.flush()
        print(f'{name}: {verdict} {failed}', flush=True)
    finally:
        shutil.move(path + '.orig', path)
        # The restored file is newer than any build of a mutant: otherwise cargo keeps the mutant's build and the
        # next mutant of another file runs against it (2312 run 1: M10-M12 ran on M9's stale chaos).
        os.utime(path)
