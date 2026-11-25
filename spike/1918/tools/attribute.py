#!/usr/bin/env python3
"""AET-WO-1918 (1915's tool, its paths renamed): attribute one run's samples to phase and process.

usage: attribute.py <samples.jsonl> [--timeline | --table]
Prints one JSON summary:
  - memory.peak at the last sample; the largest memory.current, and the
    largest anon, file and shmem from memory.stat, with their times;
  - per phase (the nearest `aethelgard <subcommand>` ancestor; `prove`
    split by harness): first and last time seen, the largest sum of
    RssAnon + RssShmem over its processes at one sample, and its largest VmHWM;
  - per process class (cbmc, kani-compiler, rustc, cargo, kani driver, goto,
    renode, ngspice, kicad, aethelgard, podman/conmon, other): the largest
    RssAnon + RssShmem of one process, and its largest VmHWM;
  - at the sample of the largest anon, every process holding 64 MiB or more;
  - overlap: samples where a cargo, rustc or kani-compiler job is alive beside
    cbmc (WO §5, a cargo job still alive during Kani), and samples where two
    or more processes each hold 1 GiB or more;
  - orphans: processes with no aethelgard ancestor in the scope.
Stdlib only; run with python3 -I.
"""
import json
import sys

GIB = 1 << 30
KIB = 1024


def klass(p):
    c, cmd = p["comm"], p["cmd"]
    exe = cmd.split(" ", 1)[0].rsplit("/", 1)[-1] if cmd else c
    for name, keys in (
        ("cbmc", ("cbmc",)),
        ("kani-compiler", ("kani-compiler",)),
        ("goto", ("goto-cc", "goto-instrument", "goto-analyzer")),
        ("kani-driver", ("kani-driver", "cargo-kani", "kani")),
        ("rustc", ("rustc",)),
        ("cargo", ("cargo",)),
        # ngspice runs in-process in the analog bridge (libngspice, RUL-17).
        ("ngspice", ("ngspice", "aethelgard-bridge", "aethelgard-brid")),
        ("kicad", ("kicad-cli", "kicad")),
        ("aethelgard", ("aethelgard",)),
        ("podman/conmon", ("podman", "conmon", "catatonit")),
    ):
        if exe in keys or c in keys:
            return name
    if "enode" in c or "enode" in exe or "Renode" in cmd or c in ("mono", "dotnet", "mono-sgen"):
        return "renode"
    return "other:" + c


def gate_of(p, by_pid):
    """The nearest `aethelgard <subcommand>` ancestor (self included)."""
    seen = set()
    q = p
    while q and q["pid"] not in seen:
        seen.add(q["pid"])
        argv = q["cmd"].split()
        if argv and argv[0].endswith("aethelgard") and len(argv) > 1 and not argv[1].startswith("--"):
            sub = argv[1]
            if sub == "check" and len(argv) > 2:
                sub = "check " + argv[2]
            if sub == "prove" and "--harness" in argv:
                sub = "prove " + argv[argv.index("--harness") + 1].removeprefix("proofs::")
            return sub
        q = by_pid.get(q["ppid"])
    return None


def mem(p):
    return (p.get("RssAnon", 0) + p.get("RssShmem", 0)) * KIB


