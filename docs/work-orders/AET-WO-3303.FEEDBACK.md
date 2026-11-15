# AET-WO-3303 FEEDBACK — package preconditions, the harness split, the pair judge's allowlist, a strict number reader, `main` kept green

**Status: in progress** on branch `wo/aet-3303-preconditions`, from `main` at `4246200`.

The predictions in §1 were committed before any container run, any measured run of the CLI and any Kani run. They are not edited afterwards; corrections go in later sections.

Labels:
- **[read]**: read at source, this repository at `4246200`.
- **[measured]**: run. On the host for tier-1 behaviour and refusals that come before any tool; in the pinned tools image (`container/IMAGE`) under rootless podman inside a capped user scope (`~/bin/capped-run`, caps read back) for Kani, KiCad and Renode.
- **[inferred]**: neither; reasoning from the two.

## 0. Source reads that shape the plan

- **A6 [read].** `cli/src/verify.rs:489-517`: `harness_plan` drops `proofs::every_valid_sequence` on a board with motor outputs and `skipped_finding` reports it at `info`; `report.ok` (`:726-728`) counts only `error` findings and gate verdicts, and `package` (`:784`) writes when `report.ok`.
- **The harness list [read].** `verify::harnesses` reads `src/proofs.rs` line by line: a `fn` after a `#[kani::proof]` line. A harness whose attribute is not on a line of its own, or whose `fn` line is not recognised, is not in the plan and not proved.
- **Coverage [read].** The campaign's override kinds (`chaos/src/record.rs:37-50`): `PullUpOhms` and `LineHeldLow` (an I2C bus), `MaxCapacitance` (a constrained net), `Droop` (a domain), `SpiDeviceAbsent` (an SPI device). `cli/src/campaign.rs:271` refuses a droop on any domain but the MCU's supply (`DroopNotMcuSupply`). No kind names an SPI bus, a USART or a rail other than the MCU's supply.
- **The no-fault case [read].** `scenarios/sentinel-01/pullup-2k2.json` sets `SENSORS` to 2200 Ω, the reference's declared value: every package test today (`fab_tier1`, `against_fab_tier2`, `verify_2k2_tier2`) packages a campaign with no fault in it. `verify_47k_tier2` packages one fault, on `SENSORS` only.
- **A9 [read].** `cli/src/main.rs:786-809` runs `route::read`, `connectivity`, `check` and `pairs` on the strict tree with no copper allowlist; `extract` has one.
- **A11 [read].** `eda/src/sexpr.rs:101`: `Sx::arg` returns an atom or a quoted string alike, so a quoted `"0.2"` reaches `nm`. `nm` (`:378-398`) strips one `-` and hands the rest to `i64::parse`, which takes a sign of its own: `--5` and `+0.2` read. `extract.rs:600` and `route.rs:3426,3672` recognise copper by `ends_with(".Cu")`.
- **A15.1 [read].** `pincheck/src/elf.rs:160`: `position(|&b| b == 0)` over the whole tail of the string table, once per symbol.

## 1. Predictions (committed before any run)

- **P1, the split (§1.2).** On `motors-pwm.json` and `motors-dshot600.json`, one harness per peripheral and per timer (I2C1, SPI1, TIM1 and TIM2 or as solved), each with every order of its own actions and a per-write frame assertion, finishes in under 5 minutes in the image on Furnace, and the sum of the parts is under 10 minutes. [inferred: the targeted timer harnesses verify in seconds (`4300a` §4); the whole reference schedule takes 121 s with I2C1 and SPI1 together]
- **P2, the split keeps the claim.** A mutant that makes one unit's start write a word or bit another unit owns (an I2C start writing a TIM1 register; a timer start writing a chip select's BSRR bit) fails its part on the frame assertion, though the whole-schedule oracle of the other unit alone would not see it. [inferred]
- **P3, the reference covers.** The reference with `pullup-47k`, `spi-absent-u4` and `droop-tolerated` has a fault on `SENSORS`, `FLASH` and `VDD_3V3`, and every precondition is met. [read]
- **P4, coverage cannot be met everywhere (§7).** `connectors.json` declares `VDD_5V` (not the MCU's supply) and the SPI bus `EXT` with no device: no override kind can fault either. A droop on `VDD_5V` is refused `DroopNotMcuSupply`, and `spi_device_absent` cannot name `EXT`. Sentinel-02's power fixture (`ir/tests/fixtures/sentinel-02-power.json`) declares `VDD_5V` and `VDD_1V8` beside the MCU's `VDD_3V3`. This meets the second flag-and-stop condition: D1. [read]
- **P5, campaign cost.** Each added scenario costs one Renode boot of the reference, about 15–40 s on a CI runner, so a packaged verify with three faults instead of one costs about one more minute. [inferred]
- **P6, numbers.** Every numeric atom KiCad 10.0.7 writes in the committed boards, the strict corpus and the vendored footprints matches `-?[0-9]+(\.[0-9]{1,6})?`; none uses an exponent, a leading `.` or `+`. [inferred]
- **P7, the version.** The `(version …)` KiCad 10.0.7 writes into a board is `20260206`, the emitter's `pcb::VERSION`. [inferred: the emitted boards are KiCad-loaded byte-stable today]
- **P8, the arc stub.** `check drc --spec` on `pair-standin.json`'s board with one `(arc …)` on `/SPI_SCK` is refused `UnmodelledCopper` once extraction's allowlist runs first, and the pair report of the unmodified board is unchanged. [inferred]
- **P9, the ELF bound.** With the compare bounded to `SYMBOL.len() + 1` bytes, a 16 MiB ELF of about 400,000 symbols pointing at one unterminated 8 MiB name is refused or read in under a second, against minutes before. [inferred]

## 2. Decisions needed

- **D1 (§7, campaign coverage needs fault kinds the chaos crate does not have).** Open. See P4; the list is completed in §3 with measurements.
