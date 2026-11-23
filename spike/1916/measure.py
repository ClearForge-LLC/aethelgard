#!/usr/bin/env python3
"""AET-WO-1916 §1.1: the measurement. Read-only over fetched CI runs (spike/1912/fetch.sh layout, then
`stamp.sh`): each run's tools-shard jobs, split into setup (job start to the `shard.sh run` step), the test step
(that step to the next step) and the rest; the plan's planned seconds; and each test's in-shard seconds by
`ci/shard.sh times` (AET-WO-1914 P3), against its row.

  measure.py shards RUNDIR...            per shard: job, setup, step, after, planned (minutes / seconds)
  measure.py tests ROWS COLO RUNDIR...   per test of 10 s or more: samples, worst, row
  measure.py blocks ROWS COLO RUNDIR...  per run, shard and binary: wall against planned seconds
  measure.py clause SHARDS RUN,RUN,RUN    the REFIT RULE's persistent-shard clause (D1) over three runs of one plan
  measure.py check BLOCKS SHARDS BASE NEW,NEW RUN,RUN,...   predictions.md P1-P3
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


def check(blocks_path, shards_path, base, news, plan_runs):
    """predictions.md P1-P3 over `blocks` and `shards` output."""
    import math
    import statistics as st
    planned, steps = {}, {}
    rid = None
    for line in Path(shards_path).read_text().splitlines():
        if line.startswith("## "):
            rid = line.split()[1]
        elif line and line[0].isdigit():
            f = line.split("\t")
            planned[(rid, int(f[0]))] = f[5]
            steps[(rid, int(f[0]))] = float(f[3])
    for n in news:
        same = all(planned[(n, k)] == planned[(base, k)] for k in range(12))
        print(f"P1 {n}: planned equal to {base}'s on every shard: {same}")
    wall = {}
    for line in Path(blocks_path).read_text().splitlines()[1:]:
        r, k, b, _, w, _, _ = line.split("\t")
        wall[(r, int(k), b)] = float(w)
    for n in news:
        agree = total = 0
        factors = []
        for k in range(12):
            lr = [math.log(wall[(n, k, b)] / wall[(base, k, b)]) for (r, kk, b) in wall
                  if r == n and kk == k and (base, k, b) in wall and wall[(n, k, b)] >= 40 and wall[(base, k, b)] >= 40]
            if len(lr) >= 2:
                total += 1
                agree += max(lr) - min(lr) <= 0.15
                factors.append(st.mean(lr))
                print(f"   shard {k}: " + " ".join(f"{x:+.2f}" for x in lr))
        print(f"P2 {n} against {base}: {agree} of {total} shards within 0.15; shard factors "
              f"{min(factors):+.2f} to {max(factors):+.2f}")
    vec = {r: [steps[(r, k)] for k in range(12)] for r in plan_runs}
    rs = []
    for i, a in enumerate(plan_runs):
        for b in plan_runs[i + 1:]:
            rs.append(st.correlation(vec[a], vec[b]))
            print(f"   r({a}, {b}) = {rs[-1]:+.2f}")
    print(f"P3 mean pairwise correlation over {len(plan_runs)} runs: {st.mean(rs):+.2f}")


def clause(shards_path, trio):
    """The REFIT RULE's persistent-shard clause (AET-WO-1916 D1) over three runs of one plan: each shard's mean of
    ln(step / that run's median step); it fires when one passes ln 1.25."""
    import math
    import statistics as st
    steps, rid = {}, None
    for line in Path(shards_path).read_text().splitlines():
        if line.startswith("## "):
            rid = line.split()[1]
            steps[rid] = {}
        elif line and line[0].isdigit():
            f = line.split("\t")
            steps[rid][int(f[0])] = float(f[3])
    acc = [0.0] * 12
    for r in trio:
        m = st.median(steps[r].values())
        for k in range(12):
            acc[k] += math.log(steps[r][k] / m) / len(trio)
    worst = max(range(12), key=lambda k: acc[k])
    print(" ".join(f"{k}:x{math.exp(a):.2f}" for k, a in enumerate(acc)))
    print(f"worst shard {worst} at x{math.exp(acc[worst]):.2f}: the clause "
          f"{'FIRES' if acc[worst] > math.log(1.25) else 'does not fire'}")


if __name__ == "__main__":
    if sys.argv[1] == "shards":
        shards(sys.argv[2:])
    elif sys.argv[1] == "tests":
        tests(sys.argv[2], sys.argv[3], sys.argv[4:])
    elif sys.argv[1] == "blocks":
        blocks(sys.argv[2], sys.argv[3], sys.argv[4:])
    elif sys.argv[1] == "clause":
        clause(sys.argv[2], sys.argv[3].split(","))
    elif sys.argv[1] == "check":
        check(sys.argv[2], sys.argv[3], sys.argv[4], sys.argv[5].split(","), sys.argv[6].split(","))
    else:
        sys.exit(__doc__)
