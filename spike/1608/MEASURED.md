# AET-WO-1608 §1.1 — measured (2026-11-23, Furnace, main 34f48d61)

Predictions: `PREDICTIONS.md`, committed first (b2a19751). Harness: `corpus.py`, run as
`python3 -I spike/1608/corpus.py --bin target/debug/aethelgard --rows <rows.jsonl> <specs>`
against the debug build of 34f48d61. Rows are in `~/.cache/aet-1608/{examples,fixtures}.jsonl` (not committed).

## 1. The corpus: one key deleted, or one string emptied, at a time

Base set: every `schema_version: 28` file that `validate` accepts as committed. Skipped as refused at
base: `examples/invalid-zero-domain.json` (`ZeroNominalVoltage`, by design) and
`ir/tests/fixtures/d2-drv8833.json` (`PartLimitUndeclared`).

| | `examples/` (5 specs) | | test fixtures (28 specs) | |
|---|---:|---:|---:|---:|
| **mutants** | **2,391** | | **14,058** | |
| `SchemaViolation` | 1,354 | **56.6 %** | 9,070 | **64.5 %** |
| `InvalidIdentifier` | 402 | 16.8 % | 2,942 | 20.9 % |
| `UnwitnessedAcknowledgementStale` | 360 | 15.1 % | 897 | 6.4 % |
| `InvalidSource` | 106 | 4.4 % | 458 | 3.3 % |
| `UnwitnessedClassUnknown` | 65 | 2.7 % | 161 | 1.1 % |
| `UnwitnessedRegisterMalformed` | 65 | 2.7 % | 161 | 1.1 % |
| `LayerTable`, `DielectricTable` | 10 + 10 | 0.8 % | 66 + 76 | 1.0 % |
| `Lifecycle*` (three kinds) | 9 | 0.4 % | 81 | 0.6 % |
| passes (`OK`) | 7 | 0.3 % | 45 | 0.3 % |
| `InvalidJson` (exit 3) | 1 | | 7 | |
| everything else (10 kinds) | 2 | | 65 | |

**By mutation (examples):** a deleted key is `SchemaViolation` 1,324 of 1,334 (99.3 %); an emptied string
is `InvalidIdentifier` 402, `UnwitnessedAcknowledgementStale` 360, `InvalidSource` 106 of 1,057. **No
emptied string passes `validate`** in either corpus (0 of 1,057; 0 of 5,179).

**By distinct field** (array indices folded, examples): `SchemaViolation` 161 field paths,
`InvalidIdentifier` 42, `InvalidSource` 13, every other kind 1. `UnwitnessedAcknowledgementStale`'s 360
are one field path (`acknowledge_unwitnessed[].fields[]`), inflated by sentinel-01's 30–35 entries.

**`SchemaViolation`'s text** (both corpora, 10,424 rows): `missing field \`x\`` 10,117 (97 %);
`unknown variant \`\`, expected …` 296; `missing field … / missing field …: an idle holds exactly one`
11. serde's text names the key and a line and column, never the path.

**The `WO §7` stop condition is met:** the corpus's most common kind is `SchemaViolation`.

## 2. Two findings the corpus turned up

- **F1, pre-existing: a well-formed file refused as `InvalidJson`, exit 3.** Deleting the one key of an
  externally tagged enum object leaves `{}` (`mcu.hse.external`, `mcu.hse.crystal`, a strap's `tie`,
  an analog `full_scale.milliamps` or `transfer.current_output`): 8 mutants. serde_json reports
  `expected value at line L column C`, category `Syntax`, so `parse_spec` (`main.rs`) calls it
  unreadable input, not an invalid specification (exit 2). Out of this WO's fence (another refusal's
  bytes); a backlog row.
- **F2: the 7 deletions that pass** are `#[serde(default)]` fields a specification may omit:
  `mcu.reset_switch`, `test_points`, `board.drawn`, `spi_devices[].orientation`. None is a value a
  person must state.

## 3. Candidate templates

| example | bytes | compact | leaves | person-owned prose | ack / lifecycle | drawn |
|---|---:|---:|---:|---:|---|---|
| `sentinel-02-bench.json` | 5,893 | 3,903 | 128 | 9 | 0 / 0 | yes |
| `sentinel-02-bench-hse8.json` | 6,410 | 4,334 | 134 | 11 | 0 / 0 | **no** (twin stand-in) |
| `sentinel-01-ext-bmp581.json` | 9,457 | 6,032 | 231 | 11 | 0 / 1 | yes |
| `sentinel-01-minimal.json` | 17,802 | 12,436 | 418 | 37 | 30 / 1 | yes |
| `sentinel-01.json` | 22,133 | 15,382 | 520 | 44 | 35 / 1 | yes |

