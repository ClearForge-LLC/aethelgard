# AET-WO-1609 — the measurement (§1.1) and the stop (§1.2)

Measured 2026-11-23 on `e5984c86` (main), predictions committed first (`170cae45`, `PREDICTIONS.md`).
Code side read at source (two read-only sub-measurements, each spot-checked at the cited lines; one
correction below). Twin side run in the pinned tools image (`container/IMAGE`, `1e0ab105…`, Renode
`1.17.0+20260907gitf1dd1b4af.aet1605`) under `~/bin/capped-run` (8 GiB, swap 0), `--network none`.

**Three answers up front.**
1. **The late panic passes today.** ArduPilot's own `AP_HAL::panic()`, planted straight after U5's last
   witness, gets exit 0 and verdict `pass`, every witness present (§3).
2. **Three of `verify`'s gates have an on/off decision that no specification key names:** `campaign`
   (the `--scenarios` flag), `attribute` (a run's outcome) and the `prove` harness set (the generator's).
   That is §7's stop: "a gate's on/off decision cannot be named by a key".
3. **The verdict bump breaks no reader outside the fence.** The one pin that moves is
   `fab/sentinel-01-bench/package/MANIFEST.json`'s `verdict_sha256`. It is moved by its generator (the
   bench dispatch), as `2306`'s was, so it is not §7's other stop. The dispatch is asked for at the build.

Predictions held on P2, P3 and P4's pass. They missed on P1 for `gpio`/`adc`/`bridge` (keys exist) and
on the via ring (not a gate), and on P4's HardFault (it fails, but by memory, not by exception).

---

## 1. Every gate, its on/off decision and its key

**`verify`'s gates** (`crates/aethelgard-cli/src/verify.rs`; `judged` from `scope_of`, L593-607).

| Gate (part) | Judged today | On/off decided at | Key that turns it on | Off today shows |
|---|---|---|---|---|
| `erc`, `drc`, `support`, `extract` | 1, 1, roles+straps, see parts | `run_gates` L2320-2325 | `board.drawn` (default true) | `not_applicable` + `NO_BOARD` (the one traced "off") |
| `extract` › lines | per bounded line | the topology's buses | `i2c_buses`, `spi_buses` | in `judged`; 0 → `nothing_in_scope` |
| `extract` › supply | +1 per domain | **eda** `supply.rs:1349-1357` (no `power`, no `drop_limits`, or empty → `Vec::new()`) | `power.drop_limits` | **silent**: `supplies` omitted (`extract.rs:45-48`); `judged` just shrinks |
| `extract` › oscillator | +1 per net (**counted, never judged**: see F1) | **eda** `extract.rs:1382` | `mcu.hse` (a `Crystal`) | silent (omitted) |
| `extract` › pairs | conductors counted as lines | the spec's pairs | `diff_pairs` | silent (omitted) |
| `drc` › pairs, `drc` › power loops | not counted | `main.rs:931`, `main.rs:954` → **eda** `loops.rs:38` | `diff_pairs`, `power` | silent; `gate_report` drops both keys even when on |
| `prove` | 1 per harness | the harnesses of the generated `src/proofs.rs` (`verify.rs:1804`), emitted by **svd-hal** `board.rs:5345-5347` (flash, pwr always; HSE only with `mcu.hse`) | **none: the generator's choice** | zero harnesses → `no_harness` error (unreachable); a missing feature harness is silent |
| `build` | 1 | always | always on | — |
| `simulate` | 1 | after `build` (L2437-2438) | always on | `not_run`, no reason |
| `simulate` › HSE clock rule | not counted | **twin** `hse.rs:118`, `renode.rs:3013` | `mcu.hse` | silent (in no verdict struct) |
| `gpio`, `adc` | items (an all-input `gpio` judges 0) | `verify.rs:772-773` drops an empty list; the list is `simulate`'s `boot.gpio`/`boot.adc` | `pin_devices[].signals`, `pin_devices[].analog` | absent, and absent too when `build` fails (no `not_run`) |
| `bridge` | items | same filter; items from the topology (`bridges()`, L1959) | `pin_devices[].bridges` | absent |
| `campaign` | runs | `expected.is_empty()` (L2439, L2464) | **flag `--scenarios`, not a key** | absent without the flag |
| `attribute` | 1 per run | only a `Seeded::Detect` run that failed as expected with a passing baseline (~L2547-2554) | **none: the run's outcome** | absent |

**Judging outside `gates`:** `silicon` (always, per SPI/I2C/USART list; empty lists show `[]`),
`ledger` (always, with `total`), `twin_limits` (named, never judged), `declared_peers` (a list),
`lifecycle` (banner), `preconditions`/`coverage` (only with `--fab-out`/`--against-fab`, flag-switched).

