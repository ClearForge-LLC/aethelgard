"""For each timestamp, fetch trixie, trixie-updates, trixie-security amd64 Packages
from snapshot.debian.org, and report which packages of a remainder list would not
resolve to their published version (apt candidate = highest version across suites)."""
import lzma, os, subprocess, sys, urllib.request

def vcmp(a, b):
    r = subprocess.run(["dpkg", "--compare-versions", a, "gt", b])
    return r.returncode == 0

def index(ts, cache):
    out = {}
    for arch, suite in (("debian", "trixie"), ("debian", "trixie-updates"), ("debian-security", "trixie-security")):
        p = os.path.join(cache, f"{ts}-{suite}.xz")
        if not os.path.exists(p):
            url = f"http://snapshot.debian.org/archive/{arch}/{ts}/dists/{suite}/main/binary-amd64/Packages.xz"
            with urllib.request.urlopen(url, timeout=300) as r, open(p + ".tmp", "wb") as f:
                f.write(r.read())
            os.rename(p + ".tmp", p)
        name = None
        for line in lzma.open(p, "rt", encoding="utf-8", errors="replace"):
            if line.startswith("Package: "):
                name = line[9:].strip()
            elif line.startswith("Version: ") and name:
                v = line[9:].strip()
                out.setdefault(name, {})[suite] = v
    return out

def main():
    cache, rem = sys.argv[1], sys.argv[2]
    want = {}
    for l in open(rem):
        n, v = l.rstrip("\n").split("\t")[:2]
        want[n.split(":")[0]] = v
    for ts in sys.argv[3:]:
        idx = index(ts, cache)
        bad = []
        for n, v in sorted(want.items()):
            vs = idx.get(n, {})
            cand = None
            for s, x in vs.items():
                if cand is None or vcmp(x, cand):
                    cand = x
            if cand != v:
                bad.append(f"{n}={v}->{cand} {vs if v in vs.values() else ''}")
        print(ts, "OK" if not bad else f"{len(bad)} differ: " + "; ".join(bad))

main()
