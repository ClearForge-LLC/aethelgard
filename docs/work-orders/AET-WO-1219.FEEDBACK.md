# AET-WO-1219 — FEEDBACK: how STM32-specific is the IR? (a measurement, STOPPED at §7)

**Branch:** `wo/aet-1219-stm32-coupling-audit` from `main` `8225ebd1`. Main had not moved at the last fetch.
**Builder:** Claude (Opus 5.5 orchestrating). Workers:
- five Sonnet first-lens classifiers, one per unit;
- two Opus blind second lenses, which were not shown the first lens's answers.

Every worker report was checked at source before use (§6). Written 2026-11-22.

**Status: STOPPED at §7 (after §1.4).** No crate, specification, `fab/`, workflow or `ci/` file changes. The only changes are `spike/1219/` and this file.

**The answer, in six lines:**
- **1,433 entries listed and classed:** 312 specification keys, 292 refusal kinds, 174 held tables and source items (390 table columns), and 265 output-type fields. The reader shows none is missing: every key of every committed specification and of its serde dump is a source key, and the reverse is listed by name.
- **The specification itself is barely coupled.** K: **V 291 · N 8 · S 13.** R: **V 248 · N 11 · S 33.**
- **The depth is in held data and code, not in the schema:**
  - H source items: S 33 of 174;
  - T output fields: S 67 of 265;
  - three of the four held tables are S (`power-parts.json`, `output-speeds.json`, the pin table);
  - 37 solver and judge functions hold ST facts in literals.
- **The 13 S keys:**
  - under `mcu`: `bor_level`, `core_supply` and `.mode`, `foreign_clock.vos` and `.vos.value`, `foreign_clock.usarts`, and the APB1/APB2 timer clocks;
  - outside it: the I2C, SPI and serial `peripheral` keys, and `acknowledge_unwitnessed[].register` and `.fields`.
- **The RP2350 column is "not held" in every one of its 227 cells** (every N and S entry, in all five units). No RP2350 datasheet is on the box (§3), and nothing was fetched.
- **Lean: no S key must change before the epoch.** One schedule question goes to Scotty: whether `mcu`'s ST-only keys move under a part-family tag at the epoch (§5).

## 0. How it was made

1. **The rule first.** `spike/1219/RULES.md` (the V/N/S/D rule) and `PREDICTIONS.md` were committed at `c9c35e09`, before any file under `crates/aethelgard-ir` was opened. `PREDICTIONS.md` lists what had been seen by then.
   - Four amendments followed, each dated with its reason in RULES §6. Amendments 1 and 2 came before any classing; 3 and 4 came after the first pass, to settle what the workers disagreed on.
2. **The reader** (`reader.py`, run by `run.sh`; no `Cargo.toml` under `spike/`). It parses every serde struct and enum in `crates/aethelgard-ir/src`. It honours:
   - `rename`, `rename_all`, `alias`, `flatten` and `skip`;
   - `tag` and `untagged`;
   - the hand-written `Deserialize` impls: `Mcu` reads through `McuWire`, `SolvedTopology` through `SolvedWire`, and `Idle`/`Esc` read their derive's shape.

   It walks four roots:
   - K from `Specification`;
   - H columns from each held table's envelope and row types (rows are parsed one by one from a `RawValue`);
   - the `PinTable` schema;
   - T from `SolvedTopology`, plus every serde type no other walk reaches.

   Refusal kinds come from every refusal enum's `fn kind`/`fn name` arms, nested matches included, checked against the enum's variants and against `versions::CHANGES[].rules`.
3. **The serde dump** (`dump.rs`, built by `run.sh` in its own workspace under `~/.cache/aet-1219`). It deserializes each file as `aethelgard_ir::Specification` and serializes it back.
4. **Cross-check.** Every committed current specification and its dump are walked along the schema. A data key the source lacks fails the run. So do:
   - a file path serde does not write back, except `mcu.hse_hz`, which is read only;
   - an unresolved type, unless it is explained;
   - a `Deserialize` type no walk reaches.
5. **Classing.**
   - **Lens 1:** five Sonnet workers classed every entry.
   - **Lens 2:** two Opus workers re-classed, blind, these 589 entries:
     - every K and H-column entry lens 1 had put in N or S;
     - every primitive-typed K entry lens 1 had put in V;
     - every MCU, speed and pin-table column;
     - all 292 refusal kinds.
   - **Settlement:** they agree on **550 of 589**. I settled the 39 disagreements one by one in `overrides.tsv`, each with its reason; `out/disagreements.md` shows both lenses' answers beside the final one.
   - **Generated files:** `merge.py` builds `classes.tsv`, and the reader's join writes `LIST.md` and `COUNT.md`. The join fails on an unclassed entry, on a class row with no entry, and on any reason naming a second chip.

