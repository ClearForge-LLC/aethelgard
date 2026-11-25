#!/usr/bin/env python3
"""The CI probe (AET-WO-1918; measured as spike/1918's ci_probe.py, ruled at its stop): run on the
runner's host beside a job's container, it reads /proc every 0.2 s for

  - each `kani-driver`: its harness, from `--harness` in its argv, and its own VmHWM, a high-water
    mark, so a missed poll loses only the growth after the last one (kani-driver's last growth
    runs at about 1.25 GiB/s, so at most about 0.25 GiB: AET-WO-1915 section 5);
  - each Renode process (`--boot-comm`, default `renode`, the comm AET-WO-1915 measured): its
    lifetime, first seen to gone, with its argv.

When a watched process is gone it appends one line to OUT:
  PROBE peak harness=<name> vmhwm_kib=<n> seconds=<s>
  PROBE boot seconds=<s> comm=<c> argv=<first 300 chars>
It stops when the file STOP exists. ci/ceiling.sh judges the peaks against ci/kani-ceiling.
Stdlib only; run with python3 -I.

usage: probe.py OUT STOP [--boot-comm NAME]...
"""
import os
import sys
import time


def read(path):
    try:
        with open(path, "rb") as f:
            return f.read()
    except OSError:
        return None


def status(pid):
    text = read(f"/proc/{pid}/status")
    if text is None:
        return None
    out = {}
    for line in text.decode(errors="replace").splitlines():
        k, _, v = line.partition(":")
        if k in ("Name", "VmHWM"):
            out[k] = v.strip()
    return out


def argv(pid):
    raw = read(f"/proc/{pid}/cmdline")
    return [] if raw is None else [a.decode(errors="replace") for a in raw.split(b"\0") if a]


def main():
    out, stop = sys.argv[1], sys.argv[2]
    boots = set()
    rest = sys.argv[3:]
    while rest:
        if rest[0] == "--boot-comm" and len(rest) > 1:
            boots.add(rest[1])
            rest = rest[2:]
        else:
            sys.exit(f"ci_probe: unknown argument {rest[0]}")
    boots = boots or {"renode"}
    live = {}  # pid -> [kind, label, first_seen, vmhwm_kib]
    with open(out, "a", buffering=1) as log:
        while not os.path.exists(stop):
            now = time.monotonic()
            seen = set()
            for name in os.listdir("/proc"):
                if not name.isdigit():
                    continue
                pid = int(name)
                comm = (read(f"/proc/{pid}/comm") or b"").decode(errors="replace").strip()
                if comm != "kani-driver" and comm not in boots:
                    continue
                st = status(pid)
                if st is None:
                    continue
                seen.add(pid)
                hwm = int(st.get("VmHWM", "0 kB").split()[0] or 0)
                if pid not in live:
                    a = argv(pid)
                    if comm == "kani-driver":
                        label = a[a.index("--harness") + 1] if "--harness" in a[:-1] else "?"
                        live[pid] = ["peak", label, now, hwm]
                    else:
                        live[pid] = ["boot", f"comm={comm} argv={' '.join(a)[:300]}", now, hwm]
                live[pid][3] = max(live[pid][3], hwm)
            for pid in [p for p in live if p not in seen]:
                kind, label, first, hwm = live.pop(pid)
                if kind == "peak":
                    log.write(f"PROBE peak harness={label} vmhwm_kib={hwm} seconds={now - first:.1f}\n")
                else:
                    log.write(f"PROBE boot seconds={now - first:.1f} {label}\n")
            time.sleep(0.2)


if __name__ == "__main__":
    main()
