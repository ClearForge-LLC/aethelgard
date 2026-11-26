#!/usr/bin/env python3
"""The CI probe (AET-WO-1918; measured as spike/1918's ci_probe.py, ruled at its stop): run on the
runner's host beside a job's container, it reads /proc every 0.2 s for

  - each `kani-driver`: its harness, from `--harness` in its argv, and its own VmHWM, a high-water
    mark, so a missed poll loses only the growth after the last one (kani-driver's last growth
    runs at about 1.25 GiB/s, so at most about 0.25 GiB: AET-WO-1915 section 5);
  - each `cbmc`, labelled `<harness>/cbmc` by the kani-driver beside it, the same way;
  - each Renode process (comm `renode`, as AET-WO-1915 measured): its lifetime, first seen to gone,
    with its argv;
  - each `aethelgard` subcommand (AET-WO-1919): `verify` and every gate it runs as its own process,
    with its argv, so a job's log names every gate and harness that ran and how long each took (the
    CLI's internal reaper, confine and guard processes are not gates, and are skipped);
  - each `kani-compiler`, labelled `<harness>/compile` by the kani-driver beside it: a prove
    gate's compile, apart from CBMC's solve and kani-driver's parse.

When a watched process is gone, or the probe stops while it lives, it appends one line to OUT, where
start is the seconds from the probe's own start to the process's first sighting:
  PROBE peak harness=<name> vmhwm_kib=<n> seconds=<s> start=<s>
  PROBE boot seconds=<s> start=<s> comm=<c> argv=<first 300 chars>
  PROBE gate seconds=<s> start=<s> argv=<first 300 chars>
and at the stop, once (AET-WO-1919 (X), the two lanes' memory side by side):
  PROBE combined peak_rss_kib=<n> at=<s>   the highest sum, at one poll, of every watched process's
                                           VmRSS (each kani-driver, cbmc, kani-compiler, Renode and
                                           gate), and of each kicad-cli, cargo, rustc and the analog
                                           bridge, counted but given no line
  PROBE host peak_used_kib=<n> at=<s>      the highest MemTotal - MemAvailable the probe saw
It stops when the file STOP exists. ci/ceiling.sh judges the peaks against ci/kani-ceiling.
Stdlib only; run with python3 -I.

usage: probe.py OUT STOP
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
        if k in ("VmHWM", "VmRSS"):
            out[k] = v.strip()
    return out


def argv(pid):
    raw = read(f"/proc/{pid}/cmdline")
    return [] if raw is None else [a.decode(errors="replace") for a in raw.split(b"\0") if a]


def main():
    out, stop = sys.argv[1:3]
    live = {}  # pid -> [kind, label, first_seen, vmhwm_kib]
    origin = time.monotonic()
    combined, host = [0, 0.0], [0, 0.0]

    def host_used():
        info = {}
        for line in (read("/proc/meminfo") or b"").decode(errors="replace").splitlines():
            k, _, v = line.partition(":")
            info[k] = v.split()[0] if v.split() else "0"
        return int(info.get("MemTotal", 0)) - int(info.get("MemAvailable", 0))

    def emit(log, pid, now):
        kind, label, first, hwm = live.pop(pid)
        seconds, start = f"{now - first:.1f}", f"{first - origin:.1f}"
        if kind == "peak":
            log.write(f"PROBE peak harness={label} vmhwm_kib={hwm} seconds={seconds} start={start}\n")
        else:
            log.write(f"PROBE {kind} seconds={seconds} start={start} {label}\n")

    def driving():
        # The harness the newest live kani-driver proves (one harness at a time).
        drivers = [v for v in live.values() if v[0] == "peak" and "/" not in v[1]]
        return drivers[-1][1] if drivers else "?"

    with open(out, "a", buffering=1) as log:
        while True:
            stopping = os.path.exists(stop)
            now = time.monotonic()
            seen = set()
            rss = 0
            for name in os.listdir("/proc"):
                if not name.isdigit():
                    continue
                pid = int(name)
                comm = (read(f"/proc/{pid}/comm") or b"").decode(errors="replace").strip()
                if comm in ("kicad-cli", "cargo", "rustc", "aethelgard-brid"):
                    st = status(pid)
                    if st is not None:
                        rss += int(st.get("VmRSS", "0 kB").split()[0] or 0)
                    continue
                if comm not in ("kani-driver", "cbmc", "renode", "aethelgard", "kani-compiler"):
                    continue
                st = status(pid)
                if st is None:
                    continue
                seen.add(pid)
                hwm = int(st.get("VmHWM", "0 kB").split()[0] or 0)
                rss_now = int(st.get("VmRSS", "0 kB").split()[0] or 0)
                if pid not in live:
                    a = argv(pid)
                    if comm == "kani-driver":
                        label = a[a.index("--harness") + 1] if "--harness" in a[:-1] else "?"
                        live[pid] = ["peak", label, now, hwm]
                    elif comm == "cbmc":
                        # CBMC's argv names no harness: it is the one its
                        # kani-driver is proving (one harness at a time).
                        live[pid] = ["peak", driving() + "/cbmc", now, hwm]
                    elif comm == "kani-compiler":
                        live[pid] = ["compile", f"harness={driving()}/compile", now, hwm]
                    elif comm == "aethelgard":
                        if len(a) < 2 or any(x.startswith("--aethelgard-internal") for x in a[1:]):
                            seen.discard(pid)
                            continue
                        live[pid] = ["gate", f"argv={' '.join(a[1:])[:300]}", now, hwm]
                    else:
                        live[pid] = ["boot", f"comm={comm} argv={' '.join(a)[:300]}", now, hwm]
                live[pid][3] = max(live[pid][3], hwm)
                rss += rss_now
            if rss > combined[0]:
                combined[:] = [rss, now - origin]
            used = host_used()
            if used > host[0]:
                host[:] = [used, now - origin]
            # A process gone is written; at the stop, so is every one still
            # alive, so nothing measured is dropped.
            for pid in [p for p in live if stopping or p not in seen]:
                emit(log, pid, now)
            if stopping:
                log.write(f"PROBE combined peak_rss_kib={combined[0]} at={combined[1]:.1f}\n")
                log.write(f"PROBE host peak_used_kib={host[0]} at={host[1]:.1f}\n")
                break
            time.sleep(0.2)


if __name__ == "__main__":
    main()
