#!/usr/bin/env bash
# ci/foreign-judge.sh: AET-WO-4400e §1.2 and §2, inside the tools image with
# --network none. The foreign judge (`simulate --foreign`) on unmodified
# ArduPilot built for the generated sentinel-02-own board, then each of §2's
# mutations that needs the real firmware, refused by name.
#
#   ci/foreign-judge.sh BOARDS
#
# BOARDS holds sentinel-02/arducopter.elf (the generated job's),
# sentinel-02-noimu/arducopter.elf (the same hwdef without its IMU line) and
# four-port/arducopter.elf (the generated job's four-port board, AET-WO-2311) and
# sentinel-03/arducopter.elf (the generated job's, with its defaults.parm; AET-WO-4412).
# The CLI and the analog bridge are built in target/ (CARGO_TARGET_DIR).
# Each run prints one `ANSWER 4400e:` line with its exit code and verdict.
# AET-WO-4400g: the specification declares ArduPilot's clock
# (`mcu.foreign_clock`), the window is 5 s, and the clock's refusals and
# cross-check each run on the real firmware.
set -euo pipefail
boards=$1
bin=target/debug/aethelgard
spec=crates/aethelgard-cli/tests/fixtures/sentinel-02-own.json
elf=$boards/sentinel-02/arducopter.elf
noimu=$boards/sentinel-02-noimu/arducopter.elf
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
"$bin" generate --spec "$spec" --out "$work/gen" > "$work/generate.json"
# AET-WO-4412 D1 (a): a declared foreign clock adds the foreign platform beside the generated one.
platform=$work/gen/board/sentinel-02-own.foreign.repl

# AET-WO-4400g: the board with its declaration removed, which an undeclared
# foreign run takes; and with its timers declared at the stock file's 250 MHz,
# which validates (a firmware's own clock) and is not ArduPilot's tree. Each
# with its own emitted platforms, since the declaration builds the foreign one.
derive() {
  python3 -I - "$spec" "$work/spec-$1.json" "$1" <<'PY'
import json, sys
src, dst, how = sys.argv[1:]
d = json.load(open(src))
if how == "undeclared":
    del d["mcu"]["foreign_clock"]
else:
    for bus in ("apb1_timers_hz", "apb2_timers_hz"):
        d["mcu"]["foreign_clock"][bus]["value"] = 250000000
json.dump(d, open(dst, "w"), indent=2)
PY
  "$bin" generate --spec "$work/spec-$1.json" --out "$work/gen-$1" > "$work/generate-$1.json"
}
derive undeclared
derive contradicted

# run NAME WANT_EXIT CHECK ARGS...: one simulate, its exit code and the
# verdict held to CHECK, a Python expression over `v` (the verdict) and
# `kinds` (its failing findings' kinds), with `devices` (boot.devices).
run() {
  local name=$1 want=$2 check=$3
  shift 3
  local status=0
  "$bin" simulate --spec "${run_spec:-$spec}" "$@" > "$work/$name.json" || status=$?
  python3 -I - "$work/$name.json" "$name" "$want" "$status" "$check" <<'PY'
import json, sys
path, name, want, status, check = sys.argv[1:]
v = json.load(open(path))
kinds = [f["kind"] for f in v.get("findings", []) if f["severity"] != "info"]
boot = v.get("boot") or {}
devices = boot.get("devices", [])
witnessed = {(d["reference"], d["step"]): d["observed_ns"] for d in devices}
print(f"ANSWER 4400e: {name} exit={status} verdict={v.get('verdict')} "
      f"error={(v.get('error') or {}).get('kind')} kinds={kinds} witnessed={witnessed} "
      f"memory={v.get('memory')} log_kept={boot.get('log_kept_bytes')}")
if status != want:
    sys.exit(f"{name}: exit {status}, not {want}")
if not eval("(" + check + ")"):
    sys.exit(f"{name}: {check} does not hold")
PY
}

# The foreign window, FOREIGN_MAX_VIRTUAL_NS (AET-WO-4400g §1.4).
window=5000ms

