#!/usr/bin/env python3
"""AET-WO-1211a: a stored solved topology moved from version 13's shape to 14's by rule alone, so
that its words can be compared with what the branch's `solve` writes: each `gpio` entry becomes an
output at `high` (a chip select), `adc_inputs` holds each power.sense net on its pin with ADC1 or
ADC3 (1211 Q8), and each PWM output's copied `pwm` gains idle {pulse_us: pulse_min_us} and
resolution_ps 1,000,000. Only re-serialization's own bytes move otherwise (checked: round trip).
Usage: python3 -I topology_v14.py FILE...  (rewrites each in place)"""
import collections, json, sys

def migrate(d):
    out = collections.OrderedDict()
    for k, v in d.items():
        if k == "gpio":
            out[k] = collections.OrderedDict(
                (n, collections.OrderedDict(pin=p, direction="output", initial="high")) for n, p in v.items())
            adc = collections.OrderedDict()
            for net, s in sorted(((d.get("power") or {}).get("sense") or {}).items()):
                chans = [c.split("_", 1) for c in s["channels"]]
                pick = next((c for a in ("ADC1", "ADC3") for c in chans if c[0] == a), None)
                assert pick, (net, s)
                adc[net] = collections.OrderedDict(pin=s["pin"], peripheral=pick[0], channel=pick[1])
            out["adc_inputs"] = adc
        elif k == "motor_outputs":
            for m in v.values():
                if m.get("pwm"):
                    m["pwm"]["idle"] = collections.OrderedDict(pulse_us=m["pwm"]["pulse_min_us"])
                    m["pwm"]["resolution_ps"] = 1_000_000
            out[k] = v
        else:
            out[k] = v
    return out

for path in sys.argv[1:]:
    text = open(path).read()
    d = json.loads(text, object_pairs_hook=collections.OrderedDict)
    nl = "\n" if text.endswith("\n") else ""
    assert json.dumps(d, separators=(",", ":")) + nl == text, f"{path} does not round-trip"
    assert d["schema_version"] == 14, path
    open(path, "w").write(json.dumps(migrate(d), separators=(",", ":")) + nl)
    print(path)
