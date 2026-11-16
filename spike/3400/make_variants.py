#!/usr/bin/env python3
"""AET-WO-3400 S2: build the specification variants from today's Sentinel-01.

Every variant starts from examples/sentinel-01-minimal.json and only ADDS
sections grafted from fixtures already on main; V0's net_constraints and
board (rules, stackup) are never edited. Run from the repository root:

    python3 -I spike/3400/make_variants.py

It writes spike/3400/variants/v*.json. V0 is a byte copy of the example.
"""
import copy
import json
import pathlib
import sys

ROOT = pathlib.Path.cwd()
OUT = ROOT / "spike/3400/variants"
EXAMPLE = ROOT / "examples/sentinel-01-minimal.json"
POWER = ROOT / "crates/aethelgard-ir/tests/fixtures/sentinel-02-power.json"
MOTORS = ROOT / "crates/aethelgard-cli/tests/fixtures/motors-dshot600.json"
SERIAL = ROOT / "crates/aethelgard-cli/tests/fixtures/serial-four.json"
CONNS = ROOT / "crates/aethelgard-cli/tests/fixtures/connectors.json"
FOUR = ROOT / "crates/aethelgard-cli/tests/fixtures/four-layer.json"

# The off-board load the fixtures declare for a JST lead (connectors.json, serial-four.json).
JST_OFFBOARD = ("condition: a JST cable of at most 100 mm at 50 pF/m (5 pF), the mated contact (5 pF) "
                "and one remote pin of at most 10 pF (UM10204 Table 10 Ci for I2C)")


def load(p):
    return json.loads(p.read_text())


def pin(n, net, off=None):
    return {"pin": str(n), "net": net, "offboard": off}


def v1(base):
    """V0 + F1: the battery input, two LMR33630 bucks (5 V, 3.3 V), the battery sense."""
    v = copy.deepcopy(base)
    p = copy.deepcopy(load(POWER)["power"])
    p["input"]["reference"] = "J4"  # the fixture's J1 is the console on Sentinel-01
    p["regulators"] = [r for r in p["regulators"] if r["part"] == "LMR33630ADDA"]
    p["loads"] = [l for l in p["loads"] if l["domain"] != "VDD_1V8"]
    p["crossings"] = []
    v["power"] = p
    v["voltage_domains"].append({"id": "VDD_5V", "nominal_millivolts": 5000, "tolerance_permille": 50})
    v["mcu"]["supply_source"] = None  # U11 outputs VDD_3V3 (4300g D3: a supply has one source)
    v["net_classes"] = copy.deepcopy(load(POWER)["net_classes"])
    v["project_name"] = "s01-v1"
    return v


def v2(base):
    """V1 + F3 (four DShot600 to a GH-6), F4 (CRSF on a DS-009 TELEM port), a MAVLink TELEM port."""
    v = v1(base)
    # Measured on the first V2 run: with motors, the solver moves the console from PA9/PA10 to
    # PB14/PB15 (TIM1 takes PA8-PA11), which leaves the system bootloader's USART1 pins (T8) and
    # makes V0's GPIOA acknowledgements stale. Pinning the console where V0's solve put it is a
    # constraint added, not one dropped.
    for ser in v["serial_ports"]:
        if ser["label"] == "CONSOLE":
            ser["tx"]["pin"], ser["rx"]["pin"] = "PA9", "PA10"
    v["motor_outputs"] = copy.deepcopy(load(MOTORS)["motor_outputs"])
    # Measured on the pinned run: the motors solve to PA8, PE11, PE13, PA11, and 4230's ruling pulls a
    # motor pin down from the first GPIO initialisation, so PUPD8 and PUPD11 leave the ledger's
    # same-source row and validate refuses V0's entry naming them (UnwitnessedAcknowledgementStale).
    # Only those two names are removed; nothing is acknowledged that V0 did not acknowledge.
    for a in v["acknowledge_unwitnessed"]:
        if a["register"] == "GPIOA.PUPDR" and a["class"] == "same-source":
            a["fields"] = [f for f in a["fields"] if f not in ("PUPD8", "PUPD11")]
    motors = copy.deepcopy(load(MOTORS)["connectors"][0])
    motors["reference"] = "J5"
    v["connectors"].append(motors)
    ser = load(SERIAL)
    for label, ref in (("CRSF", "J6"), ("MAVLINK", "J7")):
        port = copy.deepcopy(next(s for s in ser["serial_ports"] if s["label"] == label))
        v["serial_ports"].append(port)
        tx, rx = port["tx"]["net"], port["rx"]["net"]
        conn = copy.deepcopy(next(c for c in ser["connectors"]
                                  if any(q["net"] == tx for q in c["pins"])))
        conn["reference"] = ref
        assert [q["net"] for q in conn["pins"]][:3] == ["VDD_5V", tx, rx], conn
        v["connectors"].append(conn)
    v["project_name"] = "s01-v2"
    return v


