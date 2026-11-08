import hashlib, statistics, os, re

def sha(b):
    return hashlib.sha256(b).hexdigest()

def stats(ns):
    s = sorted(ns)
    n = len(s)
    p95 = s[min(n - 1, int(0.95 * n + 0.999999) - 1)]
    return dict(n=n, min=s[0], median=statistics.median(s), p95=p95, max=s[-1], total=sum(s))

def fmt_stats(label, ns):
    st = stats(ns)
    return "%s per-transient ms: min=%.3f median=%.3f p95=%.3f max=%.3f total=%.1f (n=%d)" % (
        label, st['min'] / 1e6, st['median'] / 1e6, st['p95'] / 1e6, st['max'] / 1e6, st['total'] / 1e6, st['n'])

def parse_wrdata(txt):
    t, v = [], []
    for ln in txt.splitlines():
        f = ln.split()
        if len(f) >= 2:
            t.append(float(f[0])); v.append(float(f[1]))
    return t, v

def rise(t, v, lo=0.99, hi=2.31):
    """linear-interpolated first rising crossings of lo then hi"""
    def cross(level, start):
        for k in range(start, len(v) - 1):
            if v[k] < level <= v[k + 1]:
                return k, t[k] + (level - v[k]) * (t[k + 1] - t[k]) / (v[k + 1] - v[k])
        return None, None
    k, t0 = cross(lo, 0)
    if k is None: return None
    k2, t1 = cross(hi, k)
    if k2 is None: return None
    return t1 - t0

def theory(r): return 0.8473 * r * 100e-12
