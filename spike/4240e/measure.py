#!/usr/bin/env python3
"""AET-WO-4240e §1.1: every committed specification's generated clock
sequence, its kernel-clock selector writes against the PLL enables and the
peripheral starts, and the p371 outcome per kernel-clocked peripheral.

Usage: python3 -I measure.py <aethelgard binary> <repo root> <scratch dir>

For each git-tracked .json with a top-level "schema_version" that
`aethelgard validate` accepts, it runs `aethelgard generate` into
<scratch>/gen/<slug>/ and reads the generated firmware's main.rs (the call
order) and the board crate's board.rs (the words each call writes). It
never reads the generator's source: what it judges is what was emitted.

The rule applied is RM0433 Rev 9 §8.5.10 pp. 371-372 and the CCIPR
footnotes (pp. 412, 413, 416, 418): a selector write moves the mux's input
only if the current input and the new one both run; otherwise no clock
reaches the peripheral. Selector reset inputs are §8.7.19-§8.7.22,
pp. 412-420; the ADC's clock is adc_sclk when ADCx_CCR.CKMODE != 00
(§25.4.3, p. 920).
"""
import json
import re
import subprocess
import sys
from pathlib import Path

# Peripheral -> (register, field, lsb, width, reset input, {code: input}),
# RM0433 Rev 9 pp. 412-420 (held text, ~/.cache/aet-held/rm0433-rev9.txt).
SPI123 = ("D2CCIP1R", "SPI123SEL", 12, 3, "pll1_q_ck",
          {0: "pll1_q_ck", 1: "pll2_p_ck", 2: "pll3_p_ck", 3: "I2S_CKIN", 4: "per_ck"})
SPI45 = ("D2CCIP1R", "SPI45SEL", 16, 3, "rcc_pclk2",
         {0: "rcc_pclk2", 1: "pll2_q_ck", 2: "pll3_q_ck", 3: "hsi_ker_ck", 4: "csi_ker_ck", 5: "hse_ck"})
SPI6 = ("D3CCIPR", "SPI6SEL", 28, 3, "rcc_pclk4",
        {0: "rcc_pclk4", 1: "pll2_q_ck", 2: "pll3_q_ck", 3: "hsi_ker_ck", 4: "csi_ker_ck", 5: "hse_ck"})
I2C123 = ("D2CCIP2R", "I2C123SEL", 12, 2, "rcc_pclk1",
          {0: "rcc_pclk1", 1: "pll3_r_ck", 2: "hsi_ker_ck", 3: "csi_ker_ck"})
I2C4 = ("D3CCIPR", "I2C4SEL", 8, 2, "rcc_pclk4",
        {0: "rcc_pclk4", 1: "pll3_r_ck", 2: "hsi_ker_ck", 3: "csi_ker_ck"})
U16 = ("D2CCIP2R", "USART16SEL", 3, 3, "rcc_pclk2",
       {0: "rcc_pclk2", 1: "pll2_q_ck", 2: "pll3_q_ck", 3: "hsi_ker_ck", 4: "csi_ker_ck", 5: "lse_ck"})
U2345 = ("D2CCIP2R", "USART234578SEL", 0, 3, "rcc_pclk1",
         {0: "rcc_pclk1", 1: "pll2_q_ck", 2: "pll3_q_ck", 3: "hsi_ker_ck", 4: "csi_ker_ck", 5: "lse_ck"})
ADC = ("D3CCIPR", "ADCSEL", 16, 2, "pll2_p_ck", {0: "pll2_p_ck", 1: "pll3_r_ck", 2: "per_ck"})
SELECTOR = {
    "spi1": SPI123, "spi2": SPI123, "spi3": SPI123, "spi4": SPI45, "spi5": SPI45, "spi6": SPI6,
    "i2c1": I2C123, "i2c2": I2C123, "i2c3": I2C123, "i2c4": I2C4,
    "usart1": U16, "usart6": U16,
    "usart2": U2345, "usart3": U2345, "uart4": U2345, "uart5": U2345, "uart7": U2345, "uart8": U2345,
    "adc1": ADC, "adc2": ADC, "adc3": ADC,
}
OFFSET = {0x04C: "D1CCIPR", 0x050: "D2CCIP1R", 0x054: "D2CCIP2R", 0x058: "D3CCIPR"}
# Runs from reset with nothing written: HSION resets 1 (p. 383), SW = HSI,
# every bus prescaler /1, CKPERSEL 00 = hsi_ker_ck (p. 412).
ALWAYS = {"hsi_ck", "hsi_ker_ck", "per_ck", "rcc_pclk1", "rcc_pclk2", "rcc_pclk4", "adc_sclk"}


