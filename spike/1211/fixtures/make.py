#!/usr/bin/env python3
"""AET-WO-1211 §1.2: write the two fixtures in the proposed schema (FEEDBACK §2). They are never
committed as specifications and no engine reads them: today's `validate` refuses them on the first
unknown key. The base is the committed 4600 driver (crates/aethelgard-cli/tests/fixtures/
driver-judges-nothing.json): its MCU, board, CMD port and debug connector are kept unchanged, and
its acknowledgements are dropped. A board's acknowledgements are learned from `verify`'s ledger, and
no ledger exists for a schema nothing builds.

Citations (held under ~/.cache/aet-1211/ds, sha256 in SHA256SUMS):
  SLVSDS7B = TI DRV8876 datasheet, Aug 2019 rev Nov 2019 (f96a70b1...)
  SLVSAR1E = TI DRV8833 datasheet, Jan 2011 rev Jul 2015 (3e400e33...)
  DS12117  = ST STM32H753xI datasheet Rev 10 (607b9ead...)
  RM0433   = ST reference manual Rev 9 (672a9433...)
Usage: make.py  (writes d1-drv8876.json and d2-drv8833.json beside itself)
"""
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
BASE = HERE.parents[2] / "crates/aethelgard-cli/tests/fixtures/driver-judges-nothing.json"
# Not a number on purpose: the version is the next free one after 4240b's (FEEDBACK §5).
VERSION = "next-after-4240b"


def pwm(rate_hz, lo, hi, idle, resolution_ps):
    return {"rate_hz": rate_hz, "pulse_min_us": lo, "pulse_max_us": hi, "idle": idle,
            "resolution_ps": resolution_ps}


def output(label, pin, timing):
    return {"label": label, "pin": pin, "protocol": "pwm", "pwm": timing, "esc": None}


# 20 kHz at the 64 MHz timer kernel clock (motor.rs SolvedClock::reset; RM0433 Table 57):
# one kernel tick, 1/64 MHz = 15,625 ps, gives PSC 0 and ARR 3199 (3,200 steps).
TICK_64MHZ_PS = 15_625
SERVO = pwm(50, 1000, 2000, {"pulse_us": 1500}, 1_000_000)  # centre, not the end stop (4600 F2)


def mcu_pull_up():
    return {"mcu": "up", "source": "DS12117 Rev 10 Table 49 p. 137 (rev Y) = Table 146 p. 241 (rev V): RPU 30/40/50 kOhm"}


def base():
    d = json.loads(BASE.read_text())
    d["schema_version"] = VERSION
    d["acknowledge_unwitnessed"] = []
    d["voltage_domains"].append({"id": "VMOTOR", "nominal_millivolts": 0, "tolerance_permille": 50})
    return d


def connector_motor(ref, nets):
    pins = [{"pin": str(i + 1), "net": n, "offboard": None} for i, n in enumerate(nets)]
    return {"reference": ref, "part": "SM04B-GHS-TB", "standard": None, "pins": pins}


def d1():
    d = base()
    d["project_name"] = "drv8876-ph-en"
    d["voltage_domains"][-1]["nominal_millivolts"] = 12000  # SLVSDS7B p. 4: VM 4.5 to 37 V
    # J4 carried the four outputs off the board; now the bridge inputs stay on the board.
    d["connectors"] = [c for c in d["connectors"] if c["reference"] != "J4"]
    d["connectors"].append(connector_motor("J4", ["SERVO1", None, None, "GND"]))
    d["connectors"].append(connector_motor("J5", ["M1_OUT1", "M1_OUT2", "VMOTOR", "GND"]))
    d["motor_outputs"] = [
        # PH/EN mode: EN carries the PWM and PH the direction (SLVSDS7B Table 3, p. 11). Both are
        # timer channels of one timer, as the work order writes D1 (§1.2); PH holds 0 % or 100 %.
        # Idle low on both: EN = 0 is brake for any PH (Table 3, row 2).
        output("DRV_EN", None, pwm(20000, 1, 49, {"level": "low"}, TICK_64MHZ_PS)),
        output("DRV_PH", None, pwm(20000, 1, 49, {"level": "low"}, TICK_64MHZ_PS)),
        output("SERVO1", None, SERVO),
    ]
    d["pin_devices"] = [{
        "reference": "U2",
        "part": "DRV8876PWPR",
        "supplies": {"VM": "VMOTOR"},
        "straps": {
            # Table 2 p. 10, Table 3 p. 11: PMODE low = PH/EN, latched when nSLEEP enables the device.
            "PMODE": {"tie": "GND"},
            # Table 6 p. 13: IMODE = GND, fixed off-time chopping, automatic retry, nFAULT on overcurrent only.
            "IMODE": {"tie": "GND"},
            # p. 4: VVREF 0 to 3.6 V. Eq. 3 p. 13: ITRIP = VVREF / (RIPROPI x AIPROPI) = 3.3 / (1500 x 1e-3) = 2.2 A.
            "VREF": {"tie": "VDD_3V3"},
        },
        "signals": [
            # p. 3, p. 5: nSLEEP has a 100 kOhm internal pulldown, so the part sleeps (outputs Hi-Z) from MCU
            # reset until this pin is driven; low is also the declared initial level, the only Hi-Z state in
            # PH/EN mode (Table 3, row 1).
            {"device_pin": "nSLEEP", "net": "DRV_NSLEEP", "pin": None, "direction": "output", "initial": "low"},
            # p. 3: open drain, "Connect an external pullup resistor". p. 4: IOD <= 5 mA. The 10 kOhm value is
            # Figure 18's example (p. 19), which TI marks "not part of the TI component specification": no
            # value is specified, so the specification declares one.
            {"device_pin": "nFAULT", "net": "DRV_NFAULT", "pin": None, "direction": "input", "active": "low",
             "pull": {"resistor": {"reference": "R3", "part": "CRCW0603_F_K", "ohms": 10000, "to": "VDD_3V3"},
                      "source": "SLVSDS7B Figure 18 p. 19, an example value; no recommended value is specified"}},
        ],
        "analog": [
            # Eq. 1-2 p. 12: VIPROPI = (ILS1 + ILS2) x AIPROPI x RIPROPI; AIPROPI 1000 uA/A, typical only (p. 6).
            # At the declared full scale, 2 A x 1000 uA/A x 1500 Ohm = 3.0 V <= VREF+ 3.3 V.
            {"device_pin": "IPROPI", "net": "DRV_IPROPI", "pin": None,
             "transfer": {"current_output": {"resistor": {"reference": "R2", "part": "CRCW0603_F_K", "ohms": 1500}}},
             "full_scale": {"milliamps": 2000}, "adc_resolution_bits": 16},
        ],
        "bridges": [
            {"id": "M1", "mode": "ph_en", "inputs": {"EN/IN1": "DRV_EN", "PH/IN2": "DRV_PH"},
             "outputs": {"OUT1": "M1_OUT1", "OUT2": "M1_OUT2"}},
        ],
    }]
    return d


