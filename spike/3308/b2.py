#!/usr/bin/env python3
"""B2's register table for the ordered ELF: the twin's recorded rows (simulate verdict)
classed by the ordered verify ledger, then the ledger's read-back rows the twin does not
store. Usage: b2.py <simulate-verdict.json> <ordered-verdict.json>"""
import json, sys

sim = json.load(open(sys.argv[1]))
led = json.load(open(sys.argv[2]))["ledger"]["rows"]

BASE = {"GPIOA": 0x58020000, "GPIOB": 0x58020400, "I2C1": 0x40005400, "I2C2": 0x40005800,
        "SPI1": 0x40013000, "USART1": 0x40011000, "RCC": 0x58024400, "SCB": 0xE000E000,
        "FLASH": 0x52002000, "PWR": 0x58024800}
OFF = {"MODER": 0x00, "OTYPER": 0x04, "OSPEEDR": 0x08, "PUPDR": 0x0C, "AFRL": 0x20, "AFRH": 0x24,
       "I2C.CR1": 0x00, "I2C.CR2": 0x04, "I2C.TIMINGR": 0x10,
       "SPI.CR1": 0x00, "SPI.CFG1": 0x08, "SPI.CFG2": 0x0C, "SPI.SR": 0x14,
       "USART.CR1": 0x00, "USART.CR2": 0x04, "USART.CR3": 0x08, "USART.BRR": 0x0C,
       "SCB.CPACR": 0xD88, "FLASH.OPTSR_CUR": 0x1C, "FLASH.ACR": 0x00,
       "PWR.CR3": 0x0C, "PWR.CSR1": 0x04, "PWR.D3CR": 0x18,
       "RCC.CR": 0x00, "RCC.CFGR": 0x10, "RCC.D1CFGR": 0x18, "RCC.D2CFGR": 0x1C,
       "RCC.D1CCIPR": 0x4C, "RCC.D2CCIP1R": 0x50, "RCC.D2CCIP2R": 0x54}
# The word each I2C's init writes (main.rs: I2C1 Fast400k, I2C2 Standard100k; board.rs L175, L199).
# The ledger's field values for I2C1.TIMINGR carry the Standard100k word: not used.
TIMINGR = {"I2C1.TIMINGR": 0x10C7123C, "I2C2.TIMINGR": 0x50D32A40}
LABEL = {"observed": "twin", "independent-rule": "rule", "same-source": "generator",
         "ignored-by-model": "not predicted (no model reacts)", "silicon-only": "not predicted (silicon-only)"}


def addr(reg):
    p, r = reg.split(".")
    if p.startswith("GPIO"):
        return BASE[p] + OFF[r]
    if p == "RCC":
        return BASE[p] + OFF[reg]
    kind = {"I2C1": "I2C", "I2C2": "I2C", "SPI1": "SPI", "USART1": "USART"}.get(p, p)
    return BASE[p] + OFF[f"{kind}.{r}"]


def fields(reg, mask):
    out = []
    written = {r["field"] for r in led if r["register"] == reg and r["access"] == "written"}
    for r in led:
        if r["access"] != "written" and r["field"] in written:
            continue
        if r["register"] != reg or r.get("value") is None:
            continue
        m = ((1 << r["width"]) - 1) << r["pos"]
        if mask is None or m & mask:
            out.append(r)
    return out


def classes(rows):
    seen = []
    for r in rows:
        c = LABEL[r["class"]]
        if c not in seen:
            seen.append(c)
    return "; ".join(seen)


print("| Register | Address | Mask | Expected | Source |")
print("|---|---|---|---|---|")
twin_regs = set()
for r in sim["boot"]["registers"]:
    reg, mask = r["name"], int(r["mask"], 16)
    twin_regs.add(reg)
    f = fields(reg, mask)
    src = classes(f) if f else "twin (Renode's reset value: the firmware never writes it)"
    print(f"| {reg} | `{r['address']}` | `{r['mask']}` | `{r['expected']}` | {src} |")

# Read-back rows the twin does not store: every ledger register with fixed written values,
# not already a twin row, minus write-only and self-clearing registers.
SKIP = {"ICR", "BSRR", "TXDR", "TDR", "IFCR", "SR"}
done = set()
for r in led:
    reg = r["register"]
    if reg in twin_regs or reg in done or reg.split(".")[1] in SKIP or reg == "SPI1.CR2":
        continue
    if reg == "FLASH.OPTSR_CUR":
        continue  # B3's row
    done.add(reg)
    fs = [x for x in fields(reg, None) if x["field"] not in ("AUTOEND", "START")]
    if not fs:
        continue
    mask = 0
    val = 0
    for x in fs:
        mask |= ((1 << x["width"]) - 1) << x["pos"]
        val |= x["value"] << x["pos"]
    src = classes(fs)
    if reg in TIMINGR:
        val, src = TIMINGR[reg], "generator, read back (the rule holds the fields by inequalities, not the word)"
    print(f"| {reg} | `0x{addr(reg):08x}` | `0x{mask:08x}` | `0x{val:08x}` | {src} |")
