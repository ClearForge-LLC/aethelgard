#!/usr/bin/env bash
# ci/foreign-judge.sh: AET-WO-4400e §1.2 and §2, inside the tools image with
# --network none. The foreign judge (`simulate --foreign`) on unmodified
# ArduPilot built for the generated sentinel-02-own board, then each of §2's
# mutations that needs the real firmware, refused by name.
#
#   ci/foreign-judge.sh BOARDS
#
# BOARDS holds sentinel-02/arducopter.elf (the generated job's) and
# sentinel-02-noimu/arducopter.elf (the same hwdef without its IMU line).
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
platform=$work/gen/board/sentinel-02-own.repl

# AET-WO-4400g: the board with its declaration removed, which a generated
# run and an undeclared foreign run take; and with its timers declared at the
# stock file's 250 MHz, which ArduPilot's RCC registers contradict. Each with
# its own emitted platform, since the declaration builds the platform.
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
and [(c["clock"], c["declared_hz"], c["decoded_hz"], c["holds"]) for c in boot["clock"]["checked"]]
    == [("apb1_timers_hz", 200000000, 200000000, True), ("apb2_timers_hz", 200000000, 200000000, True)]
and len(boot["clock"]["declared_only"]) == 8
and any(c.startswith("usart_clock_declared_unchecked:") for c in boot["clock"]["conditions"])
and v["tool"].get("build")
' --platform "$platform" --firmware "$elf" --foreign "$window"

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

# AET-WO-4400g: --foreign with no declared clock, and a generated run on a
# specification that declares one, are each refused by name before a boot.
run_spec=$work/spec-undeclared.json run clock-undeclared 3 'v["error"]["kind"] == "ForeignClockUndeclared"' \
  --platform "$work/gen-undeclared/board/sentinel-02-own.repl" --firmware "$elf" --foreign "$window"
run clock-on-generated-run 3 'v["error"]["kind"] == "ForeignClockOnGeneratedRun"' \
  --platform "$platform" --firmware "$elf" --for 100ms

# Timers declared at 250 MHz: the twin runs them there, and ArduPilot's RCC
# registers, read back, decode to 200 MHz on both buses, so the run fails by
# name with both figures.
run_spec=$work/spec-contradicted.json run clock-contradicted 1 '
kinds == ["foreign_clock_contradicted", "foreign_clock_contradicted"]
and [(c["declared_hz"], c["decoded_hz"]) for c in boot["clock"]["checked"]]
    == [(250000000, 200000000), (250000000, 200000000)]
' --platform "$work/gen-contradicted/board/sentinel-02-own.repl" --firmware "$elf" --foreign "$window"

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
