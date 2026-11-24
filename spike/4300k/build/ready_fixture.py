# AET-WO-4300k (T), ruled 2026-11-24T11:25Z: the ready test fixture. Sentinel-01-minimal with the
# reset supervisor Sentinel-03 declares, fab tolerances, a fabricator and every condition acknowledged,
# so a `--fab-out` test whose point is a downstream gate meets the readiness gate. Never ordered.
# Usage: python3 -I spike/4300k/build/ready_fixture.py [conditions.json]  (the second pass acknowledges).
import json, collections, sys
OD = collections.OrderedDict
SRC, OUT = "examples/sentinel-01-minimal.json", "crates/aethelgard-cli/tests/fixtures/sentinel-01-ready.json"
SCN = "crates/aethelgard-cli/tests/fixtures/sentinel-01-ready"
s = json.load(open(SRC), object_pairs_hook=OD)
s03 = json.load(open("examples/sentinel-03.json"), object_pairs_hook=OD)
s["project_name"] = "sentinel-01-ready"
s["mcu"]["reset_supervisor"] = s03["mcu"]["reset_supervisor"]
st = s["board"]["stackup"]
FIX = "test fixture (AET-WO-4300k (T)): never ordered, so no fabricator document stands behind it"
st["fabricator"] = OD([("name", "test fixture"), ("documents", [OD([
    ("id", "test-fixture"), ("title", "AET-WO-4300k (T): a test-only specification, never ordered"),
    ("url", "crates/aethelgard-cli/tests/fixtures/sentinel-01-ready.json"), ("read", "2026-11-24")])])])
tol = s03["board"]["stackup"]["fab_tolerances"]
st["fab_tolerances"] = OD((k, OD([("value", v["value"]), ("source", FIX + ": OSH Park's four-layer figure, as Sentinel-03 declares it")]))
                          for k, v in tol.items())
def cite(o):
    if isinstance(o, dict):
        if "source" in o and "cites" not in o:
            o["cites"] = OD([("condition", "test-fixture")])
        for v in o.values():
            cite(v)
    elif isinstance(o, list):
        for v in o:
            cite(v)
for k, v in st.items():
    if k != "fabricator":
        cite(v)
if len(sys.argv) > 1:
    ids = json.load(open(sys.argv[1]))
    s["acknowledge_conditions"] = [OD([("id", i), ("reason", "test fixture (AET-WO-4300k (T)): never ordered; " + why)])
                                   for i, why in ids.items()]
open(OUT, "w").write(json.dumps(s, indent=2, ensure_ascii=False) + "\n")
for p in ["scenarios/sentinel-01/seeded/pullup-3k3.json", "scenarios/sentinel-01/droop-bor3.json",
          "scenarios/sentinel-01/seeded/spi-absent-u4.json", "scenarios/sentinel-01/pullup-2k2.json",
          "scenarios/sentinel-01/seeded/serial-rx-open-console.json"]:
    sc = json.load(open(p), object_pairs_hook=OD)
    sc["base"] = "../sentinel-01-ready.json"
    open(f"{SCN}/{p.split('/')[-1]}", "w").write(json.dumps(sc, indent=2, ensure_ascii=False) + "\n")
print("written", OUT)