# The positive case: pass, the foreign judge, booted, and each modelled
# device's identity and a data read witnessed in the run; the timers'
# declared 200 MHz held by the RCC registers read back, and the USARTs'
# clock stated as declared and unchecked (AET-WO-4400g).
run positive 0 '
v["verdict"] == "pass" and boot["judge"] == "foreign" and boot["booted"]
and all(witnessed.get((r, "identity")) for r in ("U4", "U5", "U7"))
and witnessed.get(("U5", "data (reads[0])"))
and witnessed.get(("U7", "data (script)"))
and boot["log_lines_dropped"] == 0 and boot["log_kept_bytes"] < boot["log_budget_bytes"]
and boot["panic"]["entered_ns"] == [] and boot["panic"]["address"].startswith("0x")
and [(c["clock"], c["declared_hz"], c["decoded_hz"], c["holds"]) for c in boot["clock"]["checked"]]
    == [("apb1_timers_hz", 200000000, 200000000, True), ("apb2_timers_hz", 200000000, 200000000, True)]
and len(boot["clock"]["declared_only"]) == 8
and any(c.startswith("usart_clock_declared_unchecked:") for c in boot["clock"]["conditions"])
and v["tool"].get("build")
' --platform "$platform" --firmware "$elf" --foreign "$window"

# AET-WO-4400j D1 (ruled 2026-11-25T18:49Z): Renode's stdin is a pipe the CLI
# holds open, so its console reader waits, and queues no end of `/dev/null`
# (about 48 MiB of live heap per wall second before: spike/4400j/RESULTS.md
# §2.3). The same run at 1 s and at `positive`'s 5 s stays within 160 MiB.
# Measured on Furnace, 4 cores, this ELF: held, 17–113 MB from 1 s to 5 s;
# main's /dev/null, 238–578 MB.
run one-second 1 'v["memory"]["peak_rss_kib"] > 0' \
  --platform "$platform" --firmware "$elf" --foreign 1000ms
python3 -I - "$work/one-second.json" "$work/positive.json" <<'PY'
import json, sys
one, five = (json.load(open(p))["memory"]["peak_rss_kib"] for p in sys.argv[1:3])
print(f"ANSWER 4400j: held stdin, peak at 1 s {one} KiB, at 5 s {five} KiB, grew {five - one} KiB")
if five - one >= 160 * 1024:
    sys.exit(f"Renode's peak grew {five - one} KiB from 1 s to 5 s, past the 160 MiB band")
PY

# The window too short for a declared witness: U5's identity (4.343 s at the
# declared clock) and its first FIFO read (4.394 s) are past it.
run window-before-u5 1 '
"spi_transaction_unwitnessed" in kinds and not witnessed.get(("U5", "identity"))
' --platform "$platform" --firmware "$elf" --foreign 4300ms

# Between U5's identity and its first FIFO read: the data witness alone.
run window-before-u5-data 1 '
"spi_transaction_unwitnessed" in kinds and witnessed.get(("U5", "identity"))
and not witnessed.get(("U5", "data (reads[0])"))
' --platform "$platform" --firmware "$elf" --foreign 4370ms

# ArduPilot as the generated firmware, on the board with no declared clock:
# today's judge refuses its D2 SRAM.
run_spec=$work/spec-undeclared.json run undeclared 3 'v["error"]["kind"] == "FirmwareOutsideMemory"' \
  --platform "$work/gen-undeclared/board/sentinel-02-own.repl" --firmware "$elf" --for 100ms

# AET-WO-4400g: --foreign with no declared clock is refused by name before a
# boot. AET-WO-4412 D1 (a): a generated run on a specification that declares
# one drops it, takes the generated platform and meets the undeclared run's
# refusal; the foreign platform is not the one it emits.
run_spec=$work/spec-undeclared.json run clock-undeclared 3 'v["error"]["kind"] == "ForeignClockUndeclared"' \
  --platform "$work/gen-undeclared/board/sentinel-02-own.repl" --firmware "$elf" --foreign "$window"
