#!/usr/bin/env python3
"""AET-WO-1916 §1.1: how often a plan meets the REFIT RULE's spread, given the noise the held runs show.

  odds.py SHARDS_TSV PAIRS...    PAIRS are RUNA,RUNB of the same plan (`measure.py shards` output)

The model, from the held runs (measurement.md): each tools-shard job's test step is its planned share of the work times
one speed factor for its runner, exp(N(0, s)), the same for every test it runs; its setup (checkout, image pull,
artifact, unpack) is drawn from the held setups. s is read from the same-plan pairs: the sd of log(step_a / step_b)
over their shards, over sqrt 2; and, as the lower bracket, the median within-run sd of log step (which also holds any
persistent difference between shards, so it is an upper bound on the noise in a run, and it reads lower than the pairs). A job's minutes are setup plus step; the spread is the slowest job less the fastest.
"""
import math
import random
import statistics as st
import sys


def load(path):
    runs, rid = {}, None
    for line in open(path):
        line = line.rstrip("\n")
        if line.startswith("## "):
            rid = line.split()[1]
            runs[rid] = {}
        elif line and line[0].isdigit():
            f = line.split("\t")
            runs[rid][int(f[0])] = (float(f[2]), float(f[3]))  # setup s, step s
    return runs


def main():
    runs = load(sys.argv[1])
    pairs = [p.split(",") for p in sys.argv[2:]]
    logs = [math.log(runs[a][k][1] / runs[b][k][1]) for a, b in pairs for k in range(12)]
    s_pairs = st.pstdev(logs) / math.sqrt(2)
    within = [st.pstdev([math.log(v[1]) for v in r.values()]) for r in runs.values()]
    setups = [v[0] for r in runs.values() for v in r.values()]
    means = {rid: st.mean(v[1] for v in r.values()) for rid, r in runs.items()}
    print(f"speed sd (same-plan pairs, {len(logs)} shard pairs): {s_pairs:.3f}")
    print("speed sd within each run (all shards, log step): " + ", ".join(f"{x:.3f}" for x in within))
    print(f"setup s over {len(setups)} jobs: mean {st.mean(setups):.0f}, sd {st.pstdev(setups):.0f}, "
          f"{min(setups):.0f}-{max(setups):.0f}")
    print("mean step s per run: " + ", ".join(f"{k}:{v:.0f}" for k, v in means.items()))
    for s in (s_pairs, st.median(within)):
        table(s, setups)


def table(s, setups):
    rng = random.Random(1916)
    n = 20000
    print(f"\nspeed sd {s:.3f}")
    print("shards\tmean step s\tP(spread <= 6 min)\tP(two in a row <= 6)\tmedian spread min\tP(slowest > 20 min)")
    for shards, m in [(12, 790), (12, 720), (12, 700), (12, 650), (12, 600), (12, 550), (12, 500),
                      (14, 720 * 12 / 14), (16, 720 * 12 / 16)]:
        ok = over20 = 0
        spreads = []
        for _ in range(n):
            jobs = [rng.choice(setups) + m * math.exp(rng.gauss(0, s)) + 4 for _ in range(shards)]
            sp = (max(jobs) - min(jobs)) / 60
            spreads.append(sp)
            ok += sp <= 6
            over20 += max(jobs) > 1200
        p = ok / n
        print(f"{shards}\t{m:.0f}\t{p:.2f}\t{p * p:.2f}\t{st.median(spreads):.1f}\t{over20 / n:.2f}")
    # At 12 shards and the held mean step (700 s): how often the rule fires (two runs in a row over X min).
    spreads = []
    for _ in range(n):
        jobs = [rng.choice(setups) + 700 * math.exp(rng.gauss(0, s)) + 4 for _ in range(12)]
        spreads.append((max(jobs) - min(jobs)) / 60)
    print("12 shards, 700 s: X min, P(spread > X), P(fires: two in a row > X) = "
          + "; ".join(f"{x}: {sum(v > x for v in spreads) / n:.2f}, {(sum(v > x for v in spreads) / n) ** 2:.2f}"
                      for x in (6, 7, 8, 9, 10)))


if __name__ == "__main__":
    main()
