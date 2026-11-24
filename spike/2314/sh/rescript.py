"""AET-WO-2314: the CLI's monitor script from a copied Renode argv, its platform, models and ELF moved to
copies. python3 -I rescript.py ARGV CAPDIR ELF OUT"""
import sys
argv, cap, elf, out = sys.argv[1:]
a = open(argv).read().split("\n")
res = []
for c in (c.strip() for c in a[a.index("-e") + 1].split(";")):
    if c.startswith("machine LoadPlatformDescription"):
        c = f"machine LoadPlatformDescription @{cap}/platform.repl"
    elif c.startswith("include @") and "/models/" in c:
        c = f"include @{cap}/models/" + c.split("/models/", 1)[1]
    elif c.startswith("sysbus LoadELF"):
        c = f"sysbus LoadELF @{elf}"
    res.append(c)
open(out, "w").write("; ".join(res))
