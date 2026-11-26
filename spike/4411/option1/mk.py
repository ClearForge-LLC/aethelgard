# Scratch: Sentinel-03 with U30 on its own LMR33630 (U13) from the battery. usage: python3 -I mk.py in.json out.json [limit_mv]
import json, sys, copy
s = json.load(open(sys.argv[1]))
RF = "VDD_3V3_RF"
s["voltage_domains"].append({"id": RF, "nominal_millivolts": 3300, "tolerance_permille": 50})
p = s["power"]
u11 = next(r for r in p["regulators"] if r["reference"] == "U11")
u13 = copy.deepcopy(u11)
ren = {"U11": "U13", "R20": "R30", "R21": "R31", "C20": "C30", "C61": "C63", "C21": "C31", "C22": "C32",
       "C23": "C33", "C24": "C34", "C25": "C35", "C26": "C36", "C27": "C37", "L2": "L3"}
u13["reference"] = "U13"; u13["output"] = RF
u13["feedback"]["top"]["reference"] = "R30"; u13["feedback"]["bottom"]["reference"] = "R31"
for e in u13["external"]: e["reference"] = ren[e["reference"]]
u13["junction"] = {"loss_milliwatts": {"value": 540, "source": "SNVSAN3F p. 27, Figure 9-8, typical at TA 25 C: 76 % on the 36 V curve at 0.5 A (read 76.7 to 78.7 %, AET-WO-4411 option 1); P_loss = 3.417 V x 500 mA x (1/0.76 - 1) = 540 mW"}, "loss_at_milliamps": 500}
p["regulators"].append(u13)
m = s["radio_modules"][0]; m["supply"] = RF
m["enable"]["resistor"]["to"] = RF; m["straps"]["IO2"]["resistor"]["to"] = RF
for l in p["loads"]:
    if l.get("reference") in ("R90", "R91"): l["domain"] = RF
if len(sys.argv) > 3:
    p["drop_limits"].append({"domain": RF, "max_millivolts": int(sys.argv[3]), "source": "scratch: floor(3,144,727 uV) - 3.0 V (the module's minimum)"})
json.dump(s, open(sys.argv[2], "w"), indent=2); open(sys.argv[2], "a").write("\n")