**`not_run`/`not_applicable` today:** `not_applicable` only at L2325 (`board.drawn: false`) →
`off: board.drawn`. `not_run` only at L2438 and L2440 (`build` failed or named no ELF) →
`not reached: build failed`. A gate that refuses stops `verify` with no verdict (L561-572), so a
failed `build` is the only "not reached" a verdict can hold today.

**The other subcommands with parts of their own:** `extract` (supply, oscillator, pairs, as above),
`check drc` (pairs, power loops), `simulate` (the HSE clock rule; with `--foreign`, pwr, clock, motors
and serials are off by the flag, and `RULES` already names each kept, replaced or dropped rule), and
`check pinout` (`decodes` and `identity`, switched by the `--hwdef-h` flag and already self-reported:
`identity: "not judged: no hwdef.h"`, pincheck `lib.rs:575`). `check speeds`, `check support`,
`campaign`, `attribute`, `replay`, `prove` and `build` have no optional part.

**What the committed specifications declare** (every `examples/*.json` and test fixture with an `mcu`):
no specification declares `power.drop_limits`, so **the supply part is off on every committed board**.
Two declare `power` (`cli/tests/fixtures/sentinel-03.json`, `ir/tests/fixtures/sentinel-02-power.json`;
correcting the sub-measurement's "none declares `power`", which read `examples/` only). Three declare
`mcu.hse`. Two (`d1-drv8876`, `d2-drv8833`) declare `pin_devices`, both `drawn: false`.

**Where "off" is decided outside the fence:** the supply part (eda), the oscillator part (eda), drc's
power loops (eda), the HSE clock rule (twin), the harness set (svd-hal). **Off leaves a trace in its own
gate's result** only for `board.drawn` and pinout's `--hwdef-h`. Everywhere else the result just omits
a key or counts fewer.

## 2. The verdicts' readers, and the version

- **No verdict carries a version.** `verify`'s `Report` has `engine_version` (the crate version, moved
  only by a release). Every past verdict change was an additive key with no version: `2306`
  (`judged`, `judged_nothing`; its FEEDBACK §3) and `4230` (`ledger`, `silicon`, `declared_peers`,
  `twin_limits`). The only versions in the repo are input and archive formats (specification 29, solved
  18, trace). So "the repo's own versioning" for a verdict is the additive key (D6).
- **Readers of `verify`'s line**, each by key; none holds an exact key set, a byte golden or
  `deny_unknown_fields`:

| Reader | Reads | A new key | Fence |
|---|---|---|---|
| `fab.rs` via `verify.rs:2148-2160` | the line without `fab`, hashed into `MANIFEST.json` | the hash moves by construction | in |
| `fab/sentinel-01-bench/package/MANIFEST.json:48` | `verdict_sha256` `ecef5719…` | **moves; `ci.yml` bench-package (L534-571) red until the dispatch re-commits it** | out, moved by its generator (as `2306`) |
| `fab/sentinel-01-bench/ORDERED.sha256` + `bench_board_tier1.rs:147-175` (`3308`) | the ten ordered Gerbers and drill; `MANIFEST.json` is `NOT_ORDERED` | no | — |
| `verify_2k2_tier2.rs:531-539` | cuts the line at `,"fab":` and hashes it | the new key must sit before `against_fab`/`fab` | in |
| `verify_2k2_tier2`, `verify_47k_tier2`, `against_fab_tier2`, `four_layer_fab_tier2`, `board_tier2:1883-1959` | `judged_nothing`, `against_fab`, gates by key | no | in |
| `mcp.rs:317-326`, `mcp-tools.json` | the line passed through; the snapshot holds input schemas and descriptions | only if a tool description changes | in |
| `crates/aethelgard-cli/README.md:931-950` | a documented example line | no test pins it | in |
| `verify`'s own `gate()` (L508-560) | each gate subcommand's line, by key | a new key on a gate line is ignored | in |
| `ci/foreign-judge.sh:57-72` | `simulate`'s verdict by key | no | in |

  No `ci/*.sh` script and no workflow script outside `foreign-judge.sh` parses a verdict. `replay` and
  `attribute` read the CBOR archive, never a verdict. No trace or record version moves.

## 3. The late panic (A15)

**The plant** (`plant.py`). The last witness of unmodified ArduPilot on `sentinel-02-own` is U5's (the
ICM-42688-P) `data (reads[0])` at 4.392420560 s of the 5 s window. That is the FIFO **data** transfer in
`AP_InertialSensor_Invensensev3::read_fifo()`, not the FIFO_COUNTH read before it: a plant after the
count read leaves `data (reads[0])` unwitnessed and fails `spi_transaction_unwitnessed`, measured.

The plant goes straight after the data transfer returns (`0x080360aa`, before `accumulate_samples`), so it
follows the witness by causality. The site is found by symbol, and its bytes are checked before patching;
each patch was disassembled. Four variants:
- `panic`: ArduPilot's own `AP_HAL::panic("\n")` (`system.cpp:327`: print, then `delay(500)` forever, in
  the calling thread);
