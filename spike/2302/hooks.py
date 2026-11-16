"""AET-WO-2302 spike: compose watchpoint hooks (AET2302_EXTRA) for a solved topology.

Each hook logs the core's executed instruction count (10 ns each at 100 MIPS), so a phase's
virtual time is exact to the instruction, not the 100 us quantum a CPU-caused log line carries.
Usage: python3 -I hooks.py <solved.json> [record_words]
"""
import json, sys

TIM = {"TIM1": 0x40010000, "TIM2": 0x40000000, "TIM3": 0x40000400, "TIM4": 0x40000800,
       "TIM5": 0x40000C00, "TIM8": 0x40010400, "TIM15": 0x40014000, "TIM16": 0x40014400,
       "TIM17": 0x40014800, "TIM12": 0x40001800, "TIM13": 0x40001C00, "TIM14": 0x40002000}
UART = {"USART1": 0x40011000, "USART2": 0x40004400, "USART3": 0x40004800, "UART4": 0x40004C00,
        "UART5": 0x40005000, "USART6": 0x40011400, "UART7": 0x40007800, "UART8": 0x40007C00}

def hook(addr, width, access, tag):
    py = '"self.Log(LogLevel.Warning, \\"%s %%x %%x %%d\\" %% (address, value, cpu.ExecutedInstructions))"' % tag
    return f"sysbus AddWatchpointHook {addr:#010x} {width} {access} {py}"

def main():
    solved = json.load(open(sys.argv[1]))
    words = int(sys.argv[2]) if len(sys.argv) > 2 else 64
    periph = {a["peripheral"] for a in solved["assignments"].values()}
    c = [hook(0x20000000 + 4 * k, "DoubleWord", "Write", "AETW") for k in range(words)]
    for p in sorted(periph & TIM.keys()):
        c.append(hook(TIM[p], "DoubleWord", "Write", "AETT_" + p))
    for p in sorted(periph & UART.keys()):
        c.append(hook(UART[p] + 0x28, "DoubleWord", "Write", "AETX_" + p))
        c.append(hook(UART[p] + 0x24, "DoubleWord", "Read", "AETR_" + p))
    print(";;".join(c))

main()
