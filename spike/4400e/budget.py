"""AET-WO-4400e §1.1: what the CLI's log filter would count of a run's log (renode.rs LOG_FILTER: 8 MiB budget;
an access line the judge never reads is discarded uncounted: any read but the flash controller's, except a stock
USART's reads at offsets other than ISR (0x1C) and RDR (0x24); a GPIO write other than MODER/AFRL/AFRH;
twin serial.rs is_unjudged_access). Also: lines per SPI2 transaction and SPI1's chip-select windows.
usage: python3 -I budget.py console.log"""
import re, sys
acc = re.compile(r"^\[[0-9:.]+\] \[[A-Z]+\] (\S+): \[cpu: 0x[0-9A-Fa-f]+\] (Read|Write)\S* (?:from|to) 0x([0-9A-Fa-f]+)")
USARTS = {"usart1", "usart2", "usart3", "uart4", "uart5", "usart6", "uart7", "uart8", "lpuart1"}
tot = counted = disc = 0
spi2_lines = 0; spi2_tx = 0; spi1_cs = 0; spi1_err = 0; spi2_err = 0; spi2_cs = 0
for raw in open(sys.argv[1], "rb"):
    n = len(raw); tot += n
    line = raw.decode("utf-8", "replace").rstrip("\n").strip("\r")
    m = acc.match(line)
    d = False
    if m:
        src, rw, off = m.group(1), m.group(2), int(m.group(3), 16)
        if rw == "Read":
            d = src != "flashController" and (src not in USARTS or (off & ~3) in (0x1C, 0x24))
        else:
            d = src.startswith("gpioPort") and (off & ~3) not in (0x00, 0x20, 0x24)
    if d: disc += n
    else: counted += n
    if "] spi2" in line: spi2_lines += 1
    if "spi2.spi2mux.u5: bank" in line: spi2_tx += 1
    if "spi1.spi1mux: GPIO #" in line and "set to False" in line: spi1_cs += 1
    if "spi2.spi2mux: GPIO #" in line and "set to False" in line: spi2_cs += 1
    if "[ERROR] spi1: Attempted to set peripheral into SPI slave mode" in line: spi1_err += 1
    if "[ERROR] spi2: Attempted to set peripheral into SPI slave mode" in line: spi2_err += 1
print(f"total_bytes {tot} counted_bytes {counted} discarded_bytes {disc} budget 8388608")
print(f"spi2 lines {spi2_lines} u5 transactions {spi2_tx} spi2 cs-low {spi2_cs} spi2 errors {spi2_err}; spi1 cs-low {spi1_cs} spi1 errors {spi1_err}")