- `udf`: an undefined instruction (HardFault; this build's handler is CrashCatcher);
- `halt`: `cpsid i; b .`;
- `reset`: an `AIRCR` SYSRESETREQ.

**Control first.** It uses the same binary, specification and window, on CI's ELF from
`ardupilot-image` run 45245344589 (main `8d727980`; the ELF's sha256 is `ce4ec8c6…`). It passes.

| Run | Exit | Verdict | Failing kinds | Final PC | ICSR |
|---|---|---|---|---|---|
| control | 0 | pass | — | `0x0806187a` (`AP_Param::next_group`) | `0x800` |
| **panic** | **0** | **pass** | — | `0x08103c20` (`__idle_thread`) | `0x800` |
| halt | 0 | pass | — | `0x080360ac` (the plant's self-loop) | `0x42800` (IRQ 50 pending, masked) |
| udf | 1 | fail | `memory_cap_exceeded` only | — | — |
| reset | 1 | fail | `core_in_exception`, `pc_outside_firmware`, `foreign_clock_contradicted` ×2, `register_mismatch` ×7, `spi_step_error` | `0xeffffffe` | `0x803` |

Every run's row, including the count-site plants, is in `out/runs.md`. The panic run's verdict is
`out/panic-simulate.json`, beside `out/control-simulate.json`.

**Why the panic passes (the mechanism, from the same run re-executed with hooks).** `sh/capture.sh`
copied the CLI's own monitor script from Renode's argv. `measure.py` added execution hooks and two
reads, and nothing else. The panic re-run executed the same 78,253,743 instructions as the judged run.
- `panic` was entered at **4.392440850 s**, 20 ns after U5's data witness.
- The IMU thread then sleeps inside `panic()`'s `delay`. Every thread parks, and the core ends in
  `__idle_thread` in thread mode: ICSR `0x800`, CFSR and HFSR 0.
- The final PC is in an executable `PT_LOAD`. Every witness is already in the log.
- **Nothing the judge reads differs from a healthy run**, except that U5 goes silent: 0 U5 log lines
  after 4.393 s, against 2,365 in the control.
- The judge has no rule that reads silence, a panic entry or the console.

**The HardFault fails, but not by name.**
- The hook saw `HardFault_Handler` entered at 4.392440830 s.
- Without the memory bound, the run ends in HardFault: ICSR `0x803`, CFSR `0x00010000` (UNDEFINSTR),
  HFSR `0x40000000` (FORCED), PC `0x8001434` (`CrashCatcher_getc`). `core_in_exception` would name that.
- It never gets to. CrashCatcher polls USART1's ISR, and the access log `2311` enabled on each declared
  USART logs every poll: 20,250,235 lines, 2.35 GB in 0.6 s. Renode passes the 1 GiB cap and is killed.
- The verdict says `memory_cap_exceeded`, with `booted: null` and no witness read.
- So it fails closed, but on a firmware whose fault handler polls a USART that is not declared
  (or a run with less contention), the outcome depends on the log budget, not on the exception check.
  That is §5's "a HardFault that the exception check misses".

**The reset fails by name** (`core_in_exception` and others). That is §5's "a watchdog reset" case: it
is not a pass today.

**The `halt` passes as the panic does.** `chSysHalt` is not in this build's symtab, so ChibiOS's own
halt is compiled out. The `halt` plant stands for any thread-mode hang, not for an ArduPilot path.

---

## 4. The table's shape (proposed)

One key, `ran`, on `verify`'s line, after `judged_nothing` and before `against_fab`/`fab`, so the
manifest's line is still the verdict without the fab keys. One row per gate, and per part where a part
can be off:

```json
"ran": [
  {"gate": "erc", "judged": 1},
  {"gate": "extract.lines", "judged": 3},
  {"gate": "extract.supply", "off": "power.drop_limits"},
  {"gate": "extract.oscillator", "off": "mcu.hse"},
  {"gate": "prove", "harness": "proofs::flash", "judged": 1},
  {"gate": "simulate", "not_reached": "build failed"},
  {"gate": "gpio", "off": "pin_devices[].signals"},
  {"gate": "campaign", "off": "--scenarios"}
]
```

Each row holds exactly one of `judged` (n ≥ 0, never omitted at 0), `off` (the key as a JSON path) or
`not_reached` (why). **Derivation:** each gate's own line states its parts' rows (the gate subcommands
`extract`, `check drc` and `simulate` are in the fence), and `verify` builds `ran` from `report.gates`
and those lines. It is never built from a list. A unit test plants a gate with no row and fails by name.

## 5. Findings and decisions for the ruling

- **D1, the derivation of "off" (needs a ruling).** For every part but `board.drawn`, the decision is
  made in eda, the twin or svd-hal, and the result does not say it was off. Options:
  - **(a) The gate subcommand in the CLI states the part's key** beside its call, from the
    specification it already holds: a second decision site in the fence. A test per part removes the
    key and asserts that eda returns nothing and the row says `off`, and that adding it back gives a
    row with `judged` ≥ 1, so the two sites cannot drift.
  - **(b) eda and the twin report "off" themselves.** This is outside the fence.

  **Lean (a).**
- **D2 (§7 stop): gates whose on/off no specification key names.**
  - `campaign`: the `--scenarios` flag. **Lean:** `off: --scenarios`, a flag named in the key's place.
  - `attribute`: the run's outcome. **Lean:** one row per seeded-detect run, `judged 1`, or
    `not_reached` with the campaign's own `why`.
  - The `prove` harness set: the generator's. **Lean:** one `judged 1` row per harness. A missing
    feature harness is the generator's concern (svd-hal, outside the fence) and gets a backlog row.
- **D3, which parts get rows.** The acceptance row (`off: drop_limits`) needs parts below a gate.
  **Lean:** `extract.lines`, `extract.supply`, `extract.oscillator`, `extract.pairs`, `drc.pairs`,
  `drc.power` and `simulate.hse_clock`. This is every part the code survey found that an optional key
  switches. The key is written as its full path (`power.drop_limits`).
- **D4, `gpio`, `adc` and `bridge`.** **Lean:** `off: pin_devices[].signals` (`.analog`, `.bridges`)
  when empty, and `not_reached: build failed` when `build` failed. Today they are absent either way.
- **D5, `judged_nothing`.** `ran` holds everything it says. **Lean:** keep it in this work order, since
  two tier-2 tests and the bench test read it, and propose its removal in the reduction pass, by
  ruling.
- **D6, the version.** **Lean:** the additive key, as `2306` and `4230` did. There is no verdict version
  to bump, and adding one would be a new mechanism. The bench package's `MANIFEST.json` moves through
  the dispatch.
- **D7, the panic's refusal (it passes today).** Options:
  - **(a) An execution hook on `AP_HAL::panic`'s entry**, its address from the ELF's own symtab →
    `foreign_panic` (error) with its virtual time and the last witness's. A missing symbol is refused
    by name (`INV-06`). This catches a panic in any thread, before or after the last witness. The
    symtab locates the hook only; no pass criterion is read from the artifact (`INV-07`).
  - **(b) A hook on the vector table's fault entries** (HardFault, MemManage, BusFault, UsageFault, read
    from the vector table at the ELF's VTOR, which the architecture defines, not ArduPilot) →
    `foreign_fault`, so a fault is named before the log budget or memory cap can stand in for it. It
    needs a measurement of whether the hook can also pause the machine, so that the 1 GiB kill does not
    discard the finding.
  - **(c) Liveness:** each witnessed device logs again in the window's last part →
    `foreign_device_silent`. It catches the panic and the halt. It misses a panic in a thread that reads
    no device, and adds a timing-dependent rule.

  **Lean: (a) now, named `foreign_panic`** ("after witness" is a property its message states, not its
  name), plus a CI case and a tier-1 test. **(b) and (c) are recorded as backlog** unless ruled into
  this work order.
- **F1 (pre-existing, §5's "count of inputs").** `extract`'s `judged` counts oscillator nets
  (`extract.rs:441-445`), which carry no verdict ("reported, not judged", `extract.rs:53-54`). That
  breaks `2306`'s rule: a crystal board's `judged` overstates by its oscillator nets. **Lean:** the
  oscillator row reads `judged 0`, and the nets are reported apart from it. This changes a count, not
  a judgement; ruled before it moves.
- **F2.** The via-ring rule is not a gate. Without `fab_tolerances`, `check drc` judges vias against the
  engine's default via instead of a ring-derived one: a threshold changes, and no gate goes off. That is
  outside the fence (eda, generator side) and `4300k` owns it.
- **F3.** Other optional keys whose absence silently skips a check outside any gate:
  `spi_devices[].orientation` (`schematic.rs:496-498`), `board.max_outline_um` (`schematic.rs:985-987`)
  and `power.loads[].reference` (`main.rs:2855-2857`). Each is a `validate` or emit-time check, not a
  verdict's gate. Recorded for the backlog.

## 6. Files

`PREDICTIONS.md` (committed first); `plant.py` (the ELF plant); `measure.py` with `sh/capture.sh`,
`sh/measure.sh` and `sh/foreign.sh` (the runs); `out/runs.md` (every run); `out/panic-simulate.json` and
`out/control-simulate.json`. The ELFs (GPL, built by CI) are not committed: they are in
`~/.cache/aet-1609/ap`, re-derivable from run 45245344589 with `plant.py`.
