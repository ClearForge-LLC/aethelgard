"""Append scratch parts rows to a parts.json (AET-WO-4201 dry run; never committed).
usage: rows.py <parts.json> <PART>...   PART in BMP390 DPS368 SPA06003 BME680 SHT45
Values: each cited to the datasheet held under ~/.cache/aet-4201/ds by sha256."""
import json, sys
AP = "https://github.com/ArduPilot/ardupilot/blob/fb773567896014da007bca58358704d4a0d8516f/libraries/AP_Baro/"
def c(doc, rev, sec, sha=None, pending=None):
    return {"document": doc, "revision": rev, "section": sec, "sha256": sha, "pending": pending}
def row(part, lifecycle_url, addrs, idw, idv, vdd, vddio, doc, rev, sha, sec, ap=None, mask=None, stand_in=False, dsv=True):
    cite = c(doc, rev, sec, sha)
    return {
        "part": part, "lifecycle": "active", "stand_in": stand_in,
        "bus": {"i2c": {"addresses": addrs}},
        "identity": {"write": idw, "read": 1, "value": idv, "mask": mask, "datasheet_value": dsv},
        "supply_millivolts": {"min": vdd[0], "max": vdd[1]},
        "io_millivolts": None if vddio is None else {"min": vddio[0], "max": vddio[1]},
        "pin_capacitance": {"max_femtofarads": None, "typical_femtofarads": None},
        "spi_output_delay_max_ns": None, "renode": None, "script": None, "reads": None,
        "citations": {"bus": cite, "identity": cite, "supply": cite, "io": None if vddio is None else cite,
                      "pin_capacitance": cite, "spi_output_delay": None,
                      "lifecycle": c(part + " product page", "not held (dry run)", "no status word read", pending=lifecycle_url),
                      "configure": None, "start_up": None, "data": None, "reads": None},
        "ardupilot": None if ap is None else {"kind": "baro", "driver": ap[0], "spidev": None, "id": idv,
                      "at": {"document": "ArduPilot Copter-4.7.2, " + ap[1], "revision": "commit fb773567896014da007bca58358704d4a0d8516f",
                             "section": ap[2], "sha256": None, "pending": AP + ap[1].split("/")[-1]}, "frame": None},
        "axes": None,
    }
R = {
 "BMP390": row("BMP390", "https://www.bosch-sensortec.com/products/environmental-sensors/pressure-sensors/bmp390/",
   ["0x76", "0x77"], ["0x00"], "0x60", (1650, 3600), (1200, 3600), "Bosch Sensortec BST-BMP390-DS002-07", "1.7 (2021-03)",
   "8dcf0f74449cb1627e865c7f24d328b76c204d8b9621091ede8c6869fb393e48", "p. 3 (supply), 4.3.1 and p. 30 (CHIP_ID 0x00 = 0x60)",
   ("BMP388", "libraries/AP_Baro/AP_Baro_BMP388.cpp", "line 29: BMP390_ID 0x60")),
 "DPS368": row("DPS368", "https://www.infineon.com/cms/en/product/sensor/pressure-sensors/pressure-sensors-for-iot/dps368/",
   ["0x76", "0x77"], ["0x0D"], "0x10", (1700, 3600), (1200, 3600), "Infineon DPS368 datasheet IFX-duu1493988991355", "v1.1 (2019-07-03)",
   "16d3f60378b9554c2ae6406f709a03624977fd29afc005c9a423e0dee36c281a", "p. 37 (ID 0x0D reset 0x10)",
   ("DPS310", "libraries/AP_Baro/AP_Baro_DPS280.cpp", "line 41: DPS280_WHOAMI 0x10")),
 "SPA06003": row("SPA06003", "https://www.goertek.com/", ["0x76", "0x77"], ["0x0D"], "0x11", (1700, 3600), (1080, 3600),
   "Goertek SPA06-003 datasheet (no document number)", "Ver 2.0 (2023-12-05)",
   "aee86bb46a424bc88263b2e7a993c32ca8adb177b8dffb292febcf11b3ff5f6c", "p. 32 (ID 0x0D = 0x11)",
   ("SPL06", "libraries/AP_Baro/AP_Baro_SPL06.cpp", "line 25: SPA06_CHIP_ID 0x11")),
 "BME680": row("BME680", "https://www.bosch-sensortec.com/products/environmental-sensors/gas-sensors/bme680/",
   ["0x76", "0x77"], ["0xD0"], "0x61", (1710, 3600), (1200, 3600), "Bosch Sensortec BST-BME680-DS001-09", "1.9 (2024-02)",
   "8e42b591a9cabd84ae3377f2ea11e794120e4140dc4f0e4f29a30be3d1f6c9f6", "p. 28 (chip_id 0xD0 = 0x61)"),
 # SHT45: no identity register; the serial-number read (0x89) has no fixed value (datasheet 4.7).
 "SHT45": row("SHT45", "https://sensirion.com/products/catalog/SHT45", ["0x44"], ["0x89"], "0x00", (1080, 3600), None,
   "Sensirion Datasheet SHT4x", "7.3 (2026-06)", "8db4a43f17149b76811cfb504caaeca4ef844ddc710cb9b45905c51c7ddfe3c2",
   "4.7 p. 13 (serial number, no fixed value)", dsv=False),
}
p = json.load(open(sys.argv[1]))
for k in sys.argv[2:]:
    p["parts"].append(R[k])
json.dump(p, open(sys.argv[1], "w"), indent=2)
