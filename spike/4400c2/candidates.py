"""AET-WO-4400c2: candidate platforms, written as the emitter would write them (lines in the
platform file, no overlay), from the emitted sentinel-02-own.repl; and the injected MAVLink frame.
usage: python3 -I candidates.py EMITTED.repl OUTDIR
Request IDs: RM0433 Rev 9, Table 124 (p. 697): spi1_rx_dma 37, spi2_rx_dma 39, i2c1_rx_dma 33,
usart1_rx_dma 41. Line names: renode-infrastructure cd4b002a, STM32H7_SPI.cs L85 (DMARecieve),
STM32F7_USART.cs L87 (ReceiveDmaRequest), STM32F7_I2C.cs L111 (DmaReceive)."""
import os, struct, sys

emitted, out = sys.argv[1], sys.argv[2]
os.makedirs(out, exist_ok=True)
text = open(emitted).read()
spi = text.replace("    IRQ -> nvic@35\n", "    IRQ -> nvic@35\n    DMARecieve -> dmamux1@37\n", 1)
spi = spi.replace("    IRQ -> nvic@36\n", "    IRQ -> nvic@36\n    DMARecieve -> dmamux1@39\n", 1)
assert spi.count("DMARecieve") == 2
usart = spi + "\n// USART1's receive DMA request, RM0433 Table 124 usart1_rx_dma 41.\nusart1:\n    ReceiveDmaRequest -> dmamux1@41\n"
i2c = usart + "\n// I2C1's receive DMA request, RM0433 Table 124 i2c1_rx_dma 33.\ni2c1:\n    DmaReceive -> dmamux1@33\n"
usart_only = text + "\n// USART1's receive DMA request, RM0433 Table 124 usart1_rx_dma 41.\nusart1:\n    ReceiveDmaRequest -> dmamux1@41\n"
for name, t in [("p0-emitted", text), ("p1-spi", spi), ("p2-spi-usart", usart), ("p3-spi-usart-i2c", i2c), ("p4-usart", usart_only)]:
    open(os.path.join(out, name + ".repl"), "w").write(t)


def x25(data, crc=0xFFFF):
    for b in data:
        tmp = b ^ (crc & 0xFF)
        tmp = (tmp ^ (tmp << 4)) & 0xFF
        crc = ((crc >> 8) ^ (tmp << 8) ^ (tmp << 3) ^ (tmp >> 4)) & 0xFFFF
    return crc

# MAVLink 2 COMMAND_LONG (msgid 76, CRC_EXTRA 152): MAV_CMD_REQUEST_MESSAGE (512), param1 148
# (AUTOPILOT_VERSION), to system 1, component 1, from GCS 255/190. Payload order by field size.
payload = struct.pack("<7fHBBB", 148.0, 0, 0, 0, 0, 0, 0, 512, 1, 1, 0).rstrip(b"\0")
hdr = bytes([len(payload), 0, 0, 0, 255, 190]) + (76).to_bytes(3, "little")
crc = x25(hdr + payload + bytes([152]))
frame = b"\xfd" + hdr + payload + crc.to_bytes(2, "little")
open(os.path.join(out, "command_long.hex"), "w").write(frame.hex())
print(frame.hex())
