# AET-WO-1610 — FEEDBACK: no harness is not "off", three silent skips, an enum left `{}`, and a stale acknowledgement that quotes the ledger

**Branch:** `wo/aet-1610-cli-backlog` from `main` `72d85eef`; `main` `7bf6163d` and `1cb994c1` merged (§7).
**Builder:** Claude (Opus 5.5) as project manager.
- **A fresh Opus subagent** did the independent adversarial review (§5). It was read-only, and given the code and the
  WO, not my conclusions.
- **Single-threaded:** every edit. The table's derivation, the outline judge and the parse path are gate logic.

Written 2026-11-24.

**Status:** READY, pending the bench package dispatch (§7).
- **Predictions:** `spike/1610/PREDICTIONS.md`, committed `5ea10b95` before every run.
- **Measurement and stop:** `spike/1610/MEASURED.md`, `STOP.md`, at `16e639cf` on PR #385. Ruled 10:00Z on the bus.
- **Mutants:** `spike/1610/mutants.sh` (host), plus the tier-2 ones in `spike/1610/tier2-mutants.sh` (§6).

## 1. What was built, to the ruling

- **(1) R6: `prove` with no harness reads `not_reached: no_harness`.**
  - `verify` pushes a `not_run` `prove` gate whose reason is the finding's kind, `no_harness`.
  - `GATES`' `prove` is now `Absent::Never`. A run that pushes no `prove` gate stops (exit 70); it is not read as off.
  - The verdict still fails, by the error finding `no_harness`.
  - Through the CLI this path is reachable only by a unit test: the generator always writes at least two harnesses.
- **(2) R5: `judged_nothing` is gone. D5 is accepted, and the `ran` doc comment says it.**
  - A gate that judged nothing is its `judged 0` row.
  - A droop run that witnessed no device says so only at `campaign[i].observation_witness`, which the table does not
    carry.
  - Its three tests read `ran` instead:
    - `verify::tests::the_table_names_exactly_the_gates_that_judged_nothing` (renamed from `judged_nothing_names_…`);
    - `verify_2k2_tier2`: no row is `judged 0`, and the key is absent;
    - `board_tier2::verify_names_what_judged_nothing_on_the_motor_driver`: the `judged 0` rows are exactly `extract`
      and `extract.lines`, and the droop is read from its run. The name is kept, so `ci/test-weights.tsv` does not move.
- **(3) F3, with absent keys reading `off: <key>` (D1).**
  - **`drc.orientation` (D2):** a part of `check drc`.
    - With `--spec`, it is `judged n`, where n is each declared IMU seat the read-back judged (`ImuOrientationDisagrees`).
    - It is off by `spi_devices[].orientation` when none is declared.
    - The count is `Stack::of`'s `imus`, the declared set. Clippy's `RUL-16` ban kept the CLI off the specification's
      device list, and it was right to.
  - **`drc.outline` (D3):** with `--spec` and `board.max_outline_um`, `check drc` reads the board's `Edge.Cuts` from
    the strict tree.
    - It must be one `gr_rect` and nothing else on `Edge.Cuts`, read by `layer` or `layers`, of any `gr_*` shape.
    - Its width is compared with the bound's width and its height with the bound's height, as the placer's
      `OutlineUnreachable` compares.
    - Over the bound: `OutlineExceedsBound`, exit 3. No single rectangle: `BoardUnreadable`, exit 3.
    - Row: `judged 1`, else off by `board.max_outline_um`. Sentinel-03's own board passes at exactly its outline, and a
      bound 1 µm short in either direction is refused.
  - **`power.loads[].reference` (D4):** no row. `SupplySinkUncounted` outside a drop limit is a backlog row (§6).
- **(4) F1: `VariantMissing`, exit 2 (D6).**
  - When serde reports a syntax error on a document that parses whole as a `Value`, and its position is the `}` of an
    empty object, the path is found by a byte walk (`json_path_at`). The variants are serde's own: the object is
    re-read as `{"": null}`.
  - It covers every enum a specification reaches, unit enums included. A key of letters, digits and `_` reads bare;
    any other is quoted, as in `straps["P.M[1]"]`.
  - Any other syntax error on a well-formed document is `SchemaViolation`, exit 2. A document that is not JSON
    (trailing data, invalid UTF-8) is still `InvalidJson`, exit 3.
  - The kind is declared in `aethelgard-ir` (`IrValidationError::VariantMissing`), the WO's fence for (4).
  - **Correction to `MEASURED.md` §4:** for a strap, the enum left `{}` is the strap itself
    (`pin_devices[0].straps.PMODE`, variants `tie` or `resistor`), not `….PMODE.tie`. `tie` is the variant the corpus
    deleted. The tests assert the right paths.
