#!/usr/bin/env python3
"""AET-WO-3309 §2, the sweep (3307's rejudge.py, for a change that moves the judge too).

Each run is made twice: once by main's CLI (RUNS_MAIN) and once by the branch's (RUNS_BRANCH), since
placement (3) runs at judge time. For each run, by group and scenario:
  main      main's CLI on main's archive;
  b-on-m    the branch's CLI on main's archive: evolve's (1), (2), (4) alone, the placement as main made it;
  branch    the branch's CLI on the branch's archive: everything.
Compared: rule, evidence, masked, symptoms and unexplained (the hashes differ whenever kind 13 does);
kind 14 (pass, kinds and counts); kind 13 line by line (spike/3306/dump); and for an S rule, propose's
files and exit code. Writes OUT/compare.jsonl and OUT/compare.md.

Usage: compare.py RUNS_MAIN RUNS_BRANCH MAIN_CLI BRANCH_CLI DUMP REPO OUT
"""
import json
import pathlib
import subprocess
import sys
import tempfile

rm, rb, main_cli, branch_cli, dump, repo, out = map(pathlib.Path, sys.argv[1:8])
out.mkdir(parents=True, exist_ok=True)


def runs(root):
    found = {}
    for g in sorted((root / "camp").iterdir()):
        co = g / "campaign.out"
        if not co.exists():
            continue
        for line in co.read_text().splitlines():
            try:
                j = json.loads(line)
            except Exception:
                continue
            if "run" in j:
                found[(g.name, j["scenario"])] = (root / pathlib.Path(j["run"]).relative_to("/cache/" + root.name), j)
    return found


def attribute(cli, run):
    p = subprocess.run([str(cli), "attribute", "--archive", str(run)], capture_output=True)
    try:
        a = json.loads(p.stdout)
    except Exception:
        return {"rc": p.returncode, "raw": p.stdout.decode(errors="replace")[:300]}
    keep = ("ok", "domain", "rule", "evidence", "symptoms", "masked", "unexplained", "subjects", "error")
    return {"rc": p.returncode, **{k: a[k] for k in keep if k in a}}


def records(run):
    """Kind 13 lines and kind 14, as the dump prints them."""
    t = subprocess.run([str(dump), str(run)], capture_output=True, text=True).stdout.splitlines()
    return [l.split("] ", 1)[1] for l in t if " kind 13:" in l or " kind 14:" in l]


def propose(cli, run, scenario):
    with tempfile.TemporaryDirectory(dir=out) as d:
        o = pathlib.Path(d) / "p"
        p = subprocess.run(
            [str(cli), "propose", "--archive", str(run), "--scenario", str(scenario), "--out", str(o)],
            capture_output=True,
            cwd=scenario.parent,
        )
        files = {f.name: f.read_text() for f in sorted(o.glob("*"))} if o.exists() else {}
        return [p.returncode, files]


def scenario_file(root, j):
    f = pathlib.Path(j["file"])
    return next((c for c in (root / "sc" / f, repo / f) if c.exists()), None)


def short(a):
    if "rule" not in a:
        return f"rc {a.get('rc')} {a.get('error', a.get('raw', ''))}"[:120]
    ev = ",".join(f"{e['index']}:{e['kind']}" for e in a.get("evidence", []))
    s = f"{a['rule']} [{ev}]"
    if a.get("masked"):
        s += " masked [" + ",".join(f"{e['index']}:{e['kind']}" for e in a["masked"]) + "]"
    if a.get("unexplained"):
        s += f" unexplained {a['unexplained']}"
    return s


M, B = runs(rm), runs(rb)
rows = []
for key in sorted(set(M) | set(B)):
    row = {"group": key[0], "scenario": key[1]}
    if key not in M or key not in B:
        row["missing"] = "main" if key not in M else "branch"
        rows.append(row)
        continue
    (mr, mj), (br, bj) = M[key], B[key]
    am, abm, ab = attribute(main_cli, mr), attribute(branch_cli, mr), attribute(branch_cli, br)
    km, kb = records(mr), records(br)
    row.update(
        main=am,
        b_on_m=abm,
        branch=ab,
        evolve_moved=abm != am,
        attribution_moved=ab != am,
        kind14_main=[l for l in km if " kind 14:" in l],
        kind14_branch=[l for l in kb if " kind 14:" in l],
        kind13_diff=[[x, y] for x, y in zip(km, kb) if x != y] + ([["len", len(km), len(kb)]] if len(km) != len(kb) else []),
    )
    if any(str(a.get("rule", "")).startswith("S") for a in (am, abm, ab)):
        sm, sb = scenario_file(rm, mj), scenario_file(rb, bj)
        if sm and sb:
            pm, pbm, pb = propose(main_cli, mr, sm), propose(branch_cli, mr, sm), propose(branch_cli, br, sb)
            row.update(propose_main=pm, propose_b_on_m=pbm, propose_branch=pb,
                       propose_evolve_moved=pbm != pm, propose_moved=pb != pm)
    rows.append(row)

with open(out / "compare.jsonl", "w") as f:
    for r in rows:
        f.write(json.dumps(r, sort_keys=True) + "\n")
with open(out / "compare.md", "w") as f:
    f.write("| group | scenario | main | branch's CLI, main's archive | branch | moved |\n|---|---|---|---|---|---|\n")
    for r in rows:
        if "missing" in r:
            f.write(f"| {r['group']} | {r['scenario']} | missing on {r['missing']} | | | |\n")
            continue
        moved = []
        if r["evolve_moved"]:
            moved.append("attribution (evolve)")
        if r["attribution_moved"]:
            moved.append("attribution")
        if r.get("propose_evolve_moved"):
            moved.append("propose (evolve)")
        if r.get("propose_moved"):
            moved.append("propose")
        if r["kind14_main"] != r["kind14_branch"]:
            moved.append("kind 14")
        if r["kind13_diff"]:
            moved.append(f"kind 13 ({len(r['kind13_diff'])})")
        f.write(f"| {r['group']} | {r['scenario']} | {short(r['main'])} | {short(r['b_on_m'])} | {short(r['branch'])} | {', '.join(moved) or '='} |\n")
print(f"{len(rows)} runs; moved: " + "; ".join(
    f"{r['group']}/{r['scenario']}" for r in rows
    if "missing" in r or r["evolve_moved"] or r["attribution_moved"] or r.get("propose_moved") or r.get("propose_evolve_moved")
    or r["kind14_main"] != r["kind14_branch"] or r["kind13_diff"]
))
