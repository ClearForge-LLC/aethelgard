#!/usr/bin/env python3
"""Regenerates the 1302 reviewer's congested specifications (AET-WO-1302 A6/B10).

`many()` is copied from the reviewer's generator, recovered verbatim from the 1302
reviewer's session transcript (subagent a32c7f67, its `specs.py`), with only its
imports replaced. D1 = many(): 4 I2C buses x 6 devices, short references S0..S23,
one SPI bus with one W25Q64JV (S1000). Usage: gen.py <reference.json> <out-dir>.
"""
import copy, json, os, sys

REF, OUT = sys.argv[1], sys.argv[2]
BASE = json.load(open(REF))
os.makedirs(OUT, exist_ok=True)

I2C_PARTS = [("BME280", 118), ("BME280", 119), ("TMP117", 72), ("TMP117", 73), ("TMP117", 74), ("TMP117", 75)]


def many(n_i2c=4, per_bus=6, n_spi=1, flash_per_spi=1, ref=lambda i: f"S{i}", net=lambda s: s, domain="VDD_3V3", domains=None):
    s = copy.deepcopy(BASE)
    if domains:
        s["voltage_domains"] = [{"id": d, "nominal_millivolts": 3300, "tolerance_permille": 50} for d in domains]
        s["mcu"]["supply"] = domains[0]
    buses, devs, cons = [], [], []
    for b in range(n_i2c):
        dom = domains[b % len(domains)] if domains else domain
        buses.append({"id": f"B{b}", "peripheral": None, "speed_hz": 400000, "pull_up_ohms": 2200, "domain": dom,
                      "sda": {"net": net(f"SDA{b}"), "pin": None}, "scl": {"net": net(f"SCL{b}"), "pin": None}})
        for (part, addr) in I2C_PARTS[:per_bus]:
            devs.append({"reference": ref(len(devs)), "part": part, "supply": dom, "bus": f"B{b}", "address": addr})
    sbuses, sdevs = [], []
    for b in range(n_spi):
        dom = domains[b % len(domains)] if domains else domain
        sbuses.append({"id": f"F{b}", "peripheral": None, "speed_hz": 20000000, "domain": dom,
                       "sck": {"net": net(f"SCK{b}"), "pin": None}, "mosi": {"net": net(f"MOSI{b}"), "pin": None},
                       "miso": {"net": net(f"MISO{b}"), "pin": None}})
        for k in range(flash_per_spi):
            sdevs.append({"reference": ref(1000 + len(sdevs)), "part": "W25Q64JV", "supply": dom, "bus": f"F{b}",
                          "cs": {"net": net(f"CS{b}_{k}"), "pin": None}})
    s["i2c_buses"], s["i2c_devices"] = buses, devs
    s["spi_buses"], s["spi_devices"] = sbuses, sdevs
    # 4100 S3: the schema now requires a rise-time constraint on every I2C line (1302 had none).
    s["net_constraints"] = [{"net_name": b[k]["net"], "max_capacitance_femtofarads": 100000,
                             "max_rise_time_nanoseconds": 300, "impedance_ohms": None}
                            for b in buses for k in ("sda", "scl")]
    return s


def save(name, s):
    json.dump(s, open(f"{OUT}/{name}.json", "w"), indent=1)


save("D1_24i2c", many())
save("2x5", many(n_i2c=2, per_bus=5))
save("2x5_nospi", many(n_i2c=2, per_bus=5, n_spi=0))
save("D3_24i2c_4spi_x3", many(n_spi=4, flash_per_spi=3))
