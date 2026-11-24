# AET-WO-1609 — FEEDBACK: one "ran or not" table in every verdict, and the foreign judge's late panic

**Branch:** `wo/aet-1609-ran-or-not` from `main` `e5984c86`; `main` `9b246a6a`, then `ee239949` (`4240e`), merged.
**Builder:** Claude (Opus 5.5) as project manager.
- **Subagents** did the read-only parts: the measurement's gate and reader inventories, spot-checked at source (one
  corrected), and an independent adversarial review of the build (§5).
- **Single-threaded:** the table's derivation, the judge's hook and every edit. They are gate logic.

Written 2026-11-23.

**Status: READY** (§9).
- **Predictions:** `spike/1609/PREDICTIONS.md`, committed `170cae45` before any measurement.
- **Measurement and stop:** `spike/1609/RESULTS.md`, at `0810faea` on PR #376, ruled 20:58Z on the bus.
- **Mutants:** `spike/1609/mutants.py`, with `spike/1609/sh/ci-mutants.sh`.

**The answer:**
- **Every `verify` verdict carries `ran`.** It has one row per gate `verify` can run, each followed by its parts. Each
  row reads exactly one of:
  - `judged n` (0 included, never omitted);
  - `off: <key>` (the specification key, or for a keyless gate the flag or condition that turns it on, as ruled);
  - `not_reached: <why>` (the reason the gate itself carries).

  **Derivation:**
  - The rows come from the gates `verify` pushed and from each gate's own line. `GATES` says only what an absent gate's
    row reads.
  - A gate pushed under a name not in `GATES` stops `verify` (exit 70). A source scan and a planted gate fail tests by
    name.
  - The `--against`-differs verdict, printed before any gate, lists every gate as not reached, with the reason.
- **The verdicts of `extract`, `check drc` and a generated `simulate` state their parts' rows.** The parts and the
  predicates that turn them on:
  - `extract.lines` (always);
  - `extract.supply` (a non-empty `power.drop_limits`);
  - `extract.oscillator` (`mcu.hse.crystal`);
  - `extract.pairs` and `drc.pairs` (`diff_pairs`);
  - `drc.power` (`power`);
  - `simulate.hse_clock` (`mcu.hse`).

  Each is the predicate the gate's own code tests, driven both ways, the key present but off included. A gate that
  states parts and ran without them is refused.
- **On every committed board the supply part reads `off: power.drop_limits`.**
  - Asserted on Sentinel-01's seeded `verify`, and on Sentinel-03's `extract`, which has `power` but no limit.
  - Judged, by its domains, when a limit is declared.
- **The late panic, a confirmed false pass, now fails by name.**
  - The foreign judge hooks `AP_HAL::panic` by its symbol, reads every line of the run for an entry, and keeps the first
    past the 8 MiB budget. A run that enters it is `foreign_panic`, with each entry in `boot.panic.entered_ns`.
  - An ELF whose symbol table does not name it is refused before any boot (`ForeignPanicSymbolMissing`, exit 3).
  - `ci/foreign-judge.sh` adds two cases: the panic planted after U5's last witness (passed before, fails by name now),
    and the symbol renamed (refused). Every earlier case is unchanged against main's CI.
- **The verdict "bump"** is an additive key, as `2306` and `4230` did. No verdict carries a version (D6, ruled). No
  reader outside the fence reads `verify`'s line strictly. The bench package's `MANIFEST.json` `verdict_sha256` moves,
  through the dispatch (§8).
- **Unchanged:** no IR, generator, eda, twin or trace change; no gate judges anything new or different (the panic is the
  ruled exception, D7); no optional key made required.

## 1. The measurement (§1.1) and the stop (§1.2)

`spike/1609/RESULTS.md` holds it in full. In short:
- **Every gate and part, with its on/off site and key.**
  - Three gates have no specification key: `campaign` (the `--scenarios` flag), `attribute` (a run's outcome) and
    `prove`'s harness set (the generator's). That is the §7 stop.
  - Most "off" decisions are made in eda, the twin or svd-hal, and leave no trace in the gate's own result.
- **No verdict carries a version.** Nothing outside the fence breaks; the one pin that moves is the bench package's
  `MANIFEST.json`.
