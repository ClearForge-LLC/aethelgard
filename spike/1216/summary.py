"""AET-WO-1216: one line per committed board from walk 1 (out/spec) and out/boards.tsv.
usage: python3 -I summary.py out/spec out/boards.tsv -> markdown table on stdout"""
import csv, os, sys
from collections import Counter
boards = {r["name"]: r for r in csv.DictReader(open(sys.argv[2]), delimiter="\t")}
print("| board | validate / solve / emit | paths | by class | stated (worst at VDD < 1 V, pin, source) |")
print("|---|---|---|---|---|")
for n in sorted(boards):
    b = boards[n]
    rows = [r for r in csv.DictReader(open(os.path.join(sys.argv[1], n + ".tsv")), delimiter="\t")
            if r["can_lead"].startswith("yes")]
    c = Counter(r["class"] for r in rows)
    st = [f"{r['worst_mv']} mV on {r['pin']} ({r['structure']}), {r['class']}, {r['rail']}" for r in rows if r["worst_mv"].isdigit()]
    gate = f"{b['validate']} / {b['solve']} / {b['emit']}" + (f" ({b['note'].strip()})" if b["note"].strip() else "")
    print(f"| `{b['file']}` | {gate} | {len(rows)} | {', '.join(f'{k} {v}' for k, v in sorted(c.items())) or '-'} | {'; '.join(st) or 'none'} |")
