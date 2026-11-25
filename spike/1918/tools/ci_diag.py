#!/usr/bin/env python3
"""AET-WO-1918: summarize one diagnostic CI run from its job logs (`gh run view --job ID --log`).

For each job log <run>-<job>.log in DIR: the job name, the runner's CPU model (the lscpu step),
the Renode boots the probe logged (n, median, p95, max, and every boot over 15 s), and the
kani-driver peaks (each harness's VmHWM). Writes TSV rows to stdout:
  boots  <run> <job> <cpu> <n> <median_s> <p95_s> <max_s> <over15 list>
  peak   <run> <job> <cpu> <harness> <gib> <seconds>
Stdlib only; run with python3 -I.

usage: ci_diag.py DIR RUN
"""
import glob
import os
import re
import statistics
import sys

d, run = sys.argv[1], sys.argv[2]
for path in sorted(glob.glob(os.path.join(d, f"{run}-*.log"))):
    job = cpu = None
    boots, peaks = [], []
    with open(path, errors="replace") as f:
        for line in f:
            parts = line.rstrip("\n").split("\t", 2)
            if len(parts) < 3:
                continue
            job = job or parts[0]
            text = re.sub(r"^\S+Z ", "", parts[2].lstrip("﻿"))
            m = re.match(r"Model name:\s+(.*)", text)
            if m and cpu is None:
                cpu = m.group(1).strip()
            m = re.match(r"PROBE boot seconds=([0-9.]+) ", text)
            if m:
                boots.append(float(m.group(1)))
            m = re.match(r"PROBE peak harness=(\S+) vmhwm_kib=(\d+) seconds=([0-9.]+)", text)
            if m:
                peaks.append((m.group(1), int(m.group(2)) / 2**20, float(m.group(3))))
    if boots:
        b = sorted(boots)
        p95 = b[min(len(b) - 1, int(len(b) * 0.95))]
        over = ",".join(f"{x:.1f}" for x in b if x > 15)
        print(f"boots\t{run}\t{job}\t{cpu}\t{len(b)}\t{statistics.median(b):.1f}\t{p95:.1f}\t{b[-1]:.1f}\t{over}")
    for h, g, s in sorted(peaks, key=lambda p: -p[1]):
        if g >= 1.0:
            print(f"peak\t{run}\t{job}\t{cpu}\t{h}\t{g:.3f}\t{s:.1f}")
