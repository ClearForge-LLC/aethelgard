"""AET-WO-4300h §1.3: what moving each TT pin to an FT alternative changes, run.

Each variant is a scratch copy of a committed specification with hand pins
set (never a committed file). For the copy and its base it runs `validate`,
`solve` and `emit`, and reports what moved in the solved topology and on the
drawn board.

usage: python3 -I moves.py <aethelgard binary> <repo root> <scratch dir> -> out/moves.tsv
"""
import hashlib
import json
import os
import re
import subprocess
import sys

S3 = "crates/aethelgard-cli/tests/fixtures/sentinel-03.json"
S1 = {"sentinel-01": "examples/sentinel-01.json",
      "sentinel-01-bench": "crates/aethelgard-cli/tests/fixtures/sentinel-01-bench.json",
      "sentinel-01-ext-bmp581": "examples/sentinel-01-ext-bmp581.json"}
BENCH = {"sentinel-02-bench": "examples/sentinel-02-bench.json"}


def motors(pins):
    def f(s):
        for m, p in zip(s["motor_outputs"], pins):
            m["pin"] = p
    return f


def spi(sck, mosi, bus=0):
    def f(s):
        s["spi_buses"][bus]["sck"]["pin"] = sck
        s["spi_buses"][bus]["mosi"]["pin"] = mosi
    return f


VARIANTS = [
    ("sentinel-03", S3, "MOTOR1 pin null (the solver's own choice)", motors([None, "PE11", "PE13", "PE14"])),
    ("sentinel-03", S3, "MOTOR1 on PA8 (TIM1_CH1, FT_fha; 3400 kept it free for MCO1)", motors(["PA8", "PE11", "PE13", "PE14"])),
    ("sentinel-03", S3, "MOTOR1 on PD12 (TIM4_CH1, FT_fh; motors split over TIM1 and TIM4)", motors(["PD12", "PE11", "PE13", "PE14"])),
    ("sentinel-03", S3, "all four on TIM4, PD12-PD15 (every pin FT)", motors(["PD12", "PD13", "PD14", "PD15"])),
    ("sentinel-03", S3, "all four motors null (the solver's free choice)", motors([None, None, None, None])),
] + [
    (n, f, "SPI_SCK on PB3, SPI_MOSI on PB5 (both FT)", spi("PB3", "PB5")) for n, f in S1.items()
] + [
    (n, f, "SPI_SCK on PG11, SPI_MOSI on PD7 (FT_h)", spi("PG11", "PD7")) for n, f in S1.items()
] + [
    (n, f, "IMU_SCK on PB3 (FT); MOSI stays PB5", spi("PB3", "PB5")) for n, f in BENCH.items()
]


def run(ae, args, cwd=None):
    p = subprocess.run([ae] + args, capture_output=True, text=True, cwd=cwd)
    return p.returncode, p.stdout


def kind(out):
    try:
        return json.loads(out).get("error", {}).get("kind", "?")
    except ValueError:
        return "?"


def board_stats(pcb):
    t = open(pcb).read()
    return {"segments": len(re.findall(r"\n\t\(segment", t)), "vias": len(re.findall(r"\n\t\(via", t)),
            "sha": hashlib.sha256(t.encode()).hexdigest()[:12]}


def main(ae, root, scratch):
    os.makedirs(scratch, exist_ok=True)
    out = [["board", "variant", "validate", "solve", "emit", "moved_in_solve", "board_drawn"]]
    base = {}
    for board, f, label, edit in VARIANTS:
        if board not in base:
            rc, sj = run(ae, ["solve", "--file", os.path.join(root, f)])
            d = os.path.join(scratch, "base-" + board)
            subprocess.run(["rm", "-rf", d])
            erc, _ = run(ae, ["emit", "--file", os.path.join(root, f), "--out", d])
            pcb = [x for x in os.listdir(d) if x.endswith(".kicad_pcb")] if erc == 0 else []
            base[board] = (json.loads(sj), board_stats(os.path.join(d, pcb[0])) if pcb else None)
        s = json.load(open(os.path.join(root, f)))
        edit(s)
        tag = re.sub(r"[^a-z0-9]+", "-", label.lower())[:40].strip("-")
        vf = os.path.join(scratch, f"{board}--{tag}.json")
        json.dump(s, open(vf, "w"), indent=2)
        vrc, vout = run(ae, ["validate", "--file", vf])
        src, sout = run(ae, ["solve", "--file", vf])
        moved, drawn, erc = "-", "-", "skip"
        if src == 0:
            t, b0 = json.loads(sout), base[board][0]
            ch = []
            for k in sorted(set(t["assignments"]) | set(b0["assignments"])):
                a, z = b0["assignments"].get(k), t["assignments"].get(k)
                if a != z:
                    ch.append(f"{k}: {a['pin'] if a else '-'} {a['signal'] if a else ''} -> {z['pin'] if z else '-'} {z['signal'] if z else ''}")
            for k in sorted(set(t) - {"assignments", "specification_sha256"}):
                if t.get(k) != b0.get(k):
                    ch.append(f"{k} differs")
            moved = "; ".join(ch) or "nothing"
            d = os.path.join(scratch, "emit-" + os.path.basename(vf)[:-5])
            subprocess.run(["rm", "-rf", d])
            erc, eout = run(ae, ["emit", "--file", vf, "--out", d])
            if erc == 0:
                pcb = [x for x in os.listdir(d) if x.endswith(".kicad_pcb")][0]
                st, b1 = board_stats(os.path.join(d, pcb)), base[board][1]
                drawn = f"segments {b1['segments']}->{st['segments']}, vias {b1['vias']}->{st['vias']}, pcb {b1['sha']}->{st['sha']}"
            else:
                drawn = kind(eout)
        out.append([board, label, str(vrc) if vrc == 0 else f"{vrc} {kind(vout)}",
                    str(src) if src == 0 else f"{src} {kind(sout)}", str(erc), moved, drawn])
    with open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "out/moves.tsv"), "w") as fh:
        for r in out:
            fh.write("\t".join(r) + "\n")


if __name__ == "__main__":
    main(*sys.argv[1:4])
