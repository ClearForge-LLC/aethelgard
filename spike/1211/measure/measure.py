#!/usr/bin/env python3
"""AET-WO-1211 §1.4: what the proposed schema moves, measured on today's committed files.

Read-only on the repository. For every committed specification (a tracked .json whose top level has
`schema_version` 19, `mcu` and `motor_outputs`, outside spike/) and every committed solved topology with
motor outputs:

  1. today's words: `aethelgard solve --file` (the engine built at this SHA) for a specification; the
     committed `motor_outputs` words for a solved topology;
  2. the proposed derivation (FEEDBACK §2, (d)), written here independently of `solve_timing`:
       tick       = resolution_ps                       (declared; exact at the kernel clock or refused)
       PSC        = kernel_hz * resolution_ps / 1e12 - 1  (an integer in 0..=0xFFFF, else ResolutionNotDerivable)
       ARR        = round_half_up(1e12 / (rate_hz * resolution_ps)) - 1   (2..=0xFFFF, else RateOutOfRange)
       CCR(idle)  = pulse: idle_ps / resolution_ps (exact, else IdleNotOnTick); low: 0; high: ARR + 1
     with idle = {"pulse_us": pulse_min_us} and resolution = {"ps": 1_000_000} stated on every PWM output;
  3. the comparison, word by word; and the fields each file must gain.

Usage: measure.py <aethelgard binary> <out.json>
"""
import json
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[3]
EXCLUDED = ("spike/",)  # spike/4600's boards were never committed specifications (its §9)


def tracked_json():
    out = subprocess.run(["git", "-C", str(REPO), "ls-files", "*.json"], check=True,
                         capture_output=True, text=True).stdout.split()
    return [p for p in out if not p.startswith(EXCLUDED)]


def kernel_hz(clock):
    # RM0433 Rev 9 Table 57: rcc_hclk1 while D2PPREx <= 2 (TIMPRE 0). Every committed topology has the
    # reset clock (d2ppre1 = d2ppre2 = 1), asserted rather than assumed.
    assert clock["d2ppre1"] == 1 and clock["d2ppre2"] == 1 and not clock["timpre"], clock
    return clock["sysclk_hz"] // clock["d1cpre"] // clock["hpre"]


def proposed(pwm, idle, resolution_ps, kernel):
    num = kernel * resolution_ps
    if num % 10**12:
        return {"refused": "ResolutionNotDerivable"}
    psc = num // 10**12 - 1
    if not 0 <= psc <= 0xFFFF:
        return {"refused": "ResolutionNotDerivable"}
    period = 10**12
    den = pwm["rate_hz"] * resolution_ps
    ticks = (2 * period + den) // (2 * den)
    if not 3 <= ticks <= 0x10000:
        return {"refused": "RateOutOfRange"}
    if pwm["pulse_max_us"] * 10**6 >= ticks * resolution_ps:
        return {"refused": "PulseExceedsPeriod"}
    if "pulse_us" in idle:
        ps = idle["pulse_us"] * 10**6
        if ps % resolution_ps:
            return {"refused": "IdleNotOnTick"}
        ccr = ps // resolution_ps
    elif idle == {"level": "low"}:
        ccr = 0
    else:
        ccr = ticks
    return {"psc": psc, "arr": ticks - 1, "ccr": ccr}


def words(o):
    return {k: o[k] for k in ("psc", "arr", "ccr")}


def main():
    binary, out_path = sys.argv[1], sys.argv[2]
    specs, topologies = [], []
    for rel in tracked_json():
        try:
            d = json.loads((REPO / rel).read_text())
        except (json.JSONDecodeError, UnicodeDecodeError):
            continue
        if not isinstance(d, dict) or "schema_version" not in d:
            continue
        if "mcu" in d and "motor_outputs" in d and "project_name" in d:
            specs.append((rel, d))
        elif "specification_sha256" in d and d.get("motor_outputs"):
            topologies.append((rel, d))
    rows, moved, summary = [], [], {"pwm": 0, "dshot": 0}
    for rel, d in specs:
        outs = d["motor_outputs"]
        pwm_n = sum(1 for o in outs if o["protocol"] == "pwm")
        dshot_n = len(outs) - pwm_n
        summary["pwm"] += pwm_n
        summary["dshot"] += dshot_n
        row = {"file": rel, "schema_version": d["schema_version"], "outputs": len(outs), "pwm": pwm_n,
               "dshot": dshot_n, "gains": {"motor_outputs[].pwm.idle": pwm_n,
                                           "motor_outputs[].pwm.resolution": pwm_n}}
        if outs:
            r = subprocess.run([binary, "solve", "--file", str(REPO / rel)], capture_output=True, text=True)
            row["solve_exit"] = r.returncode
            if r.returncode == 0:
                t = json.loads(r.stdout)
                row["words"] = compare(t, rel, moved)
            else:
                row["solve_error"] = r.stdout.strip()[:300]
        rows.append(row)
    for rel, t in topologies:
        rows.append({"file": rel, "solved_topology": True, "schema_version": t["schema_version"],
                     "words": compare(t, rel, moved)})
    json.dump({"rows": rows, "moved": moved, "summary": summary}, open(out_path, "w"), indent=1)
    print(json.dumps({"specifications": len(specs), "with_outputs": sum(1 for _, d in specs if d["motor_outputs"]),
                      "solved_topologies": len(topologies), "moved": moved, "summary": summary}, indent=1))


def compare(t, rel, moved):
    k = kernel_hz(t["clock"])
    res = []
    for label, o in t["motor_outputs"].items():
        today = words(o)
        if o["protocol"] != "pwm":
            # DShot: `pwm` is null, so no field reaches it; untouched by construction, not compared.
            res.append({"output": label, "protocol": o["protocol"], "today": today, "untouched": True})
            continue
        new = proposed(o["pwm"], {"pulse_us": o["pwm"]["pulse_min_us"]}, 1_000_000, k)
        same = new == today
        res.append({"output": label, "protocol": o["protocol"], "kernel_hz": k, "today": today,
                    "proposed": new, "same": same})
        if not same:
            moved.append({"file": rel, "output": label, "today": today, "proposed": new})
    return res


if __name__ == "__main__":
    main()
