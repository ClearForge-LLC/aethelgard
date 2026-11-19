#!/bin/bash
# Inside the image, under MEM=16G: the unmodified CLI's `verify` on Sentinel-03, no --fab-out, no
# --scenarios (the gates it runs today), once. Peak RSS: the largest child process (getrusage) and,
# when the scope's cgroup files are visible inside the container, the scope's memory.peak.
set -u
O=/cache/verify-s03; rm -rf $O; mkdir -p $O; cd $O
CG=$(sed -n 's/^0:://p' /proc/self/cgroup)
echo "cgroup: $CG"; ls /sys/fs/cgroup/memory.peak 2>/dev/null && cat /sys/fs/cgroup/memory.max /sys/fs/cgroup/memory.swap.max 2>/dev/null
python3 -I - <<'PY'
import resource, subprocess, time, json
t = time.time()
p = subprocess.run(["/cache/target/debug/aethelgard", "verify", "--spec",
                    "/repo/crates/aethelgard-cli/tests/fixtures/sentinel-03.json"],
                   stdout=open("verify.json", "w"), stderr=open("verify.err", "w"))
r = resource.getrusage(resource.RUSAGE_CHILDREN)
peak = None
try:
    peak = int(open("/sys/fs/cgroup/memory.peak").read())
except OSError:
    pass
print(json.dumps({"exit": p.returncode, "wall_s": round(time.time() - t, 1),
                  "max_child_rss_kib": r.ru_maxrss, "scope_memory_peak_bytes": peak}))
PY
python3 -I -c "import json; v=json.load(open('verify.json')); print('verdict', v.get('verdict'), [ (g.get('gate'), g.get('verdict')) for g in v.get('gates',[])][:12] if isinstance(v.get('gates'),list) else list(v.keys())[:12])" 2>&1 | head -5
