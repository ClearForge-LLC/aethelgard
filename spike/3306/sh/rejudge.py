#!/usr/bin/env python3
"""AET-WO-3306 §1.1, on the host: every archive measure.sh made, judged by main's CLI and the branch's.

For each run: dump3306's per-device class, `attribute` from both CLIs (rule, evidence, masked, and the
whole output's bytes), and for a software rule `propose` from both (the generator report's bytes). Writes
one JSON line per run to OUT/rejudge.jsonl and a table to OUT/table.md.

Usage: rejudge.py RUNS MAIN_CLI BRANCH_CLI DUMP REPO OUT
"""
import json
import pathlib
import subprocess
import sys
import tempfile

runs, main_cli, branch_cli, dump, repo, out = map(pathlib.Path, sys.argv[1:7])
out.mkdir(parents=True, exist_ok=True)


def host(p):
    """An in-image path (/cache/runs/...) as the host sees it."""
    return runs / pathlib.Path(p).relative_to("/cache/runs")


def attribute(cli, run):
    p = subprocess.run([str(cli), "attribute", "--archive", str(run)], capture_output=True)
    return p.returncode, p.stdout


def propose(cli, run, scenario):
    with tempfile.TemporaryDirectory(dir=out) as d:
        o = pathlib.Path(d) / "p"
        p = subprocess.run(
            [str(cli), "propose", "--archive", str(run), "--scenario", str(scenario), "--out", str(o)],
            capture_output=True,
            cwd=scenario.parent,
        )
        files = {f.name: f.read_bytes() for f in sorted(o.glob("*"))} if o.exists() else {}
        return p.returncode, p.stdout, files


def short(a):
    ev = ",".join(f"{e['index']}:{e['kind']}" for e in a.get("evidence", []))
    ma = ",".join(f"{e['index']}:{e['kind']}" for e in a.get("masked", []))
    return f"{a.get('rule')} [{ev}] masked [{ma}]"


rows = []
for g in sorted((runs / "camp").iterdir()):
    co = g / "campaign.out"
    if not co.exists():
        continue
    for line in co.read_text().splitlines():
        try:
            j = json.loads(line)
        except Exception:
            continue
        if "run" not in j:
            continue
        run = host(j["run"])
        classes = [
            l.split(" ", 3)[1:3]
            for l in subprocess.run([str(dump), str(run)], capture_output=True, text=True).stdout.splitlines()
            if l.startswith("DEVICE ")
        ]
        mrc, mout = attribute(main_cli, run)
        brc, bout = attribute(branch_cli, run)
        ma, ba = json.loads(mout), json.loads(bout)
        row = {
            "group": g.name,
            "scenario": j["scenario"],
            "run": str(run.relative_to(runs)),
            "classes": classes,
            "main": short(ma),
            "branch": short(ba),
            "attribute_bytes_equal": (mrc, mout) == (brc, bout),
        }
        if any(str(x.get("rule", "")).startswith("S") for x in (ma, ba)):
            # The campaign names the file it ran, relative to where it ran: the repository, or runs/sc.
            f = pathlib.Path(j["file"])
            scenario = next((c for c in (runs / "sc" / f, repo / f) if c.exists()), None)
            if scenario is None:
                row["propose"] = "no scenario file"
            else:
                pm, pb = propose(main_cli, run, scenario), propose(branch_cli, run, scenario)
                # stdout names the --out directory, a fresh temporary one per call: compare the exit and the files.
                row["propose_equal"] = (pm[0], pm[2]) == (pb[0], pb[2])
                row["propose_main"] = {k: v.decode() for k, v in pm[2].items()}
                row["propose_branch"] = {k: v.decode() for k, v in pb[2].items()}
                row["propose_rc"] = [pm[0], pb[0]]
        rows.append(row)

with open(out / "rejudge.jsonl", "w") as f:
    for r in rows:
        f.write(json.dumps(r, sort_keys=True) + "\n")
with open(out / "table.md", "w") as f:
    f.write("| group | scenario | devices (class) | main | branch | moved |\n|---|---|---|---|---|---|\n")
    for r in rows:
        cl = " ".join(f"{d}:{c}" for d, c in r["classes"])
        moved = []
        if not r["attribute_bytes_equal"]:
            moved.append("attribute")
        if r.get("propose_equal") is False:
            moved.append("propose")
        f.write(f"| {r['group']} | {r['scenario']} | {cl} | {r['main']} | {r['branch']} | {', '.join(moved) or '='} |\n")
print(f"{len(rows)} runs; moved: " + ", ".join(
    f"{r['group']}/{r['scenario']}" for r in rows if not r["attribute_bytes_equal"] or r.get("propose_equal") is False
))
