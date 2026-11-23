#!/usr/bin/env python3
"""AET-WO-1916, the plan's D1: a clause that fires on what a refit can change, a shard persistently over the others.
Candidate: over three runs of one plan, a shard's mean of log(step / that run's median step) passes log(T). Per run, each
shard's step is exp(N(0, s)) times its own persistent factor (1 for every shard, or one shard at 1 + d).
  persist.py   ->  for T and d: P(fires) over 12 shards"""
import math
import random
import statistics as st

rng = random.Random(1916)
N = 20000
for s in (0.154, 0.163):
    print(f"speed sd {s}")
    for t in (1.2, 1.25, 1.3):
        cells = []
        for d in (0.0, 0.2, 0.3, 0.4):
            fired = 0
            for _ in range(N):
                acc = [0.0] * 12
                for _ in range(3):
                    x = [rng.gauss(0, s) + (math.log(1 + d) if k == 0 else 0) for k in range(12)]
                    m = st.median(x)
                    for k in range(12):
                        acc[k] += (x[k] - m) / 3
                fired += max(acc) > math.log(t)
            cells.append(f"d {d:.1f}: {fired / N:.2f}")
        print(f"  T {t}: " + ", ".join(cells))
