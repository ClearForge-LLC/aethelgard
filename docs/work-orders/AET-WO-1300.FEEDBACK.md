# AET-WO-1300 — FEEDBACK

**Branch:** `wo/aet-1300-schematic` · **Base:** `1b4fe5f` (PR #37), started early by the kickoff; `origin/main` merged at `3f3924e` (AET-WO-1200 #32, AET-WO-0300d #39/#41) by merge commit `bfb847f`, on the architect's notes · **Builder:** Claude Code (Opus 5.5) as `forge` on Furnace · **Result:** spike answered and ruled on PR #40 (D1 to D10); §1 built under that ruling on the same branch, after merging `origin/main` at `425347e` (`4a3ea97`) and `23d0b55` (AET-WO-1201, AET-WO-1502 and the rulings in #46), each by merge commit.

Entry schema: `finding · where · type · recommendation · decision-needed`.

## Spike

**How the probe was built** (deleted by the build half, as ruled; it is at commit `44c1b60` under `spike/1300/`, read with `git show 44c1b60:spike/1300/<file>`). The probes were Rust under `spike/1300/`:
- `sexpr.rs`, a reader and writer;
- `kilib.rs`, library access with `extends` flattening;
- `sch.rs`, the schematic emitter;
- `pcb.rs`, the board emitter and grid router;
- `env.rs`, the library-configuration treatments;
- `s1.rs` … `s5.rs`, one per question.

They are compiled into one test, `crates/aethelgard-cli/tests/spike_1300.rs`, which is `#![cfg(feature = "with-tools")]` and pulls the probe in through `#[path]`. That way CI's `tools` job runs them inside the image pinned by `container/IMAGE`, under `--network none`, with no change to `container/` or `.github/workflows/`. Every measurement is an `ANSWER <Sn> …` line on the step's stderr, written past libtest's capture. A probe never panics; the test fails only on a harness error.

- No `unsafe`, no new dependency (`serde_json` and `sha2` are already the CLI's), no Python, no Java.
- Root `Cargo.toml` and `Cargo.lock` are unchanged relative to the merge base.
- No fixture was added under `container/fixtures/`: every input is emitted by the probe at run time.
- The build half deleted `spike/1300/` and the test hook; the ANSWER records moved to `docs/work-orders/AET-WO-1300.repro/`.

**Runs.** Each run's ANSWER lines are committed verbatim in `docs/work-orders/AET-WO-1300.repro/`, so the record outlives the Actions logs. Each answer below cites the run that measured it.

| Run | Commit | Record |
|---|---|---|
| [42426631742](https://github.com/ClearForge-LLC/aethelgard/actions/runs/42426631742) | `02d951d` | `docs/work-orders/AET-WO-1300.repro/run-42426631742.txt` |
| [42427541833](https://github.com/ClearForge-LLC/aethelgard/actions/runs/42427541833) | `985f607` | `docs/work-orders/AET-WO-1300.repro/run-42427541833.txt` |
| [42428065833](https://github.com/ClearForge-LLC/aethelgard/actions/runs/42428065833) | `aab865b` (includes the `main` merge) | `docs/work-orders/AET-WO-1300.repro/run-42428065833.txt` |
| [42428589322](https://github.com/ClearForge-LLC/aethelgard/actions/runs/42428589322) | `44c1b60` | `docs/work-orders/AET-WO-1300.repro/run-42428589322.txt` |

Image: `kicad-cli` 10.0.7, libraries at `/usr/share/kicad/{symbols,footprints}`, with `KICAD10_*_DIR` unset (every run's `ENV` line).

### S1 Symbols, footprints, licence

**Answer:**
- Every part has a library symbol and footprint in the image except the W25Q64JV.
- The libraries are CC-BY-SA 4.0 with a design exception. Embedding them in an emitted schematic or board is allowed and carries no obligation.
- The image ships no licence text for its libraries; the text below is upstream's, fetched at the matching tag.

Run 42426631742 (also 42427541833):

| Part | Symbol (`lib:name`, pins) | Footprint the symbol names (exists in the image) |
|---|---|---|
| STM32H753ZI, LQFP144 | `MCU_ST_STM32H7:STM32H753ZITx`, 144 | `Package_QFP:LQFP-144_20x20mm_P0.5mm` ✓ |
| BME280 | `Sensor:BME280`, 8 | `Package_LGA:Bosch_LGA-8_2.5x2.5mm_P0.65mm_ClockwisePinNumbering` ✓ |
| TMP117 | `Sensor_Temperature:TMP117xxDRV`, 6 (also `TMP117xxYBG`, DSBGA-6) | `Package_SON:WSON-6-1EP_2x2mm_P0.65mm_EP1x1.6mm` ✓ |
| W25Q64JV | **none** | — |
| 0402 / 0603 R and C | `Device:R`, `Device:C` | `Resistor_SMD:R_0402_1005Metric`, `R_0603_1608Metric`, `Capacitor_SMD:C_0402_1005Metric`, `C_0603_1608Metric` ✓ |

**The W25Q64JV substitute.** The `W25Q` symbols present are:
- `W25Q16JVSS` and `W25Q32JVSS`, SOIC-8 on `Package_SO:SOIC-8_5.3x5.3mm_P1.27mm`;
- `W25Q128JVS`, the same footprint;
- `W25Q128JVE`, `W25Q128JVP` and `W25Q32JVZP`, on WSON-8.

The smallest honest substitute is `Memory_Flash:W25Q32JVSS` with its Value set to `W25Q64JVSS`. Same family, same SOIC-8 208-mil footprint. That the pinout is identical is Winbond's datasheet, not a measurement here.

**Library format.** 223 `.kicad_sym` files (no `.kicad_symdir`), symbol library version `20251024`, and 155 `.pretty` directories.

**Licence.**
- **In the image:** no file. KiCad is not a dpkg package (`dpkg-query`: no packages match `kicad*`). `find / -ipath '*kicad*' -iname '*licen*'` outside `demos/` finds nothing (run 42427541833). The only licence files under `/usr/share/kicad` are the demo projects' own, such as `stickhub`'s CC BY-NC-SA 4.0 and `cm5_minima`'s CERN-OHL-S. Those are not the libraries' licence.
- **Upstream:** `https://gitlab.com/kicad/libraries/kicad-symbols/-/raw/10.0.7/LICENSE.md`, byte-identical in `kicad-footprints`. 2101 bytes, sha256 `45d2bce75e5a4208f5afb01b8fb2c406e700371c4fe2b5f5cd5c443d46db4d8f`, fetched 2026-11-08 and now vendored as `crates/aethelgard-eda/data/kicad-10.0.7/LICENSE.md`. It reads:

  > The KiCad libraries are licensed under the Creative Commons CC-BY-SA 4.0 License, with the following exception: *To the extent that the creation of electronic designs that use 'Licensed Material' can be considered to be 'Adapted Material', then the copyright holder waives article 3 of the license with respect to these designs and any generated files which use data provided as part of the 'Licensed Material'.*
  >
  > … if you wish to redistribute the KiCad libraries, or parts thereof (including in modified form) as a collection then the exception above does not apply.

**Embedding.**
- **Allowed?** Yes: an emitted `.kicad_sch` or `.kicad_pcb` is a design, or a generated file, and the exception waives article 3 for it. That fits `RUL-11`'s "generated artifacts carry no licence obligation".
- **Deterministic?** Yes, as a function of the library bytes. The four files the probe embeds from are recorded by sha256 (`Device` `d8cd2847…`, `power` `dfd086b4…`, `Sensor` `38a35259…`, `MCU_ST_STM32H7` `6f095b49…`). Two emissions are identical. With a library table present, ERC raises no `lib_symbol_mismatch`, so the flattened copy matches what KiCad loads (S2).
- **Footprints:** the same holds. The board emitter copies the `.kicad_mod` and re-derives every UUID inside it.

| # | Finding | Where | Type | Recommendation | Decision-needed |
|---|---|---|---|---|---|
| S1.1 | Every part has a symbol and footprint in the image except W25Q64JV; the substitute is `W25Q32JVSS` with Value `W25Q64JVSS` | run 42426631742 S1 lines | measured | Use the substitute, or add a symbol in the build | yes: substitute or own symbol |
| S1.2 | The libraries are CC-BY-SA 4.0 with a design exception; embedding them in emitted files fits `RUL-11` | upstream `LICENSE.md` at 10.0.7 | document fact; the image ships no copy (measured) | Embed at emit time | no |
| S1.3 | **Where the emitter gets the symbols decides `INV-01`'s inputs.** Read from the image at emit time, the output is a function of specification + pin table + library bytes, a third input that the image digest pins. Vendoring the few symbols and footprints into the repo removes that input, but vendored copies are "parts thereof as a collection": they stay CC-BY-SA 4.0 with attribution and share-alike, outside the exception. That is new licensed data in an `MIT OR Apache-2.0` repository, which `RUL-11`'s inventory does not list | S1.2; `RUL-11`; `INV-01` | design choice | Vendor, with the licence beside the files (as ST's are), and amend `RUL-11`'s inventory. Or read from the image and pin the library hashes in the emitter | **yes** |
| S1.4 | The published tools image redistributes KiCad's library collection with no licence text in it (KiCad's own official image does the same) | run 42427541833 S1 provenance | observation | Low: add the licence file in a later image work order, or record it as upstream's packaging | yes, low |

### S2 A minimal ERC-clean schematic

**Answer:** the emitter writes the schematic below, and it is **ERC-clean (exit 0, zero findings) once KiCad has a library configuration** (run 42427541833). The schematic carries:
- the MCU, the BME280 on I2C1 (`PB8` SCL, `PB9` SDA), 2k2 pull-ups, 100 nF decoupling, 2u2 VCAP capacitors and one rail `VDD_3V3`;
- 113 no-connects, a `PWR_FLAG` on each rail, VCAP1 and VCAP2 on separate nets, and SDO to GND through R3.

Without a library configuration, the 55 library warnings remain, and today's `aethelgard check erc` fails it for that reason alone. Every UUID is derived from content, and two emissions are byte-identical.

**Findings and what clears each.** Variant names are the probe's; counts are from `kicad-cli sch erc --exit-code-violations`, and run 42427541833 repeats run 42426631742.

| Finding (naive emission) | Count | Cleared by |
|---|---|---|
| `pin_not_connected` (error) | 113 | a no-connect on every unused pin's point (`nc`: 113 markers) |
| `pin_not_driven` (error), NRST (input) | 1 | the same no-connect |
| `power_pin_not_driven` (error), the `VDD_3V3` and `GND` power symbols | 2 | a `PWR_FLAG` on each rail (`flags`) |
| `pin_to_pin` (error): "Pins of type Power output and Power output are connected", U1 pins 71 and 106, both `VCAP` | 1 | **separate nets**, VCAP1 and VCAP2, each with its own capacitor. KiCad's symbol makes VCAP `power_out`. A `PWR_FLAG` on the joined net made it 2 errors |
| `pin_to_pin` (warning), BME280 SDO (bidirectional) on the flagged GND net; appears once the flags exist | 1 | SDO to GND through a resistor (R3, 10k) |
| `lib_symbol_issues` (warning): "The current configuration does not include the symbol library 'Device'" | 39–46 | a library configuration (below) |
| `footprint_link_issues` (warning), the same for footprint libraries | 7–9 | the same |

The MCU symbol has 144 pins at 136 points (8 hidden, stacked). By type: 114 bidirectional, 4 input, 8 passive, 16 power in, 2 power out. The emitter connects stacked pins once.

**Library configuration.** The `fixed` variant, with raw `kicad-cli` in a directory of its own (run 42427541833):

| Treatment | ERC exit | Findings |
|---|---|---|
| the file alone (as `check erc` copies it) | 5 | 55: `lib_symbol_issues`×46, `footprint_link_issues`×9 |
| project `sym-lib-table` + `fp-lib-table`, no `.kicad_pro` | 5 | 55, the same: the tables are not read |
| the same tables beside a minimal `.kicad_pro` | **0** | 0 |
| a `.kicad_pro` setting `erc.rule_severities` for the three library checks to `ignore` | **0** | 0 |
| `KICAD_CONFIG_HOME` holding KiCad's template tables, with `KICAD10_SYMBOL_DIR` and `KICAD10_FOOTPRINT_DIR` set | **0** | 0 |

`aethelgard check erc` on the `fixed` schematic: exit 1, verdict `fail`, `footprint_link_issues` 9 and `lib_symbol_issues` 46. It copies only the schematic, so no real schematic can pass it today.

**UUIDs and determinism.**
- **Derivation:** each UUID is sha256 of a domain-separated content path (`<project>/symbol/<ref>`, `…/pin/<n>/wire`, …), first 16 bytes, with RFC 9562's version-8 and variant bits set.
- **KiCad keeps them:** every finding's item UUID is one the emitter wrote (164 of 164 naive, 55 of 55 fixed). `kicad-cli sch upgrade` kept 409 of 409.
- **Two emissions are identical**, the second built with its parts and flags in reverse order. `fixed` is sha256 `90c9789a1fadcc161fc0c01ab4e369aae14e96bebaa215e8a8c933a80acbf6a0` in runs 42427541833, 42428065833 and 42428589322 (the last two after the `main` merge), separate processes on separate runners.
- **Format version:** the emitter writes `(version 20231120)`. `kicad-cli sch upgrade` rewrites the file to `20260306`, KiCad 10's own: 185209 → 197969 bytes, not identical. The same schematic emitted at `20260306` judges the same (`fixed@20260306`).
- **Net names:** a root-sheet local label's net is `/NAME` (`/I2C_SDA`); a power net is its Value (`VDD_3V3`, `GND`).

| # | Finding | Where | Type | Recommendation | Decision-needed |
|---|---|---|---|---|---|
| S2.1 | The emitter must add: a no-connect on every unused pin; a `PWR_FLAG` per rail; one net per `power_out` pin (VCAP); and a resistor between a bidirectional pin and a flagged rail | table above | measured | Build these into the emitter as rules from pin types | no |
| S2.2 | **ERC needs a library configuration, which today's `check erc` cannot supply.** It copies only the schematic, so any emitted schematic fails on library warnings | run 42427541833 S2 `fixed` | measured | The gate supplies one in its scratch copy, as `RUL-03` has it supply DRC's rule files. A `.kicad_pro` with ERC severities is the smallest and keeps the judge's rules in a gate-written file; tables + `.kicad_pro`, or `KICAD_CONFIG_HOME`, also work and keep the library checks live | **yes**: which |
| S2.3 | Emitted UUIDs survive KiCad; output is byte-identical across emissions and runners | above | measured | Keep content-path UUIDs | no |
| S2.4 | Emit at `20231120` or KiCad 10's `20260306`; both judge the same | `fixed`, `fixed@20260306` | measured | Emit `20260306`, KiCad 10's own version | yes, low |

### S3 The gate's own rules

**Answer:** a gate that overwrites the setup block, writes the project, and writes its own `.kicad_dru` makes DRC report the violation; without those steps it does not. **Two steps are load-bearing as measured: the `.kicad_dru`, and the setup overwrite. The project step is redundant with a gate setup that carries `clearance_min`.** The steps alone are not enough: a local clearance override written in the board defeats the gate's custom rule.

- **The gate's `.kicad_dru` is the only step that catches loosening at net-class or rule level** (the board's own `.kicad_pro` net class, or its own `.kicad_dru`). In the matrix below, the violation appears exactly when that step is applied. The pattern is identical in runs 42426631742, 42427541833 and 42428065833 (`r3-none-*`).
- **The setup block's `clearance_min` sets the board minimum, and it overrides the project's `min_clearance`.** Its `trace_clearance` decides nothing (the baselines). A pad or footprint carrying a local 0.05 mm clearance escapes the gate's custom rule, and only the board minimum catches it. So, with the board's own setup carrying `clearance_min 0.05`, a local override passes the gate's project and rules both: 0 violations (run 42428065833, `r3-pad-local-*-setup_loose-pro_gate-dru_gate`). Overwriting the setup catches it: 1 violation, "board minimum clearance 0.2000 mm".
- **The `.kicad_pro`'s `min_clearance` is the board minimum only when the setup block carries none.** In row `setup_plain`, the gate writes a setup with no clearance tokens, and its project catches the override. With the gate's setup carrying `clearance_min 0.2`, removing the project step lets no measured board escape (row "gate / board's own / gate": 1 and 1). The project becomes load-bearing only under D5, a token-free gate setup. The row that would show it, `setup_plain` with the board's own project and the gate's rules, was not measured; that it escapes is inferred from `r3-read-*`.


The board has two tracks, `NET_A` and `NET_B`, with a 0.1 mm gap. The specification says 0.2 mm; the board's own files say 0.05 mm:
- its `(setup …)` block carries `(trace_clearance 0.05) (clearance_min 0.05)`;
- its `.kicad_pro` sets the Default net class to 0.05;
- its `.kicad_dru` holds `(rule … (constraint clearance (min 0.05mm)))`.

The gate's versions carry 0.2. Raw `kicad-cli pcb drc` ran in a directory holding `input.kicad_pcb` and the sidecars named for it. Every row also has two `track_dangling` warnings (the tracks end on nothing), so every row exits 5. The column that matters is the clearance count. Runs 42426631742 and 42427541833 agree:

| Gate step applied → | setup | `.kicad_pro` | `.kicad_dru` | clearance violations |
|---|---|---|---|---|
| none (the board's own three files) | – | – | – | **0** |
| | ✓ | | | 0 |
| | | ✓ | | 0 |
| | ✓ | ✓ | | 0 |
| | | | ✓ | **1**, "rule 'gate_spec_clearance' clearance 0.2000 mm; actual 0.1000 mm" |
| | ✓ | | ✓ | 1 |
| | | ✓ | ✓ | 1 |
| all three | ✓ | ✓ | ✓ | **1** |

Baselines, each file alone:
- No sidecars, any setup block (plain, 0.05 or 0.2): 1 violation at "clearance 0.2000 mm", KiCad's default net class. The setup block's clearance tokens do not decide a track-to-track clearance.
- The board's own `.kicad_pro` alone (netclass 0.05): 0.
- The board's own `.kicad_dru` alone: 0.

`aethelgard check drc` on the same board, with its own `.kicad_pro` and `.kicad_dru` beside it: 1 violation. It copies only the board, so it judged by KiCad's default, 0.2, which here happens to equal the specification.

**Round 3, local overrides** (run 42428065833). The board is an `R_0603` on `NET_A` with a `NET_B` track 0.1 mm from pad 1. Pad-level and footprint-level overrides give identical rows, and so do the round-1 project shape and the demo-derived shape (`fmtA`, `fmtD`).

| setup | `.kicad_pro` | `.kicad_dru` | local 0.05 override | no override |
|---|---|---|---|---|
| gate (0.2) | gate | gate | 1, board minimum | 1, gate rule |
| **board's own (0.05)** | gate | gate | **0** | 1, gate rule |
| gate, no clearance tokens | gate | gate | 1, board minimum | 1, gate rule |
| gate | board's own | gate | 1, board minimum | 1, gate rule |
| gate | gate | board's own | 1, board minimum | **0** |
| board's own | board's own | gate | 0 | 1, gate rule |
| board's own | board's own | board's own | 0 | 0 |

**KiCad 10 reads the project shape the gate writes.** Three shapes were tested, each with a 0.05 net class, a 0.2 board minimum and `track_dangling` set to ignore:
- round 1's;
- one with no `design_settings.meta`;
- one derived from the image's first demo project, `demos/cm5_minima/CM5_MINIMA_3.kicad_pro` (`meta.version` 3, `design_settings.meta.version` 2, `net_settings.meta.version` 5, 58 `rule_severities` keys).

All three report the gap as "board minimum clearance 0.2000 mm", and none reports `track_dangling` (`r3-read-*`). So `min_clearance` and `rule_severities` are read from each.

| # | Finding | Where | Type | Recommendation | Decision-needed |
|---|---|---|---|---|---|
| S3.1 | **Load-bearing, measured:** the `.kicad_dru` step, for net-class and rule-level loosening (matrix); and the setup overwrite, for the board minimum, because its `clearance_min` beats the project's (round 3, row "board's own / gate / gate": 0). **The project step is redundant** while the gate's setup carries `clearance_min`. It is load-bearing only if the gate writes a token-free setup, which is inferred and not measured (the `setup_plain` + board-own project row was not run) | matrix; round-3 table | measured | Keep all three steps (D5) | no |
| S3.2 | **A local clearance override (pad or footprint) is judged only against the board minimum, never by the gate's custom rule.** With the minimum equal to the specification's smallest clearance, the override is still caught. But a per-net rule larger than the minimum, such as an isolation gap, would be escaped by a local override on that net's pads. That is §5.1's "rules embedded in a footprint", confirmed | round-3 table | measured | The gate also refuses or strips local `(clearance …)` on pads and footprints in its scratch copy. Refusing is `INV-06`'s shape; stripping is `INV-07`'s. The adversarial pass then covers net classes in the board, a `.kicad_pro` symlink and a renamed `.kicad_dru` | **yes**: refuse or strip, and whether zones' local clearances join them |
| S3.3 | `aethelgard check drc` today judges by KiCad's defaults, because it copies only the board | `check drc` row | measured | `check drc --spec` replaces this in the build | no |


### S4 Placement and routing, for 1301 and 1302

All from run 42427541833 (and run 42426631742), except where a row names run 42428065833 or 42428589322.

1. **Specctra DSN/SES: no.**
   - `kicad-cli` 10.0.7's commands are `fp, jobset, pcb, sch, sym, version`.
   - `pcb export` offers `3dpdf, brep, drill, dxf, gencad, gerbers, glb, hpgl, ipc2581, ipcd356, odb, pdf, ply, pos, ps, stats, step, stl, stpz, svg, u3d, vrml, xao`.
   - `pcb import` takes `pads, altium, eagle, cadstar, fabmaster, pcad, solidworks`.
   - No `dsn`, `specctra` or `ses` appears anywhere in the help. Freerouting therefore has no headless path through `kicad-cli`, besides needing Java (absent).
2. **Python `pcbnew`: present, not used.** `/usr/lib/python3/dist-packages/pcbnew.py` and `_pcbnew.so` exist, and `python3` is on `PATH`; `java` is not. Nothing imported them (`RUL-12`).
3. **Zones: `pcb drc` does not refill by default; `--refill-zones` does.**
   - The test board is a GND zone over a footprint with two GND pads, emitted with no `filled_polygon`.
   - Without the flag: 1 `unconnected_items`.
   - With `--refill-zones`: 0, and the file on disk is unchanged.
   - With `--save-board` as well, KiCad writes the fills. **Two such saves of one input are not byte-identical:** 12 of 299 lines differ, and the first three are fill UUIDs that KiCad generates fresh.
   - So the emitter must never write or keep KiCad's fills. The gate runs `--refill-zones` on its scratch copy.
4. **The Rust router.** `pcb.rs` is Dijkstra over a 0.1 mm grid on F.Cu and B.Cu. Obstacles are other nets' copper, inflated by half the width plus clearance plus half the pitch. A via costs 40 steps. Ties break by node index, so the route is a function of the board.
   - The test case is two `R_0603` footprints whose nets cross (`/A`: R1.1 to R2.2; `/B`: R1.2 to R2.1).
   - The router routed `/A` around and `/B` straight: 8 segments, 0 vias. Two emissions are identical.
   - With F.Cu allowed only within 1.5 mm of each net's pads, it routed on B.Cu: 10 segments, 4 vias.
   - DRC with `--schematic-parity` gave **0 clearance violations and 0 unconnected items on both** (P10, P11).
   - **The full gate passes:** exit 0, with 0 violations, 0 unconnected and 0 parity findings, on the routed board (P14, run 42428589322). That needs two things:
     - footprint libraries in KiCad's configuration (`KICAD_CONFIG_HOME` here);
     - `ki_fp_filters` dropped from the schematic's embedded `Device:R`.

     KiCad 10's parity check asks for that field in the footprint, but a footprint property of that name does not satisfy it. P13 shows that setting the check to `ignore` in the project does not silence it, and neither does it silence `lib_footprint_issues` (P9, P12). Neither was silenced by `rule_severities` keyed by its report type (P9, P12, P13), while `track_dangling` was (S3 `r3-read-*`). The path works, so it is these keys, or these checks, that it does not reach.
     The cost: with the symbol library loaded, that schematic's ERC raises `lib_symbol_mismatch` ×2 (warning). Under S2's ERC-severities treatment, which ignores `lib_symbol_mismatch`, it would not.

**What parity required** (rows P0–P14):

| Row | Mismatch | Parity finding |
|---|---|---|
| P1 | board reference `R9` for `R2` | `missing_footprint` "Missing footprint R2 (10k)" + `extra_footprint` |
| P2 | footprint lib id `R_0402…` against the symbol's `R_0603…` | `footprint_symbol_mismatch` "…doesn't match footprint given by symbol…" |
| P3 | board nets `A`, `B` against the schematic's `/A`, `/B` | `net_conflict` ×4 "Pad net (A) doesn't match net given by schematic (/A)" |
| P5 | Value differs | `footprint_symbol_mismatch` |
| P0, P7 | symbol fields absent from the footprint | `footprint_symbol_field_mismatch` "Missing symbol field 'ki_keywords'", then `'ki_fp_filters'` once the others were copied |
| P4 | `(path …)` naming no symbol | **none**: with the reference matching, parity did not check the path |
| P6 | no `.kicad_pro` beside the board | no change |
| P13, P14 | `ki_fp_filters`: copied as a footprint property, or ignored by severity | still `footprint_symbol_field_mismatch`; it clears only when the embedded symbol drops the field |

`lib_footprint_issues` warnings (DRC's library check) need the same library configuration as S2: P8 (project tables) and P10 (`KICAD_CONFIG_HOME`) clear them.

**Recommended shape:**
- **`AET-WO-1301` (placement):** placement is Rust, deterministic, from the specification: a grid or row placer over courtyards, keyed by reference. It emits footprints copied from the library (S1.3) with:
  - reference, value and library id equal to the schematic's;
  - the symbol's fields copied as hidden properties;
  - `(path "/<symbol uuid>")` from the shared derivation, which parity does not need but KiCad's own tools use.
- **`AET-WO-1302` (routing):** routing is the Rust grid router, grown from this proof of concept:
  - multi-pin nets by tree growth;
  - net-class widths and clearances from the specification;
  - vias, and zones emitted unfilled.
  - Its own geometry is where the parasitics come from.
  - The gate runs `pcb drc --exit-code-violations --format json --refill-zones --schematic-parity` on a scratch copy holding:
    - the emitted schematic;
    - the gate's rule files (S3);
    - a library configuration (S2.2).

  No Freerouting, no DSN, no Python, no Java. The routing lead "zones may need their filled polygons written by the emitter" is answered no: the gate refills, and KiCad's fills must not be persisted, because they are not deterministic.

| # | Finding | Where | Type | Recommendation | Decision-needed |
|---|---|---|---|---|---|
| S4.1 | `kicad-cli` 10.0.7 has no Specctra DSN/SES export or import | help output | measured | Roadmap lead confirmed | no |
| S4.2 | `pcbnew` Python and `python3` are in the image; Java is not | find, `command -v` | measured | Do not use (`RUL-12`) | no |
| S4.3 | DRC refills only with `--refill-zones`; KiCad's saved fills are not byte-deterministic (fresh UUIDs) | lead 3 rows | measured | Emit zones unfilled; the gate passes `--refill-zones`; never `--save-board` | no |
| S4.4 | A Rust grid router's two-layer board passes DRC + parity: exit 0 with nothing reported (P14), and 0 unconnected with vias (P11) | P10, P11, P14 | measured | Adopt for `1302`; the prior is confirmed | yes: the shape above |
| S4.5 | Parity matches footprints by reference, not path; it requires the library id, value, net names as KiCad writes them (`/` prefix for root-sheet labels) and every symbol field | P1–P7 | measured | Emitter rules for `1301` | no |
| S4.6 | Passing parity needs a library configuration for DRC (S2.2). Severities did not do it: `lib_footprint_issues` and `footprint_symbol_field_mismatch` were not silenced by `rule_severities` keyed by their report types (P9, P12, P13). It also needs `ki_fp_filters` dropped from embedded symbols, which ERC then calls a `lib_symbol_mismatch` unless that check is ignored | P9–P14 | measured | ERC: the gate writes a project with the library checks ignored. DRC: the gate supplies footprint tables. Embedded symbols drop `ki_fp_filters` | **yes** |

### S5 Escaping

**Answer:** the S-expression quoting is enough. `"` and `\` are escaped with a backslash, and every other character goes raw inside double quotes. Every printable character `RUL-14` admits then survives KiCad's reader, in every class, as itself; `(`, `)`, `;`, `"` and `\` included (run 42428065833: 39 of 50 names intact in all six classes).

What does not survive is KiCad's own text layer above the S-expression, in four ways:
1. **`/` in a net name becomes KiCad's escape token** `{slash}`: label `LA/B` gives net `/LA{slash}B`, and power net `DA/B` gives `DA{slash}B`. ERC shows the unescaped text. The board's net names must be written the same way for parity (S4 P3).
2. **KiCad's escape tokens are read as escapes,** so a name containing one collides with the character it stands for:
   - a reference, value or net written `A{slash}B` reads back as `A/B`;
   - `{backslash}` reads back as `\`, `{colon}` as `:`, `{brace}` as `{`;
   - in ERC, such a name's item text no longer matches the name.

   Two specification identifiers, `A/B` and `A{slash}B`, would become one name in KiCad.
3. **`${…}` is a text variable, expanded or an error.**
   - `${PROJECTNAME}` came back as the empty string in labels, power nets, values and the title: `/L`, `D`, `V`, `T`.
   - `${X}` raised `unresolved_variable` on every symbol that showed it.
   - Its label also raised `bus_to_net_conflict`.
4. **Bus syntax is parsed:**
   - labels `LA{B}` and `LA[0..1]` raise `bus_to_net_conflict` (error), and their global labels drop out of the ERC report;
   - their nets still read back by name.

Two earlier "changes", `A#B` and `~{X}`, were the probe's layout and not KiCad: loose labels landed on parts' wires (run 42427541833 `multiple_net_names`). Run 42428065833 moved them, and both names round-trip.

The project-name class holds for 49 of 50 names; `${PROJECTNAME}` is the one that fails, as above.

| # | Finding | Where | Type | Recommendation | Decision-needed |
|---|---|---|---|---|---|
| S5.1 | S-expression quoting (`\"`, `\\`) is correct and sufficient for every character `RUL-14` admits | run 42428065833 S5 | measured | The emitter quotes every string so | no |
| S5.2 | `/`, KiCad's `{token}` escapes, `${…}` and bus syntax (`{…}`, `[…]`) change meaning inside KiCad. The visible effects are a collision, an expansion, or an ERC error | S5 rows | measured | Either narrow `RUL-14` for KiCad-bound identifiers (refuse `/`, `{`, `}`, `$`, `[`, `]`), or have the emitter apply KiCad's own escaping (`/`→`{slash}`, `{`→`{brace}`, …) and prove that an escape exists for `$` and `[`. Narrowing is smaller and removes the class | **yes** (`RUL-14` is a steering ruling) |

**How it was tested.** Every printable ASCII character that `RUL-14` admits, other than letters and digits, was carried in a name of the form `A<c>B`. Eighteen KiCad-shaped names were added:
- markup and variables: `~{X}`, `^{X}`, `_{X}`, `${X}`, `${PROJECTNAME}`;
- escape tokens and bus syntax: `A{slash}B`, `A{backslash}B`, `A{colon}B`, `A{brace}B`, `A{B}`, `A[0..1]`;
- quoting and paths: `\"`, `\\`, `)(`, `9A`, `A//B`, `/A`;
- and 63 × `Z`.

Each name was emitted in every class, as the thing it becomes:
- a net name, as a local label and a loose global label;
- a domain id, as a power symbol's Value;
- a reference and a part, as a Reference and a Value;
- a project name, as the `instances` project and the title, one file per name.

Names were read back from ERC's item descriptions and from the netlist, by node. The specification's other identifiers (`bus.id`, `peripheral`, a signal's `pin`) never become schematic text, so they have nothing to round-trip.

## Decisions needed for the ruling (collected)

| # | Decision | From | Builder's recommendation |
|---|---|---|---|
| D1 | Where the emitter's symbols and footprints come from | S1.3 | Vendor the few used, under CC-BY-SA 4.0 with the licence beside them, and amend `RUL-11`'s inventory. The alternative reads the image at emit time and makes the library bytes a pinned third input to `INV-01` |
| D2 | The W25Q64JV symbol | S1.1 | `Memory_Flash:W25Q32JVSS` with Value `W25Q64JVSS` |
| D3 | How the gate gives ERC and DRC a library configuration | S2.2, S4.6 | ERC: a gate-written `.kicad_pro` that ignores the library checks. DRC: footprint tables, written by the gate (`KICAD_CONFIG_HOME` or project tables + project). Embedded symbols drop `ki_fp_filters` |
| D4 | Local clearance overrides in the board under judgement | S3.2 | Refuse them (`INV-06`); include zone-local clearances |
| D5 | The gate's setup block | S3.1 | Write a setup with no clearance tokens, and put the board minimum in the gate's `.kicad_pro`, so there is one place for it. That makes the project step load-bearing; the build measures the row the spike did not |
| D6 | Identifiers KiCad reinterprets: `/`, `{`, `}`, `$`, `[`, `]` | S5.2 | Narrow `RUL-14` for KiCad-bound identifiers |
| D7 | The emitted format version | S2.4 | `20260306` |
| D8 | The shape of `AET-WO-1301` and `1302` | S4 | As recommended in S4: Rust placement and a Rust grid router; zones unfilled; the gate runs `--refill-zones --schematic-parity` |
| D9 | `aethelgard emit --file <spec> --out <dir>` (§1.3) | — | Adopt. Validate, solve, emit; as a command-tree leaf it becomes an MCP tool |
| D10 | The tools image carries KiCad's libraries without their licence text | S1.4 | Low; record it, or add the file in a later image work order |

## Flag-and-stop check (§7)

- **Licence:** embedding fits `RUL-11` (S1.2). Vendoring needs an amendment (D1), so it is recorded as a decision, not parked.
- **Python, Java or an image change:** none needed. The image's `pcbnew.py` is noted and unused (S4.2). Freerouting is not viable (no DSN, no Java).
- **A gate that reads a rule from the board under judgement: present in today's `check drc`.** It copies the board, `(setup …)` block included. S3 shows that block's `clearance_min` sets the board minimum, which is the only rule applied to a locally overridden pad. This is the defect `RUL-03`'s amendment and this work order's `check drc --spec` exist to remove. It is flagged here for the ruling, not parked, because the spike's job was to measure it and the build half is its fix.
- **Non-deterministic emission:** none in the emitter; every emission was identical, and across runs on different runners. KiCad's own saved zone fills are not deterministic, which is why the emitter never persists them (S4.3).
- **`unsafe`, or a skipped, ignored or weakened test:** none.
- **`sudo`, a system package, or a secret:** none, beyond the S25-minted push token. That token was minted per push, filled through `gh-write prep` from the transcript, and revoked after each use (HTTP 204 each time).

## Flag-and-stop check (§7), build half

None arose:
- **A gate that reads a rule from the board under judgement:** the spike's finding above (today's `check drc` carried the board's setup block into the judge) is **closed**. The gate writes its own setup block, project, rules file and library tables, and refuses every board-carried rule (`LocalClearance`, `BoardNetClass`, `BoardWaiver`, and `MalformedBoard` for anything its reader and KiCad's could read differently). The adversarial pass's bypass (A1) is refused by tests that ran green in CI run 42436232690.
- **Licence:** vendoring is the ruled D1, with the licence beside the files.
- **Python, Java or an image change; `unsafe`; a skipped or weakened test; `sudo` or a secret:** none, beyond the push route. B5 lists the tests whose board input changed, and why each still proves what it proved.
- **Non-deterministic emission:** none; two emissions are byte-identical, across processes, environments and runners.

## Gates (spike half)

On Furnace, at `d8066ec` with `origin/main` merged (`bfb847f`), every gate passes (only this file changes after it):
- `cargo fmt --all -- --check`;
- `cargo clippy --workspace --all-targets --locked -- -D warnings`, with and without `--features aethelgard-cli/with-tools`;
- `cargo test --workspace --locked`.

In CI, runs 42426631742, 42427541833, 42428065833 and 42428589322 are green in both `gates` and `tools`. The protected surfaces diff to empty against the merge base. The steering documents, `README.md`, `LICENSE-*`, `docs/inputs/**`, `container/` and `.github/workflows/` are untouched. The only paths added are `spike/1300/`, `crates/aethelgard-cli/tests/spike_1300.rs` and this file. The reduction pass and the adversarial pass belong to the build half.

**At the spike's close the build half had not started; it is the Build section below.** §6's promote-or-needs-none pass over `AET-WO-1200`'s FEEDBACK belongs to the build half's FEEDBACK, with the gates line.

## Build (after the ruling on PR #40)

**What was built, by ruling item.**

- **D1, vendoring.** The symbols and footprints the emitter uses are copied byte for byte from KiCad's libraries at tag 10.0.7 into `crates/aethelgard-eda/data/kicad-10.0.7/`, with KiCad's `LICENSE.md` and a README naming the tag.
  - At 10.0.7, `kicad-symbols` keeps each library as a `<Lib>.kicad_symdir/` directory, one file per symbol. The monolithic `.kicad_sym` files the image carries are build products the repository does not contain.
  - So the per-symbol source files are vendored whole: 10 symbols, including `TMP102xxDRL`, the parent `TMP117xxDRV` extends. Six footprints come with them.
  - Each file's git blob SHA, from GitLab's files API at ref 10.0.7, equals the blob SHA of the vendored bytes. `src/library.rs` checks every file's sha256 before any is used.
  - `aethelgard-eda`'s licence is `(MIT OR Apache-2.0) AND CC-BY-SA-4.0`.
- **D2:** `W25Q64JV` is drawn with `Memory_Flash:W25Q32JVSS`, Value `W25Q64JVSS`. Its bill-of-materials row carries the substitution note.
- **D3:** every library configuration is a gate-written file in the scratch copy; no environment variable is set.
  - `input.kicad_pro` holds the Default net class, the board minimum and ERC's library checks set to `ignore`, as one file. This follows `RUL-03` as amended, and it is the shape run 42427541833 measured.
  - `input.kicad_dru` holds the same minimums as custom rules.
  - `sym-lib-table` and `fp-lib-table` name the vendored libraries, which the gate copies into `lib/`.
  - Embedded symbols drop `ki_fp_filters`.
- **D4:** a board carrying a `(clearance …)` anywhere outside its top-level setup block is refused by name before any tool runs: exit 3, `LocalClearance`, `id` such as `pad 1 of footprint R7` or `zone on net GND`. So is an in-board net class (`BoardNetClass`) and a board that is not one KiCad S-expression (`MalformedBoard`, nesting past 256 included).
- **D5:** the gate's setup block carries no clearance token, and the board minimum lives in its project. The row the spike did not run is a tier-2 test, `with_a_token_free_setup_the_project_step_is_load_bearing`. It asserts that a pad-local override escapes the gate's rules file under the board's own project, and is caught under the gate's. Measured in CI run 42436232690: the project step is load-bearing (Gates, below).
- **D6:** `aethelgard-ir`'s identifier check refuses `/`, `{`, `}`, `$`, `[` and `]` (`REFUSED_IN_IDENTIFIERS`), with its tests and README row. No checked-in example or fixture broke.
- **D7:** the schematic is written at format `20260306`.
- **D9:** `aethelgard emit --file <spec> --out <dir>` validates, solves and emits `<project_name>.kicad_sch` and `<project_name>.bom.csv`. It is an MCP tool, `emit`; the tool-list tests and the CLI README are updated.
- **`check drc` without `--spec`:** the same files, holding KiCad's defaults, and `"rules":"kicad-default"`. With `--spec`, `"rules":"specification"` and `specification_sha256`. DRC runs with `--refill-zones` (`RUL-03` as amended); the board is never saved.
- **D8 and D10:** nothing to build here. `--schematic-parity` belongs to `1301` and `1302`.

**§1.4, where each lives:**
- ERC passes on the reference schematic: tier 2, `the_emitted_reference_schematic_passes_erc`.
- Two regenerations are byte-identical: tier 1, `emit_writes_the_schematic_and_bom_and_two_runs_are_byte_identical`. That test runs two processes, writes into two directories and compares every byte.
- S3's looser board is judged by the gate: tier 2, `a_board_whose_own_files_are_looser_is_judged_by_the_gate`, under `--spec` and without it. Tier 1 also checks every file the gate writes, and that no board-owned file reaches the judge: `the_gate_writes_every_rule_and_library_file_and_reads_none_from_the_board`, with a stand-in `kicad-cli`.
- S5's names survive: tier 2, `identifiers_with_s_expression_characters_survive_kicad`. It covers every class, read back from ERC's verdict and KiCad's own netlist.

| # | Finding | Where | Type | Recommendation | Decision-needed |
|---|---|---|---|---|---|
| B1 | **The specification declares no spacing, track-width or fabrication field**, so `--spec` cannot derive one. The gate uses one named engine floor, `rules::FLOOR_NM`: 0.2 mm clearance and board minimum, and a 0.2 mm minimum track width. These are KiCad's own defaults. Isolation between domains is bounded by each domain's spacing to ground, so one minimum covers it. A voltage-derived spacing (IPC-2221) was not built, because its values would come from memory, not a ruled source | `rules.rs` | gap in the specification | Add a fabrication-class or spacing field to the specification in a later work order, with a cited source for any voltage table | **yes** |
| B2 | **KiCad writes `(connect_pads (clearance …))` on every zone it saves**, so D4 refuses every KiCad-authored board with a zone. Emitted boards must omit that token, and the zone clearance then falls to the board default | `board.rs`; D4 | consequence of a ruling | `1302` emits zones without it, and measures what KiCad does then | yes, for `1302` |
| B3 | ST's pin table and KiCad's symbol name package pins 28 and 29 differently (`PC2`/`PC2_C`, `PC3`/`PC3_C`). The emitter therefore draws each solved pin at the pin table's **package position**, never by name match. A test pins `SPI_MISO` to `PC2` by hand, and another checks that every ST position is a symbol pin with the names agreeing but for those two | `schematic::emit`, `emit_tier1.rs` | measured here | none | no |
| B4 | `emit` refuses a specification that uses a name the emitter needs for itself: `GND`, or `<ref>_SDO`, `<ref>_WP`, `<ref>_HOLD`, `<mcu>_VCAP1`/`2` (`ReservedName`). It refuses rather than renames, so a net's name is never changed behind the specification | `schematic.rs` | design | none, unless renaming is preferred | no |
| B5 | Tests that gave `check drc` a non-board (`"x"`, or 16 MiB of NUL) now give it a minimal board, because the gate parses the copy before any tool runs. Each test still proves what it proved. The files are `descriptors_tier1.rs`, `lifecycle_tier1.rs` (×2), `mcp_tier1.rs` (×4), `sockets_tier1.rs` (its targets are now lines inside `(kicad_pcb …)`), and `tools_tier1.rs`'s byte-cap test (whitespace-padded to exactly 16 MiB) | tests | incidental input change | none | no |
| B6 | `check drc`'s `input.sha256` stays the hash of the input's bytes. The copy KiCad judges has the gate's setup block in place of the board's, a function of those bytes alone. The CLI README says so | `kicad::prepare` | contract wording | none | no |
| B7 | The vendored footprints are format `20260206`. The tier-2 test boards here are `20240108`, as the spike's were. `1301` chooses the emitted board version | data | for `1301` | `1301` measures `20260206` boards | no |
| B8 | TMP117's `ALERT` (open collector) gets a no-connect, and WSON-6's exposed pad 7 has no symbol pin. Both matter at placement | `schematic.rs` | for `1301` | `1301` connects the exposed pad | no |
| B9 | The vendored symbols are KiCad's source files, while the image carries libraries built from them, so the two could differ. Run 42427541833's `fixed` ERC with library tables raised no `lib_symbol_mismatch` against the image's symbols. That was the spike, not these files. The tier-2 ERC test judges the emitted schematic with these files embedded | D1 | residual | A later image work order could pin the image's library hashes against these | no |

**AET-WO-1200's FEEDBACK, promote or needs none.** That work order was closed upward in #43, so only what this work order touches is listed:
- **S4.1, `gpio`:** consumed. The emitter draws each SPI device's `cs` from `gpio`. Needs none.
- **S1.7, byte-order keys:** they decide which pins the emitter draws, deterministically. Needs none.
- **B7, the unrenamed tool-list test:** done here. `the_five_tools_come_from_the_tree` is now `the_tools_come_from_the_tree`, with seven names.
- **All others:** needs none for this work order.

## Adversarial pass (§5, full, on the rule injection)

**How it was run.**
- **Reviewer:** a fresh Opus subagent in an isolated worktree, at `b7b4e58`, given the code and the ruling, not the builder's conclusions.
- **Measured, not inferred:** it ran real KiCad 10.0.7 in the tools image, which a local rootless podman on Furnace holds. That image's digest is `sha256:74f69bd3…`, exactly `container/IMAGE`. The builder checked the digest.
- **Kept as regression tests:** its tests are `crates/aethelgard-eda/tests/adversarial_1300.rs` and `crates/aethelgard-cli/tests/adversarial_1300.rs`, adjusted to the fixes below. Every case it demonstrated now asserts the secure outcome.

| # | Finding | Severity | Fixed by | Decision-needed |
|---|---|---|---|---|
| A1 | **The gate's reader and KiCad's disagreed in three ways**, so a board could hide its own setup block and pad clearances from the scan while KiCad honoured them. The verdict was **`pass`** with `"rules":"specification"` on a board with a 0.05 mm violation. The three ways (measured):<br>• KiCad skips a line whose first non-blank byte is `#`, so `#"` … `#"` lines made the rules a string to the gate only;<br>• KiCad reads NUL as whitespace, so `(\0setup …)` passed;<br>• KiCad keeps a `"` inside an atom (`(descr x"y) (clearance 0.01) (descr z"w)`).<br>Either hidden piece alone was still caught; the bypass needed both | **critical** (`INV-07`) | The reader's tokenizer (`sexpr::tokens`) refuses each construct (`INV-06`):<br>• a control or non-ASCII byte outside a string;<br>• a raw line break or control byte inside one;<br>• a `#` line;<br>• a `"` touching an atom.<br>`board` also refuses any list whose head is not a lowercase keyword. KiCad itself will not load an upper-case or quoted keyword, and the gate no longer relies on that | no |
| A2 | Footprint tokens that **waive a check**: `net_tie_pad_groups` turned two clearance, a short and two mask bridges into nothing (measured; verdict `pass`), and `allow_soldermask_bridges` removed a mask bridge. `jumper_pad_groups`, `duplicate_pad_numbers_are_jumpers yes` and `allow_missing_courtyard` are the same kind (unmeasured) | high | All refused (`BoardWaiver`). The vendored footprints say `duplicate_pad_numbers_are_jumpers no`, which stays | **yes**: `RUL-03` names only `(clearance …)`; ratify the wider refusal |
| A3 | Mask and paste margins (`solder_mask_margin`, `solder_paste_margin`) are not refused. They are the design's geometry, which DRC judges, not waivers | — | kept deliberately | yes, if they should be |
| A4 | A reference starting with `#` (`#PWR001`, `#U9`) was emitted. KiCad treats it as a virtual symbol and drops it from its netlist and BOM (measured: 14 rows against the emitter's 15), and `#PWR001` collided with the emitter's own power symbol | medium (`INV-01`) | `emit` refuses it (`ReservedName`). `RUL-14` still admits `#`; whether to narrow it is the architect's | yes, low |
| A5 | A 16 MiB board (the input cap) of `()` peaked at 663 MiB, and of `(a)` at 1.4 GiB | low–medium | `sexpr::MAX_TOKENS`, 1,000,000 tokens, refused past it. Peak RSS is now 40–92 MB for the worst 16 MiB boards, debug build, `/usr/bin/time` on Furnace. A board over the cap fails closed; this engine's boards have tens of parts | yes: raise the cap or stream the scan when boards grow |
| A6 | A reference such as `=1+1` is a spreadsheet formula in the BOM's first cell | low | not changed: prefixing would alter the data `INV-01` ties to the specification. Recorded here | yes, low |
| A7 | `(options (clearance outline))` on a custom pad was refused as a local clearance | low (it failed closed) | Allowed exactly for `outline` and `convexhull`; every other clearance, however spelled (`5e-2`, `.01`, `+0.01`), is refused | no |
| A8 | Plausible, unmeasured: `check erc` does not scan the schematic, so a hierarchical sheet naming a file outside the scratch copy would be judged without being hashed. Landlock's read-only system paths include `/etc` | low (`INV-03`) | out of this work order's rule-injection scope | yes, for a later work order |
| A9 | Plausible: `generator_version` is the crate version, so an emission depends on the engine's version as well as the specification | none | intended: a new engine may emit differently | no |

**What it tried and the code withstood** (kept as tests):
- the board's own `.kicad_pro` and `.kicad_dru` beside it, a `.kicad_dru` under another name, a symlinked board and project. None reaches the judge; the scratch listing is exactly the gate's files;
- a setup, net class or padstack-layer clearance nested anywhere;
- `\"` and `\\` escapes;
- trailing data, a byte-order mark, a second top-level list;
- deep nesting;
- TOCTOU: the original is read once into a 0700 scratch directory, and only the copy is used;
- emission under `env_clear`, other locales, time zones, `SOURCE_DATE_EPOCH`, working directories, out paths and specification layouts, which is byte-identical;
- every identifier character `RUL-14` admits, through the `.kicad_sch` and the BOM, which RFC 4180 parses back to five fields per row;
- project names `.`, `..`, `-` and `~`, which stay inside `--out`.

**Mutation (diff-sized, one mutant per guard this work order added, 31):** every mutant fails its suite. That covers the tokenizer's seven lexical refusals and the reader's depth and token caps; the board's clearance, net-class, waiver, odd-list and setup guards; the hash check; the dropping of `ki_fp_filters`; the identifier narrowing; the emitter's reserved names, `#` references, addresses and pin-by-position; the board minimum; the board preparation; `--refill-zones`; `O_NOFOLLOW`; and CSV quoting. Each run was in a `systemd-run --user --scope -p MemoryMax=32G` cgroup.

## Gates (build)

All on Furnace at the PR's code head, `origin/main` `6525506` merged:
- **Workspace gates:** pass. `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --locked -- -D warnings`, with and without `--features aethelgard-cli/with-tools`; `cargo test --workspace --locked`.
- **The `tools` job's steps in the pinned image**, run locally under rootless podman (`--network none`, `--pids-limit=-1`). Rootless podman's default PID limit makes the parallel tier-1 suite fail to spawn with `EAGAIN`; that is this harness, not the code:
  - `cargo fmt`, `cargo clippy --features aethelgard-cli/with-tools` and `cargo test --features aethelgard-cli/with-tools` pass, 32 suites, exit 0.
  - **Tier 2 there:**
    - ERC on the emitted `sentinel-01`: `"verdict":"pass"`, no findings.
    - The D5 row: the board's own project gave 0 clearance violations; the gate's project gave 1, "board minimum clearance 0.2000 mm". **So the project step is load-bearing**, as ruled, and the fallback (`clearance_min` in the gate's setup) is not needed.
    - S3's loose board fails with one clearance finding, under both `specification` and `kicad-default`.
    - The S5 names read back from KiCad's netlist unchanged: `/I2C(SDA)`, `VDD"3V3\`, `/CS"(0)`, `U(2)`, `U"3`, `U\4`, the title `p"(x)\#`.

  This is a pre-check, not acceptance; CI's run is cited below.
- **CI:** [run 42436232690](https://github.com/ClearForge-LLC/aethelgard/actions/runs/42436232690), commit `ccbec65`, the first push after the ruling: `gates` success, `tools` success (§3.3). Every suite is `ok`. Its log carries the same `AET-WO-1300` lines as the local run: ERC `pass` on the emitted `sentinel-01`; the D5 row 0 then 1 ("board minimum clearance 0.2000 mm"); the loose board failing under both rule sources; the names read back unchanged. §1.4 passes in `gates` and `tools` (§3.2). After the second reduction pass, the final code's [run 42436890377](https://github.com/ClearForge-LLC/aethelgard/actions/runs/42436890377) (`ee0ddec`) is green again in `gates` and `tools`, with every §1.4 test `ok`.
- **Reduction pass (diff), run twice by the builder:**
  - **First,** while the adversarial reviewer was running and so before its fixes: −51 lines net in the reduction edits (tests excluded). 5 duplicates removed: the specification reader, sha256 hex ×2, the reference counter, and a one-use trait. 1 dead constant removed.
  - **Second,** over the final code, after the adversarial fixes: −30 lines net.
    - The fixes had added a second string scanner, `lexically_plain`, which restated `tokens()`'s escape rule. Its refusals now live in the tokenizer, which scans each byte once.
    - Folding them in made the string-start quote check dead, since the atom scan always meets a glued `"` first, so it was removed.
    - The "the board, or the enclosing item" naming, written three times in `board.rs`, is one closure.
    - Each moved guard's mutant was re-run against the new code and fails its suite. One was untested before, a control byte after a backslash, and now has a case.
  - **Totals:** 8 duplicates removed · 2 dead paths removed · 0 protected kept · 1 finding: the test helpers `Dir`, `Temp` and `json_line` are copied across test binaries, kept because each binary compiles alone.
