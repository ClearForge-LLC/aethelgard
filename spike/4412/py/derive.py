"""AET-WO-4400j: main's Sentinel-03 (examples/sentinel-03.json at 589030ca, schema 30) made runnable as a foreign board.
One declaration is added and nothing else changes: mcu.foreign_clock, ArduPilot's declared clock tree for this board's
16 MHz HSE (`OSCILLATOR_HZ 16000000`, which `generate hwdef` emits from mcu.hse). Its values are read from the pinned
ArduPilot image (e1809b90...), libraries/AP_HAL_ChibiOS/hwdef/common/stm32h7_mcuconf.h:
  - STM32_HSECLK 16000000U: PLL1/2/3 DIVM 2/2/4 (lines 134-139);
  - the 8/16/32 MHz tree: PLL1 N 100 P 2 (400 MHz sys_ck), PLL2 N 75 Q 6 (lines 184-207);
  - D2PPRE1/2 DIV2 (292-293), TIMPRE (312): the APB timers at 200 MHz, as on the HSI tree (RM0433 Rev 9 Table 57);
  - USART16SEL and USART234578SEL PLL2_Q (343-344): 16 MHz / 2 x 75 / 6 = 100 MHz (72 MHz on the HSI tree);
  - STM32_VOS_SCALE1 (109).
4400h/4400i's derive.py copied sentinel-02-own's HSI values (72 MHz USARTs); on this HSE board they would be wrong.
Usage: python3 -I derive.py SENTINEL_03 OUT"""
import json, sys
s03, out = sys.argv[1:]
d = json.load(open(s03))
assert "foreign_clock" not in d["mcu"]
assert d["mcu"]["hse"]["crystal"]["part"].startswith("ABM8AIG-16.000MHZ")
src = "Copter-4.7.2 stm32h7_mcuconf.h, OSCILLATOR_HZ 16000000"
tim = {"value": 200000000, "source": src + ": PLL1 HSE/2*100/2 = 400 MHz (L134-139, L184-207), HPRE /2, D2PPRE1/2 /2 (L292-293), TIMPRE 1 (L312; RM0433 Rev 9 Table 57)"}
usart = src + ": USART16SEL, USART234578SEL PLL2_Q (L343-344), HSE/2*75/6 = 100 MHz (L134-139, L184-207)"
fc = {
    "apb1_timers_hz": tim,
    "apb2_timers_hz": dict(tim),
    "vos": {"value": "scale1", "source": src + ": STM32_VOS_SCALE1 (L109)"},
    "usarts": {u: {"value": 100000000, "source": usart}
               for u in ["UART4", "UART5", "UART7", "UART8", "USART1", "USART2", "USART3", "USART6"]},
}
assert all(len(v["source"]) <= 200 for v in [fc["apb1_timers_hz"], fc["vos"], fc["usarts"]["USART1"]])
d["mcu"]["foreign_clock"] = fc
imu = [s for s in d["spi_devices"] if s["part"] == "ICM42688P"]
assert len(imu) == 1 and imu[0].get("orientation") and d["board"].get("forward")
json.dump(d, open(out, "w"), indent=2)
