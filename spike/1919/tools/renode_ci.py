#!/usr/bin/env python3
"""AET-WO-1919 (2): every Renode run in CI's job logs (ci/probe.py's `PROBE boot` lines), by kind and
by runner CPU. Kinds, by the argv the CLI builds (main.rs simulate_with; renode::commands for a load,
BootPlan::commands for a boot, which starts `emulation SetSeed`):
  version  `--version` (simulate's and doctor's pin check)
  load     `-e` starting `include @` or `mach create` (simulate --platform)
  boot     `-e` starting `emulation SetSeed` (simulate --firmware, campaign, verify, the bridge's)
  direct   no `--config`: a test driving Renode itself (twin_tier2's helpers, models_tier2)
  other    none of these (a tier-1 stand-in, for one)
Each kind but direct is prefixed `cli-` when its configuration is in the CLI's own scratch
(/var/lib/aethelgard/, where the image's CLI puts it), else `test-`: a test that wrote its own
configuration and drove Renode itself (models_tier2's /tmp/aet-4220-models-*), or a stand-in.
usage: renode_ci.py OUT [--list KIND]"""
import glob, os, re, statistics, sys
from collections import defaultdict

def kind(argv):
    k = kind0(argv)
    if k == "direct":
        return k
    return ("cli-" if "--config /var/lib/aethelgard/" in argv else "test-") + k

def kind0(argv):
    if "--config" not in argv:
        first = (argv.split() or [""])[0]
        return "direct" if "renode" in first and "/tmp/" not in first else "other"
    if argv.rstrip().endswith("--version"):
        return "version"
    m = re.search(r" -e (.*)", argv)
    if not m:
        return "other"
    e = m.group(1)
    if e.startswith("emulation SetSeed"):
        return "boot"
    if e.startswith("include @") or e.startswith("mach create"):
        return "load"
    return "other"

def cpu(text):
    m = re.search(r"Model name:\s+(.+)", text)
    return m.group(1).strip() if m else "?"

rows = defaultdict(list)  # (kind, cpu) -> [(seconds, run, job, argv)]
for d in sorted(glob.glob(os.path.join(sys.argv[1], "*-a*"))):
    for log in glob.glob(os.path.join(d, "*.log")):
        text = open(log, errors="replace").read()
        c = cpu(text)
        for line in text.splitlines():
            m = re.search(r"PROBE boot seconds=([0-9.]+) (?:start=[0-9.]+ )?comm=\S+ argv=(.*)", line)
            if m:
                rows[(kind(m.group(2)), c)].append((float(m.group(1)), os.path.basename(d), os.path.basename(log)[:-4], m.group(2)))
if "--list" in sys.argv:
    k = sys.argv[sys.argv.index("--list") + 1]
    for (kk, c), v in sorted(rows.items()):
        if kk == k:
            for s, r, j, a in sorted(v, reverse=True)[:15]:
                print(f"{s:7.1f}\t{c}\t{r}\t{j}\t{a[:220]}")
    sys.exit()
print("kind\tcpu\tn\tmedian_s\tp95_s\tmax_s\tmax_at")
for (k, c), v in sorted(rows.items()):
    s = sorted(x[0] for x in v)
    top = max(v)
    print(f"{k}\t{c}\t{len(s)}\t{statistics.median(s):.1f}\t{s[int(0.95 * (len(s) - 1))]:.1f}\t{s[-1]:.1f}\t{top[1]}/{top[2]}")
