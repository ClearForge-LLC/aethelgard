"""Enumerate integer PLL1 settings reaching a target pll1_p_ck exactly, against limits passed in.
Scratch prototype of the independent rule's search (AET-WO-4240 §1.3/§1.4). Exact rationals."""
import sys, json
from fractions import Fraction as F

def enum(hse, target, L):
    out = []
    for m in range(L["divm"][0], L["divm"][1] + 1):
        ref = hse / m
        rge = next((i for i, (lo, hi) in enumerate(L["rge"]) if lo <= ref <= hi), None)
        if rge is None:
            continue
        for p in range(L["divp"][0], L["divp"][1] + 1):
            if L["p_even"] and p != 1 and p % 2:
                continue
            if p == 1 and not L["p_one"]:
                continue
            n = target * p / ref
            if n.denominator != 1:
                continue
            n = int(n)
            if not (L["divn"][0] <= n <= L["divn"][1]):
                continue
            vco = ref * n
            sels = [name for name, (lo, hi, rlo, rhi) in L["vco"].items() if lo <= vco <= hi and rlo <= ref <= rhi]
            if sels:
                out.append(dict(m=m, n=n, p=p, ref=float(ref), vco=float(vco), rge=rge, vcosel=sels))
    return out

if __name__ == "__main__":
    L = json.load(open(sys.argv[1]))
    for k in ("rge",):
        L[k] = [(F(a), F(b)) for a, b in L[k]]
    L["vco"] = {k: tuple(F(x) for x in v) for k, v in L["vco"].items()}
    for hse_s in sys.argv[3].split(","):
        hse = F(hse_s)
        for t_s in sys.argv[2].split(","):
            r = enum(hse, F(t_s), L)
            print(f"hse {float(hse):>12.3f} -> {int(F(t_s)):>10}: {len(r)} settings" + (f"; first {r[0]}" if r else ""))