def main():
    path = sys.argv[1]
    timeline = "--timeline" in sys.argv
    samples, stops = [], []
    for line in open(path):
        s = json.loads(line)
        if "procs" in s:
            samples.append(s)
        elif "STOP" in s:
            stops.append(s)
    if not samples:
        sys.exit("attribute.py: no samples")
    last = samples[-1]
    best = {k: max(samples, key=lambda s, k=k: s["stat"].get(k, 0)) for k in ("anon", "file", "shmem")}
    cur = max(samples, key=lambda s: s["current"])
    phases, classes, overlap, two_big, orphans, lines = {}, {}, [], [], set(), []
    for s in samples:
        by_pid = {p["pid"]: p for p in s["procs"]}
        per_phase = {}
        kinds = set()
        big = []
        for p in s["procs"]:
            k = klass(p)
            kinds.add(k)
            g = gate_of(p, by_pid)
            if g is None and k not in ("podman/conmon",):
                if not (k == "other:bash" or k == "other:sh"):
                    orphans.add(f"{k} {p['cmd'][:80]}")
                g = "(no gate)"
            if g is None:
                g = "(podman)"
            m = mem(p)
            per_phase.setdefault(g, [0, 0])
            per_phase[g][0] += m
            per_phase[g][1] = max(per_phase[g][1], p.get("VmHWM", 0) * KIB)
            c = classes.setdefault(k, {"rss_max_gib": 0, "hwm_max_gib": 0, "at": None, "phase": None})
            if m / GIB > c["rss_max_gib"]:
                c.update(rss_max_gib=round(m / GIB, 3), at=s["t"], phase=g)
            c["hwm_max_gib"] = max(c["hwm_max_gib"], round(p.get("VmHWM", 0) * KIB / GIB, 3))
            if m >= GIB:
                big.append(f"{k}@{g}")
        for g, (m, hwm) in per_phase.items():
            ph = phases.setdefault(g, {"first": s["t"], "last": s["t"], "rss_max_gib": 0, "hwm_max_gib": 0, "at": None})
            ph["last"] = s["t"]
            if m / GIB > ph["rss_max_gib"]:
                ph.update(rss_max_gib=round(m / GIB, 3), at=s["t"])
            ph["hwm_max_gib"] = max(ph["hwm_max_gib"], round(hwm / GIB, 3))
        if "cbmc" in kinds and kinds & {"cargo", "rustc", "kani-compiler"}:
            overlap.append(s["t"])
        if len(big) >= 2:
            two_big.append((s["t"], big))
        if timeline:
            top = max(s["procs"], key=mem)
            by_k = {}
            for p in s["procs"]:
                by_k[klass(p)] = by_k.get(klass(p), 0) + mem(p)
            lines.append(f"{s['t']:8.1f} anon={s['stat'].get('anon', 0) / GIB:6.3f} file={s['stat'].get('file', 0) / GIB:6.3f} "
                         f"cbmc={by_k.get('cbmc', 0) / GIB:6.3f} kani-driver={by_k.get('kani-driver', 0) / GIB:6.3f} "
                         f"cur={s['current'] / GIB:6.3f} top={klass(top)}@{gate_of(top, by_pid)} {mem(top) / GIB:.3f}")
    at = best["anon"]
    by_pid = {p["pid"]: p for p in at["procs"]}
    at_peak = sorted(
        ({"class": klass(p), "phase": gate_of(p, by_pid), "rss_gib": round(mem(p) / GIB, 3),
          "hwm_gib": round(p.get("VmHWM", 0) * KIB / GIB, 3), "cmd": p["cmd"][:160]}
         for p in at["procs"] if mem(p) >= 64 << 20),
        key=lambda x: -x["rss_gib"])
    out = {
        "samples": len(samples),
        "wall_s": last["t"],
        "memory_peak_gib": round(last["peak"] / GIB, 3) if last.get("peak") else None,
        "current_max_gib": round(cur["current"] / GIB, 3), "current_max_at": cur["t"],
        "anon_max_gib": round(at["stat"]["anon"] / GIB, 3), "anon_max_at": at["t"],
        "file_max_gib": round(best["file"]["stat"].get("file", 0) / GIB, 3),
        "shmem_max_gib": round(best["shmem"]["stat"].get("shmem", 0) / GIB, 3),
        "oom_events": last["events"].get("oom_kill", 0),
        "host_avail_min_gib": round(min(s["host"].get("MemAvailable", 0) for s in samples) * KIB / GIB, 1),
        "stops": len(stops),
        "at_anon_peak": at_peak,
        "phases": phases,
        "classes": dict(sorted(classes.items(), key=lambda kv: -kv[1]["rss_max_gib"])),
        "cbmc_beside_build_samples": overlap,
        "two_procs_over_1gib": two_big[:20],
        "orphans": sorted(orphans)[:20],
    }
    # The scope's memory.peak read inside the container as its last step (ctl.sh):
    # it sees growth after the last 1 s sample, which the sampler cannot.
    import os
    log = os.path.join(os.path.dirname(path), "cap.log")
    for line in open(log, errors="replace") if os.path.exists(log) else []:
        if line.startswith("CTL1918 "):
            kv = dict(x.split("=", 1) for x in line.split()[1:])
            out["ctl_rc"] = int(kv["rc"])
            out["ctl_wall_s"] = round(float(kv["wall_s"]), 1)
            out["inner_memory_peak_gib"] = round(int(kv["memory.peak"]) / GIB, 3)
    if "--table" in sys.argv:
        top = at_peak[0] if at_peak else {}
        print("\t".join(str(x) for x in (
            os.path.basename(os.path.dirname(path)), out.get("ctl_rc"), out.get("ctl_wall_s"),
            out.get("inner_memory_peak_gib"), out["memory_peak_gib"], out["anon_max_gib"],
            out["file_max_gib"], out["shmem_max_gib"], out["oom_events"],
            f"{top.get('class')}@{top.get('phase')}:{top.get('rss_gib')}",
            len(out["cbmc_beside_build_samples"]), len(out["two_procs_over_1gib"]))))
        return
    print(json.dumps(out, indent=1))
    if timeline:
        print("\n".join(lines))


if __name__ == "__main__":
    main()
