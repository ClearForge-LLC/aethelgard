#!/usr/bin/env python3
"""AET-WO-4600: write each specification's scenario set under spike/4600/scenarios/<spec>/.

The set is what verify's coverage asks for (verify.rs coverage, L882-946): a
baseline (no override); a tolerated fault on each I2C bus (its pull-up raised
to 3.3 kOhm, as Sentinel-01's pullup-3k3); spi_device_absent for each SPI
device; a droop of the MCU's supply below its band with a BOR-3 reset (as
Sentinel-01's droop-bor3); serial_rx_open for each port. No fault kind
reaches a motor output, a rail other than the MCU's supply, or the battery
(chaos/src/record.rs L173-191). Run from the repository root:
python3 -I spike/4600/sh/scenarios.py
"""
import json
import pathlib

D = pathlib.Path("spike/4600")
SPECS = ["logger", "driver", "logger-i2c-only", "logger-spi-only", "logger-offboard", "driver-tree"]


def write(spec, name, expect, findings, overrides, version=3, counts=None):
    s = {"schema_version": version, "name": name, "base": f"../../{spec}.json",
         "expect": expect, "expect_findings": findings}
    if counts is not None:
        s["expect_counts"] = counts
    s["overrides"] = overrides
    out = D / "scenarios" / spec
    out.mkdir(parents=True, exist_ok=True)
    (out / f"{name}.json").write_text(json.dumps(s, indent=2) + "\n")
    return f"spike/4600/scenarios/{spec}/{name}.json"


# The droop and its expected counts are Sentinel-01-bench's, read from its scenario, not restated.
S01_DROOP = json.loads(pathlib.Path("scenarios/sentinel-01-bench/droop-bor3.json").read_text())
DROOP = S01_DROOP["overrides"][0]
lists = {}
for name in SPECS:
    spec = json.loads((D / f"{name}.json").read_text())
    files = [write(name, "baseline", "pass", [], [])]
    for b in spec["i2c_buses"]:
        files.append(write(name, f"pullup-3k3-{b['id'].lower()}", "pass", [],
                           [{"bus": b["id"], "pull_up_ohms": 3300}]))
    for d in spec["spi_devices"]:
        files.append(write(name, f"spi-absent-{d['reference'].lower()}", "fail",
                           ["spi_identity_mismatch", "spi_transaction_unwitnessed"],
                           [{"spi_device_absent": d["reference"]}]))
    files.append(write(name, "droop-bor3", "pass", [], [DROOP],
                       counts=S01_DROOP["expect_counts"]))
    for p in spec["serial_ports"]:
        files.append(write(name, f"serial-rx-open-{p['label'].lower()}", "fail",
                           ["serial_echo_missing"], [{"serial_rx_open": p["label"]}]))
    lists[name] = files
(D / "scenarios" / "lists.json").write_text(json.dumps(lists, indent=2) + "\n")
