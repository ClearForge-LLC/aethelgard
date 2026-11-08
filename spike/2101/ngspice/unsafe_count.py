import re, glob, sys
tot_kw = 0; tot_in = 0; out = []
for p in sorted(glob.glob("/repo/spike/2101/ngspice/src/**/*.rs", recursive=True)):
    L = open(p).read().split("\n"); depth = 0; inside = False; start = None; kw = []; ins = set()
    i = 0
    code = [re.sub(r"//.*$", "", l) for l in L]
    for n, l in enumerate(code):
        if re.search(r"\bunsafe\b", l):
            kw.append(n)
    # region: from a line with `unsafe` to the matching close brace
    for n in kw:
        d = 0; started = False
        for m in range(n, len(code)):
            for ch in code[m]:
                if ch == "{": d += 1; started = True
                elif ch == "}": d -= 1
            ins.add(m)
            if started and d <= 0: break
            if not started and ";" in code[m]: break
    out.append("%s: lines containing `unsafe`: %d (%s); total lines inside unsafe blocks/fns/extern blocks (incl. those): %d" % (p.replace("/repo/", ""), len(kw), ",".join(str(k + 1) for k in kw), len(ins)))
    for m in sorted(ins): out.append("   %d: %s" % (m + 1, L[m].rstrip()))
    tot_kw += len(kw); tot_in += len(ins)
print("TOTAL lines containing `unsafe`: %d ; TOTAL lines inside unsafe regions: %d" % (tot_kw, tot_in)); print("\n".join(out))