run clock-on-generated-run 3 'v["error"]["kind"] == "FirmwareOutsideMemory"' \
  --platform "$work/gen/board/sentinel-02-own.repl" --firmware "$elf" --for 100ms
run clock-on-generated-run-foreign-platform 3 'v["error"]["kind"] == "PlatformNotEmitted"' \
  --platform "$platform" --firmware "$elf" --for 100ms

# Timers declared at 250 MHz: not ArduPilot's tree for no crystal (200 MHz,
# aethelgard-ir data/ardupilot-h7-clock.tsv), and the ELF names AP_HAL::panic,
# so the run is refused by name before a boot (AET-WO-4412 D1 (b), ruled
# 2026-11-26T11:26Z). The judge's read-back check, which this case reached
# before (AET-WO-4400g §1.2), is held by boot_tier1's
# a_foreign_clock_the_rcc_registers_contradict_fails_by_name.
run_spec=$work/spec-contradicted.json run clock-not-ardupilots 3 '
v["error"]["kind"] == "ForeignClockNotArduPilots"
' --platform "$work/gen-contradicted/board/sentinel-02-own.foreign.repl" --firmware "$elf" --foreign "$window"

# SPI2's receive DMA line dropped from the platform, and U5's model removed:
# neither is the platform the specification emits.
grep -c 'DMARecieve -> dmamux1@39' "$platform" | grep -qx 1
sed '/DMARecieve -> dmamux1@39/d' "$platform" > "$work/no-spi2-rx-dma.repl"
run no-spi2-rx-dma 3 'v["error"]["kind"] == "PlatformNotEmitted"' \
  --platform "$work/no-spi2-rx-dma.repl" --firmware "$elf" --foreign "$window"
grep -c '^u5: Sensors.ICM42688P' "$platform" | grep -qx 1
sed '/^u5: Sensors.ICM42688P/d' "$platform" > "$work/no-u5.repl"
run no-u5-model 3 'v["error"]["kind"] == "PlatformNotEmitted"' \
  --platform "$work/no-u5.repl" --firmware "$elf" --foreign "$window"

# The IMU line removed: ArduPilot never probes the ICM.
run no-imu-line 1 '
"spi_transaction_unwitnessed" in kinds and not witnessed.get(("U5", "identity"))
and witnessed.get(("U7", "identity")) and witnessed.get(("U4", "identity"))
' --platform "$platform" --firmware "$noimu" --foreign "$window"

# AET-WO-2311 (ruled at §1.2): each declared serial port's USART_CR2 options,
# from its access log, on the four-port board (SERIAL0-3: console, GPS, CRSF
# as RC input, GPS2) built with unmodified ArduPilot by the generated matrix.
# four-port.json declares no mcu.foreign_clock; it takes sentinel-02-own's,
# ArduPilot's declared clock on the same MCU, as the other variants here are
# derived. `inverted` declares SERIAL1 (GPS) inverted as well (F2: the
# fixture has no inverted port).
four=$boards/four-port/arducopter.elf
derive_four() {
  python3 -I - crates/aethelgard-cli/tests/fixtures/four-port.json "$spec" "$work/spec-four-$1.json" "$1" <<'PY'
import json, sys
src, s02, dst, how = sys.argv[1:]
d = json.load(open(src))
assert "foreign_clock" not in d["mcu"]
d["mcu"]["foreign_clock"] = json.load(open(s02))["mcu"]["foreign_clock"]
if how == "inverted":
    assert d["serial_ports"][1]["label"] == "GPS" and not d["serial_ports"][1]["framing"]["inverted"]
    d["serial_ports"][1]["framing"]["inverted"] = True
json.dump(d, open(dst, "w"), indent=2)
PY
  "$bin" generate --spec "$work/spec-four-$1.json" --out "$work/gen-four-$1" > "$work/generate-four-$1.json"
}
derive_four plain
derive_four inverted

