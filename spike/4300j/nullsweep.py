"""AET-WO-4300j §1.4, the null sweep: out/baseline.tsv (cd79024c) against
out/after/baseline.tsv (the build), per specification.

Judged:
- exit codes, refusal kinds and emit file counts are unchanged;
- no `/assignments/*/pin` moves on any board (the §7 stop);
- a solve moves only in `schema_version`, `specification_sha256`, and a
  `pull` object added to each `motor_outputs` entry;
- emitted files move only on a board with motor outputs;
- repeats are identical.

usage: python3 -I nullsweep.py <base scratch> <after scratch>
writes out/null-sweep.tsv and prints a summary; exit 1 on any unexpected move.
"""
import csv
import json
import os
import sys

B, A = sys.argv[1], sys.argv[2]
here = os.path.dirname(os.path.abspath(__file__))


def load(p):
    return {r["name"]: r for r in csv.DictReader(open(p), delimiter="\t")}


# BASELINE names another baseline table, e.g. out/baseline-34f48d61.tsv
# after main moved under the branch (its scratch is the first argument).
b = load(os.environ.get("BASELINE", f"{here}/out/baseline.tsv"))
a = load(f"{here}/out/after/baseline.tsv")
motor = {r["board"] for r in csv.DictReader(open(f"{here}/out/motors.tsv"), delimiter="\t")}
keys = ["validate_rc", "validate_kind", "solve_rc", "solve_kind", "emit_rc", "emit_kind", "emit_files"]


def paths(o, p=""):
    if isinstance(o, dict):
        for k, v in o.items():
            yield from paths(v, f"{p}/{k}")
    elif isinstance(o, list):
        for i, v in enumerate(o):
            yield from paths(v, f"{p}/{i}")
    else:
        yield p, o


def moved(x, y):
    px, py = dict(paths(x)), dict(paths(y))
    return sorted(k for k in set(px) | set(py) if px.get(k, "<absent>") != py.get(k, "<absent>"))


def allowed(path):
    if path in ("/schema_version", "/specification_sha256"):
        return True
    parts = path.split("/")
    return len(parts) >= 4 and parts[1] == "motor_outputs" and parts[3] == "pull"


rows, bad = [], 0
# A board that main added after the baseline has no baseline row: it is
# listed, not judged. A baseline board missing after the build is a failure.
missing = sorted(set(b) - set(a))
added = sorted(set(a) - set(b))
bad += len(missing)
for n in sorted(set(b) & set(a)):
    x, y = b[n], a[n]
    codes = all(x[k] == y[k] for k in keys)
    pins, other, nsolve = [], [], 0
    pb, pa = f"{B}/solve/{n}.json", f"{A}/solve/{n}.json"
    if x["solve_rc"] == "0" and os.path.exists(pb) and os.path.exists(pa):
        m = moved(json.load(open(pb)), json.load(open(pa)))
        nsolve = len(m)
        pins = [p for p in m if p.startswith("/assignments/") and p.endswith("/pin")]
        other = [p for p in m if not allowed(p)]
    emit_moved = x["emit_tree_sha256"] != y["emit_tree_sha256"]
    ok = codes and not pins and not other and (not emit_moved or n in motor) and y["repeat_identical"] == "yes"
    bad += not ok
    rows.append([n, "yes" if codes else "NO", str(nsolve), ",".join(pins) or "-", ",".join(other) or "-",
                 "moved" if emit_moved else "same", "yes" if n in motor else "no", y["repeat_identical"],
                 "ok" if ok else "UNEXPECTED"])
with open(f"{here}/out/null-sweep.tsv", "w") as f:
    f.write("name\tcodes_same\tsolve_paths_moved\tpin_moves\tunexpected_paths\temit\tmotor_board\trepeat\tverdict\n")
    for r in rows:
        f.write("\t".join(r) + "\n")
print(f"{len(rows) - (bad - len(missing))}/{len(rows)} as expected; missing after: {missing}; added since the baseline (not judged): {added}")
print("emit moved on:", [r[0] for r in rows if r[5] == "moved"])
print("pin moves:", [r[0] for r in rows if r[3] != "-"])
for r in rows:
    if r[-1] != "ok":
        print("UNEXPECTED", r)
sys.exit(1 if bad else 0)