def v3(base):
    """V2 + a second I2C bus (100 kHz, 10 kOhm) on a DS-009 I2C port with a declared off-board load,
    and an external SPI port (connectors.json's EXT): what specification 12 can say of T1, T2, J1."""
    v = v2(base)
    v["i2c_buses"].append({
        "id": "EXT", "peripheral": None, "speed_hz": 100000, "pull_up_ohms": 10000, "domain": "VDD_3V3",
        "sda": {"net": "EXT_SDA", "pin": None}, "scl": {"net": "EXT_SCL", "pin": None}})
    for n in ("EXT_SDA", "EXT_SCL"):
        v["net_constraints"].append({"net_name": n, "max_capacitance_femtofarads": 100000,
                                     "max_rise_time_nanoseconds": 1000, "impedance_ohms": None})
    off = {"capacitance_femtofarads": 20000, "source": JST_OFFBOARD}
    v["connectors"].append({"reference": "J8", "part": "SM04B-GHS-TB", "standard": "DS-009 I2C", "pins": [
        pin(1, "VDD_5V"), pin(2, "EXT_SCL", off), pin(3, "EXT_SDA", off), pin(4, "GND")]})
    c = load(CONNS)
    v["spi_buses"].append(copy.deepcopy(next(b for b in c["spi_buses"] if b["id"] == "EXT")))
    v["spi_buses"][-1]["id"] = "XSPI"
    for b in ("sck", "mosi", "miso"):
        v["spi_buses"][-1][b]["net"] = "X" + v["spi_buses"][-1][b]["net"]
    for k in c["net_constraints"]:
        if k["net_name"].startswith("EXT_"):
            k = copy.deepcopy(k)
            k["net_name"] = "X" + k["net_name"]
            v["net_constraints"].append(k)
    j = copy.deepcopy(next(x for x in c["connectors"] if x["reference"] == "J2"))
    j["reference"] = "J9"
    for q in j["pins"]:
        if q["net"] and q["net"].startswith("EXT_"):
            q["net"] = "X" + q["net"]
    v["connectors"].append(j)
    v["project_name"] = "s01-v3"
    return v


def v4(base):
    """V3 with the later jobs' pins kept free, by pinning (constraints added, none dropped). Measured on
    V3: the solver put MOTOR1 on PA8 (MCO1's only pin), MOTOR4 on PA11 and XSPI's SCK on PA12 (OTG_FS
    DM/DP exist only on PA11/PA12; FDCAN1 on PA11/PA12 or PD0/PD1). Pin table: PE9/PE11/PE13/PE14 are
    TIM1_CH1-4; PE2/PE5/PE6 are SPI4 SCK/MISO/MOSI (STM32H753ZITx.xml)."""
    v = v3(base)
    for m, p in zip(v["motor_outputs"], ("PE9", "PE11", "PE13", "PE14")):
        m["pin"] = p
    x = next(b for b in v["spi_buses"] if b["id"] == "XSPI")
    x["sck"]["pin"], x["miso"]["pin"], x["mosi"]["pin"] = "PE2", "PE5", "PE6"
    v["project_name"] = "s01-v4"
    return v


