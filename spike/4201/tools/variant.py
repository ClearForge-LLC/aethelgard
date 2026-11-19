"""Write a variant of a Sentinel specification: U2's part and address changed,
or U2 removed. usage: variant.py <in.json> <out.json> <part|-> [address]"""
import json, sys
src, dst, part = sys.argv[1], sys.argv[2], sys.argv[3]
s = json.load(open(src))
devs = s["i2c_devices"]
u2 = [d for d in devs if d["reference"] == "U2"]
assert len(u2) == 1 and u2[0]["part"] == "BME280", u2
if part == "-":
    s["i2c_devices"] = [d for d in devs if d["reference"] != "U2"]
else:
    u2[0]["part"] = part
    if len(sys.argv) > 4:
        u2[0]["address"] = int(sys.argv[4], 0)
json.dump(s, open(dst, "w"), indent=2)
