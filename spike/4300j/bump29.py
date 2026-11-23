"""AET-WO-4300j: specification 28 -> 29 by text edits only (never a re-serialisation; the locator is 4300i bump28.py's).
Each committed specification gets `"schema_version": 29`. Each motor output's `pull` gets `ohms` 442000 (D1 (a), D2),
a `link` with its allocated reference and part `CRCW0402_Z_0` (D4, D5), `"fitment": "fitted"` (D8) and the new
`source`, key order resistor, link, fitment, source. The parsed file must differ from the original only there.

usage: python3 -I bump29.py <repo root> <universe.tsv> <link-refs.tsv>"""
import json, os, re, sys

root, universe, refs_tsv = sys.argv[1:4]
OHMS = 442000
PART = "CRCW0402_Z_0"
# The ruled text (293 bytes) exceeds `board::MAX_SOURCE` (200): shortened by words, every figure and citation kept.
SOURCE = ("AN4899 Rev 4 5.1.5 p. 18 mirrored (4300j); DS12117 Rev 10 T49 pp. 136-137 = T146 p. 241; "
          "cross-rail 422561..895714 ohm; least E96, 1 %+2 % drift (VISHAY_20035 p. 2). Ruled 2026-11-23T10:11Z")
assert len(SOURCE) <= 200 and SOURCE == SOURCE.strip() and SOURCE.isascii(), len(SOURCE)
dec = json.JSONDecoder()
links = {}
for line in open(refs_tsv).read().splitlines()[1:]:
    c = line.split("\t")
    links[c[1]] = c[5].split(",")


def key_value(t, start, end, key):
    """(index of the key's quote, value start, value end) of `key` directly inside the object spanning [start, end)."""
    i = start + 1
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


def items_of(t, a0):
    out, i = [], a0 + 1
    while True:
        m = re.compile(r"\s*").match(t, i)
        if t[m.end()] == "]":
            return out
        s0, s1 = obj_span(t, m.end())
        out.append((s0, s1))
        n = re.compile(r"\s*,").match(t, s1)
        if not n:
            return out
        i = n.end()


for f in [l.split("\t")[1] for l in open(universe).read().splitlines()]:
    p = os.path.join(root, f)
    t = open(p).read()
    before = json.loads(t)
    assert before["schema_version"] == 28, f
    top0, top1 = obj_span(t, t.index("{"))
    k, v0, v1 = key_value(t, top0, top1, "schema_version")
    edits = [(v0, v1, "29")]
    mo = key_value(t, top0, top1, "motor_outputs")
    names = links.get(f, [])
    outs = items_of(t, mo[1]) if mo else []
    assert len(outs) == len(names), (f, len(outs), names)
    for (s0, s1), ref in zip(outs, names):
        _, p0, p1 = key_value(t, s0, s1, "pull")
        rk, r0, r1 = key_value(t, p0, p1, "resistor")
        _, o0, o1 = key_value(t, r0, r1, "ohms")
        assert t[o0:o1] == "40200", (f, t[o0:o1])
        sk, q0, q1 = key_value(t, p0, p1, "source")
        ind = indent_of(t, rk)
        link = '{"reference": "%s", "part": "%s"}' % (ref, PART)
        if ind is None:  # one line, as Sentinel-03's
            ins = ', "link": %s, "fitment": "fitted"' % link
        else:
            u = ind + "  "
            link = "{\n" + u + '"reference": "%s",\n' % ref + u + '"part": "%s"\n' % PART + ind + "}"
            ins = ",\n" + ind + '"link": ' + link + ",\n" + ind + '"fitment": "fitted"'
        edits += [(o0, o1, str(OHMS)), (r1, r1, ins), (q0, q1, json.dumps(SOURCE))]
    for a, b, s in sorted(edits, reverse=True):
        t = t[:a] + s + t[b:]
    after = json.loads(t)
    b = before
    b["schema_version"] = 29
    for m, ref in zip(b.get("motor_outputs", []), names):
        r = m["pull"]["resistor"]
        r["ohms"] = OHMS
        m["pull"] = {"resistor": r, "link": {"reference": ref, "part": PART}, "fitment": "fitted", "source": SOURCE}
    assert after == b, f
    for m in after.get("motor_outputs", []):
        assert list(m["pull"]) == ["resistor", "link", "fitment", "source"], f
    open(p, "w").write(t)
    print(f"edited\t{f}\t{len(names)}")
