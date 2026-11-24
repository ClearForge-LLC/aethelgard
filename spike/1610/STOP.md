# AET-WO-1610 — STOP at §1.1 (2026-11-24, main 72d85eef)

The measurement is `MEASURED.md` (predictions `PREDICTIONS.md`, committed before every run, 5ea10b95; the reader
searches behind P1–P3 were read before it). The predictions held. Two were corrected: R5 is only partly derivable, and
F1's set is the corpus's, not the types' (a unit enum given `{}` is `InvalidJson` too). **No `§7` stop on (1), (2), (4) or (5).** For (3), the
refuse-the-absence option for each field **would refuse committed specifications** (a `§7` stop), so the leans
below are rows, and one check that runs where the data exists (`max_outline_um`).

Nothing below is built. Decisions D1–D7, each with my lean.

## D1. The label for an absent key (all three F3 fields)

The WO says `not_reached: <reason>`. The table's own definition (`1609` (1)) is `off: <the key, flag or condition
that turns it on>`, and every absent key reads `off` today (`extract.supply off: power.drop_limits`,
`drc.pairs off: diff_pairs`, `simulate.hse_clock off: mcu.hse`). R6's ruling turns on *whose choice* the gap is:
a missing harness is no one's, so `not_reached`; an undeclared key is the specification author's.

- (a) `not_reached: <reason>`, as the WO words it.
- **(b) `off: <key>` — lean.** It matches the definition and every precedent, and keeps `not_reached` for
  "could not run", which is what R6 just made it mean.

## D2. `spi_devices[].orientation`

- **(i) A `drc` part — lean.** `check drc --spec` already judges a declared seat (`ImuOrientationDisagrees`); its
  line gains `drc.orientation`: `judged n` (each declared IMU it judged), else off by `spi_devices[].orientation`
  (D1's label). `verify` reads it as it reads `drc.pairs`. Inside `aethelgard-cli` (`drc_parts`,
  `stack.imus` is already public).
  - Residual: a board with two IMUs, one declared, reads `judged 1`. No committed specification has two.
- (ii) Refuse an IMU whose part cites `axes` and declares no orientation, on a drawn board. The data exists
  (`parts.json` `axes`). **It refuses `cli/tests/fixtures/sentinel-03.json`, `hse-crystal-16.json` and
  `sentinel-02-shaped.json`.** `generate hwdef` already refuses the same absence (`ImuOrientationUndeclared`), so
  only the board path is silent today. Sentinel-03 is `4300k`'s board.

## D3. `board.max_outline_um`

- **(i) Judge it on the board, in `drc` — lean (the WO's preferred kind, where the data exists).** With
  `--spec` and a declared bound, `check drc` reads the board's one `Edge.Cuts` `gr_rect` from the strict tree it
  already holds, and refuses one wider or taller than the bound by name (a new CLI kind, `OutlineExceedsBound`,
  path `board.max_outline_um`). Today the bound is judged at `validate` and `emit` only, never on the file a gate
  reads. Its row is `drc.outline` `judged 1`, else off by `board.max_outline_um`.
  - **The reader is narrow on purpose.** `route::read` and `extract::read_tree` would also give the outline, but
    each refuses far more (unmodelled top-level items, track widths, holes): `drc` would gain refusals beyond the
    outline, which the WO's NOT forbids. The CLI reads only the `gr_rect` on `Edge.Cuts`; its only other refusal
    is a board with no such rectangle, or more than one, under a declared bound.
  - **The same rectangle the placer judged:** `pcb.rs` writes the outline as that one `gr_rect` and refuses
    `OutlineUnreachable` when `x1 − x0` exceeds the width or `y1 − y0` the height. `drc` compares the same way.
  - Inside `aethelgard-cli`. It runs before KiCad, so it is tier-1 testable, as `ImuOrientationDisagrees` is.
  - Predicted to hold on Sentinel-03 (the placer placed inside the bound); measured at the build. If it refuses
    Sentinel-03, I stop again.
- (ii) The row alone, with no new judgement.
- Refusing the absence would refuse every drawn committed specification but Sentinel-03: not offered.

## D4. `power.loads[].reference`

The weakest of the three: there is no subject to judge. A load without a reference is an off-board or lumped
budget (the three committed ones: a 5 V port budget, a 1.8 V domain budget), validated for domain, source and
milliamps and counted in its domain's sum like every load. Only the part binding is skipped, and it has no part.

- (i) A row. There is no natural gate: no gate reads loads except `extract.supply`, which is off on every
  committed board. The row would be `extract.supply`'s, as a count of reference-less loads, and says little.
- **(ii) No row and no check, the measurement recorded — lean, if the architect accepts that this field is not a
  silent skip.** The real gap is a different check: `SupplySinkUncounted` (every drawn part drawing current on a
  domain is named by a load) runs only under a drop limit. I could not measure whether it would hold on the
  committed specifications (MEASURED §3), so it is a row for the backlog, not an option here.
- Refusing the absence would refuse `sentinel-03` and `ir/tests/fixtures/sentinel-02-power.json`: not offered.

## D5. R5: `campaign:<scenario>` after `judged_nothing` goes

The gate members of `judged_nothing` are `judged 0` rows. The `campaign:<scenario>` members are not in the table:
they stay at `campaign[i].observation_witness` (`nothing_in_scope` and its reason), which `board_tier2`'s test
already asserts whole.

- **(a) Accept that — lean.** Test 3 reads `ran`'s `extract` `judged 0` and the run's `observation_witness`.
- (b) The table gains a row per droop run that witnessed nothing.

## D6. F1: kind, path and mechanism

The corpus's eight mutants reach four data enums (`mcu.hse`, a strap's `tie`, an analog's `full_scale` and
`transfer`). By type, **every serde-derived enum given `{}` is `InvalidJson`, a unit enum too**
(`mcu.core_supply.mode = {}`, measured). The mechanism below covers every one of them, not only the four. Lean:

- **Kind `VariantMissing`, exit 2, `path` the enum's (`mcu.hse`, `pin_devices[0].straps.PMODE.tie`), the
  message naming the variants serde expects.** The kind is declared in `aethelgard-ir` (the WO's fence for (4)).
- **Mechanism, all in `parse_spec`, no new dependency:** when serde reports `Syntax` but the bytes are one JSON
  document, the file is an invalid specification, not unreadable input. A small span walk maps serde's line and
  column to the JSON path of the `{}` there. The variant names come from serde itself (the `{}` re-read as
  `{"": null}` gives serde's `unknown variant`, `expected …` text, measured), so no list is copied. Measured: serde's
  line and column land on the `}` of the `{}`. Any other `Syntax`
  error on a well-formed document becomes `SchemaViolation`, exit 2, never `InvalidJson`.
- The alternative, a hand-written `Deserialize` for each of the four enums in the IR, gives `Data` errors with no
  path (as `SchemaViolation` has none today) and puts the fix outside (4)'s fence.

## D7. F2: what the message says

Today it quotes the entry's stale fields, which are specification text (MEASURED §5: a planted field reaches it).
"Never from text a specification supplies" means that echo goes. Lean:

- **The stale fields are named by their index in `fields` (`fields[6]`), never by their text. The message ends
  with the ledger's fields for that register and class, from the ledger's rows:**
  `…; the ledger's fields of \`GPIOA.MODER\` as \`same-source\`: \`MODER0\`, \`MODER5\`, …`. The register is
  quoted from the ledger's own row; the class is the parsed enum's name.
