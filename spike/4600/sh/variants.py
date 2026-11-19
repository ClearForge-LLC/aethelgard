#!/usr/bin/env python3
"""AET-WO-4600: write each one-change variant from logger.json or driver.json.

Every variant changes one thing (named in its file name); everything that one
change forces (a net constraint on a removed net, a lifecycle entry for a part
no longer used, the supply_source a power tree replaces) goes with it and is
listed here. Run from the repository root: python3 -I spike/4600/sh/variants.py
"""
import copy
import json
import pathlib

D = pathlib.Path("spike/4600")
FIX = pathlib.Path("crates/aethelgard-ir/tests/fixtures/sentinel-02-power.json")


def load(name):
    return json.loads((D / name).read_text())


def write(name, spec):
    (D / name).write_text(json.dumps(spec, indent=2) + "\n")


def constraint(net, ff, ns):
    return {"net_name": net, "max_capacitance_femtofarads": ff,
            "max_rise_time_nanoseconds": ns, "impedance_ohms": None}


L = load("logger.json")
M = load("driver.json")
fixture = json.loads(FIX.read_text())

# logger-i2c-only: no SPI bus. Forced with it: its flash, its two net
# constraints, and the flash's lifecycle acknowledgement.
v = copy.deepcopy(L)
v["spi_buses"] = []
v["spi_devices"] = []
v["net_constraints"] = [c for c in v["net_constraints"] if not c["net_name"].startswith("LOG_")]
v["accept_lifecycle"] = []
write("logger-i2c-only.json", v)

# logger-spi-only: a vibration logger, no I2C bus. Its IMU on its own SPI bus
# beside the flash. Forced: the I2C nets' constraints go, VIB's come.
v = copy.deepcopy(L)
v["project_name"] = "vibration-logger"
v["i2c_buses"] = []
v["i2c_devices"] = []
v["spi_buses"].insert(0, {
    "id": "VIB", "peripheral": None, "speed_hz": 10000000, "domain": "VDD_3V3",
    "sck": {"net": "VIB_SCK", "pin": None}, "mosi": {"net": "VIB_MOSI", "pin": None},
    "miso": {"net": "VIB_MISO", "pin": None}})
v["spi_devices"].insert(0, {
    "reference": "U40", "part": "ICM42688P", "supply": "VDD_3V3", "bus": "VIB",
    "cs": {"net": "VIB_CS", "pin": None}, "pin_capacitance": None})
v["net_constraints"] = [constraint("VIB_SCK", 30000, 20), constraint("VIB_MOSI", 30000, 20)] + [
    c for c in v["net_constraints"] if c["net_name"].startswith("LOG_")]
write("logger-spi-only.json", v)

# logger-1s: L powered from one Li-ion/LiPo cell through the power tree. The
# only regulator that makes 3.3 V is the buck (VIN >= 3.8 V); the only
# power-in connector row is the XT30. Forced: mcu.supply_source goes null
# (the tree supplies the MCU) and the bench-power connector J3 goes.
v = copy.deepcopy(L)
v["mcu"]["supply_source"] = None
v["connectors"] = [c for c in v["connectors"] if c["reference"] != "J3"]
fp = fixture["power"]
u11 = copy.deepcopy(next(r for r in fp["regulators"] if r["reference"] == "U11"))
v["power"] = {
    "conditions": copy.deepcopy(fp["conditions"]),
    "input": {"reference": "J4", "part": "XT30PW-M",
              "hot_plug": [{"reference": "C1", "part": "UCZ1H470MCL1GS", "side": "board"}]},
    "battery": {
        "chemistry": "LiPo or Li-ion, one cell",
        "cells": {"min": 1, "max": 1},
        "cell_min_millivolts": {"value": 3000, "source": "declared: the cell's discharge floor, as the power fixture declares it per cell"},
        "cell_max_millivolts": {"value": 4200, "source": "declared: a standard LiPo cell's full charge"},
    },
    "regulators": [u11],
    "loads": [
        {"domain": "VDD_3V3", "milliamps": 25, "source": "Winbond W25Q64JV rev J 9.4 p. 61: ICC4 to ICC7 max 25 mA", "reference": "U30"},
    ],
    "sense": [],
    "crossings": [],
}
write("logger-1s.json", v)

# logger-offboard: one TMP117 (U13) moves to the far end of a probe cable.
# No construct declares a device off the board: an i2c_devices entry is a
# placed part. So U13 leaves i2c_devices (the engine never sees it) and the
# cable is declared as the connector pins' offboard capacitance, as
# Sentinel-03's J8 declares its external I2C port.
v = copy.deepcopy(L)
v["i2c_devices"] = [d for d in v["i2c_devices"] if d["reference"] != "U13"]
cable = "condition: a probe lead of at most 1 m at 50 pF/m (50 pF), the mated contact (5 pF) and one TMP117 pin (UM10204 Table 10 Ci at most 10 pF)"
v["connectors"].append({
    "reference": "J5", "part": "SM04B-GHS-TB", "standard": None,
    "pins": [
        {"pin": "1", "net": "VDD_3V3", "offboard": None},
        {"pin": "2", "net": "TEMP_SCL", "offboard": {"capacitance_femtofarads": 65000, "source": cable}},
        {"pin": "3", "net": "TEMP_SDA", "offboard": {"capacitance_femtofarads": 65000, "source": cable}},
        {"pin": "4", "net": "GND", "offboard": None},
    ]})
write("logger-offboard.json", v)

# driver-tree: M powered from an XT30 (3S-6S) through two LMR33630 bucks, to
# VDD_3V3 and to VDD_5V for the servos, as the power fixture does. Forced:
# the VDD_5V domain, mcu.supply_source null, J3 gone, the servo connector's
# spare pin carries VDD_5V.
v = copy.deepcopy(M)
v["mcu"]["supply_source"] = None
v["connectors"] = [c for c in v["connectors"] if c["reference"] != "J3"]
for c in v["connectors"]:
    if c["reference"] == "J4":
        c["pins"][4]["net"] = "VDD_5V"
v["voltage_domains"].append({"id": "VDD_5V", "nominal_millivolts": 5000, "tolerance_permille": 50})
regs = [copy.deepcopy(r) for r in fp["regulators"] if r["reference"] in ("U10", "U11")]
v["power"] = {
    "conditions": copy.deepcopy(fp["conditions"]),
    "input": {"reference": "J5", "part": "XT30PW-M",
              "hot_plug": [{"reference": "C1", "part": "UCZ1H470MCL1GS", "side": "board"}]},
    "battery": copy.deepcopy(fp["battery"]),
    "regulators": regs,
    "loads": [
        {"domain": "VDD_5V", "milliamps": 2000, "source": "declared: two hobby servos' stall budget at 5 V, 1 A each; no servo datasheet is held"},
    ],
    "sense": [],
    "crossings": [],
}
write("driver-tree.json", v)

# driver-idle-zero: the H-bridge inputs idle at 0 us (coast), an H-bridge's
# real idle.
v = copy.deepcopy(M)
for m in v["motor_outputs"]:
    if m["label"].startswith("HB_"):
        m["pwm"]["pulse_min_us"] = 0
write("driver-idle-zero.json", v)