- **The late panic** passed: exit 0, measured with a control, a causal plant, hooks, and an identical instruction count
  in the explanatory re-run.
  - A thread-mode halt passed too.
  - A HardFault failed only by `memory_cap_exceeded`.
  - A software reset failed by name.

## 2. The rulings (20:58Z, `~/.cache/aet-bus/1609.md`) and what was built

| Ruling | Built |
|---|---|
| `foreign_panic`, a hook by symbol, the planted case its regression test | `foreign.rs` `panic_hook`, `panic_entry`, `PanicReport`; `elf.rs` `function`; `main.rs` `ForeignPanicSymbolMissing`; `ci/foreign-judge.sh` `late-panic`, `panic-unnamed` |
| Keyless gates read `off: <flag or condition>`, tested both ways | `campaign` `off: --scenarios`; `attribute` `off: --scenarios with a seeded fault that fails as expected beside a passing no-fault baseline`; `prove` `off: a Kani harness in the generated board crate's src/proofs.rs`. Each is judged when present and off when absent, in `verify.rs`'s tests |
| D1 (a): "off" derived in the CLI, beside each eda call, tested both ways | `extract.rs` `parts`, `main.rs` `drc_parts`, `simulate_parts`, each with tests covering the key absent, present but off, and on; and tests that the CLI's predicate is the gate's own (`Stack::of`'s pairs, the twin's `clock_rule`) |
| D2–D5 | The parts of D3; D4's `gpio`/`adc`/`bridge` rows; `judged_nothing` kept (D5) |
| D6: additive key | `ran` before `against_fab`/`fab` (the manifest's line is still the verdict without them) |
| D7: fault-vector hook and liveness as rows | §6 |
| F1 | **Not built: the premise was wrong.** Each oscillator net is judged by its no-via rule (`OscillatorNetVia`, eda `extract.rs:1360-1370`); only its item carries no `verdict` field. `extract`'s count is unchanged, and the oscillator row reads `judged n`. Told on the bus at 21:06Z |

## 3. Files

- **The table:**
  - `crates/aethelgard-cli/src/verdict.rs` (`Ran`, `RanState`);
  - `verify.rs` (`ran`, `GATES`, `Absent`, `ran_table`, `parts`, `PARTED`, `not_reached`, `none_ran`);
  - `extract.rs` (`parts`);
  - `main.rs` (`drc_parts`, `simulate_parts`).
- **The panic:**
  - `renode/foreign.rs` (`PANIC_SYMBOL`, `panic_hook`, `panic_entry`, `PanicReport`, the `RULES` row, `FOREIGN_LOG_FILTER.keep`);
  - `elf.rs` (`function`);
  - `main.rs` (`Ready.panic`, the refusal).
- **Tests:**
  - unit tests in each of those files;
  - `boot_tier1.rs` (the panic, the refusal, a late line);
  - `generate_tier1.rs` (`--against`'s table);
  - `supply_tier1.rs`, `hse_eda_tier1.rs` (the parts on real `extract` runs);
  - `verify_2k2_tier2.rs` (the reference's whole table), `verify_47k_tier2.rs`, `four_layer_fab_tier2.rs`, `board_tier2.rs` (the motor driver's `judged 0`);
  - `twin_tier2.rs` (the HSE row, and the foreign boots' ELFs given the symbol);
  - `common/verify.rs` (`ran`, `row`), `common/foreign.rs` (`with_panic`).
- **CI:** `ci/foreign-judge.sh`.
- **Docs:** `crates/aethelgard-cli/README.md` ("The verdict", "Foreign firmware", `verify`, exit codes).

## 4. WO §5, the adversarial items, on the build

- **A gate that reports `judged n` without having judged.** Every count is taken after its gate's loop, of items that
  each carry a verdict or a refusal:
  - supply domains, each judged against its limit;
  - oscillator nets, each by the no-via rule;
  - pairs: `route::pairs` refuses one out of bounds;
  - `drc.power`: the `loops::judge` records;
  - `simulate.hse_clock`: 1, only once the boot word is written.

  The review checked each predicate against eda and the twin and found them the same.
- **A table built from a list a new gate can miss.**
  - The rows come from the gates pushed. `GATES` holds only what an absent gate reads.
  - An unknown pushed gate is exit 70, by name. A source scan of `verify.rs` checks every name pushed. A planted gate
    fails by name.
  - Residual (review 9): a gate built as a `Gate { gate: "x", .. }` literal escapes the scan. The runtime refusal still
    stops it.
- **A panic the judge still passes:**
  - **after the last read:** fails now (`late-panic`);
  - **in a thread:** the planted panic is in the SPI bus thread, and fails;
  - **a HardFault the exception check misses:** it fails, but by `memory_cap_exceeded`, never by `core_in_exception`
    (§6, row R3);
  - **a watchdog reset:** a software reset fails by name (`core_in_exception`, `pc_outside_firmware`, …; RESULTS §3).
  - **a thread-mode hang that never calls the panic** passes (row R2).

## 5. The independent adversarial review (read-only, a fresh Opus subagent given the code, not my conclusions)

Ten findings, each checked at source:
1. **`--against`-differs printed `"ran": []`.** Fixed: every gate is not reached, with the reason. A tier-1 test runs
   `verify --against` and checks the table. Mutant M27.
2. **The committed head failed the tier-1 foreign tests.** Already fixed in the working tree when it read it: the
   fixtures carry the panic symbol (`common::foreign::with_panic`), committed in `bb407d49`. The first host suite run
   had found the same 8 failures.
3. **A panic line printed after the begin marker was ignored.** Fixed: every line is read (only the CPU writes it).
   Tier-1 test, M29.
4. **`entered_ns: []` meant both "none" and "not read", and the line could be dropped past the budget.** Fixed:
   `PANIC_MARK` is in `FOREIGN_LOG_FILTER.keep`, and `entered_ns` is `null` until the log is read. M30, M31.
5. **A part could vanish unseen.** Fixed: `extract`, `drc` or `simulate` that ran and states no part is refused. M28.
6. **`prove` with no harness reads `off`, though the run fails `no_harness`.** Kept as ruled. Row R6 asks whether it
   should read `not_reached`.
7. **The HSE row's reason at a bound was wrong.** Fixed: `the run ended at a bound, so its end was never read`. M32.
8. **The hook's address comes from the judged ELF.** Stated in `RULES`'s "Weakens" (a decoy symbol hides the panic;
   `INV-07`: it locates the hook only). Row R4.
9. **The scan can miss a literal-built gate.** A residual (§4).
10. **The rename comment in `foreign-judge.sh` was wrong.** Fixed: the rename is now at `panic`'s `p`, as the comment
    says.

## 6. Rows for the backlog (ruled into FEEDBACK at 20:58Z, and found since)

- **R1: a fault-vector hook (D7 (b)).** Hook the vector table's fault entries (HardFault, MemManage, BusFault,
  UsageFault, read at the ELF's VTOR), so a fault is named, `foreign_fault`, before the log budget or the memory cap can
  stand in for it. It needs a measurement of whether the hook can pause the machine before a 1 GiB kill discards the
  finding.
- **R2: liveness (D7 (c)).** Each witnessed device logs again in the window's last part: `foreign_device_silent`. It
  catches a thread-mode hang (it passes today) and a panic of any kind, but misses one in a thread that reads no device,
  and its rule depends on timing.
- **R3: the HardFault that fails only as `memory_cap_exceeded`.** It fails closed, but for the wrong reason.
  CrashCatcher polls USART1's ISR, and `2311`'s access log logs every poll. Measured peak RSS:
  - 2.59 GB with the log;
  - 1.10 GB without it, still over the 1 GiB cap;
  - the control is at 844 MB.

  So `2311`'s log has a memory cost, and something else holds the rest.
- **R4: the decoy symbol (review 8).** A firmware whose symbol table names a decoy `_ZN6AP_HAL5panicEPKcz` hides its
  panic. A cross-check of the hook's address against the ELF's call sites, or against a pinned ArduPilot build, would
  close it.
- **R5: `judged_nothing` is derivable from `ran`.** Kept by D5, since three tests read it. Removing it is a reduction
  the architect can rule.
- **R6: `prove` with no harness** reads `off`, as ruled. Review 6 argues for `not_reached: no_harness`, since `INV-03`
  requires the proof.
- **From RESULTS:**
  - **F2:** the via-ring rule is a threshold, not a gate; `4300k`'s.
  - **F3:** `spi_devices[].orientation`, `board.max_outline_um` and `power.loads[].reference` each silently skip a
    `validate` or emit-time check outside any verdict.

## 7. Gates and mutants

- **The CLAUDE.md gates** (§9 gives the head they ran on):
  - `cargo fmt --all -- --check`: clean.
  - `cargo clippy --workspace --all-targets -- -D warnings`: clean on the host, and in the image with tier 2 compiled
    (`--features aethelgard-cli/with-tools`).
  - `cargo test --workspace`: 3,025 passed, 0 failed, 173 suites, on the merged head `c36776ee`; 2,995 passed, 0
    failed, on `1d9a6078` (host, each in a capped scope with swap 0).
    - The first host run, on `fffadf1a`, failed 8 tier-1 foreign tests: their fixture ELFs had no symbol table, so the new
      refusal stopped them. The refusal stayed, and the fixtures gained the symbol.
- **Tier 2, locally in the pinned tools image** (16 GiB scope, swap 0), each green:
  - `verify_2k2_tier2` (the reference's whole table), `verify_47k_tier2`, `four_layer_fab_tier2`;
  - `board_tier2`'s motor driver;
  - `twin_tier2`'s HSE boot and the three foreign-flash boots.
- **`ci/foreign-judge.sh`**, run whole in the image on main's CI ELFs (`ardupilot-image` run 45245344589): every case
  holds. `late-panic` is `foreign_panic`; `panic-unnamed` is `ForeignPanicSymbolMissing`.
- **Mutants:** 32 of 32 unit and tier-1 mutants killed, each by a named test (`spike/1609/mutants.py`). Three more
  killed where only CI or the twin can:
  - the finding demoted to info: killed by `late-panic`;
  - the refusal dropped: killed by `panic-unnamed`;
  - the HSE row's switch: killed by `twin_tier2`'s HSE test.
  - One first attempt did not compile (M26's sum lost its type). It was re-cast as the per-pair count, and killed.
- **Reduction pass (diff):** −5 net · 1 duplicate removed (`extract`'s tests now share `main`'s fixture loader) · 0
  dead paths removed · 0 protected kept · 3 findings below.
  - `FOREIGN_LOG_FILTER.keep` restates `LOG_FILTER.keep` plus the panic mark. Rust cannot concatenate const slices
    cleanly, so a test pins the superset.
  - `tests/common/verify.rs`'s `GATES` is the tests' own oracle; private consts cannot be imported.
  - `elf.rs`'s `with_symbols` and `common::foreign::with_panic` are in different crates.

## 8. The package

`ran` moves the verdict line, and so `fab/sentinel-01-bench/package/MANIFEST.json`'s `verdict_sha256`. As `2306`'s
did, `bench-package` was red on each head before its package commit.
- **First dispatch** (asked 21:44Z, queued 22:10Z, `ci.yml` run 45276370686 on `1d9a6078`): only `MANIFEST.json`
  differed, at line 48. It was committed in `d22ad30a`, and CI went green on `da75a39b` (tier 2 206, floor 205).
- **`4240e` merged first** (the architect, 23:10Z) and moved the same package. Main `ee239949` was merged, and the
  conflict was taken as main's.
- **The fresh dispatch** (queued 23:47Z, run 45291563000 on `c36776ee`, `bench-package` green) is committed byte for
  byte.
  - Of its 17 files, only `MANIFEST.json` differs from main's, at line 48: `verdict_sha256` `59e26a4b…` (4240e's)
    becomes `6ed83396…`.
  - The ten ordered files (`ORDERED.sha256`, `3308`) are unchanged, and `bench_board_tier1` passes.
- **`ardupilot-image`** on `1d9a6078` (45272646998) and on `da75a39b` is green. Its `foreign` job ran `late-panic`
  (`foreign_panic`) and `panic-unnamed` (`ForeignPanicSymbolMissing`).

## 9. Signal

**READY.** CI is green on `7c94f103`, the package head with main `7d48feb6` merged:
- `ci` 45295427079: tier 2 206 passed, floor 205; `bench-package` green against the committed package.
- `ardupilot-image` 45295427086 and `engine-image` 45295427062: green.

The last push is the commit "AET-WO-1609 READY: …", which changes only this file. Nothing is pushed after it unless
asked. The architect merges; the PR's title and body are in `~/.cache/aet-1609/pr-title-body.md`.
**`4240e` merged first;** main was merged here and the package re-dispatched.
**`2313` (#380) carries the boot bound's text on the same `foreign.rs` lines** (the architect, 23:52Z): whichever of
the two merges second re-applies.
