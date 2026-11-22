"""AET-WO-4300h: the options (a), (b), (c) as rules (RULES.md), run on walk 1's rows.

usage: python3 -I judge.py <spike/4300h dir> <repo root> <internal|external>
       -> out/options-<branch>.tsv, out/judged-<branch>.tsv, stdout summary
`internal`: the motor pulls as the engine writes them (PR #161 Q-G);
`external`: the fork in which a motor line's pull is an external resistor, so
the pin holds no internal pull after the first GPIO init.
"""
import json
import os


def tsv(path):
    lines = [l.rstrip("\n").split("\t") for l in open(path)]
    return [dict(zip(lines[0], l)) for l in lines[1:]]


def vdd_min(spec):
    sup = spec["mcu"]["supply"]
    d = next(x for x in spec.get("voltage_domains", []) if x["id"] == sup)
    return d["nominal_millivolts"] * (1000 - min(d["tolerance_permille"], 1000)) // 1000


def limit_up(structure, pulled, vdd):
    """Table 13 p. 105 with VDD at mcu.supply's minimum; None where no row is held.
    `pulled`: the pin holds an internal pull (note 3: VDD + 0.3 V)."""
    if structure.startswith("TT"):
        lim = vdd + 300
    elif structure.startswith("FT"):
        lim = min(vdd + 3600, 5499)
    elif structure == "B":
        return 9000
    elif structure == "RST":
        return vdd + 300  # embedded pull-up, note 3
    else:
        return None
    return min(lim, vdd + 300) if pulled else lim


def limit_zero(structure):
    """Table 10 p. 103 at VDD = 0."""
    return 9000 if structure == "B" else 4000