- **(5) F2 (D7):** `UnwitnessedAcknowledgementStale` names each stale field by its index (`fields[6]`).
  - It ends with the ledger's own fields for that register and class, quoted from the ledger's rows. When there are
    none, it ends `the ledger has no field of this register as <class>`.
  - The no-ledger branch names the step that failed (no pin table, `solve` refuses it, the ledger cannot be built),
    never that step's text.
  - `id` stays the register, which `register_name_ok` shape-checks.
- **The CLI README (Q8)** states the table's place (after `gates`), `prove`'s row, the four `drc` parts, `VariantMissing`,
  the two new `check drc` refusals and the stale message.

## 2. Files

- **`crates/aethelgard-cli/src/verify.rs`:** `NO_HARNESS`, `no_harness`, `GATES`' `prove`; `judged_nothing` removed;
  the `ran` doc comment; tests.
- **`src/main.rs`:**
  - `DrcJudged`, `drc_parts` (four parts), `outline_judged`;
  - `variant_missing`, `well_formed`, `json_path_at` and `parse_spec`'s branch;
  - the no-ledger stale text;
  - tests.
- **`src/ledger.rs`:** `Stale { at, held }`, its message and `stale`.
- **`crates/aethelgard-ir/src/lib.rs`:** `IrValidationError::VariantMissing`.
- **Tests:** `validate_cli.rs`, `ledger_tier1.rs`, `outline_tier1.rs`, `with_tools.rs` (the turned IMU's
  `drc.orientation`), `verify_2k2_tier2.rs`, `board_tier2.rs`.
- **`crates/aethelgard-cli/README.md`.**
- **`spike/1610/`:** the predictions, the measurement, the stop, the probes, `mutants.sh`.

## 3. WO §5, the targeted adversarial items, on the build

- **A gate that still reads `off` when nothing turned it off.** None found.
  - `prove` is always pushed.
  - `campaign` is off only without `--scenarios`.
  - `gpio`, `adc` and `bridge` are off only when the boot lists none, and a boot-less simulate line stops `verify` (70).
  - `drc.orientation` and `drc.outline` are judged whenever `--spec` sets their keys: `prepare` always returns a tree
    for DRC.