- When the ledger has no row of that register and class, the message ends `the ledger has no field of this
  register as \`<class>\``, with no register text.
- This moves `validate_cli.rs`'s assertions that the message contains `` `MODER3` `` and `` `SADD` ``.
- The refusal's `id` stays the register (validated by `register_name_ok`, upper case, digits, `_`, one `.`).

## Not decisions, for the record

- (1), the way I would build it: `prove` with no harness pushes a `not_run` `prove` gate whose reason is `no_harness`, so the row is
  `not_reached: no_harness`, the gate list says it too, and `GATES`' `prove` becomes `Absent::Never`. The verdict
  stays `fail` by the finding. No version bumps (the verdict has none).
  - The table-only alternative: `GATES`' `prove` entry reads `not_reached: no_harness` when absent, and `gates`
    stays as it is. Either is fine by me; say if you want this one.
- (2) and (3)'s `drc` parts move the `verify` verdict line, so the bench package's `MANIFEST.json`
  `verdict_sha256` moves. **One `write` dispatch of `bench-package`, after `main` (with `4300k`) is merged in.**
- Beside `4300k` (verify.rs's `--fab-out`, validate, the IR): I add, never rewrite, and merge `main` before READY.

## Q8. The CLI README

(1) and (2) make `crates/aethelgard-cli/README.md` stale:
- line 960 says `ran` comes after `judged_nothing`;
- line 965 describes an absent gate's `off`.

`2306` treated that README as outside its fence. Is it inside this one?
- **Lean: yes.** The WO fences `crates/aethelgard-cli`, and the README is the verdict's reader-facing contract.

## Gates on this stop

Spike only: Python, shell and Markdown under `spike/1610/`; no Rust changed. `cargo fmt --all -- --check` was
run (clean). `clippy` and `cargo test` were not run: no code they judge moved.
