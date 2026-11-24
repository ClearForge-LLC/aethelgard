# AET-WO-4300k: Sentinel-03's specification to the ruling (spec 30). Run from the repo root: python3 -I spike/4300k/build/s03_spec.py
import json, collections, sys
OD = collections.OrderedDict
P = "examples/sentinel-03.json"
s = json.loads(open(P).read(), object_pairs_hook=OD)
H = json.loads(open("crates/aethelgard-cli/tests/fixtures/hse-crystal-16.json").read(), object_pairs_hook=OD)
FUSE = sys.argv[1] if len(sys.argv) > 1 else "SF-1206F700-2"

def insert_after(d, after, key, value):
    out = OD()
    for k, v in d.items():
        out[k] = v
        if k == after:
            out[key] = value
    d.clear(); d.update(out)

# (6) the crystal, as hse-crystal-16 declares it (AN2867 Rev 25, M3)
m = s["mcu"]
new = OD()
for k, v in m.items():
    if k == "hse_hz":
        new["hse"] = H["mcu"]["hse"]
    else:
        new[k] = v
s["mcu"] = new
# (9) Path B: TLV803S on NRST
insert_after(s["mcu"], "reset_switch", "reset_supervisor", OD([
    ("reference", "U12"), ("part", "TLV803SDBZR"),
    ("pull_up", OD([("reference", "R61"), ("part", "CRCW0603_F_K"), ("ohms", 100000)])),
    ("bypass", OD([("reference", "C62"), ("part", "GCM188R71H104KA57")])),
]))
# 4410 D3: U5's orientation and the board's forward edge (ruled D9)
for d in s["spi_devices"]:
    if d["reference"] == "U5":
        d["orientation"] = OD([("x", "right"), ("z", "up")])
insert_after(s["board"], "copper_layers", "forward", "top")
p = s["power"]
# 4S (Scotty, 2026-11-23)
p["battery"]["chemistry"] = "LiPo, 3S to 4S, high-voltage cells allowed"
p["battery"]["cells"]["max"] = 4
# (10) the input's protection
p["input"]["protection"] = OD([
    ("fuse", OD([("reference", "F1"), ("part", FUSE)])),
    ("fet", OD([("reference", "Q1"), ("part", "DMP6023LSS-13")])),
    ("zener", OD([("reference", "D1"), ("part", "MMSZ5242BT1G")])),
    ("gate_resistor", OD([("reference", "R60"), ("part", "CRCW0603_F_K"), ("ohms", 100000)])),
])
# A6 (ruled D8): two input ceramics per buck; (a): the copper is measured, the loss keeps its current
extra = {"U10": "C60", "U11": "C61"}
loss_at = {"U10": 2000, "U11": 679}
for r in p["regulators"]:
    ext = []
    for e in r["external"]:
        ext.append(e)
        if e["role"] == "cin":
            ext.append(OD([("role", "cin"), ("reference", extra[r["reference"]]), ("part", e["part"])]))
    r["external"] = ext
    j = r["junction"]
    r["junction"] = OD([("loss_milliwatts", j["loss_milliwatts"]), ("loss_at_milliamps", loss_at[r["reference"]])])
# (2), D6, D13: the MCU's supply limited, at most 110 mV
p["drop_limits"] = [OD([("domain", "VDD_3V3"), ("max_millivolts", 110),
    ("source", "AET-WO-4300k D6 (ruled): VDD_3V3's solved minimum less this stays above TLV803S's VIT- max plus its typical hysteresis, 3030 mV")])]
# (b) OSH Park, read 2026-11-24 (docs.oshpark.com/services/four-layer/, page sha256 cf14c792...)
st = s["board"]["stackup"]
OSH = "oshpark-four-layer"
fab = OD([("name", "OSH Park"), ("documents", [OD([
    ("id", OSH), ("title", "OSH Park 4 layer service specifications"),
    ("url", "https://docs.oshpark.com/services/four-layer/"), ("read", "2026-11-24")])])])
new = OD([("fabricator", fab)])
new.update(st); st.clear(); st.update(new)
def cite(d, c): d["cites"] = c
cite(st["finished_thickness_um"], OD([("fabricator", OSH)]))
for d in st["dielectrics"]:
    cite(d, OD([("fabricator", OSH)]))
cite(st["copper_max_um"], OD([("condition", "copper-max-2oz")]))
cite(st["relative_permittivity_max_permille"], OD([("condition", "permittivity-fr4-max")]))
st["relative_permittivity_max_permille"]["source"] = "the IPC-4101 FR-4 maximum, specified at 1 MHz; IPC-4101 is not held, so a condition (OSH Park states FR408HR 3.61 at 1 GHz, nominal)"
mins = [("outer_copper_min_um", 30, "outer-copper-min", "a declared condition: OSH Park states 1 oz outer copper, nominal, and no minimum"),
        ("plane_copper_min_um", 15, "plane-copper-min", "a declared condition: OSH Park states 1/2 oz inner copper, nominal, and no minimum"),
        ("plating_min_um", 18, "plating-min", "a declared condition: OSH Park states via plating 1 mil, nominal, and no minimum"),
        ("copper_temperature_max_c", 125, "copper-temperature-max", "a declared condition: no fabricator document states the copper's temperature limit")]
order = OD()
for k, v in st.items():
    order[k] = v
    if k == "relative_permittivity_max_permille":
        for key, val, cid, src in mins:
            order[key] = OD([("value", val), ("source", src), ("cites", OD([("condition", cid)]))])
st.clear(); st.update(order)
R = "OSH Park 4-layer, docs.oshpark.com/services/four-layer/, read 2026-11-24, Drill Specifications: "
st["fab_tolerances"] = OD([
    ("hole_position_um", OD([("value", 51), ("source", R + "Drill Positional Tolerance Max: 2mil (0.0508mm), rounded up"), ("cites", OD([("fabricator", OSH)]))])),
    ("hole_size_plus_um", OD([("value", 64), ("source", R + "Drill Size tolerance Max: +/- 2.5mil (0.0635mm), rounded up"), ("cites", OD([("fabricator", OSH)]))])),
    ("hole_size_minus_um", OD([("value", 64), ("source", R + "Drill Size tolerance Max: +/- 2.5mil (0.0635mm), rounded up"), ("cites", OD([("fabricator", OSH)]))])),
    ("track_width_permille", OD([("value", 200), ("source", "a declared condition: OSH Park publishes no track-width tolerance; 20 % is declared"), ("cites", OD([("condition", "track-width-tolerance")]))])),
    ("via_ring_um", OD([("value", 102), ("source", "OSH Park 4-layer, docs.oshpark.com/services/four-layer/, read 2026-11-24, Copper Specifications: Annular Ring 4mil (0.1016mm), rounded up"), ("cites", OD([("fabricator", OSH)]))])),
])
s["schema_version"] = 30
open(P, "w").write(json.dumps(s, indent=2, ensure_ascii=False) + "\n")
print("written", P, "fuse", FUSE)