# The mutations, with no rebuild: one AP_SerialManager parameter's default,
# in its var_info row (AP_Param::GroupInfo, AP_Param.h:206-218: name pointer,
# offset, float def_value, u16 flags, u8 idx, u8 type; 16 bytes on ARM32),
# set in a copy of the ELF; exactly one row has the name and FROM. Measured
# to write the same CR2 words as rebuilding the hwdef with that
# DEFAULT_SERIALn_OPTIONS (spike/2311 RESULTS, swap and inversion).
mutate() {
  python3 -I - "$four" "$work/$1.elf" "$2" "$3" "$4" <<'PY'
import struct, sys
src, dst, name, old, new = sys.argv[1:]
data = bytearray(open(src, "rb").read())
assert data[:4] == b"\x7fELF" and data[4] == 1 and data[5] == 1, "not a 32-bit little-endian ELF"
shoff, = struct.unpack_from("<I", data, 0x20)
shentsize, shnum = struct.unpack_from("<HH", data, 0x2E)
secs = [struct.unpack_from("<IIIIIIIIII", data, shoff + i * shentsize) for i in range(shnum)]
symtab, = [s for s in secs if s[1] == 2]
strtab = secs[symtab[6]]
def cstr(off):
    return bytes(data[off:data.index(0, off)])
want = b"_ZN16AP_SerialManager8var_infoE"
(addr, size), = [
    struct.unpack_from("<II", data, symtab[4] + i * 16 + 4)
    for i in range(symtab[5] // 16)
    if cstr(strtab[4] + struct.unpack_from("<I", data, symtab[4] + i * 16)[0]) == want
]
def file_off(a):
    (s,) = [s for s in secs if s[1] == 1 and s[3] <= a < s[3] + s[5]]
    return a - s[3] + s[4]
rows = []
for i in range(0, size, 16):
    at = file_off(addr + i)
    name_p, _, val, _, idx, typ = struct.unpack_from("<IifHBB", data, at)
    if typ == 0 and idx == 255:
        break
    if cstr(file_off(name_p)).decode() == name:
        rows.append((at, val))
(at, val), = rows
assert val == float(old), f"{name} defaults to {val:g}, not {old}"
struct.pack_into("<f", data, at + 8, float(new))
open(dst, "wb").write(data)
print(f"ANSWER 2311: {sys.argv[2]} {name} {val:g} -> {float(new):g}")
PY
}
mutate four-swap 1_OPTIONS 0 8      # SWAP on GPS (USART2), OPTION_SWAP (UARTDriver.h:94)
mutate four-rc-txinv 2_OPTIONS 0 2  # TXINV on CRSF (USART3), RC input (OPTION_TXINV)
mutate four-inverted 1_OPTIONS 0 3  # RXINV|TXINV on GPS: what an inverted GPS port compiles

four_platform=$(ls "$work"/gen-four-plain/board/*.foreign.repl)
inv_platform=$(ls "$work"/gen-four-inverted/board/*.foreign.repl)

# Unmodified ArduPilot passes: every declared port's last CR2 word with UE
# clear judged, the RC port's RXINV unjudged, and the eight serial lines'
# pulls info, each named.
run_spec=$work/spec-four-plain.json run four-port 0 '
v["verdict"] == "pass" and boot["judge"] == "foreign" and boot["booted"]
and [o["label"] for o in boot["serial_options"]] == ["MAVLINK", "GPS", "CRSF", "GPS2"]
and all(o["judged"] is not None and o["writes"] >= 1 for o in boot["serial_options"])
and [e["expected"] for e in boot["serial_options"][2]["expected"]] == ["clear", "clear", "unjudged"]
and len(boot["serial_pulls"]) == 8
and sum(f["kind"] == "serial_pull" for f in v["findings"]) == 8
and boot["log_lines_dropped"] == 0
' --platform "$four_platform" --firmware "$four" --foreign "$window"

# SWAP on the GPS port, and TXINV on the RC port: each refused by name.
run_spec=$work/spec-four-plain.json run four-swap 1 '
kinds == ["serial_swap_set"]
and [f["message"] for f in v["findings"] if f["kind"] == "serial_swap_set"][0].startswith("GPS (USART2, gps): ")
' --platform "$four_platform" --firmware "$work/four-swap.elf" --foreign "$window"
run_spec=$work/spec-four-plain.json run four-rc-txinv 1 '
kinds == ["serial_txinv_wrong"]
and [f["message"] for f in v["findings"] if f["kind"] == "serial_txinv_wrong"][0].startswith("CRSF (USART3, rc_crsf): ")
' --platform "$four_platform" --firmware "$work/four-rc-txinv.elf" --foreign "$window"

# GPS declared inverted: the unmodified ELF leaves both lines clear and is
# refused by name; the ELF with both inverted passes.
run_spec=$work/spec-four-inverted.json run four-inverted-declared 1 '
kinds == ["serial_txinv_wrong", "serial_rxinv_wrong"]
' --platform "$inv_platform" --firmware "$four" --foreign "$window"
run_spec=$work/spec-four-inverted.json run four-inverted 0 '
v["verdict"] == "pass"
and [p["pupdr"] for p in boot["serial_pulls"] if p["label"] == "GPS"] == ["pull-down", "pull-down"]
' --platform "$inv_platform" --firmware "$work/four-inverted.elf" --foreign "$window"

# AET-WO-1609 (ruled at §1.2, spike/1609): ArduPilot's own AP_HAL::panic,
# planted straight after U5's last witness (its first FIFO data read: the
# device transfer in AP_InertialSensor_Invensensev3::read_fifo, before
# accumulate_samples), so it follows the witness by causality. Before 1609
# this run passed (exit 0); it fails by name, every witness still present.
# And the same ELF with the panic's name altered in its symbol table is
# refused before any boot: no run is judged without the hook.
plant() {
  python3 -I - "$elf" "$work/$1.elf" "$1" <<'PY'
import struct, sys
src, dst, how = sys.argv[1:]
data = bytearray(open(src, "rb").read())
assert data[:4] == b"\x7fELF" and data[4] == 1 and data[5] == 1, "not a 32-bit little-endian ELF"
shoff, = struct.unpack_from("<I", data, 0x20)
shentsize, shnum, shstrndx = struct.unpack_from("<HHH", data, 0x2E)
secs = [struct.unpack_from("<IIIIIIIIII", data, shoff + i * shentsize) for i in range(shnum)]
symtab, = [s for s in secs if s[1] == 2]
strtab = secs[symtab[6]]
def cstr(off):
    return bytes(data[off:data.index(0, off)])
def symbol(name):
    hits = {(v & ~1, size, strtab[4] + n) for n, v, size in (
        struct.unpack_from("<III", data, symtab[4] + i * 16) for i in range(symtab[5] // 16))
        if cstr(strtab[4] + n) == name}
    (hit,) = hits
    return hit
def file_off(a):
    (s,) = [s for s in secs if s[1] == 1 and s[3] <= a < s[3] + s[5]]
    return a - s[3] + s[4]
def bl(at, target):
    off = target - (at + 4)
    s = (off >> 24) & 1
    j1, j2 = (~(((off >> 23) & 1) ^ s)) & 1, (~(((off >> 22) & 1) ^ s)) & 1
    return struct.pack("<HH", 0xF000 | (s << 10) | ((off >> 12) & 0x3FF), 0xD000 | (j1 << 13) | (j2 << 11) | ((off >> 1) & 0x7FF))
def mov32(rd, v):
    def half(op, imm):
        return struct.pack("<HH", op | (((imm >> 11) & 1) << 10) | (imm >> 12), (((imm >> 8) & 7) << 12) | (rd << 8) | (imm & 0xFF))
    return half(0xF240, v & 0xFFFF) + half(0xF2C0, v >> 16)
panic, _, panic_name = symbol(b"_ZN6AP_HAL5panicEPKcz")
if how == "panic-unnamed":
    data[panic_name + len(b"_ZN6AP_HAL5")] = ord("q")  # _ZN6AP_HAL5panic -> _ZN6AP_HAL5qanic
    open(dst, "wb").write(data)
    print("ANSWER 1609: panic-unnamed: the panic's symbol renamed")
    sys.exit()
read_fifo, size, _ = symbol(b"_ZN30AP_InertialSensor_Invensensev39read_fifoEv")
acc, _, _ = symbol(b"_ZN30AP_InertialSensor_Invensensev318accumulate_samplesEPK8FIFODatah")
(call,) = [a for a in range(read_fifo, read_fifo + size, 2) if bytes(data[file_off(a):file_off(a) + 4]) == bl(a, acc)]
# The FIFO data transfer is the last `blx rN` (0x4780 | n << 3) before that call.
blx = max(a for a in range(read_fifo, call, 2)
          if (struct.unpack_from("<H", data, file_off(a))[0] & 0xFF87) == 0x4780)
site = blx + 2
assert call + 4 - site >= 12, f"{call + 4 - site} bytes from the transfer to accumulate_samples"
if how == "udf":
    # AET-WO-2314: `udf #0` at the same site, an undefined instruction that
    # escalates to HardFault; its address goes to udf.site for the check.
    data[file_off(site):file_off(site) + 2] = struct.pack("<H", 0xDE00)
    open(dst, "wb").write(data)
    open(dst[:-len(".elf")] + ".site", "w").write(f"{site:#010x}")
    print(f"ANSWER 2314: udf planted at {site:#010x}, after the transfer at {blx:#010x}")
    sys.exit()
# An empty string for the message: the first zero byte of .text, which also
# holds ArduPilot's read-only data (its link has no .rodata).
names = secs[shstrndx]
text, = [s for s in secs if cstr(names[4] + s[0]) == b".text"]
empty = text[3] + data[text[4]:text[4] + text[5]].index(0)
patch = mov32(0, empty) + bl(site + 8, panic)
data[file_off(site):file_off(site) + 12] = patch
open(dst, "wb").write(data)
print(f"ANSWER 1609: panic planted at {site:#010x}, after the transfer at {blx:#010x}; panic {panic:#010x}")
PY
}
plant panic
plant panic-unnamed
run late-panic 1 '
kinds == ["foreign_panic"] and boot["booted"]
and all(witnessed.get((r, "identity")) for r in ("U4", "U5", "U7"))
and witnessed.get(("U5", "data (reads[0])")) and witnessed.get(("U7", "data (script)"))
and len(boot["panic"]["entered_ns"]) == 1
and boot["panic"]["entered_ns"][0] > witnessed[("U5", "data (reads[0])")]
' --platform "$platform" --firmware "$work/panic.elf" --foreign "$window"
run panic-unnamed 3 'v["error"]["kind"] == "ForeignPanicSymbolMissing"' \
  --platform "$platform" --firmware "$work/panic-unnamed.elf" --foreign "$window"

# AET-WO-2314 (ruled D2, spike/2314): an undefined instruction at the panic's
# site, a HardFault after U5's last witness. ArduPilot's handler polls USART1
# forever, and before 2314 this run was killed at the memory cap
# (memory_cap_exceeded alone). The hook on the handler the ELF's vector table
# names halts the core at its entry: it fails foreign_fault by name, with the
# vector and the planted PC, every witness still present, under the cap.
plant udf
udf_site=$(cat "$work/udf.site")
run late-hardfault 1 '
kinds == ["core_in_exception", "foreign_fault"]
and all(witnessed.get((r, "identity")) for r in ("U4", "U5", "U7"))
and witnessed.get(("U5", "data (reads[0])"))
and [f["message"] for f in v["findings"] if f["kind"] == "foreign_fault"][0].startswith("the core took HardFault (exception 3) at ")
and ", from PC '"$udf_site"' (read from its stack frame at 0x" in [f["message"] for f in v["findings"] if f["kind"] == "foreign_fault"][0]
and v["memory"]["peak_rss_kib"] < v["memory"]["bound_kib"]
' --platform "$platform" --firmware "$work/udf.elf" --foreign "$window"

# AET-WO-4400i (ruled on PR #396): a declared input on sentinel-02-own's CONSOLE (USART1), its frames written
# by pymavlink 2.4.50 in the pinned ArduPilot image (spike/4400i/py/input.py), each filled to 64 bytes (D3:
# declared fill). Inside the 5 s window (D1), ArduPilot echoes each PARAM_SET in its own PARAM_VALUE and
# refuses every ARM with its own "Arm: System not initialised", read from its transmission, never from the
# input; the board starts at rest (D4: U5 by its declared orientation, U7 moved by its noise, each data read
# witnessed at a rest value); the IDLE gap is named.
inputs=crates/aethelgard-cli/tests/fixtures/4400i
grep -c 'ReceiveDmaRequest -> dmamux1@41' "$platform" | grep -qx 1
run input-arm 0 '
v["verdict"] == "pass" and boot["input"]["input"]["frames"] == 41
and [p["echoed"] == p["set"] and p["error"] is None for p in boot["input"]["witness"]["params"]] == [True] * 4
and (400, 4) in [(a["command"], a["result"]) for a in boot["input"]["witness"]["acks"]]
and (400, 0) not in [(a["command"], a["result"]) for a in boot["input"]["witness"]["acks"]]
and boot["input"]["witness"]["armed"] is None and boot["input"]["witness"]["heartbeats"] > 0
and "Arm: System not initialised" in [t["text"] for t in boot["input"]["witness"]["texts"]]
and [(r["reference"], r["property"], r["values"]) for r in boot["input"]["rest"] if r["reference"] == "U5"]
    == [("U5", "AccelerationX", ["0"]), ("U5", "AccelerationY", ["0"]), ("U5", "AccelerationZ", ["1"])]
and (witnessed.get(("U7", "data (rest 0)")) or witnessed.get(("U7", "data (rest 1)")))
and witnessed.get(("U5", "data (reads[0])"))
and any(u.startswith("usart_idle_unmodelled") for u in boot["unobservable"])
and "not_at_rest" not in boot
' --platform "$platform" --firmware "$elf" --foreign "$window" --input "$inputs/arm5.json"

# Without an input (the positive case above), U5 at the parts table's witness stimulus is named not at rest.
python3 -I - "$work/positive.json" <<'PY2'
import json, sys
v = json.load(open(sys.argv[1]))
n = v["boot"].get("not_at_rest")
print(f"ANSWER 4400e: positive-not-at-rest {n}")
assert n == ["U5: the parts table's witness stimulus (X 0, Y 1, Z 0 g) is not at rest by its declared orientation (X 0, Y 0, Z 1 g)"], n
PY2

# A planted force code, and a frame that is not MAVLink: each refused by name before a boot (D8).
run input-force 2 'v["error"]["kind"] == "InputForcesArm"' \
  --platform "$platform" --firmware "$elf" --foreign "$window" --input "$inputs/force.json"
run input-not-mavlink 2 'v["error"]["kind"] == "InputNotMavlink"' \
  --platform "$platform" --firmware "$elf" --foreign "$window" --input "$inputs/junk.json"

# A parameter ArduPilot does not have: reported rejected from its own PARAM_ERROR (D9), and nothing else.
run input-reject 1 '
kinds == ["foreign_param_rejected"]
and [(p["id"], p["error"]) for p in boot["input"]["witness"]["params"] if p["error"] is not None]
    == [("AET_NOT_A_PARAM", 1)]
' --platform "$platform" --firmware "$elf" --foreign "$window" --input "$inputs/reject.json"

# USART1's receive request line dropped from the platform: not the platform the specification emits.
sed '/ReceiveDmaRequest -> dmamux1@41/d' "$platform" > "$work/no-usart1-rx-dma.repl"
run no-usart1-rx-dma 3 'v["error"]["kind"] == "PlatformNotEmitted"' \
  --platform "$work/no-usart1-rx-dma.repl" --firmware "$elf" --foreign "$window" --input "$inputs/arm5.json"

# D6 (R1): the input run archived (trace 20), then re-judged by `replay` from its run directory alone, with
# no input file and no Renode, to the same witness.
mkdir "$work/archive"
run input-archive 0 'v["verdict"] == "pass" and len(boot["input"]["run_id"]) == 32' \
  --platform "$platform" --firmware "$elf" --foreign "$window" --input "$inputs/arm5.json" --archive "$work/archive"
id=$(python3 -I -c 'import json, sys; print(json.load(open(sys.argv[1]))["boot"]["input"]["run_id"])' "$work/input-archive.json")
"$bin" replay --archive "$work/archive/$id" > "$work/input-replay.json"
python3 -I - "$work/input-archive.json" "$work/input-replay.json" <<'PY'
import json, sys
v, r = (json.load(open(p)) for p in sys.argv[1:])
w = v["boot"]["input"]
print(f"ANSWER 4400e: input-replay judge={r['judge']} input_verdict={r['input_verdict']} run_id={r['run_id']} "
      f"events={r['event_count']} same_witness={r['witness'] == w['witness']}")
assert r["judge"] == "foreign" and r["input_verdict"] == "pass" and r["run_id"] == w["run_id"]
assert r["input_sha256"] == w["input"]["sha256"] and r["witness"] == w["witness"]
PY

# AET-WO-4412 (ruled 2026-11-26T10:43Z D1 (a), 14:09Z (b), 15:27Z (ii), 19:38Z): Sentinel-03's three foreign runs,
# on its committed specification (its foreign_clock) and the generated job's ELF (MOT_PWM_TYPE 6 from its
# defaults.parm, so DShot600 from boot), each in a 12 s window. Their inputs are pymavlink 2.4.50's, written in
# the pinned ArduPilot image by `python -I spike/4412/py/input.py NAME OUT.json` on CONSOLE (sha256:
# armread12 1f1f6a3c, dshot12 adcc235b, reboot12 d46aaf76). Each is held by name only, never by a count or a
# stamp: a DShot run's instruction count and end PC are not_rerun_stable (15:27Z (ii)), held here as present with
# their reasons (the unit test names them; boot_tier1's no-binding test keeps their names out of CI). Each prints its wall,
# the runner's CPU and nproc: the worst of the three is the CI wall figure (14:09Z (b)), under a foreign run's hang guard,
# renode::foreign_boot_wall, 729.3 s for a 12 s window (21:47Z (a), from the s03-wall measurement: arm 145.85 s on CI).
s03=examples/sentinel-03.json
"$bin" generate --spec "$s03" --out "$work/gen-s03" > "$work/generate-s03.json"
s03plat=$work/gen-s03/board/sentinel-03.foreign.repl
s03elf=$boards/sentinel-03/arducopter.elf
s03in=crates/aethelgard-cli/tests/fixtures/4412
s03_dshot='
v["verdict"] == "pass" and "foreign_motor_protocol_unmet" not in [f["kind"] for f in v["findings"]]
and [(s["timer"], s["request"]) for s in boot["dshot"]["transfers"]] == [("TIM1", 15)]
and all(p["frames"] > 0 and p["malformed"] == 0 for s in boot["dshot"]["transfers"] for p in s["positions"])
and len(boot["not_rerun_stable"]) == 2 and all(u["why"] for u in boot["not_rerun_stable"])'
s03run() {
  local name=$1 check=$2 t0 t1
  t0=$(date +%s.%N)
  run_spec=$s03 run "$name" 0 "$check" --platform "$s03plat" --firmware "$s03elf" --foreign 12000ms --input "$s03in/$3.json"
  t1=$(date +%s.%N)
  echo "ANSWER 4412: $name wall_s=$(python3 -I -c "print(round($t1 - $t0, 2))") cpu=\"$(grep -m1 'model name' /proc/cpuinfo | cut -d: -f2- | sed 's/^ //')\" nproc=$(nproc)"
}
s03run s03-arm "$s03_dshot"'
and boot["input"]["witness"]["armed"] is not None
and [(r["id"], r["value"]) for r in boot["input"]["witness"]["reads"]] == [("MOT_PWM_TYPE", 6.0)]' armread12
s03run s03-dshot "$s03_dshot"'
and boot["input"]["witness"]["armed"] is not None' dshot12
s03run s03-reboot "$s03_dshot"'
and "foreign_reboot" in [f["kind"] for f in v["findings"]] and len(boot["resets_ns"]) == 1
and boot["input"]["witness"]["armed"] is None' reboot12
