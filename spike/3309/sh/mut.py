#!/usr/bin/env python3
"""AET-WO-3309 mutants: each guard this order adds or changes, one mutant each, run against the suite it
lives in; the file is restored by writing its text back (a new mtime, the mutant-restore lesson).
Usage: mut.py REPO [names...]"""
import pathlib
import subprocess
import sys

repo = pathlib.Path(sys.argv[1])
only = set(sys.argv[2:])
L = "crates/aethelgard-evolve/src/lines.rs"
E = "crates/aethelgard-evolve/src/lib.rs"
R = "crates/aethelgard-cli/src/renode.rs"
EVOLVE = ["cargo", "test", "-q", "-p", "aethelgard-evolve"]
RENODE = ["cargo", "test", "-q", "-p", "aethelgard-cli", "--bin", "aethelgard", "renode::"]
M = [
    # (1) counts, D5
    ("M1 no Write.bytes", E, 'fields.extend(count("Write", reg_want.1, written));', "", EVOLVE),
    ("M2 no Read.bytes", E, 'fields.extend(count("Read", want.1, returned));', "", EVOLVE),
    # (4) one transaction
    ("M3 read stitched from any transaction", E,
     "let got = got.into_iter().next();",
     "let got = got.iter().find(|t| !t.reads.is_empty()).or(got.first()).cloned();", EVOLVE),
    ("M24 a step's read counts every transfer", E,
     "        let returned = got.as_ref().and_then(|t| match t.reads.first() {\n            Some(r) => Some(r.as_slice()),",
     "        let all: Option<Vec<u8>> = got.as_ref().map(|t| t.reads.concat()).filter(|r| !r.is_empty());\n        let returned = got.as_ref().and_then(|t| match t.reads.first() {\n            Some(_) => all.as_deref(),", EVOLVE),
    ("M4 zero read without a claim", E, "None => (claimed && none.is_empty()).then_some(&[][..]),",
     "None => none.is_empty().then_some(&[][..]),", EVOLVE),
    ("M5 zero read beside an unread line", E, "None => (claimed && none.is_empty()).then_some(&[][..]),",
     "None => claimed.then_some(&[][..]),", EVOLVE),
    # (2) the judged form, D4
    ("M6 the ICP judged", E, "lines::form(script, device).filter(|f| *f != Form::Icp)",
     "lines::form(script, device)", EVOLVE),
    ("M7 whole log ignores a line that is none", E, "none.is_empty().then_some(ts)", "Some(ts)", EVOLVE),
    # the readers
    ("M8 a write closes on a line that is none", L, "        if part.is_none() {\n            none.push(i);\n        }",
     "        if part.is_none() {\n            none.push(i);\n            open = false;\n        }", RENODE),
    ("M9 ICP padded digits", L, "&& !(d.len() == 2 && d.starts_with('0'))", "", EVOLVE),
    ("M10 ICP read without its tail", L, 'after.contains(", requested: ").then_some(())?;', "", EVOLVE),
    ("M11 no ICP reader", L, "    } else if any(&|l| icp(l, device).is_some()) {\n        Some(Form::Icp)\n",
     "", RENODE),
    # placement, R3
    ("M12 ICP key one byte", L, "let n = if form == Form::Icp { 2 } else { 1 };", "let n = 1;", EVOLVE),
    ("M13 key without direction", L,
     "let key = |written: &[u8], read: bool| (written.get(..n).map(<[u8]>::to_vec), read);",
     "let key = |written: &[u8], _read: bool| (written.get(..n).map(<[u8]>::to_vec), true);", EVOLVE),
    ("M14 same key always to the first", L, "                .find(|&s| taken[s] == 0)\n                .unwrap_or(last);", "                .next()\n                .unwrap_or(last);", RENODE),
    ("M15 no fallback by direction", L,
     "if let Some(s) = (0..steps.len()).find(|&s| (steps[s].1 > 0) == read && taken[s] == 0) {",
     "if let Some(s) = None::<usize> {", RENODE),
    ("M16 fallback to a taken step", L,
     "if let Some(s) = (0..steps.len()).find(|&s| (steps[s].1 > 0) == read && taken[s] == 0) {",
     "if let Some(s) = (0..steps.len()).find(|&s| (steps[s].1 > 0) == read) {", RENODE),
    # the judge, D3
    ("M17 unplaced not quoted in a failing finding", R,
     '            f.message = format!("{}; {quoted}", f.message);', "", RENODE),
    ("M18 no -min finding", R,
     "        } else if let Some(k) = (0..checks.len()).find(|&k| {",
     "        } else if let Some(k) = (0..0).find(|&k: &usize| {", RENODE),
    ("M19 -min beside a failing finding too", R,
     "        if let Some(f) = out.get_mut(first) {",
     "        if let Some(f) = out.get_mut(usize::MAX) {", RENODE),
    ("M20 -min without a claim", R,
     "                && observed.records[k].0 == aethelgard_twin::boot::STATUS_OK", "", RENODE),
    ("M21 order never checked", R, "unplaced[g[0]].out_of_order |= s < reached;", "", RENODE),
    ("M22 an unread form unnamed", R, "let unread = aethelgard_evolve::lines::form(&script, &c.device).is_none();",
     "let unread = false;", RENODE),
    ("M23 an error to the first step", R, ".max_by_key(|&s| begun[s])", ".min_by_key(|&s| begun[s])", RENODE),
]
for name, f, old, new, suite in M:
    if only and name.split()[0] not in only:
        continue
    p = repo / f
    text = p.read_text()
    if text.count(old) != 1:
        print(f"{name}: SITE NOT UNIQUE ({text.count(old)})", flush=True)
        continue
    p.write_text(text.replace(old, new))
    try:
        r = subprocess.run(suite, cwd=repo, capture_output=True, text=True)
    finally:
        p.write_text(text)
    out = r.stdout + r.stderr
    if r.returncode == 0:
        verdict = "SURVIVED"
    elif "error[" in out or "could not compile" in out:
        verdict = "COMPILE ERROR " + next((l for l in out.splitlines() if l.startswith("error")), "")
    else:
        failed = [l.split()[0] for l in out.splitlines() if l.strip().startswith("---- ")]
        failed = [l for l in out.splitlines() if l.startswith("---- ")]
        verdict = "killed by " + ", ".join(x.split()[1] for x in failed[:3])
    print(f"{name}: {verdict}", flush=True)