def run(cmd, **kw):
    return subprocess.run(cmd, capture_output=True, text=True, **kw)


def specs(root):
    out = []
    for rel in run(["git", "ls-files", "*.json"], cwd=root).stdout.split():
        try:
            doc = json.loads((root / rel).read_text())
        except (ValueError, UnicodeDecodeError):
            continue
        if isinstance(doc, dict) and "schema_version" in doc:
            out.append(rel)
    return out


def measure_one(ae, root, rel, scratch):
    slug = rel.replace("/", "__").removesuffix(".json")
    row = {"spec": rel}
    v = run([ae, "validate"], stdin=open(root / rel))
    if v.returncode != 0:
        row["skipped"] = "validate exit %d: %s" % (v.returncode, v.stdout.strip()[:160])
        return row
    out = scratch / "gen" / slug
    if not out.exists():
        g = run([ae, "generate", "--spec", str(root / rel), "--out", str(out)])
        if g.returncode != 0:
            row["skipped"] = "generate exit %d: %s" % (g.returncode, g.stdout.strip()[:200])
            return row
    main = (out / "board/firmware/src/main.rs").read_text().splitlines()
    board = (out / "board/src/board.rs").read_text()
    row["hse"] = any("hse.start(" in l for l in main)
    # The selector words select_spi_kernels writes (the emitted body).
    writes = []
    # Every RCC write in select_spi_kernels, in order (AET-WO-4240e: the
    # switch's PLL words and RCC_CR come around the selects).
    body_writes = []
    m = re.search(r"fn select_spi_kernels.*?\n(.*?)\n\s*Ok\(SpiKernels \{|fn select_spi_kernels.*?\n(.*?)\n\s*SpiKernels \{", board, re.S)
    if m:
        # A whole-word write, or RCC_CR's PLL1ON set or cleared by a
        # read-modify-write (AET-WO-4240e), read as the word it leaves:
        # bit 24 set or clear.
        pat = (r"r\.write\(TOKEN, Block::Rcc, (0x[0-9a-f]+), (0x[0-9a-f]+)\)"
               r"|r\.modify\(TOKEN, Block::Rcc, (0x[0-9a-f]+), \|v\| v (\| |& !)(0x[0-9a-f]+)\)")
        for w in re.finditer(pat, m.group(1) or m.group(2)):
            if w.group(1):
                body_writes.append((int(w.group(1), 16), int(w.group(2), 16)))
            else:
                bit = int(w.group(5), 16)
                body_writes.append((int(w.group(3), 16), bit if w.group(4) == "| " else 0))
    writes = [(OFFSET[o], v) for o, v in body_writes if o in OFFSET]
    row["select_writes"] = ["%s=%#010x" % w for w in writes]
    row["switch_writes"] = ["%#05x=%#010x" % w for w in body_writes if w[0] not in OFFSET]
    # Every access to a CCIPR register anywhere in the board crate, by kind
    # (read, write, modify) and register: P1.1 shown, not inferred.
    acc = {}
    for op, off in re.findall(r"r\.(read|write|modify)\(TOKEN, Block::Rcc, (0x[0-9a-f]+)", board):
        reg = OFFSET.get(int(off, 16))
        if reg:
            acc.setdefault(reg, {}).setdefault(op, 0)
            acc[reg][op] += 1
    row["ccipr_accesses"] = acc
    # Every clock enable main.rs calls, so a kernel-clocked peripheral the
    # table lacks is listed, not dropped (GPIO ports and timers take no
    # kernel mux: RM0433 Rev 9 §8.5.8, Figure 58/Table 60, pp. 354-357).
    enables = re.findall(r"rcc\.enable_([a-z0-9]+)\(", "\n".join(main))
    row["unclassified_enables"] = [e for e in enables
                                   if e not in SELECTOR and not re.fullmatch(r"gpio[a-k]|tim[0-9]+", e)]
    # PLLCFGR as written by Hse::start (DIVxEN), when there is one.
    pll_on = set()
    if row["hse"]:
        pc = re.search(r"PLLCFGR[^\n]*?(0x[0-9a-f]{8})|(0x[0-9a-f]{8})[^\n]*PLLCFGR", board)
        word = int(pc.group(1) or pc.group(2), 16) if pc else None
        row["pllcfgr"] = "%#010x" % word if word is not None else "not found"
        pll_on = {"hse_ck"}
        if word is not None:
            for bit, out_ in ((16, "pll1_p_ck"), (17, "pll1_q_ck"), (18, "pll1_r_ck")):
                if word >> bit & 1:
                    pll_on.add(out_)
    # The call order: index of each step in main.rs.
    order = []
    for i, l in enumerate(main, 1):
        s = l.strip()
        if "rcc.reset_clock(" in s:
            order.append((i, "reset_clock", None))
        elif "hse.start(" in s:
            order.append((i, "hse_start", None))
        elif "select_spi_kernels(" in s:
            order.append((i, "select", None))
        else:
            e = re.search(r"rcc\.enable_([a-z0-9]+)\(", s)
            if e and e.group(1) in SELECTOR:
                order.append((i, "enable", e.group(1)))
            # A peripheral's start: SPI/I2C `init`, USART `configure`, ADC
            # `bring_up` (the generated API's names; each precedes the first
            # SPE/PE/UE/ADEN the peripheral sets).
            st = re.search(r"\b([a-z]+[0-9])\.(init|configure|bring_up)\b", s)
            if st and st.group(1) in SELECTOR:
                order.append((i, "start", st.group(1)))
    row["order"] = ["%d:%s%s" % (i, k, "" if p is None else "(" + p + ")") for i, k, p in order]
    ckmode = re.search(r"CKMODE (\d\d)", board)
    row["adc_ckmode"] = ckmode.group(1) if ckmode else None
    # Simulate the muxes along the order.
    running = set(ALWAYS)
    mux_input = {}  # (register, field) -> current input
    selected = {}   # (register, field) -> selector value's input
    for sel in SELECTOR.values():
        mux_input[sel[:2]] = sel[4]
        selected[sel[:2]] = sel[4]
    events = []
    per = {}
    for i, kind, p in order:
        if kind == "hse_start":
            running |= pll_on
        elif kind == "select":
            pllcfgr = 0
            for off, word in body_writes:
                if off == 0x02C:
                    pllcfgr = word
                    continue
                if off == 0x000:
                    # RCC_CR: PLL1ON (bit 24) on runs PLL1's enabled outputs
                    # (the bounded PLL1RDY poll precedes any select); off
                    # stops them. PLLSRC/DIVM/VCO validity is the IR rule's.
                    outs = {"pll1_p_ck": 16, "pll1_q_ck": 17, "pll1_r_ck": 18}
                    if word >> 24 & 1:
                        running |= {o for o, b in outs.items() if pllcfgr >> b & 1}
                    else:
                        running -= set(outs)
                    continue
                if off not in OFFSET:
                    continue
                reg = OFFSET[off]
                for sel in list({s[:2]: s for s in SELECTOR.values() if s[0] == reg}.values()):
                    code = word >> sel[2] & ((1 << sel[3]) - 1)
                    new = sel[5].get(code, "disabled")
                    key = sel[:2]
                    if new == mux_input[key]:
                        selected[key] = new
                        continue
                    ok = mux_input[key] in running and new in running
                    events.append("L%d %s %s->%s: %s" % (
                        i, sel[1], mux_input[key], new,
                        "switched" if ok else "NOT switched (%s not running)" % (
                            mux_input[key] if mux_input[key] not in running else new)))
                    selected[key] = new
                    if ok:
                        mux_input[key] = new
                    else:
                        mux_input[key] = "pending:" + mux_input[key] + "->" + new
        elif kind in ("enable", "start"):
            sel = SELECTOR[p]
            key = sel[:2]
            inp = mux_input[key]
            if p.startswith("adc") and row["adc_ckmode"] not in (None, "00"):
                clocked, why = True, "adc_sclk (CKMODE %s); ADCSEL input %s unused" % (row["adc_ckmode"], inp)
            elif inp.startswith("pending:"):
                clocked, why = False, "%s %s" % (sel[1], inp)
            else:
                clocked, why = inp in running, "%s = %s%s" % (sel[1], inp, "" if inp in running else " (not running)")
            if kind == "start" or p not in per:
                per[p] = {"at": "L%d %s" % (i, kind), "clocked": clocked, "why": why}
    row["mux_events"] = events
    row["peripherals"] = per
    row["unclocked"] = sorted(p for p, r in per.items() if not r["clocked"])
    return row


def main():
    ae, root, scratch = sys.argv[1], Path(sys.argv[2]).resolve(), Path(sys.argv[3]).resolve()
    rows = [measure_one(ae, root, rel, scratch) for rel in specs(root)]
    json.dump(rows, sys.stdout, indent=1)
    print()


if __name__ == "__main__":
    main()
