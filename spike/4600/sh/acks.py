#!/usr/bin/env python3
"""AET-WO-4600 stage 2: write a specification's acknowledge_unwitnessed entries.

The rows come from the refusal `verify --fab-out` printed (UnwitnessedFieldNotAcknowledged)
and their classes from the same run's ledger (`verify`'s JSON, `ledger.rows`): the
refusal names fields only, so the ledger is where a person learns each row's class.
The REASONS below are written by hand, one per (register kind, class), in the
builder's words and not copied from the ledger's judge column: an acknowledgement
copied from the artifact it acknowledges would say nothing the artifact did not.
A (register kind, class) with no hand-written reason stops the script.

Usage: python3 -I spike/4600/sh/acks.py <spec.json> <verify.json> <verify-fab.json>
"""
import json
import re
import sys

BENCH = "Bench: read back after boot."
REASONS = {
    ("SCB.CPACR", "same-source"): "The FPU enable is written by the generated reset code and no model reacts to it. " + BENCH,
    ("RCC.APBxENR", "same-source"): "A clock gate the RCC model acts on only for a peripheral the platform registers; nothing independent judges it here. " + BENCH,
    ("GPIO.OTYPER", "ignored-by-model"): "The GPIO model does not store an output type, so the twin cannot see push-pull versus open-drain. " + BENCH,
    ("GPIO.PUPDR", "same-source"): "The pulls are stored but nothing reacts to them in the twin; no line on this board depends on an internal pull. " + BENCH,
    ("GPIO.OSPEEDR", "same-source"): "Stored with no reaction: the twin has no edge rate. check speeds judges the lines it covers. " + BENCH,
    ("GPIO.AFR", "same-source"): "The alternate function is stored but the model routes by it only for a timer channel. " + BENCH,
    ("GPIO.MODER", "same-source"): "Mode is stored; the model drives a pin whatever MODER says. " + BENCH,
    ("USART.CR1", "ignored-by-model"): "Fields the USART model does not store (word length, FIFO, wake-up). The serial judge reads them only from the access log. " + BENCH,
    ("USART.CR1", "same-source"): "Stored with no reaction claimed; only the generator's own oracle states them. " + BENCH,
    ("USART.CR2", "ignored-by-model"): "Fields the USART model does not store (inversion, swap, LIN, auto-baud). " + BENCH,
    ("USART.CR2", "same-source"): "STOP is stored with no reaction claimed. " + BENCH,
    ("TIM.CR1", "same-source"): "Stored with no reaction claimed beyond the counter running, which the twin witnesses as edges. " + BENCH,
    ("TIM.CR1", "ignored-by-model"): "Outside the bits the timer model stores (clock division). " + BENCH,
    ("TIM.CCMR", "same-source"): "The output-compare mode is stored; the twin witnesses edges, not the mode. " + BENCH,
    ("TIM.CCMR", "ignored-by-model"): "Preload and fast-enable bits are tags in the timer model. " + BENCH,
    ("TIM.CCER", "same-source"): "Channel enables and polarities stored with no reaction claimed. " + BENCH,
    ("TIM.CCER", "ignored-by-model"): "Complementary-output bits are tags in the timer model. " + BENCH,
    ("TIM.BDTR", "ignored-by-model"): "Every BDTR field is a tag, MOE included: on silicon the outputs stay off without MOE and the twin cannot see it. Bench: a scope on the outputs.",
    ("TIM.EGR", "same-source"): "The update event is written once; the model's handling is not measured. " + BENCH,
    ("RCC.D1CCIPR", "ignored-by-model"): "A kernel-clock select the RCC model ignores; every rule assumes its reset value, which the firmware keeps. " + BENCH,
    ("RCC.D2CCIP1R", "ignored-by-model"): "A kernel-clock select the RCC model ignores; every rule assumes its reset value. " + BENCH,
    ("RCC.D2CCIP2R", "ignored-by-model"): "A kernel-clock select the RCC model ignores; every rule assumes its reset value. " + BENCH,
    ("RCC.CR", "ignored-by-model"): "D2CKRDY is a tag in the model, set by hardware on silicon. " + BENCH,
    ("FLASH.OPTSR_CUR", "silicon-only"): "The BOR level is an option byte the factory step programs; the twin presents it itself. " + BENCH,
    ("I2C.CR1", "same-source"): "Stored with no reaction claimed beyond the transfers the twin witnesses. " + BENCH,
    ("I2C.CR2", "same-source"): "Transfer-control bits stated only by the generator's oracle. " + BENCH,
    ("I2C.ICR", "same-source"): "Flag clears with no reaction claimed. " + BENCH,
    ("I2C.TIMINGR", "same-source"): "The I2C timing word is stored; the twin does not time the bus. ngspice and the rise-time rule judge the electrical side. " + BENCH,
    ("SPI.CR1", "same-source"): "Stored with no reaction claimed. " + BENCH,
    ("SPI.CR2", "ignored-by-model"): "TSER is a tag in the SPI model. " + BENCH,
    ("SPI.IFCR", "ignored-by-model"): "Flag clears the SPI model ignores. " + BENCH,
    ("SPI.SR", "ignored-by-model"): "MODF is not modelled (the first scar, northstar s2). " + BENCH,
    ("SPI.CFG1", "same-source"): "Stored with no reaction claimed. " + BENCH,
    ("SPI.CFG2", "same-source"): "Stored with no reaction claimed; SSM and SSI are judged by 4230's rule. " + BENCH,
}


def kind(register):
    periph, reg = register.split(".", 1)
    p = re.sub(r"\d+$", "", periph)
    if periph.startswith("GPIO"):
        p = "GPIO"
        reg = re.sub(r"^(AFR)[LH]$", r"\1", reg)
    if p in ("TIM",):
        reg = re.sub(r"^(CCMR)\d$", r"\1", reg)
    if periph == "RCC" and re.match(r"APB\d[LH]?ENR|AHB\dENR", reg):
        reg = "APBxENR"
    return f"{p}.{reg}" if p not in ("SCB", "RCC", "FLASH") else f"{periph}.{reg}"


spec_path, verify_path, fab_path = sys.argv[1:4]
spec = json.load(open(spec_path))
rows = json.load(open(verify_path))["ledger"]["rows"]
msg = json.load(open(fab_path))["error"]["message"]
ids = set(msg.split(": ", 2)[-1].split(", "))
groups = {}
for r in rows:
    if r["id"] in ids:
        groups.setdefault((r["register"], r["class"]), []).append(r["field"])
missing = sorted({(kind(reg), c) for (reg, c) in groups} - set(REASONS))
if missing:
    sys.exit(f"no hand-written reason for {missing}")
long = {k: len(v) for k, v in REASONS.items() if not (0 < len(v) <= 200 and v.isascii())}
if long:
    sys.exit(f"reasons over 200 bytes or not ASCII (validate's source rule): {long}")
spec["acknowledge_unwitnessed"] = [
    {"register": reg, "class": c, "fields": f, "reason": REASONS[(kind(reg), c)]}
    for (reg, c), f in groups.items()
]
open(spec_path, "w").write(json.dumps(spec, indent=2) + "\n")
print(f"{spec_path}: {len(groups)} entries, {sum(len(f) for f in groups.values())} fields")