"Prose" is every string whose key ends `source`, `condition`, `reason` or `statement`, less the enum
`peer_deviation_source`.

**`sentinel-02-bench`, each candidate placeholder emptied alone** (from the corpus rows):

| path | kind | refusal's `path` |
|---|---|---|
| `project_name` | `InvalidIdentifier` | same |
| `mcu.core_supply.source` | `InvalidSource` | same |
| `i2c_devices[0].pin_capacitance.declared.condition` | `InvalidSource` | same |
| `i2c_devices[0].pin_capacitance.declared.source` | `InvalidSource` | same |
| `spi_devices[0].pin_capacitance.declared.condition` | `InvalidSource` | same |
| `spi_devices[0].pin_capacitance.declared.source` | `InvalidSource` | same |
| `board.stackup.finished_thickness_um.source` | `InvalidSource` | same |
| `board.stackup.dielectrics[0].source` | `InvalidSource` | same |
| `board.stackup.copper_max_um.source` | `InvalidSource` | same |
| `board.stackup.relative_permittivity_max_permille.source` | `InvalidSource` | same |

In `sentinel-01-minimal`, an emptied `acknowledge_unwitnessed[i].reason` is `InvalidSource` with its own
path (30 of 30), but an emptied `accept_lifecycle[0].statement` is `LifecycleStatementInvalid` with **no
`path`**: it would fail by kind, not by path, unless that refusal gains a path (another refusal's bytes).

## 3a. The bench's 30 numbers, each set to 0 (mutant `zero`)

28 refuse; **7 name a `path`**: `board.rules` x4 (`RuleBelowFloor`, `UnsupportedViaRule`) and
`StackupValue` x3. The other 21 name an `id` or nothing: `UnsupportedSchemaVersion` (`schema_version`, not a
person's value), and
`ZeroCapacitance` 4, `ZeroRiseTime` 4, `FinishedThickness` 3, `PinCapacitanceOutOfRange` 2,
`UnsupportedLayerCount`, `ZeroPullUp`, `UnsupportedI2cSpeed`, `I2cAddressOutOfRange`, `ZeroSpiSpeed`,
`ZeroNominalVoltage`, `ToleranceOutOfRange`. `mcu.bor_level` and `mcu.hse_hz` **pass** at 0 (already 0 in
the example: 0 means not programmed, and no HSE).

## 4. The package

`cargo package -p aethelgard-cli --list --allow-dirty` with `crates/aethelgard-cli/templates/bench.json`
present lists `templates/bench.json` (229 files), with no `include` change in the manifest (it has
none). `include_str!("../templates/<name>.json")` from `src/init.rs` resolves inside the package,
which is what CI's include-target gate (`ci.yml:163-171`) checks. The probe file was removed.

## 5. Tests that pin today's texts (what a text change touches)

`is not an identifier`: 9 lines in 6 files. `is not a source`: 3 lines in 2 files.
`an acknowledgement names exactly`: 1 line. `missing field`: 32 lines in 10 files.

## 6. Predictions scored

| # | Prediction | Measured | |
|---|---|---|---|
| P1 | deletion over 80 % `SchemaViolation` | 99.3 % | held |
| P2 | under 10 % of deletions pass, the defaulted vectors | 0.5 %, but `reset_switch`, `test_points`, `drawn`, `orientation`, not the acknowledgement vectors | count held, fields wrong |
| P3 | empties: `InvalidIdentifier`, then `InvalidSource`, then `SchemaViolation` | `InvalidIdentifier`, then `UnwitnessedAcknowledgementStale`, then `InvalidSource` | **wrong order** |
| P4 | an emptied `condition` passes | none passes; `condition` is `InvalidSource` with its path | **wrong** |
| P5 | most common kind `SchemaViolation` | 56.6 % / 64.5 % | held |
| P6 | the bench, eight placeholders | the bench, **ten**: the prediction named every one, `project_name` included, then counted them as eight | count wrong |
| P7 | `templates/` packages with no manifest change | yes | held |
