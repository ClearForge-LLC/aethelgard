#!/usr/bin/env python3
"""AET-WO-1916 §1.1: the measurement. Read-only over fetched CI runs (spike/1912/fetch.sh layout, then
`stamp.sh`): each run's tools-shard jobs, split into setup (job start to the `shard.sh run` step), the test step
(that step to the next step) and the rest; the plan's planned seconds; and each test's in-shard seconds by
`ci/shard.sh times` (AET-WO-1914 P3), against its row.

  measure.py shards RUNDIR...            per shard: job, setup, step, after, planned (minutes / seconds)
  measure.py tests ROWS COLO RUNDIR...   per test of 10 s or more: samples, worst, row
  measure.py blocks ROWS COLO RUNDIR...  per run, shard and binary: wall against planned seconds
"""
import datetime as dt
import re
import sys
from pathlib import Path

ISO = re.compile(r"^\ufeff?(\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d\.\d+)Z (.*)$")


def iso(s):
    return dt.datetime.fromisoformat(s[:26]).replace(tzinfo=dt.timezone.utc).timestamp()


def jobs(run):
    out = {}
    for line in (run / "jobs.tsv").read_text().splitlines():
        jid, name, concl, a, b = line.split("\t")
        m = re.fullmatch(r"tools-shard \((\d+)\)", name)
        if m:
            out[int(m[1])] = (iso(a.rstrip("Z")), iso(b.rstrip("Z")), concl)
    return out


def steps(raw):
    """(start of the run step, start of the step after it), and the planned seconds per shard."""
    run_at = after = None
    planned = {}
    for line in raw.read_text(errors="replace").splitlines():
        m = ISO.match(line)
        if not m:
            continue
        t, rest = iso(m[1]), m[2]
        p = re.match(r"shard (\d+): \d+ tests, planned (\d+) s", rest)
        if p:
            planned[int(p[1])] = int(p[2])
        if rest.startswith("##[group]Run ") and "ci/shard.sh run " in rest:
            run_at = t
        elif run_at and after is None and rest.startswith("##[group]Run "):
            after = t
    return run_at, after, planned


def shards(runs):
    for run in runs:
        run = Path(run)
        head = (run / "run.tsv").read_text().split("\t")[0][:8]
        js = jobs(run)
        rows = []
        for k, (a, b, concl) in sorted(js.items()):
            r, n, planned = steps(run / f"raw-{k}.txt")
            rows.append((k, (b - a) / 60, r - a, n - r, b - n, planned.get(k), concl))
        jm = [x[1] for x in rows]
        sm = [x[3] for x in rows]
        print(f"## {run.name} ({head}): job max {max(jm):.1f} min {min(jm):.1f} spread {max(jm) - min(jm):.1f} min;"
              f" step max {max(sm):.0f} min {min(sm):.0f} spread {(max(sm) - min(sm)) / 60:.1f} min")
        print("shard\tjob_min\tsetup_s\tstep_s\tafter_s\tplanned_s\tconclusion")
        for k, j, s, st, af, pl, c in rows:
            print(f"{k}\t{j:.1f}\t{s:.0f}\t{st:.0f}\t{af:.0f}\t{pl}\t{c}")


def table(path, cols):
    out = {}
    for line in Path(path).read_text().splitlines():
        if not line or line.startswith("#"):
            continue
        f = line.split("\t")
        out[tuple(f[:cols - 1])] = float(f[cols - 1])
    return out


def tests(rows_path, colo_path, runs):
    rows = table(rows_path, 3)
    colo = table(colo_path, 2)
    samples = {}
    for run in runs:
        for line in (Path(run) / "times.tsv").read_text().splitlines():
            k, b, name, s = line.split("\t")
            samples.setdefault((b, name), []).append((float(s), Path(run).name, int(k)))
    print("binary\ttest\tn\tworst\tbest\trow\tworst/row\tshards")
    for (b, name), xs in sorted(samples.items(), key=lambda kv: -max(x[0] for x in kv[1])):
        worst = max(x[0] for x in xs)
        if worst < 10:
            continue
        row = rows.get((b, name))
        note = f"{row:g}" if row is not None else ("colocated" if (b,) in colo else "none")
        ratio = f"{worst / row:.2f}" if row else ""
        print(f"{b}\t{name}\t{len(xs)}\t{worst:.1f}\t{min(x[0] for x in xs):.1f}\t{note}\t{ratio}\t"
              + ",".join(f"{r[-3:]}:{k}" for _, r, k in xs))





def walls(st):
    """Each binary's wall seconds in a stamped shard log: its `running N tests` line to its `test result` line."""
    out, binary, t0 = [], None, None
    for line in Path(st).read_text(errors="replace").splitlines():
        ts, _, rest = line.partition("\t")
        rest = re.sub(r"\x1b\[[0-9;]*[A-Za-z]", "", rest)
        m = re.match(r"^ +Running (\S+)(?: (src/\S+))?", rest)
        if m:
            binary = m[1] + (" " + m[2] if m[2] and m[1] == "unittests" else "")
            continue
        if re.match(r"^running \d+ tests?$", rest):
            t0 = float(ts)
        elif rest.startswith("test result: ") and t0 is not None:
            out.append((binary, float(ts) - t0))
            t0 = None
    return out


def blocks(rows_path, colo_path, runs):
    """Per run and shard: each binary's wall against its planned seconds (its rows, its block, or the default)."""
    rows = table(rows_path, 3)
    colo = table(colo_path, 2)
    print("run\tshard\tbinary\tn\twall\tplanned\tdelta")
    for run in runs:
        run = Path(run)
        ran = {}
        for line in (run / "times.tsv").read_text().splitlines():
            k, b, name, s = line.split("\t")
            ran.setdefault((int(k), b), []).append(name)
        for k in range(12):
            for b, w in walls(run / f"st-{k}.txt"):
                names = ran.get((k, b), [])
                if not names:
                    continue
                planned = colo[(b,)] if (b,) in colo else sum(rows.get((b, n), 0.05) for n in names)
                print(f"{run.name}\t{k}\t{b}\t{len(names)}\t{w:.1f}\t{planned:.1f}\t{w - planned:+.1f}")


if __name__ == "__main__":
    if sys.argv[1] == "shards":
        shards(sys.argv[2:])
    elif sys.argv[1] == "tests":
        tests(sys.argv[2], sys.argv[3], sys.argv[4:])
    elif sys.argv[1] == "blocks":
        blocks(sys.argv[2], sys.argv[3], sys.argv[4:])
    else:
        sys.exit(__doc__)
