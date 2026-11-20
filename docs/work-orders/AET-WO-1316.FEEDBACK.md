# AET-WO-1316 FEEDBACK — the parts table and the emitter's arms agree by test, and the drawn design is read once

Builder: Claude (Opus 5.5), a fresh `1316` session, 2026-11-20, from ~06:50 UTC. Worktree `~/aethelgard-1316`, branch `wo/aet-1316-parts-arms` from `origin/main` `ff7fd53d`. The work order is read from `~/.cache/aet-1316-wo.md`, the architect's copy of the docs branch's `docs/work-orders/AET-WO-1316.md`. This branch does not commit it.

Labels: **measured** (run here), **read** (read at source), **inferred** (reasoned, not run). Evidence is under `~/.cache/aet-1316/`.

## 1. Inventory (§1.1, on `ff7fd53d`)

- **The parts table** is `crates/aethelgard-ir/data/parts.json`, embedded by `aethelgard_ir::parts::PARTS_JSON` and read only through `parts::table()` (`parse_parts`). It holds **7 rows**, sorted: `BME280`, `BMP581`, `ICM20948`, `ICM42688P`, `ICP10111`, `TMP117`, `W25Q64JVxIQ` [read; pinned by `parts.rs`'s own row-name test].
- **The emitter's arms** are two string matches in `schematic::design` [read]:
  - the I2C devices' `match d.part.as_str()` (`schematic.rs` L1393 on main): `BME280` L1394, `BMP581` L1420, `TMP117` L1440, then `_ => UnknownPart` L1455;
  - the SPI devices' (L1500): `W25Q64JVxIQ` L1501, `ICM20948` L1522, `ICM42688P` L1548, then `_ => UnknownPart` L1566.
  - **Per-part code outside the arms:** `if d.part == "BMP581"` (L1465, its second 100 nF), and the IR's `names::device_nets` (`BME280`/`BMP581` `SDO`, `W25Q64JVxIQ` `WP`/`HOLD`, `ICM20948` `REGOUT`). Neither names a part without an arm. Neither is in this agreement test; `device_nets` is IR code the test does not reach.
- **`VENDORED_PARTS`'s readers** (`ir/src/names.rs` L32): `IrValidationError::PartWithoutRow`'s message (`ir/src/lib.rs` L1886), and `schematic.rs` L580, which imported it as `KNOWN_PARTS` for the two `_ =>` arms' `UnknownPart.known`. The IR test `a_part_the_table_does_not_hold_is_refused` pinned the message text. No other crate read it [read: `grep` over `crates/`].
- **Rows with no arm:** `ICP10111` (the twin's stand-in; its arm is `4600` D4's own row). **Arms with no row:** none.

## 2. What is built

- **The arms' list lives beside the arms.** `aethelgard_eda::schematic::KNOWN_PARTS`, the same six names in the same order, byte for byte, so the emitter's `UnknownPart` message does not move. `names::VENDORED_PARTS` is removed: once `PartWithoutRow` names the table's rows, the IR has no reader of the arms' set.
  - **Why not keep it in the IR (§1.3's choice):** the set is the emitter's, and the crate graph allows eda to own it. `INV-01` holds: nothing about a specification is restated. The parts are named once in the table, and the arms are named once next to the arms, and the tests below hold each to the other.
- **`PartWithoutRow` names the set it means:** "`U4` is a `UNLISTED`, which has no row in the parts table; known: BME280, BMP581, ICM20948, ICM42688P, ICP10111, TMP117, W25Q64JVxIQ". The list is read from the table, in its order, by a private `table_parts()` beside `part_entry` (`ir/src/lib.rs`). The lead-in moved as well as the list: "has no vendored model; known: …ICP10111" would be false for ICP-10111. The variant's shape is unchanged (no new field), so the fence's "message only" holds. Kind (`UnknownPart`) and `id` are unchanged. `table_parts`'s `Err` arm prints the load's error and is unreachable in practice, since the refusal is built only after `parts::table()` loaded [read].
- **The agreement test**, `crates/aethelgard-cli/tests/parts_arms_tier1.rs`, **tier 1**, 0.12 s [measured]. It sits in the CLI test crate because it needs svd-hal's pin table and the solve; eda depends on neither.
  - **For each table row**, it builds Sentinel-01's minimal board with that row as its one device: `U2` on the I2C bus, or `U4` on the SPI bus, as the row's bus kind says. The fixture is fitted from the row's own data: the first address its straps offer; the SPI clock lowered to the part's maximum; its lifecycle accepted; and, where the row's supply or I/O range leaves out 3.3 V ± 5 %, the whole board at 1.8 V with brown-out reset off (ICM-20948 and ICP-10111). `validate` must pass it **before** `design`, so a refusal is the arm's and not the fixture's.
  - **It pins which:** a part in `KNOWN_PARTS` must draw, with its device on the MCU's supply. A row in `ROWS_WITHOUT_ARM` (`["ICP10111"]`) must be refused by `UnknownPart { reference, part, known: KNOWN_PARTS }`, the full variant, so the MCU's `UnknownPart { reference: "mcu" }` cannot stand in for it. Every `KNOWN_PARTS` name must have a row.
  - **The scan, `schematic::tests::the_arms_are_the_known_parts`** (eda's own in-file test, `include_str!("schematic.rs")`, as `the_emitter_keeps_no_copy_of_an_address` already does). It reads both `match d.part.as_str()` blocks, takes each `"X" =>` arm, and requires the arms to equal `KNOWN_PARTS` as a set, each name once. It catches an arm written for a part that neither the table nor the list names, which no drawing test can reach. It requires exactly two such matches, and refuses an or-pattern or guard arm by name, so it cannot pass vacuously. It lives in eda because CI's package gate refuses an `include_str!` reaching outside its crate.
- **One view: `schematic::Drawing<'a>`.** One `design()` call (`Drawing::of`), read by `anchored()`, `drawn()`, `drawn_nets()` and `bound_holds()`. It holds `&'a Specification`, and `of` ties `solved` and `table` to `'a`, so none of the three can change while the view is read (§4 A2, measured).
  - `schematic::anchored` and `schematic::bound_holds` keep their signatures and read the view; `bound_holds` still draws nothing for a specification that declares no bound.
  - `schematic::drawn` and `schematic::drawn_nets` are removed (the reduction pass, §5): after the CLI moved to the view, `drawn_nets` had no caller, and `drawn`'s one caller (`hot_plug_tier1.rs`) moved with it, reading `anchored` and `drawn` from one `Drawing`.
- **The CLI's `validate` draws once.** `outline_holds` and `loads_drawn` now take the view. A new `drawing_holds` solves (`quiet_solve`) and draws once, and only when either check has something to judge: a declared `max_outline_um`, or a load with a reference. Order is kept: acknowledgements, then the outline, then the loads. A specification that does not solve, or that the emitter refuses, still passes both silently, left to `solve` and `emit`, as before. `acknowledgements_current`'s own solve is untouched: it is not asked for, and its refusal depends on that solve's error.

## 3. Acceptance (§2)

| Item | Result |
|---|---|
| **The inventory** | §1. |
| **The agreement test passes** | `parts_arms_tier1` and `the_arms_are_the_known_parts` pass [measured]. |
| **Mutants** | All killed, each naming the part [measured: `mut/results.txt`]. M1, a row added with no arm (a copy of `BME280` as `BME281`): "BME281: a row the emitter has no arm for, not pinned in ROWS_WITHOUT_ARM". M2, an arm's part removed from the table (`BME280`'s row deleted): "BME280: the emitter has an arm for it, and the parts table has no row". Beyond §2: M3, an arm for a part neither the table nor the list holds (`"FOO"`): the scan, "FOO: an arm, not in KNOWN_PARTS"; the agreement test cannot see it, by design. M4, `TMP117` dropped from `KNOWN_PARTS` with its arm kept: the scan, and "TMP117: drawn, and not in KNOWN_PARTS: an arm unlisted". M5, `TMP117`'s arm renamed: both. M6–M8, `drawing_holds`'s gate `&&`→`\|\|`, the outline call dropped, the loads call dropped: killed by `load_references_tier1` and `outline_tier1`'s `validate_refuses_a_bound_that_cannot_hold_the_parts`. |
| **One `design()` per `validate`** | Counted with a temporary `eprintln!` in `design()`, built into a probe binary and never committed, over the 52-file corpus [measured: `count-main.txt`, `count-branch.txt`]. On main: 50 files 0 calls, 1 file 1 call, **Sentinel-03 2**. On the branch: 50 files 0, **2 files 1 each** (Sentinel-03 now 1). No file moved otherwise. |
| **Wall times** | Release binaries, interleaved three times at load 11–14 on shared Furnace [measured: `time.txt`]. Sentinel-03 (mean of 20 `validate`s): main 57.7–59.6 ms, branch 55.4–57.5 ms. Corpus (52 files, mean of 3 passes): main 273–279 ms, branch 276–283 ms. One solve and one draw saved on Sentinel-03, about 2 ms. The corpus is within noise, since only Sentinel-03 drew twice. |
| **No output moves** | `validate` on the corpus: byte-identical, 26 pass and 26 refused, none `UnknownPart` [measured: `corpus-main.txt` = `corpus-branch.txt`, `cmp`]. `emit` of Sentinel-03, `sentinel-02-power`, `sentinel-02-bench`, `sentinel-01` and `sentinel-01-ext-bmp581`: 15 files sha256-identical, and the five stdout lines identical up to their output directory [measured: `emit-main.sha`, `emit-branch.sha`]. The main hashes also equal `1209b`'s `emit-final.sha`. The bench package: `verify` does not reach `validate`'s checks, and the emitter's bytes did not move [read]; CI's bench-package compare is the run of record. |
| **Gates and CI** | §5. |
| **`TIER2_MIN`** | Unchanged at 194: no tier-2 test is added. |

## 4. Adversarial pass, targeted (§5)

**A1. A row whose minimal specification is refused for a reason other than its arm.**

- The fixture is checked by `validate` before `design`, and the test names the part and `validate`'s refusal if it fails. Four faults were planted in the test's fixture builder [measured: `adv/fixture-attacks.txt`]:
  - (a) an address the row does not offer: "BME280: its minimal specification is refused by validate, before any arm: … 0x10, which its straps do not offer";
  - (b) the SPI clock not fitted: "ICM20948: … above the part's 7000000 Hz";
  - (c) no 1.8 V fit: "ICM20948: … outside the part's I/O range of 1710 to 1950 mV";
  - (d) ICP-10111's refusal expected under another reference: fails, `assert_eq!` on the full variant (now naming the part).
- **A refusal inside `design` for another reason** fails as "pinned as a row with no arm, refused otherwise", or "listed as an arm, and not drawn", with the error. Only `UnknownPart` with the device's own reference and part, and `known` equal to `KNOWN_PARTS`, is accepted for ICP-10111.
- **The 1.8 V fit needs brown-out reset off.** At `bor_level` 3 the MCU's floor is 2680 mV; at 1, 2060 mV [measured: `DomainOutsideConsumer`]. The fixture is a test board, not a design.

**A2. A view that goes stale.**

- **After the specification changed:** a scratch crate that mutates `spec` while a `Drawing` from it is alive does not compile (E0502). The same code with the read moved before the mutation compiles [measured: `adv/stale/`]. Staleness is closed by the type, not by a convention.
- **A caller that still calls `design()` itself:** in `validate`, none. The probe count (§3) is at most one per specification. Outside `validate`, `design()` is still called directly by `loops.rs` (L41), `extract.rs` (L1124), `supply.rs` (L1398), `decoupling_picofarads` and `place` in `schematic.rs`. Each is a separate command's own single drawing (`emit`, `extract`, `check`), untouched and outside this fence [read].
- **`validate` still solves twice** for a specification with acknowledgements and a bound or a load reference: `acknowledgements_current`'s solve and `drawing_holds`'s. Before, it solved up to three times. This is F2.

## 5. Findings

| # | Finding | Kind |
|---|---|---|
| F1 | `crates/aethelgard-ir/README.md` step 14 says a no-row part is refused "by the emitter's kind and message". The message is now the table's own (§2); the kind is still the emitter's. The crate README is outside this fence, so it is not edited. A one-clause change ("by the emitter's kind, naming the table's parts") would fix it. | decision-needed: widen the fence to that clause, or leave it for the next IR work order |
| F2 | `validate` solves twice when a specification has `acknowledge_unwitnessed` entries and a bound or a load reference (§4 A2). `acknowledgements_current` could take the same solve. Not asked for; the refusal there depends on the solve's error text. | backlog |
| F3 | The agreement test's 1.8 V boards run with brown-out reset off (§4 A1). A test-only fixture, nothing committed. | note |
| F4 | `names::device_nets` names parts by hand, like the arms did. Nothing pins it to the arms. Out of this fence (IR names beyond the message). | §7.1 candidate |

## 6. Gates, the reduction pass, and CI

(filled in at READY)
