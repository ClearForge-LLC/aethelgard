#!/usr/bin/env python3
"""AET-WO-1211a §1.4: give every PWM output of a committed specification `pwm.idle` =
{"pulse_us": pulse_min_us} and `pwm.resolution_ps` = 1,000,000, which restate what the firmware
wrote before (1211 §3). Version lines are renumber.py's, not this script's. A file is rewritten
only if it round-trips byte for byte through json.dumps(indent=2) first, so nothing else moves.
Usage: python3 -I migrate.py REPO [FILE...]   (no FILE: every tracked specification outside spike/)"""
import collections, json, subprocess, sys

repo = sys.argv[1]
files = sys.argv[2:] or [
    f for f in subprocess.run(["git", "-C", repo, "ls-files", "*.json"], capture_output=True,
                              text=True, check=True).stdout.split() if not f.startswith("spike/")]
moved = []
for rel in files:
    path = f"{repo}/{rel}"
    try:
        text = open(path).read()
    except UnicodeDecodeError:
        continue
    try:
        d = json.loads(text, object_pairs_hook=collections.OrderedDict)
    except ValueError:
        continue
    if not (isinstance(d, dict) and all(k in d for k in ("schema_version", "mcu", "motor_outputs", "project_name"))):
        continue
    n = 0
    for m in d["motor_outputs"]:
        p = m.get("pwm")
        if p is None:
            continue
        assert "idle" not in p and "resolution_ps" not in p, rel
        p["idle"] = collections.OrderedDict(pulse_us=p["pulse_min_us"])
        p["resolution_ps"] = 1_000_000
        n += 1
    if not n:
        continue
    before = json.loads(text, object_pairs_hook=collections.OrderedDict)
    nl = "\n" if text.endswith("\n") else ""
    assert json.dumps(before, indent=2) + nl == text, f"{rel} does not round-trip"
    open(path, "w").write(json.dumps(d, indent=2) + nl)
    moved.append((rel, n))
print(json.dumps(moved))
