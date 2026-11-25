#!/usr/bin/env python3
"""AET-WO-1918 (1915's tool, its paths renamed): sample one capped-run scope at 1 s, from the host.

Reads the scope from capped-run's `scope=` stderr line, then, every second until
the scope is gone, writes one JSON line: the cgroup's memory.current, memory.peak,
memory.stat (anon, file, shmem, kernel), memory.events, the host's MemAvailable and
MemFree, and each process in cgroup.procs with its parent, command line and
/proc/<pid>/status memory (RssAnon, RssFile, RssShmem, VmHWM).

Furnace's memory rule (WO §3): if MemAvailable falls under 12 GiB, the scope is
stopped (it is ours: capped-run made it) and a STOP line is written.

usage: sample.py <capped-run stderr log> <out.jsonl>
Stdlib only, run with python3 -I. No Cargo.toml (WO §1.2).
"""
import json
import os
import subprocess
import sys
import time

# 12 GiB of MemAvailable; AET1918_FLOOR_GIB overrides it only to test the stop.
FLOOR_KIB = int(os.environ.get("AET1918_FLOOR_GIB", "12")) * 1024 * 1024
STAT_KEYS = ("anon", "file", "shmem", "kernel", "file_dirty", "file_writeback")
STATUS_KEYS = ("VmRSS", "RssAnon", "RssFile", "RssShmem", "VmHWM")


def read(path):
    try:
        with open(path) as f:
            return f.read()
    except OSError:
        return None


def meminfo():
    out = {}
    for line in (read("/proc/meminfo") or "").splitlines():
        k, _, v = line.partition(":")
        if k in ("MemAvailable", "MemFree"):
            out[k] = int(v.split()[0])
    return out


def proc(pid):
    stat = read(f"/proc/{pid}/stat")
    if stat is None:
        return None
    # comm may hold spaces and parentheses: split after the last ')'.
    rest = stat[stat.rindex(")") + 2 :].split()
    comm = stat[stat.index("(") + 1 : stat.rindex(")")]
    p = {"pid": pid, "ppid": int(rest[1]), "comm": comm, "start": int(rest[19])}
    cmd = read(f"/proc/{pid}/cmdline")
    p["cmd"] = (cmd or "").replace("\0", " ").strip()[:400]
    for line in (read(f"/proc/{pid}/status") or "").splitlines():
        k, _, v = line.partition(":")
        if k in STATUS_KEYS:
            p[k] = int(v.split()[0])  # kB
    return p


def scope_from(log):
    for _ in range(600):
        for line in (read(log) or "").splitlines():
            if line.startswith("capped-run: scope="):
                return line.split()[1].split("=", 1)[1]
        time.sleep(0.1)
    sys.exit("sample.py: no scope= line in 60 s")


def main():
    log, out = sys.argv[1], sys.argv[2]
    scope = scope_from(log)
    d = "/sys/fs/cgroup" + scope
    unit = os.path.basename(scope)
    t0 = time.time()
    stopped = False
    with open(out, "a") as f:
        f.write(json.dumps({"scope": scope, "t0": t0}) + "\n")
        while os.path.isdir(d):
            t = time.time()
            s = {"t": round(t - t0, 3)}
            cur, peak = read(d + "/memory.current"), read(d + "/memory.peak")
            if cur is None:
                break
            s["current"] = int(cur)
            s["peak"] = int(peak) if peak else None
            stat = {}
            for line in (read(d + "/memory.stat") or "").splitlines():
                k, v = line.split()
                if k in STAT_KEYS:
                    stat[k] = int(v)
            s["stat"] = stat
            ev = {}
            for line in (read(d + "/memory.events") or "").splitlines():
                k, v = line.split()
                ev[k] = int(v)
            s["events"] = ev
            s["host"] = meminfo()
            pids = [int(x) for x in (read(d + "/cgroup.procs") or "").split()]
            s["procs"] = [p for p in map(proc, pids) if p]
            f.write(json.dumps(s) + "\n")
            f.flush()
            # Once under the floor, SIGKILL the scope (ours) on every sample until it is gone:
            # `systemctl stop` waits on a PID 1 that ignores SIGTERM (measured).
            if stopped or s["host"].get("MemAvailable", FLOOR_KIB) < FLOOR_KIB:
                env = dict(os.environ, XDG_RUNTIME_DIR=f"/run/user/{os.getuid()}")
                r = subprocess.run(["systemctl", "--user", "kill", "-s", "KILL", unit], env=env,
                                   capture_output=True, text=True, check=False)
                f.write(json.dumps({"STOP": "MemAvailable under the floor", "host": s["host"],
                                    "t": s["t"], "rc": r.returncode, "err": r.stderr[-300:]}) + "\n")
                f.flush()
                stopped = True
            time.sleep(max(0.0, 1.0 - (time.time() - t)))
        f.write(json.dumps({"end": round(time.time() - t0, 3), "stopped": stopped}) + "\n")


if __name__ == "__main__":
    main()