**Reproduce:** `bash spike/1219/run.sh && bash spike/1219/mutants.sh`, then `python3 -I spike/1219/datecheck.py .` and `python3 -I spike/1219/lookups.py .`.

## 1. The list (§1.2) and the reader (§1.3)

- **The list:** `spike/1219/LIST.md`, one table per unit. Each entry cites its `file:line`; each key and column also cites where its value set is held or checked. Enum values are listed under their key, with every value no committed specification uses marked.
- **What "every committed specification" means here:** every git-tracked JSON with an `mcu` object, at version 27, that serde reads. That is **34**.
  - `versions.rs`'s own `current_specifications()` reads 30 from three directories. The other 4 are in `crates/aethelgard-cli/tests/fixtures/solve/`.
  - 26 older files (spikes, repros, `1215`'s inputs) are set aside: serde refuses each by its version.
- **The reader's result:** "every data key is a source key". In numbers:
  - 374 key paths seen in the files and 373 in the dump; the one more in the files is `mcu.hse_hz`, read only;
  - every path `versions::CHANGES` names is a source path;
  - every rule kind it names is a refusal kind read here.
- **Source keys no committed specification writes:** 16, named below for adversarial target 1 (§4).

## 2. The count (§1.4)

From `spike/1219/COUNT.md`:

| unit | V | N | S | D | `?` | total |
|---|---|---|---|---|---|---|
| K, specification keys | 291 | 8 | 13 | 0 | 3 | 312 |
| R, refusal kinds | 248 | 11 | 33 | 0 | 7 | 292 |
| H, held tables (4) and source items (170) | 103 | 6 | 33 | 32 | 2 | 174 |
| H columns, the three data files and the pin table | 369 | 12 | 9 | 0 | 1 | 390 |
| T, solved topology and other output types (not counted toward acceptance) | 163 | 35 | 67 | 0 | 19 | 265 |

- **The four held tables:**
  - `power-parts.json` is **S** for the MCU row's BOR levels and FT/TT structures;
  - `output-speeds.json` is **S** for `ospeedr` and the compensation cell;
  - the pin table is **S** for its AF numbers;
  - `parts.json` is **D**.
- **Per version.** `COUNT.md` has the K and R tables. Method: a key takes the version of the latest `versions::CHANGES` row naming the longest prefix of its path, a rename's `by` included; a key no row names is version 1.
  - **Checked against every committed versioned file** (`datecheck.py`). No key appears in a file older than its version, and each of the 44 version-1 keys appears in a committed version-1 file. Version 0 is set aside; it never existed.
  - **The 13 S keys:** versions 1 (2), 7, 9, 12 (2), 17 (3) and 19 (4). Versions 18 to 27 introduced 4 of them.
- **The 37 functions** that hold ST facts in literals (amendment 2) are in `out/lookups.md`. They are listed, not classed.
  - `is_observer_name` (`lib.rs:3031`) parses a port letter and pin number without a string literal, so the grep misses it. It belongs on that list.

## 3. The RP2350 column (§7: no datasheet held)

Every N and S cell reads "not held". The search and its positive control are in `spike/1219/RP2350.md`.

| Search | Result |
|---|---|
| File names across the filesystem | RP2040 material only, in the tools image |
| Every PDF's raw and zlib-inflated streams: 227 files, 43,358 streams | 0 hits |
| The same scan's control, `STM32H7` on DS12117 | found |
| Text extractions under `~/.cache` | 0 hits |

`pdftotext` is not installed. A first scan that relied on it returned zero vacuously, and it is not counted.

**Held and not used:**
- **The tools image's KiCad libraries** (`MCU_RaspberryPi.kicad_sym`) carry RP2350 symbols. A symbol's pin list is not the datasheet the work order names.
- **The ChibiOS port in the tools image** is for the RP2040, another chip.

**The document a ruling would need:** Raspberry Pi Ltd's *RP2350 Datasheet*, held by sha256 in `~/.cache/aet-ds/`, covering whatever it says on the list's N and S topics:
- pin function selection;
- pad structure;
- clocks and the crystal;
- core voltage;
- brown-out.

## 4. Adversarial pass (§5, targeted)

**Target 1: a key the reader misses.**
- **Source-side walk.** K comes from source, not data. A key absent from every file is still listed: **16** are, all reached from source.

  | Where | Keys |
  |---|---|
  | `board.stackup` | `outer_copper_min_um`, `plane_copper_min_um`, `plating_min_um`, `copper_temperature_max_c`, `dielectrics[].max_thickness_um` |
  | `power.drop_limits[]` | the container, `.domain`, `.max_millivolts`, `.source` |
  | `power.regulators[].controls[].wire` | `.net`, `.role` |
  | `serial_ports[].negotiated` | `.source` |
  | `pin_devices[].analog[].channel` | `.peripheral`, `.input` |
  | `pin_devices[].part_limits.input_pwm_max_hz` | `.value`, `.source` |

  None of the 16 is in the dump either: each is a `skip_serializing_if` option, so only a source walk could have found it.
- **Enum values.** **28** values no committed specification uses are listed and marked in `LIST.md`. Among them: `core_supply.mode` `bypass`, VOS `scale0`, `scale2` and `scale3`, `dshot300`, parity `odd`.
- **Types the walk cannot see into.** Each is explained: two `RawValue` row lists, each walked as its own row type, and `RawPinPart.identity`, refused unless null (`pin_parts.rs:432`).
- **Hand-written key checks.** Every `deserialize_with` keeps the key's shape:
  - `present`, `board::present` and `Option::deserialize` require a key;
  - `unique_keys` and `unique_keys_some` refuse duplicate keys.

  `CURRENT_OUTPUT_FIELDS` (`pin_parts.rs:616`) guards fields that `RawPin` also declares.
- **Mutants of this spike's own guards** (`mutants.sh`), one each, every one killed by its own message:
  - M1: a key planted in the dump;
  - M2: a serde field renamed in a source copy;
  - M3: an entry left unclassed;
  - M4: "RP2350" written into a reason.

  **The first run of M1 and M2 "failed" for the wrong reason.** `--repo .` gave relative paths, so no specification matched the dump's absolute keys, and the reader refused with "only 0 specifications". The reader now resolves `--repo`. The first `mutants.sh` also reported all four surviving, because `pipefail` turned the reader's exit 1 into a grep failure. Both were fixed and rerun before the counts above were read.

**Target 2: an entry classed V whose meaning is ST's.**
- **What lens 2 checked.** It re-read, blind, every primitive-typed K key lens 1 had put in V (170), and it grepped each value check.
  - It moved **no K key out of V.** Over all its entries it moved 3 out of V, all flagged: `R:InvalidIdentifier` N?, `R:PowerPartsTableInvalid` S?, and the `RawSupport.pins` column N?.
  - Lens 1's enum-typed V keys that lens 2 did not see (20) I read myself. All are generic: orientation, layer role, PWM idle, dielectric codes, UART parity and roles.
- **The one real miss is instance names.** `i2c_buses[].peripheral` was N? in lens 1 and S in lens 2, and lens 2 was right.
  - `solve.rs:1034` sends `I2C1`–`3` to `I2C123SEL`/pclk1 and `I2C4` to `I2C4SEL`/pclk4.
  - `spi_buses[].peripheral` (`solve.rs:962`) and `serial_ports[].peripheral` (`serial.rs:201`, `vos.rs:291`) do the same by instance number.
  - I had read only `silicon.rs:351`, the independent judge's reset-only copy, which treats all four I2C instances alike. Amendment 3 makes such keys S. It is applied to K, R and T alike.
- **The other direction.** 12 entries lens 1 had put in N or S are V by rule text:
  - tie-break g, a container classed on its own meaning: `foreign_clock`, `acknowledge_unwitnessed`, `AnalogInput.channel`;
  - tie-break c, a value set from a held table: `AnalogPadPin`, `UnheldDuringReset`, `PullAboveMcuTolerance`, `SerialBaudTolerance`;
  - rule 3, which tests the kind string rather than the message: `IdleSpelledTwice`, `reset_switch`;
  - amendment 3 and the requirement itself: `UnknownSerialPeripheral`, `ForeignClockUsarts`, `MissingSpiConstraint`.

  Each one's coupling is counted where it lives: the S column, or the D list.

**Target 3: an RP2350 answer from memory.**
- The reader fills every RP2350 cell itself; no worker wrote one.
- The join refuses any reason naming RP2350, Raspberry, Pico, ESP32, nRF and others (M4).
- `RP2350.md` names the topics a ruling needs from the list, not from that document.

**My own error, owned.** My prompt to the R second lens said "a fixed set of ST peripheral instances hard-coded in source" is S, which contradicts amendment 3. Its S? on `UnknownSerialPeripheral`, `ForeignClockUsarts`, `InvalidDeviceName`, `ReservedPin`, `UnheldDuringReset` and `PullAboveMcuTolerance` came from my wording. All six were settled by amendment 3 and tie-break c (V); see `overrides.tsv`.

## 5. The lean

**Lean: no S key must change before the epoch.** The specification's ST coupling is shallow. Its depth sits where a second vendor costs no specification bump:
- held tables, whose schemas carry their own version;
- register constants;
- solver and judge code.

### Why nothing has to change

- **The S keys under `mcu`** (8):
  - `bor_level`;
  - `core_supply`, `.mode`;
  - `foreign_clock.vos`, `.vos.value`, `.usarts`;
  - the APB1/APB2 timer clocks.

  `mcu` is already the part's own block. A second part is admitted additively: an enum gains a variant, or `mcu.part` selects a different reading. No key an existing file holds is renamed. The engine must then refuse an ST key under a non-ST part. That refusal is a bump (`1215` D4), so it belongs with the second vendor's own bump.
- **The S keys outside `mcu`** (5):
  - `i2c_buses[].peripheral`, `spi_buses[].peripheral`, `serial_ports[].peripheral`;
  - `acknowledge_unwitnessed[].register` and `.fields`.

  All are strings whose values come from part data or the twin's ledger. A second vendor adds values, not keys. The instance-to-clock mapping is code (`solve.rs:962`, `:1034`; `serial.rs:201`), not schema.
- **The N keys:** `hse`, `hse_hz`, `P<port><n>` pin values, ADC `peripheral`/`input`. Renaming any of them costs the same bump before the epoch as after, for a vocabulary no held document can replace. Leave them.

### The question for Scotty (a schedule question, not a measurement)

**Does `mcu`'s ST-only content move under a part-family tag at the epoch?** For example `mcu.family.stm32h7 { bor_level, core_supply, foreign_clock }`, the externally tagged form `mcu.hse` already uses.

- **For.** A second vendor then adds a sibling tag, additive under the epoch's rule. And from ST's side alone, none of the three is a quantity in units, so a second vendor's equivalent, if any, would need a different type, which is a bump after the epoch:
  - `bor_level: u8` holds ST's four `BOR_LEV` option-byte levels;
  - `core_supply.mode` holds RM0433 Table 34's rows;
  - `vos` holds ST's four scales.
- **Against:**
  - it renames 8 keys;
  - every committed specification moves: `bor_level` and `core_supply` are in 34 of 34;
  - the tag is designed with no second chip held, which is the speculation the work order itself warns against.
- **What decides it:** whether a second vendor is due before the next bump that cannot be avoided. Tagging at the epoch is cheaper than after it only in that case.

## 6. Predictions beside results

| # | Predicted | Measured | |
|---|---|---|---|
| P1 | K 320 (200–450) | 312 | hit |
| P2 | K V 72 % · N 12 % · S 16 % | V 93.3 % · N 2.6 % · S 4.2 % | miss: the schema is far less coupled than I expected |
| P3 | K `?` 5–15 | 3 | miss, low |
| P4 | R 180 (100–280) | 292 | miss, above range |
| P5 | R V 75 % · N 10 % · S 15 % | V 84.9 % · N 3.8 % · S 11.3 % | miss: less coupled |
| P6 | H 10 tables: D 6, S 4 | 4 tables (S 3, D 1) + 170 source items | **unit changed by amendment 1**; the predicted S tables (pin table, speed grades, VOS) are S, as source items or columns |
| P7 | S in `mcu`, clock tree, VOS, speed grades, pin structure, AF | yes, in held data and code. Not predicted: instance names (amendment 3) and the unwitnessed-ledger register names | partial |
| P8 | over half of the S keys from versions 18–27 | 4 of 13 | miss |
| P9 | 15–40 unwritten keys; at least one unused value | 16; 28 | hit |
| P10 | 1–4 S keys must change | 0, plus one schedule question | miss |
| P11 | RP2350 all "not held" | all "not held" | known in advance |

## 7. Decisions and findings for the architect

- **D1 · The serde dump links `aethelgard-ir` in a scratch workspace.** It is `~/.cache/aet-1219/dump`, with its own `[workspace]`, built from `spike/1219/dump.rs`. The CLI has no command that prints the parsed specification, and a JSON walk alone cannot see what serde writes back. Nothing under `spike/` is a crate.
- **D2 · "Every committed specification" is 34, not `versions.rs`'s 30.** The 4 extra are `crates/aethelgard-cli/tests/fixtures/solve/*.json`, at version 27, and they add no new key. `current_specifications()` does not read that subdirectory. Whether it should is a one-line row for a later work order; it is not changed here.
- **D3 · Amendments 1–4** (RULES §6). Ruling or reversing any of them changes counts as follows:
  - 1 and 4: H only;
  - 3: 4 K keys (the I2C, SPI and serial `peripheral` keys and `foreign_clock.usarts`), 6 T fields and 2 R kinds.
- **F1 · Held ST facts with no page.** Every source item classed D, S or N was checked for a page; `LIST.md`'s H table has a `cites` column.
  - **Pages found:** 42 by the extractor. The bench, `RCC_D2CCIP1R` and `vos:LIMITS` items were also found to cite through a group comment or a named constant.
  - **Document, but table or section only:**
    - `MAX_BOR_LEVEL`, `OFFSETS`, `TIMERS`, `TOLERANCE` (RM0433);
    - `I2C_AF_MIN_NS` (DS12117 Table 94);
    - `I2C_LIMITS` (RM0433 Table 390's note).
  - **No citation at all:**
    - `serial:PERIPHERALS` and `LPUART`;
    - `solve:RESERVED_PINS`;
    - the net names `NRST`, `BOOT0`, `OSC_IN`, `VBAT` and `DEBUG_NETS`;
    - `vos`'s two condition labels;
    - `speeds:LOADS_PICOFARADS` and `BANDS_MILLIVOLTS`: their values are the held `output-speeds.json`'s, which cites pp. 245–246, so these are copies.
    - `MODELLED_ADCS` and `H1_SYS_HZ`: each cites a work order, not a document.
    - `RESERVED_REFERENCES` and `OBSERVER_RULE`: engine names, which cite Renode's platform.
  - **The held data files:**
    - `power-parts.json`: 163 row citations, 129 name a page;
    - `parts.json`: 62 row citations, 30 name a page, most of the rest citing ArduPilot or Renode source files;
    - `output-speeds.json`: 2 of 2;
    - the pin table: none, since its rows cite a repository, commit and sha256.

  A backlog row, not this work order's to fix.
- **F2 · Out of scope, one line each, not audited:**
  - `aethelgard-svd-hal::pin_table` knows `STM32H753ZITx` alone;
  - `aethelgard-hwdef` refuses any ArduPilot series but `STM32H7xx` (`lib.rs:411`);
  - the twin runs Renode's STM32H753 platform (`RUL-08`).
- **F3 · Rule gaps the workers found:**
  - a container with mixed members (settled by tie-break g);
  - a table of ST facts with one register column (settled by amendment 4);
  - a datasheet parameter symbol such as `RAIN` or `tAF` as vocabulary (left N, as found).

  A second audit should start from RULES §6, not §2 alone.

## 8. Gates, fence, reduction pass

- **Gates not run.** The work order grants it: no crate, specification, `fab/`, workflow or `ci/` file changes, so `cargo fmt`, `clippy` and `test` are not run. `git diff --stat 8225ebd1 -- . ':!spike/1219' ':!docs/work-orders/AET-WO-1219.FEEDBACK.md'` is empty. No package dispatch.
- **What did run:**
  - `run.sh`: the reader, the dump and the join;
  - `mutants.sh`: 4 of 4 killed;
  - `datecheck.py`: 0 mis-dated.
- **Scratch:** `~/.cache/aet-1219`, 89 MB, most of it the dump's target directory. Removed after the PR.
- **Reduction pass (diff): −16 net · 1 duplicate removed · 6 dead paths removed · 0 protected kept · 0 findings below.** Counted from the removed and added lines of each edit.
  - The duplicate: `Crate.uses`'s first regex restated the two `use` loops below it.
  - The dead paths: the four `rename_all` branches the IR never uses (`SCREAMING_SNAKE_CASE`, `kebab-case`, `camelCase`, `PascalCase`), `Schema.root` and `Schema.flags`.
  - Two indirections were inlined, `member_offset` and the `getattr` alias set-up.
  - `version_of_path` is one `max`, and its docstring now says what it does: the longest prefix wins, and the later row on a tie.
  - The proof: `inventory.json`, `LIST.md` and `COUNT.md` were byte-identical by sha256 before and after each removal, and all four mutants stay killed.