def d2():
    d = base()
    d["project_name"] = "drv8833-in-in"
    d["voltage_domains"][-1]["nominal_millivolts"] = 6000  # SLVSAR1E p. 5: VM 2.7 to 10.8 V
    d["connectors"] = [c for c in d["connectors"] if c["reference"] != "J4"]
    d["connectors"].append(connector_motor("J4", ["SERVO1", None, None, "GND"]))
    d["connectors"].append(connector_motor("J5", ["MA_OUT1", "MA_OUT2", "VMOTOR", "GND"]))
    d["connectors"].append(connector_motor("J6", ["MB_OUT1", "MB_OUT2", None, "GND"]))
    # Two IN/IN bridges, one timer each (§1.2): pinned by hand, since the solver keys timers by
    # (protocol, rate, resolution) and would otherwise put all four on one. PA8/PA9 = TIM1_CH1/CH2 and
    # PC6/PC7 = TIM3_CH1/CH2: today's `solve` accepts exactly these pins (pins-check.json, FEEDBACK §1).
    # Idle low on both inputs: 00 is coast/fast decay (SLVSAR1E Table 1, p. 9).
    # SLVSAR1E gives no maximum input PWM frequency (NOT FOUND, FEEDBACK §7): 20 kHz is declared, not judged.
    low = {"level": "low"}
    d["motor_outputs"] = [
        output("A_IN1", "PA8", pwm(20000, 1, 49, low, TICK_64MHZ_PS)),
        output("A_IN2", "PA9", pwm(20000, 1, 49, low, TICK_64MHZ_PS)),
        output("B_IN1", "PC6", pwm(20000, 1, 49, low, TICK_64MHZ_PS)),
        output("B_IN2", "PC7", pwm(20000, 1, 49, low, TICK_64MHZ_PS)),
        output("SERVO1", None, SERVO),
    ]
    d["pin_devices"] = [{
        "reference": "U2",
        "part": "DRV8833PWPR",
        "supplies": {"VM": "VMOTOR"},
        # p. 4, p. 10: "If current control is not needed, the xISEN pins should be connected directly to ground."
        "straps": {"AISEN": {"tie": "GND"}, "BISEN": {"tie": "GND"}},
        "signals": [
            # p. 3: internal pulldown (500 kOhm, p. 6), so asleep from reset. VIH min 2.5 V (p. 6) against the
            # MCU's VOH >= VDD - 0.4 V at 8 mA (DS12117 Table 50 p. 139 = Table 147 p. 243): 3.135 - 0.4 = 2.735 V
            # at the domain's 5 % low edge.
            {"device_pin": "nSLEEP", "net": "DRV_NSLEEP", "pin": None, "direction": "output", "initial": "low"},
            # p. 4: open drain; no pull-up value or voltage is given (NOT FOUND). The MCU's own pull-up is declared.
            {"device_pin": "nFAULT", "net": "DRV_NFAULT", "pin": None, "direction": "input", "active": "low",
             "pull": mcu_pull_up()},
        ],
        "analog": [],
        "bridges": [
            {"id": "MA", "mode": "in_in", "inputs": {"AIN1": "A_IN1", "AIN2": "A_IN2"},
             "outputs": {"AOUT1": "MA_OUT1", "AOUT2": "MA_OUT2"}},
            {"id": "MB", "mode": "in_in", "inputs": {"BIN1": "B_IN1", "BIN2": "B_IN2"},
             "outputs": {"BOUT1": "MB_OUT1", "BOUT2": "MB_OUT2"}},
        ],
    }]
    return d


if __name__ == "__main__":
    for name, d in (("d1-drv8876.json", d1()), ("d2-drv8833.json", d2())):
        (HERE / name).write_text(json.dumps(d, indent=2) + "\n")
        print(name, len(json.dumps(d)))
