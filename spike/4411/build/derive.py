# AET-WO-4411: derive the radio-module test fixture from Sentinel-03 (D6 is
# Scotty's: the fixture lives in tests/, never in examples/). Adds one serial
# port of the module's own (ruled D7: a UART of its own, J7 kept) and one
# ESP32-C3-WROOM-02-N4 on it. Re-run after #390 against examples/sentinel-03.json.
# Usage: python3 -I derive.py <sentinel-03.json> <out.json> [--keep-name]
import json
import sys

src, out = sys.argv[1:3]
s = json.load(open(src))
if "radio_modules" in s or any(p["label"] == "WIFI" for p in s["serial_ports"]):
    sys.exit(f"{src} already carries the module; derive from the board without it")
# A third argument keeps the board's own name (D6: the module goes on
# Sentinel-03 itself); without it, the tests/ fixture's.
if len(sys.argv) < 4:
    s["project_name"] = "sentinel-03-radio"
s["serial_ports"].append({
    "label": "WIFI",
    "peripheral": None,
    "tx": {"net": "WIFI_TX", "pin": None},
    "rx": {"net": "WIFI_RX", "pin": None},
    "role": "mavlink",
    "framing": {"baud": 57600, "data_bits": 8, "parity": "none", "stop_bits": 1, "inverted": False},
    "negotiated": None,
    "peer_deviation_ppm": 5000,
    "peer_deviation_source": "declared",
})
s["radio_modules"] = [{
    "reference": "U30",
    "part": "ESP32-C3-WROOM-02-N4",
    "serial_port": "WIFI",
    "supply": "VDD_3V3",
    "enable": {
        "resistor": {"reference": "R90", "part": "CRCW0603_F_K", "ohms": 10000, "to": "VDD_3V3"},
        "capacitor": {"reference": "C90", "part": "GRM188R71C105KA12"},
    },
    "straps": {
        "IO2": {"resistor": {"reference": "R91", "part": "CRCW0603_F_K", "ohms": 10000, "to": "VDD_3V3"}},
        "IO8": "open",
        "IO9": "open",
    },
    "decoupling": [
        {"reference": "C91", "part": "GCM32EC71H106KA03"},
        {"reference": "C92", "part": "GCM188R71H104KA57"},
    ],
}]
with open(out, "w") as f:
    json.dump(s, f, indent=2, ensure_ascii=False)
    f.write("\n")
