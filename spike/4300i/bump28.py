"""AET-WO-4300i: specification 27 -> 28 by text edits only (never a re-serialisation).
usage: python3 -I bump28.py <repo root> <v27.txt> <pull-refs.tsv>"""
import json, os, re, sys
root, listing, refs_tsv = sys.argv[1:4]
SOURCE = ("DS12117 Rev 10 Table 49 p. 137, Table 146 p. 241: RPD 30/40/50 kOhm. External copy of the Q-G pull (D0): "
          "E96 value nearest 40 kOhm (VISHAY_20035 p. 2). Ruled 2026-11-23T01:15Z")
S03 = "crates/aethelgard-cli/tests/fixtures/sentinel-03.json"
S03_RESERVED = [
    ("PA11", "USB OTG_FS DM, kept free for USB (AET-WO-3400 F5, 4300e1)"),
    ("PA12", "USB OTG_FS DP, kept free for USB (AET-WO-3400 F5, 4300e1)"),
    ("PC9", "MCO2: AET-WO-3400 E5's clock-out, moved from MCO1/PA8 when MOTOR1 took PA8 (AET-WO-4300h D3, ruled 2026-11-23)"),
]
dec = json.JSONDecoder()
refs = {}
for line in open(refs_tsv).read().splitlines()[1:]:
    c = line.split("\t")
    refs[c[1]] = c[4].split(",")

def key_value(t, start, end, key):
    """(index of the key's quote, value start, value end) of `key` directly inside the object spanning [start, end)."""
    i = start + 1
    depth_obj = start
    while True:
        m = re.compile(r'\s*"((?:[^"\\]|\\.)*)"\s*:\s*').match(t, i)
        if not m:
            raise SystemExit(f"no key at {i}")
        v0 = m.end()
        _, v1 = dec.raw_decode(t, v0)
        if m.group(1) == key:
            return m.start() + (len(m.group(0)) - len(m.group(0).lstrip())), v0, v1
        n = re.compile(r'\s*,').match(t, v1)
        if not n:
            return None
        i = n.end()

def obj_span(t, v0):
    _, v1 = dec.raw_decode(t, v0)
    return v0, v1

def indent_of(t, i):
    ls = t.rfind("\n", 0, i) + 1
    return t[ls:i] if t[ls:i].strip() == "" else None

for f in [l.split()[0] for l in open(listing)]:
    p = os.path.join(root, f)
    t = open(p).read()
    before = json.loads(t)
    assert t.count('"schema_version": 27') == 1, f
    t = t.replace('"schema_version": 27', '"schema_version": 28')
    top0, top1 = obj_span(t, t.index("{"))
    # motor outputs first (they sit after mcu in no file? edit from the end backwards to keep indices valid)
    edits = []
    mo = key_value(t, top0, top1, "motor_outputs")
    names = refs.get(f, [])
    if mo:
        a0, a1 = mo[1], mo[2]
        items, i = [], a0 + 1
        while True:
            m = re.compile(r"\s*").match(t, i)
            if t[m.end()] == "]":
                break
            s0, s1 = obj_span(t, m.end())
            items.append((s0, s1))
            n = re.compile(r"\s*,").match(t, s1)
            if not n:
                break
            i = n.end()
        assert len(items) == len(names), (f, len(items), names)
        for (s0, s1), ref in zip(items, names):
            k, v0, v1 = key_value(t, s0, s1, "esc")
            # esc is the last key of every output
            assert re.compile(r"\s*\}").match(t, v1), f
            ind = indent_of(t, k)
            inline_esc = t[v0] == "{" and "\n" not in t[v0:v1]
            if inline_esc or ind is None:
                val = ('{"resistor": {"reference": "%s", "part": "CRCW0603_F_K", "ohms": 40200, "to": "GND"}, "source": %s}'
                       % (ref, json.dumps(SOURCE)))
            else:
                u = ind + "  "
                val = ("{\n" + u + '"resistor": {\n' + u + '  "reference": "%s",\n' % ref + u + '  "part": "CRCW0603_F_K",\n'
                       + u + '  "ohms": 40200,\n' + u + '  "to": "GND"\n' + u + "},\n" + u + '"source": ' + json.dumps(SOURCE)
                       + "\n" + ind + "}")
            sep = ",\n" + ind if ind is not None else ", "
            edits.append((v1, sep + '"pull": ' + val))
    # mcu
    k, v0, v1 = key_value(t, top0, top1, "mcu")
    close = v1 - 1
    last = len(t[:close].rstrip())
    k2, _, _ = key_value(t, v0, v1, "part")
    ind = indent_of(t, k2)
    if f == S03:
        u = ind + "  "
        body = ",\n".join(u + "{\n" + u + '  "pin": "%s",\n' % pin + u + '  "reason": %s\n' % json.dumps(r) + u + "}"
                          for pin, r in S03_RESERVED)
        val = "[\n" + body + "\n" + ind + "]"
    else:
        val = "[]"
    edits.append((last, (",\n" + ind if ind is not None else ", ") + '"reserved_pins": ' + val))
    for at, s in sorted(edits, reverse=True):
        t = t[:at] + s + t[at:]
    if f == S03:
        old = '"label": "MOTOR1",\n      "pin": "PE9",'
        assert t.count(old) == 1
        t = t.replace(old, '"label": "MOTOR1",\n      "pin": "PA8",')
    after = json.loads(t)
    # the parsed file differs only where intended
    b = before
    b["schema_version"] = 28
    b["mcu"]["reserved_pins"] = [{"pin": p_, "reason": r} for p_, r in S03_RESERVED] if f == S03 else []
    for m, ref in zip(b.get("motor_outputs", []), names):
        m["pull"] = {"resistor": {"reference": ref, "part": "CRCW0603_F_K", "ohms": 40200, "to": "GND"}, "source": SOURCE}
    if f == S03:
        b["motor_outputs"][0]["pin"] = "PA8"
    assert after == b, f
    open(p, "w").write(t)
    print("edited", f)
