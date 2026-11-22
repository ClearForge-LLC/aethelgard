"""AET-WO-4300h §1.3: for each TT pin an off-board end reaches, every pin the
solver could have put that signal on instead, from its own pin table
(out/pintable.tsv, checked against every solved assignment by pintable.py).

usage: python3 -I alts.py <spike/4300h dir> <solve dir> <power-parts.json> <repo root> -> out/alts.tsv

A candidate is judged as the solver would see it (`solve.rs`): not reserved
(RESERVED_PINS PA13, PA14, PC14, PC15; PH0/PH1 beside an HSE crystal, PH0
beside an external clock), not held by another net of the same board, and
carrying the same signal. Motor outputs take any TIMn_CHk (TIMERS, k 1-4) on
a timer that is free or already carries the output's key; SPI and serial
signals move with their instance. A pin `3400` kept free (FEEDBACK L265:
PA8, PC9 for MCO, PA11/PA12 for USB, PD0/PD1 for FDCAN1) is marked on
`sentinel-03`, whose layout `3400` V5 set: the solver would take it, but not by
ruling. Timers whose CH1-CH4 are all on free FT pins are listed for a
whole-set motor move.
"""
import json
import os
import sys

RESERVED = {"PA13", "PA14", "PC14", "PC15"}
TIMERS = {1, 2, 3, 4, 5, 8, 12, 13, 14, 15, 16, 17}
KEPT_FREE = {"sentinel-03": {"PA8": "MCO1", "PC9": "MCO2", "PA11": "USB OTG_FS DM", "PA12": "USB OTG_FS DP",
                             "PD0": "FDCAN1 RX", "PD1": "FDCAN1 TX"}}


def timer_channel(sig):
    if not sig.startswith("TIM") or "_CH" not in sig:
        return None
    n, k = sig[3:].split("_CH", 1)
    if not (n.isdigit() and k.isdigit()) or n.startswith("0"):
        return None
    n, k = int(n), int(k)
    return (n, k) if n in TIMERS and 1 <= k <= 4 else None


RESET_PULL = {"PA13": "up", "PA14": "down", "PA15": "up", "PB4": "up"}  # RM0433 Rev 9 p. 542


def main(here, solve_dir, pp_path, root):
    files = {l.split("\t")[0]: l.split("\t")[1] for l in list(open(os.path.join(here, "out/boards.tsv")))[1:]}
    table = {}
    for line in list(open(os.path.join(here, "out/pintable.tsv")))[1:]:
        pin, _, sig, af = line.rstrip("\n").split("\t")
        table.setdefault(pin, {})[sig] = int(af)
    io = {}
    for p in json.load(open(pp_path))["parts"]:
        if p.get("kind", {}).get("mcu"):
            io = p["kind"]["mcu"]["io"]["pins"]
    st = lambda p: io.get(p, {}).get("structure", "unheld")
    out = [["board", "net", "pin", "structure", "signal", "candidate", "cand_structure", "cand_signal", "af",
            "held_by", "verdict"]]
    found = []
    for name in sorted(os.listdir(os.path.join(here, "out/facing"))):
        for line in list(open(os.path.join(here, "out/facing", name)))[1:]:
            r = line.rstrip("\n").split("\t")
            if r[2] in ("connector", "test_point", "undeclared_offboard") and r[5].startswith("TT"):
                found.append((r[0], r[3], r[4], r[5], r[6].split(" ")[0]))
    for board, net, pin, structure, signal in sorted(set(found)):
        t = json.load(open(os.path.join(solve_dir, board + ".json")))
        hse = json.load(open(os.path.join(root, files[board])))["mcu"].get("hse")
        held = {v["pin"]: k for k, v in t["assignments"].items()}
        held.update({v["pin"]: k for k, v in t.get("gpio", {}).items()})
        held.update({v["pin"]: k for k, v in t.get("adc_inputs", {}).items()})
        reserved = set(RESERVED)
        if isinstance(hse, dict) and "crystal" in hse:
            reserved |= {"PH0", "PH1"}
        if isinstance(hse, dict) and "external" in hse:
            reserved.add("PH0")
        kept = KEPT_FREE.get(board, {})
        tc = timer_channel(signal)
        motors = {k: timer_channel(v["signal"]) for k, v in t["assignments"].items() if timer_channel(v["signal"])}

        def verdict(c, csig):
            why = []
            if c == pin:
                return "the pin itself"
            if c in reserved:
                why.append("reserved")
            if c in held and held[c] != net:
                why.append(f"held by {held[c]}")
            if not st(c).startswith("FT"):
                why.append(st(c))
            if tc:
                ctc = timer_channel(csig)
                used = {v for k, v in motors.items() if k != net}
                if ctc in used:
                    why.append(f"channel TIM{ctc[0]}_CH{ctc[1]} in use")
            if c in kept:
                why.append(f"kept free by 3400 ({kept[c]})")
            if not why and c in RESET_PULL:
                return f"FREE (reset pull-{RESET_PULL[c]}: VDD + 0.3 V until written, Table 13 note 3)"
            return "; ".join(why) or "FREE"

        cands = []
        for c, sigs in sorted(table.items()):
            for csig, af in sorted(sigs.items()):
                if (tc and timer_channel(csig)) or (not tc and csig == signal):
                    cands.append((c, csig, af))
        for c, csig, af in cands:
            out.append([board, net, pin, structure, signal, c, st(c), csig, str(af),
                        held.get(c, "-"), verdict(c, csig)])
        if tc:  # whole-set moves: every timer whose CH1-CH4 can each sit on a free FT pin
            for n in sorted(TIMERS):
                chans = {}
                for c, sigs in table.items():
                    for csig in sigs:
                        ct = timer_channel(csig)
                        if ct and ct[0] == n and st(c).startswith("FT") and c not in reserved \
                                and (c not in held or held[c] in motors):
                            chans.setdefault(ct[1], []).append(c + ("(kept free)" if c in kept else ""))
                if all(k in chans for k in (1, 2, 3, 4)):
                    out.append([board, "all four motors", "-", "-", f"TIM{n}", ";".join(
                        f"CH{k}:{'/'.join(sorted(chans[k]))}" for k in (1, 2, 3, 4)), "FT", "-", "-", "-",
                        "whole-set candidate"])
    with open(os.path.join(here, "out/alts.tsv"), "w") as f:
        for r in out:
            f.write("\t".join(r) + "\n")


if __name__ == "__main__":
    main(*sys.argv[1:5])