def v5(base):
    """The recommended tier-1 board (FEEDBACK section 7): V4 without the external SPI port (its J9 lines
    measured 54.0/56.1 pF against the declared 50 pF in extract) and with NRST on the SWD header's
    pin 3 (connect-under-reset; measured to validate and pass every gate in probe p-nrst)."""
    v = v4(base)
    v["spi_buses"] = [b for b in v["spi_buses"] if b["id"] != "XSPI"]
    v["net_constraints"] = [k for k in v["net_constraints"] if not k["net_name"].startswith("XEXT_")]
    v["connectors"] = [c for c in v["connectors"] if c["reference"] != "J9"]
    j2 = next(c for c in v["connectors"] if c["reference"] == "J2")
    j2["pins"][2]["net"] = "NRST"
    v["project_name"] = "s01-v5"
    return v


def v6(base):
    """V5 plus the rest of F1 as the work order names it (buck, LDO, hot-plug): the fixture's TLV75518
    LDO U12 from VDD_3V3 to a VDD_1V8 domain, with its 20 mA declared load, as tests/common/power.rs
    gates it. The hot-plug part is declared since V1 but not drawn (FEEDBACK F1)."""
    v = v5(base)
    fx = load(POWER)
    v["power"]["regulators"].append(copy.deepcopy(next(r for r in fx["power"]["regulators"] if r["reference"] == "U12")))
    v["power"]["loads"].append(copy.deepcopy(next(l for l in fx["power"]["loads"] if l["domain"] == "VDD_1V8")))
    v["voltage_domains"].append(copy.deepcopy(next(d for d in fx["voltage_domains"] if d["id"] == "VDD_1V8")))
    v["project_name"] = "s01-v6"
    return v


def v7(base):
    """V6 plus main's 1.8 V crossing (TXU0304 U13) and the ICM-20948 behind it (U5, SPI 'IMU'), grafted
    from sentinel-02-power.json with its acknowledgement: a density PROXY for F2 (an IMU in a 3 x 3 mm
    0.4 mm-pitch package on its own SPI bus), not the ICM-42688-P or BMP581, which main cannot draw."""
    v = v6(base)
    fx = load(POWER)
    v["power"]["crossings"] = copy.deepcopy(fx["power"]["crossings"])
    v["spi_buses"].append(copy.deepcopy(next(b for b in fx["spi_buses"] if b["id"] == "IMU")))
    v["spi_devices"].append(copy.deepcopy(next(d for d in fx["spi_devices"] if d["reference"] == "U5")))
    v["net_constraints"] += [copy.deepcopy(k) for k in fx["net_constraints"] if k["net_name"].startswith("IMU")]
    v["accept_lifecycle"].append(copy.deepcopy(next(a for a in fx["accept_lifecycle"] if a["part"] == "ICM20948")))
    v["project_name"] = "s01-v7"
    return v


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    raw = EXAMPLE.read_bytes()
    (OUT / "v0.json").write_bytes(raw)
    base = json.loads(raw)
    for name, f in (("v1", v1), ("v2", v2), ("v3", v3), ("v4", v4), ("v5", v5), ("v6", v6), ("v7", v7)):
        (OUT / f"{name}.json").write_text(json.dumps(f(base), indent=2) + "\n")
    # The amendment (WO section 0): every variant again at four layers, S-P-P-S as AET-WO-4100 built it.
    # Only `board` is replaced, by four-layer.json's (OSH Park's stackup, In1.Cu GND, In2.Cu VDD_3V3).
    four = load(FOUR)["board"]
    for name in ("v0", "v1", "v2", "v3", "v4", "v5", "v6", "v7"):
        v = json.loads((OUT / f"{name}.json").read_text())
        v["board"] = copy.deepcopy(four)
        v["project_name"] += "-4l"
        (OUT / f"{name}-4l.json").write_text(json.dumps(v, indent=2) + "\n")
    print("wrote", sorted(p.name for p in OUT.glob("v*.json")), file=sys.stderr)


if __name__ == "__main__":
    main()
