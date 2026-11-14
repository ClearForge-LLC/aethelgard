#!/usr/bin/env python3
"""AET-WO-4300 spike S1: the PA8 observer's edges with their virtual timestamps, from a case's raw
stdout (/cache/raw/<case>.stdout, drive.py). Prints count, first edges, and the high/low/period
statistics in microseconds. A case logged with host time has no virtual stamp: it prints counts only.

Usage: edges.py <case> [...]   (inside the image, or on the host with RAW=~/.cache/aet-4300/raw)
"""
import os, re, sys
from collections import Counter

RAW = os.environ.get("RAW", "/cache/raw")
LINE = re.compile(r"^\[(\d\d):(\d\d):(\d\d)\.(\d{9})\] \[NOISY\] gpioPortA\.pa8led: LED state changed to (True|False)")
ANY = re.compile(r"gpioPortA\.pa8led: LED state changed to (True|False)")

for case in sys.argv[1:]:
    text = open(f"{RAW}/{case}.stdout", encoding="utf-8", errors="replace").read()
    any_n = len(ANY.findall(text))
    ev = []
    for l in text.split("\n"):
        m = LINE.match(l.strip("\r"))
        if m:
            h, mi, s, ns, v = m.groups()
            ev.append((((int(h) * 60 + int(mi)) * 60 + int(s)) * 1_000_000_000 + int(ns), v == "True"))
    print(f"== {case}: {any_n} LED lines, {len(ev)} with a virtual stamp; raw tail kept {len(text)} chars")
    if not ev:
        continue
    print("   first:", [(round(t / 1000, 3), v) for t, v in ev[:6]])
    highs = [(b[0] - a[0]) / 1000 for a, b in zip(ev, ev[1:]) if a[1] and not b[1]]
    lows = [(b[0] - a[0]) / 1000 for a, b in zip(ev, ev[1:]) if not a[1] and b[1]]
    rises = [t for t, v in ev if v]
    periods = [(b - a) / 1000 for a, b in zip(rises, rises[1:])]
    for name, xs in (("high_us", highs), ("low_us", lows), ("period_us", periods)):
        if xs:
            print(f"   {name}: n={len(xs)} min={min(xs)} max={max(xs)} common={Counter(xs).most_common(3)}")
    stamps = Counter(t for t, _ in ev)
    print(f"   distinct stamps {len(stamps)}; most edges at one stamp {stamps.most_common(1)[0][1]}")
