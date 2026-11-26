#!/usr/bin/env python3
"""AET-WO-1919: each bench-package-s03 (and bench-package) job's steps, in minutes, with its runner's CPU
from ci/probe-step.sh start's lscpu lines. Reads OUT/<run>-a<n>/{run.tsv,jobs.json,s03.log,s01.log}.
usage: steps.py OUT"""
import datetime as dt, glob, json, os, re, sys

def t(s):
    return dt.datetime.fromisoformat(s.replace("Z", "+00:00"))

def cpu(log):
    try:
        text = open(log, errors="replace").read()
    except OSError:
        return "?"
    m = re.search(r"Model name:\s+(.+)", text)
    n = re.search(r"\dZ CPU\(s\):\s+(\d+)", text)
    return f"{m.group(1).strip() if m else '?'} x{n.group(1) if n else '?'}"

for d in sorted(glob.glob(os.path.join(sys.argv[1], "*-a*"))):
    head = open(os.path.join(d, "run.tsv")).read().split("\t")[0][:8]
    for line in open(os.path.join(d, "jobs.json")):
        j = json.loads(line)
        f = {"bench-package-s03": "s03", "bench-package": "s01"}.get(j["name"])
        if not f or not j.get("completed_at") or j["conclusion"] in (None, "skipped"):
            continue
        total = (t(j["completed_at"]) - t(j["started_at"])).total_seconds() / 60
        print(f"{os.path.basename(d)}\t{head}\t{j['name']}\t{j['conclusion']}\t{cpu(os.path.join(d, f + '.log'))}\t{total:.1f} min")
        for s in j["steps"]:
            if not s.get("completed_at") or s["conclusion"] == "skipped":
                continue
            m = (t(s["completed_at"]) - t(s["started_at"])).total_seconds() / 60
            if m >= 0.05:
                print(f"\t\t{m:6.1f}  {s['name'][:90]}")