- **A planted specification string reaching F2's message.**
  - Found once, in the no-ledger branch (`solve`'s text), and fixed.
  - A planted field, and a planted pin name that passes `InvalidIdentifier`, are each tested never to reach it.
- **An F3 field set to skip its check in silence.**
  - `orientation` and `max_outline_um` are judged whenever set.
  - A second outline drawn with `layers`, or as a `gr_circle`, slipped past the first outline reader. Fixed and tested.
  - `reference` is silent by ruling (D4).

## 4. The independent adversarial review (read-only, a fresh Opus subagent given the code and the WO)

Each finding was checked at source before acting on it.

1. **Defect, fixed: specification text reached the stale message.** The no-ledger branch quoted `solve`'s refusal,
   and a pin name that `InvalidIdentifier` admits (``Z`):_fix:_delete_every_…``) read as an instruction. The message
   now names the step that failed. A test plants exactly that pin (mutant M13).
2. **Defect, fixed: non-UTF-8 input moved from exit 3 to exit 2.** My well-formedness check used serde's `IgnoredAny`,
   which checks neither UTF-8 nor number range. It is now a whole `Value` parse (`well_formed`). Tested with a `0xFF`
   byte (M12).
3. **Defect, fixed: the outline check could be evaded.** It read only `(layer …)`, so a second outline written with
   `(layers …)`, or as a `gr_circle`, passed. It now counts every top-level `gr_*` on `Edge.Cuts` by either spelling,
   and requires one `gr_rect`. Tested (M7, M8).
   - The reviewer also called refusing a second `Edge.Cuts` rectangle (a cutout) an over-refusal. It stays: under a
     declared bound, the outline must be one rectangle to be judged against a width and a height. The engine draws
     exactly one, and `extract` and the fab path already refuse more. Row R3.
   - A board whose only outline is spelled `layers` is refused earlier, `BoardUnreadable` "no outline", by `eda`'s own
     `route::read` in the power gate. That is pre-existing and outside the fence. Row R4.
4. **Residual: an enum given `[]`, a number or `null`** is `SchemaViolation` with no `path`. That is not `{}`, so it is
   outside D6. Row R1.
5. **Nit, fixed:** a map key went into the path unescaped (`straps.P.M[1]`). A key of letters, digits and `_` reads
   bare; any other is quoted (`straps["P.M[1]"]`, M11).

**Held, by the reviewer's probes:**
- `prove` is pushed on every path to `conclude`.
- Every off row has its key, flag or condition.
- `VariantMissing`'s path is correct across CRLF, CR, tabs, 4-byte UTF-8 before the error (serde's column counts
  bytes), escaped keys and nested arrays.
- No input panics the walk.
- The outline's units and comparison match the placer's.
- No reader of `judged_nothing` is left.

## 5. Rows for the backlog

- **R1: an enum given `[]`, a number or `null`** is `SchemaViolation` with serde's line and column and no `path`, as
  every other `SchemaViolation` is (1608's measurement: 97 % of the corpus). A path for every `SchemaViolation`, from
  the same byte walk, would be the general fix.
- **R2: `SupplySinkUncounted` outside a drop limit** (D4, ruled). The rule that every drawn current sink is named by a
  load runs only inside `extract.supply`, which is off on every committed board. I could not measure it on the committed
  power specifications, because `extract` refuses first, for inputs they do not declare.
- **R3: an outline with a cutout.** Under a declared bound, a second `Edge.Cuts` graphic is refused `BoardUnreadable`.
  The engine draws none, so this matters only if the emitter ever draws a cutout or a non-rectangular board.
- **R4: `eda`'s `route::read` reads only `(layer …)`.** The strict reader takes `layers` for `layer`, so a board
  whose outline or a track is written with `layers` is read differently by the two. The power gate refuses such an
  outline as "no outline". It is outside the fence (`eda`).
- **R5: a footprint's own `Edge.Cuts` items** (an `fp_line` on `Edge.Cuts` inside a footprint) are not read by the
  outline judge. No vendored footprint has one; a hand-edited board could.
- **R6: the topology reader** still refuses an enum left `{}` as `InvalidJson`, exit 3. Measured: `solve` of
  `examples/sentinel-02-bench-hse8.json` with `clock.hse.declared` set to `{}`, then `generate hwdef --topology` gives
  `InvalidJson` "the topology: expected value at line 1 column 1846". It is the same fix, in another parse path; F1
  was ruled for the specification.

## 6. Gates and mutants

- **The CLAUDE.md gates on the merged head `aa80e023`:**
  - `cargo fmt --all -- --check`: clean.
  - `cargo clippy --workspace --all-targets -- -D warnings`: clean. Clippy's `RUL-16` ban caught my first
    `drc.orientation`, which read the specification's device list; it now counts `Stack::of`'s seats.
  - `cargo test --workspace --no-fail-fast`: 3,057 passed, 0 failed, 173 suites (host, 28 GiB scope, swap 0).
    `56479780`, before the review's fixes, was 3,056 passed, 0 failed.
- **Tier 2, in the pinned tools image** (`container/IMAGE`, 16 GiB scope, swap 0, read back), on `9b48e3d8`:
  - `with_tools::a_turned_keepout_is_where_kicad_draws_it` (`drc.orientation judged 1` and `drc.outline` off, each turn);
  - `board_tier2::verify_names_what_judged_nothing_on_the_motor_driver` (153 s);
  - `verify_2k2_tier2`, the reference's whole table with `drc.orientation` and `drc.outline` off (2 passed, 490 s).

  **And on the final source, `db547325`:** the same three, green (turned IMU; motor driver, 98 s; reference, 2 passed,
  380 s).
  - **A false red on the way:** the first final run failed the motor driver, with `erc` `judged 0`. That was mutant
    T3's binary, reused. `git archive | tar -x` stamps files with the commit's time, older than T3's build, so cargo
    saw nothing newer.
  - Re-extracted with `tar -m`, the run is green. The tier-2 mutant script now extracts with `-m`.
  - The mutant kills stand. Each mutation rewrote its file with a fresh mtime, and all three are in one crate, which
    cargo rebuilds whole.
- **Host mutants, 15 of 15 killed** (`spike/1610/mutants.sh`; only cargo's exit 101 counts as a kill):

  | # | guard | killed by |
  |---|---|---|
  | M1 | `no_harness` pushes the `prove` gate | `prove_with_no_harness_is_not_reached_never_off` |
  | M2 | `prove` is `Absent::Never` | the same |
  | M3 | the reason is the finding's kind | the same |
  | M4 | `judged 0` stays `judged 0` | `the_table_names_exactly_the_gates_that_judged_nothing` |
  | M5 | `drc.orientation` keyed on the seats judged | `the_seat_and_outline_parts_are_judged_or_off_by_their_key` |
  | M6 | the outline's height is compared | `check_drc_judges_the_declared_outline_on_the_board` |
  | M7 | `layers` is read as `layer` | the same |
  | M8 | the one outline is a `gr_rect` | the same (its single-circle case was added after M8 survived) |
  | M9 | an empty enum is looked for | `an_enum_left_empty_is_refused_by_its_kind_and_path` |
  | M10 | an array index advances | `the_path_of_an_empty_object_is_read_at_its_byte` |
  | M11 | an odd key is quoted | the same |
  | M12 | well-formed means a whole `Value` | `an_enum_left_empty_…` (the `0xFF` case) |
  | M13 | the no-ledger text names the step | `a_stale_unwitnessed_acknowledgement_is_refused_…` |
  | M14 | the ledger's fields of that class only | `ledger_tier1` |
  | M15 | the stale fields by index, never text | `a_stale_unwitnessed_acknowledgement_is_refused_…` |

  - M5's first run passed cargo two test filters (exit 1, an argument error), and the script then counted any non-zero
    exit as a kill. It now counts only 101, and M5 was rerun and killed.
  - That run also died part-way, because I edited the script while bash was reading it. M8 to M15 were rerun from a
    frozen copy.
- **Tier-2 mutants, 3 of 3 killed** (`spike/1610/tier2-mutants.sh`, each on a `git archive` of `db547325` in the image):
  - T1, the seats judged never counted: killed by `with_tools`' turned IMU.
  - T2, `judged 0` read as `judged 1`: killed by `board_tier2`'s motor driver.
  - T3, a gate judged whole counting 0: killed by `verify_2k2_tier2`.
- **Reduction pass (diff):** −10 net · 2 duplicates removed (`drc_parts`' two count forms became one; a refusal asserted
  twice on one input) · 1 dead path removed (a hand-kept counter, where `found.held.len()` gives the count) · 0 protected kept ·
  findings: none outside the fence beyond §5's rows. M5, M14 and M15 were rerun after it, all killed.

## 7. Main, and the package

- **`main` `7bf6163d` merged** (`aa80e023`): docs only (`AET-WO-2314`, the roadmap). No code moved and nothing
  conflicted.
- **`main` `1cb994c1` merged** (`5807f09b`, the architect's ruling of 12:20Z): `1324`'s router rule and index in
  `eda`, its tests, and docs. Nothing conflicted. Rerun on the merged head:
  - the CLAUDE.md gates: fmt and clippy clean; `cargo test --workspace` 3,068 passed, 0 failed, 173 suites;
  - tier 2 in the image: `with_tools`' turned IMU; `board_tier2`'s motor driver, 149 s; `verify_2k2_tier2`,
    2 passed, 402 s.
- `4300k`'s build is not on `main` yet.
- **The bench package:** removing `judged_nothing` and adding `drc.orientation` and `drc.outline` move the `verify`
  verdict line. So `fab/sentinel-01-bench/package/MANIFEST.json`'s `verdict_sha256` moves, and nothing else in the
  package should. It is never hand-edited: it needs the architect's `write` dispatch of `ci.yml`'s `bench-package`,
  committed byte for byte. Until then `bench-package` is red, as expected.

## 8. Signal

- **The independent review earned its cost.** It found three defects my own targeted pass had called clean:
  - my no-ledger branch quoted `solve`'s text;
  - my well-formedness check accepted non-UTF-8;
  - my outline reader looked at one spelling of one shape.

  Each was a guard written to the measured case, not to the class. "Quote nothing the entry supplies" was narrower
  than the WO's "nothing the specification supplies".
- **Two process errors, both mine and both caught:**
  - A mutant script that counted any non-zero exit as a kill reported an argument error as one.
  - Editing that script while bash was running it killed the run part-way.

  Both are fixed in the script (only 101 counts; runs use a frozen copy), and both are stated in §6.
- **The spike's strap path was wrong** (`MEASURED.md` §4 named the deleted key, not the enum). The build's tests caught
  it on their first run; §1 corrects it.
- **The ruling's D1 reversal of the WO's own wording** (`off`, not `not_reached`, for an absent key) kept the table one
  rule. Without it, three new rows would have read differently from the dozen beside them.
