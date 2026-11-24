#!/usr/bin/env python3
"""AET-WO-3309 §1.1: the placement tool's JSON lines (spike/3309/tool) as tables.

Usage: table.py RUNS PLACE_JSONL OUT_MD
For each run (campaign.out names it): main's attribution, and under each candidate rule and carrier
whether any kind 13 moves, the attribution, and the unplaced lines; then (1) counts, (2) BMP581
surplus, (4) stitches, and each device where the count rule decides (a moved step).
"""
import json
import pathlib
import sys

runs, place, out = map(pathlib.Path, sys.argv[1:4])
by_run = {}
for line in place.read_text().splitlines():
    j = json.loads(line)
    by_run[str(pathlib.Path(j["run"]).resolve())] = j


def att(a):
    if not isinstance(a, dict):
        return str(a)
    s = f"{a['rule']} [{a['evidence']}]"
    if a["masked"]:
        s += f" masked [{a['masked']}]"
    return s


rows = []
for g in sorted((runs / "camp").iterdir()):
    co = g / "campaign.out"
    if not co.exists():
        continue
    for line in co.read_text().splitlines():
        try:
            j = json.loads(line)
        except Exception:
            continue
        if "run" not in j:
            continue
        run = str((runs / pathlib.Path(j["run"]).relative_to("/cache/" + runs.name)).resolve())
        p = by_run.get(run)
        rows.append((g.name, j["scenario"], p))

V = ["R1A", "R1B", "R2A", "R2B", "R2B-line", "R2B-dev", "R2B-min", "R3B", "R3B-min"]
with open(out, "w") as f:
    f.write("| group | scenario | main | " + " | ".join(V) + " |\n|---|---|---|" + "---|" * len(V) + "\n")
    for g, s, p in rows:
        if p is None:
            f.write(f"| {g} | {s} | (no archive) |" + " |" * len(V) + "\n")
            continue
        if "refused" in p:
            f.write(f"| {g} | {s} | refused |" + " |" * len(V) + "\n")
            continue
        base = att(p["attribute"])
        cells = []
        for v in V:
            x = p["variants"][v]
            if not x["changed"]:
                cells.append("=")
            else:
                a = att(x["attribute"])
                cells.append(("**" + a + "**") if a != base else a + " (kind 13 moved)")
        f.write(f"| {g} | {s} | {base} | " + " | ".join(cells) + " |\n")
    f.write("\n## Devices whose placement differs from the count rule's, or with (1), (2), (4) findings\n\n")
    for g, s, p in rows:
        if not p or "refused" in p:
            continue
        for d in p["devices"]:
            if d.get("unread"):
                f.write(f"- {g}/{s} {d['device']}: unread\n")
                continue
            notes = []
            if not d["count_rederived"]:
                notes.append("COUNT RULE NOT RE-DERIVED")
            if d["errors"]:
                notes.append(f"{d['errors']} [ERROR] lines")
            if d["counts"]:
                notes.append(f"(1) {d['counts']}")
            if d["stitch"]:
                notes.append(f"(4) {d['stitch']}")
            sp = d["surplus"]
            if sp and (sp["extra"] or sp["held"] != sp["of"] or sp["none_lines"]):
                notes.append(f"(2) {sp}")
            for v in V:
                m = d["placed"][v]
                if m["moved"] or m["unplaced"]:
                    notes.append(f"{v}: moved {json.dumps(m['moved'])} unplaced {m['unplaced']}")
            if notes:
                f.write(f"- **{g}/{s} {d['device']}** ({d['form']}, {d['steps']} steps, {d['lines']} lines, whole log = script: {d['whole_equals_script']}):\n")
                for n in notes:
                    f.write(f"  - {n}\n")
    f.write("\n## Each run's whole-log class per device\n\n")
    for g, s, p in rows:
        if not p or "refused" in p:
            continue
        cls = " ".join(
            f"{d['device']}:{'unread' if d.get('unread') else ('script' if d['whole_equals_script'] else str(d['lines']) + ' lines')}"
            for d in p["devices"]
        )
        f.write(f"- {g}/{s}: {cls}; verdict pass={p['verdict']['pass'] if p['verdict'] else None}\n")
print(f"{len(rows)} runs")