def main(here, root, branch):
    boards = tsv(os.path.join(here, "out/boards.tsv"))
    alts = tsv(os.path.join(here, "out/alts.tsv"))
    moves = tsv(os.path.join(here, "out/moves.tsv"))
    drive = tsv(os.path.join(here, "sidecars/drive.tsv"))
    pa8 = next(a["cand_structure"] for a in alts if a["candidate"] == "PA8")
    scenarios = ["none"] + sorted({d["scenario"] for d in drive})
    judged = [["option", "scenario", "board", "end", "net", "pin", "structure", "verdict", "detail"]]
    tally = {}

    def note(opt, sc, b, r, verdict, detail, pin=None, structure=None):
        judged.append([opt, sc, b, r["end"], r["net"], pin or r["pin"], structure or r["structure"], verdict, detail])
        t = tally.setdefault((opt, sc), {"refused": set(), "moved": set(), "kinds": {}, "named": 0})
        if verdict.startswith("refused"):
            t["refused"].add(b)
            k = verdict.split(" ", 1)[1]
            t["kinds"][k] = t["kinds"].get(k, 0) + 1
        elif verdict.startswith("moved"):
            t["moved"].add(b)
        elif verdict.startswith("named"):
            t["named"] += 1

    def first_ft(board, net):
        for a in alts:
            if a["board"] == board and a["net"] == net and a["verdict"].startswith("FREE"):
                return a["candidate"], a["cand_structure"]
        return None, None

    def move_result(board, cand):
        for m in moves:
            if m["board"] == board and f"on {cand}" in m["variant"]:
                return m
        return None

    def option_a(b, rows, sc="-"):
        """Returns the rows with each moved pin replaced, for (c)."""
        out = []
        for r in rows:
            if r["class"] not in ("connector", "test_point"):
                note("a", sc, b, r, "named: not a connector or test point (a sees none)", r["class"])
                out.append(r)
                continue
            if r["pin"] == "unsolved":
                note("a", sc, b, r, "named: unsolved", "validate or solve refuses this board")
                out.append(r)
                continue
            if not r["structure"].startswith("TT"):
                out.append(r)
                continue
            if r["hand"].startswith("hand"):
                note("a", sc, b, r, "refused OffBoardPinNotFt", f"{r['hand']}: no field states why")
                out.append(r)
                continue
            cand, cst = first_ft(b, r["net"])
            m = move_result(b, cand) if cand else None
            if not cand or not m or not m["solve"].startswith("0"):
                note("a", sc, b, r, "refused OffBoardNoFtPin", "no FT pin solves")
                out.append(r)
                continue
            cost = "" if m["validate"] == "0" else f"; validate: {m['validate']} (an acknowledgement names the old pin's fields)"
            note("a", sc, b, r, f"moved to {cand}", f"{cst}; {m['board_drawn']}{cost}", cand, cst)
            out.append(dict(r, pin=cand, structure=cst))
        return out

    def option_b(opt, b, rows, spec, sc, required):
        vdd = vdd_min(spec) if spec else None
        bound = {(d["board"], d["end"]): d for d in drive if d["scenario"] == sc}
        for r in rows:
            if r["class"] == "test_point":
                note(opt, sc, b, r, "named: a test point has no connectors[] pin to bound", "-")
                continue
            if r["class"] != "connector":
                note(opt, sc, b, r, "named: not a connectors[] pin", r["class"])
                continue
            if r["pin"] == "unsolved" or vdd is None:  # adversarial F10: named before any bound
                note(opt, sc, b, r, "named: unsolved", "-")
                continue
            d = bound.get((b, r["end"]))
            if not d:
                if required:
                    note(opt, sc, b, r, "refused OffBoardDriveUndeclared", "no drive bound")
                else:
                    note(opt, sc, b, r, "named: no drive bound", "-")
                continue
            if r["pin"] == "unsolved" or vdd is None:
                note(opt, sc, b, r, "named: unsolved", "-")
                continue
            if d["kind"] == "tracks":
                note(opt, sc, b, r, "pass", "tracks mcu.supply: same-rail, as A1's exemption; 0 V at VDD = 0")
                continue
            v = int(d["drive_max_millivolts"])
            fw = r["fw_pull"] != "none" and not (branch == "external" and r["signal"].startswith("TIM"))
            pulled = r["reset_pull"] != "none" or fw
            up, zero = limit_up(r["structure"], pulled, vdd), limit_zero(r["structure"])
            why = f"{v} mV ({d['kind']})"
            if up is None:
                note(opt, sc, b, r, "named: no held structure", why)
            elif v > up:
                note(opt, sc, b, r, "refused OffBoardDriveAboveTolerance",
                     f"{why} > {up} mV, {r['structure']}{' with an internal pull (note 3)' if pulled else ''} at VDD {vdd} mV (Table 13)")
            if v > zero:
                note(opt, sc, b, r, "refused OffBoardDriveAboveLimitUnpowered", f"{why} > {zero} mV at VDD = 0 (Table 10)")
            if up is not None and v <= up and v <= zero:
                note(opt, sc, b, r, "pass", f"{why} <= {up} mV up, {zero} mV at 0")

    for bd in boards:
        b = bd["name"]
        rows = tsv(os.path.join(here, "out/facing", b + ".tsv"))
        rows = [r for r in rows if r["class"] != "connector_other"]
        spec = json.load(open(os.path.join(root, bd["file"])))
        try:
            vdd_min(spec)
        except StopIteration:
            spec = None
        moved = option_a(b, rows)
        for sc in scenarios:
            option_b("b-required", b, rows, spec, sc, True)
            option_b("b-declared", b, rows, spec, sc, False)
            option_b("c-declared", b, moved, spec, sc, False)
            option_b("c-required", b, moved, spec, sc, True)
        if b == "sentinel-03":  # the ruling's build: MOTOR1 on PA8
            after = [dict(r, pin="PA8", structure=pa8, hand="hand PA8") if r["net"] == "MOTOR1" else r
                     for r in rows]
            for sc in scenarios:
                option_b("c-declared(MOTOR1 on PA8)", b, after, spec, sc, False)
    # (c)'s refusals include (a)'s
    for (opt, sc), t in list(tally.items()):
        if opt.startswith("c-") and "PA8" not in opt:
            a = tally.get(("a", "-"))
            if a:
                t["refused"] |= a["refused"]
                t["moved"] |= a["moved"]
                for k, v in a["kinds"].items():
                    t["kinds"][k] = t["kinds"].get(k, 0) + v
    with open(os.path.join(here, f"out/judged-{branch}.tsv"), "w") as f:
        for r in judged:
            f.write("\t".join(r) + "\n")
    with open(os.path.join(here, f"out/options-{branch}.tsv"), "w") as f:
        f.write("option\tscenario\tboards_refused\tboards_moved\trows_refused_by_kind\trows_named\trefused\n")
        for (opt, sc), t in sorted(tally.items()):
            line = "\t".join([opt, sc, str(len(t["refused"])), str(len(t["moved"])),
                              ",".join(f"{k} {v}" for k, v in sorted(t["kinds"].items())) or "-",
                              str(t["named"]), ",".join(sorted(t["refused"])) if len(t["refused"]) < 6 else "…"])
            f.write(line + "\n")
            print(line)


if __name__ == "__main__":
    import sys
    main(sys.argv[1], sys.argv[2], sys.argv[3])
