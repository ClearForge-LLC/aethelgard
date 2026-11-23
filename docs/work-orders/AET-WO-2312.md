# AET-WO-2312 — Replay and attribute bound to the records: one function derives each finding kind's count, so a record set that shows a failure never replays as a pass; attribute near-linear or refused by name

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-2312-replay-bound-to-records`.
**Author:** Claude (architect) · **Builder:** strong tier, with a full adversarial pass.
**Phase:** P4, from the round-3 review. Written 2026-11-23. **Cadence:** a measurement, then a build with one stop. **After `2310`** (#346, trace 19), merged.

**Grounds** (`docs/reviews/2026-11-23-mythos-5-2.md` A8, A15, B6, and its response):
- **A8, the review's re-run at `7fbe5d11`.** One archive's records show `GPIOB.AFRL` observed `0x47000000` against `0x44000000`, and its verdict is empty. `replay` prints `pass`, `matched: true`, exit 0, while `attribute` says `I1`. `RUL-19` (3) says that replay proves integrity.
- **At `cd79024c`, checked:**
  - `check_run` (`chaos/src/campaign.rs:1134-1619`) binds the analog, held-low, boot, brown-out, flash, motor, serial, pin and droop findings to their records.
  - `cpu_halted` is bound since `2310` (`:1421-1435`).
  - `*_transaction_unwitnessed` is bound from below only (`:1442-1455`, `2310` item 7; roadmap §7.1 L1111).
  - These are not bound to any record: `register_mismatch`, `i2c_timeout`, `spi_timeout`, the six `{i2c,spi}_{identity,configure,data}_mismatch`, the two `*_record_invalid`, and `locked_peripheral_access`. Before the boot word, `BOOTED_READ_FINDINGS` (`:129-140`) are only forbidden (`:1439`).
- **The derivation already exists, in the wrong crate.** `evolve`'s `Run::select` (`evolve/src/lib.rs:2031-2158`) derives each kind's events from the records, and `PER_EVENT` (`:779-809`) lists the kinds.
- **The dependency question (B6):** the shared function belongs in chaos. Chaos depends on no `svd-hal` (`chaos/Cargo.toml`). `locked_peripheral_access` needs RCC's address range, which `evolve` takes from `cmsis::register_facts()` (`lib.rs:2361-2362`).
- **A15, the cost:** `decide` (`lib.rs:2161-2168`) recomputes `faults(b)` over every record for every bus, once per scoped symptom. The sub-reviewer measured 35 s on a 2.5 MB archive and over 500 s on 5 MB (unverified). `attribute` is an MCP tool (`cli/src/mcp.rs:527`).
- **A15, the weak tests:** `evolve/tests/serial.rs:748` and `chaos/tests/battery.rs:580` assert a bare `is_err()` (the review's :741 and :467).
- **The reviewer's four archives** (`furnace:~/aethelgard-review3/target/r3/chaos/`) are trace 18. Since `2310` (`chaos/src/event.rs:112-116`) they are refused by version, so they cannot be fixtures as they are.

> **What this is:**
> - **(1) `campaign::shown`,** one `pub` function in `aethelgard-chaos`: for each finding kind, the records that show it, and whether the verdict's count must equal their number or only reach it.
> - **(2) `check_run` binds every kind (1) returns.** A verdict whose count disagrees is refused, with a reason that names the kind.
> - **(3) `attribute` calls (1)** in place of `select`, so `replay` and `attribute` cannot disagree.
> - **(4) `attribute` near-linear.** Index the records by bus once, or refuse past a stated size with a named `AttributeError`.
> - **(5) The two bare `is_err()` tests** assert their reason.
> - **(6) The reviewer's four shapes as tests,** re-made at trace 19.
> - Every refusal has a test, and every test a mutant it kills.
>
> It is NOT:
> - a trace or scenario format bump, unless §1.1 measures it necessary (a stop);
> - a change to the Renode judge (`cli/src/renode.rs`), which writes the findings;
> - an MCP timeout;
> - a `fab/` byte.

## 1. Build

1. **Measure first,** with predictions committed in `spike/2312/`:
   - every committed archive (`spike/*/out/**`, each refused by version or not) and every CLI-made archive (the campaign over `scenarios/` in the pinned tools image): `replay`'s verdict and `matched`, and `attribute`'s output, before the change;
   - for each kind in (1), its count from the records against the judge's on every CLI-made archive (prediction: equal on all);
   - whether `locked_peripheral_access` can be derived without `svd-hal`, from kind 11's `name` (`chaos/src/record.rs:521`);
   - `attribute`'s time against `replay`'s on archives of 0.5, 2.5 and 5 MB;
   - the four shapes re-made at trace 19 by the reviewer's mutation, and their sizes. Commit them under `crates/aethelgard-chaos/tests/fixtures/` if each is under 256 KiB. Otherwise build them by hand through the tests' `archived` helper (`evolve/tests/attribute.rs:241`).
2. **Stop with:** the table, the dependency answer, and (4)'s choice with its numbers.
3. **Build (1)–(6) to the ruling.** Re-run §1.1: every archive replays as before unless it is inconsistent.

## 2. Acceptance

- **The four shapes are refused** by `replay` (nonzero exit, a JSON error naming the kind) and by `attribute` (`NotACampaignRun`).
- **No committed or CLI-made archive's verdict, `matched` or `Attribution` moves.**
- **`attribute` on a 5 MB archive** finishes within 10× of `replay`, or is refused by name.
- **Mutants:** a kind dropped from (1); `==` weakened to `>=` for an exact kind; (3) back on `select`; (4)'s bound removed. A test kills each.
- **The CLAUDE.md gates pass, and CI is green.**

## 3. Fences

- **Fence:**
  - `crates/aethelgard-chaos` (`campaign.rs`, `replay.rs`, tests, fixtures);
  - `crates/aethelgard-evolve` (`lib.rs`, tests);
  - `ci/test-weights.tsv` rows;
  - `spike/2312/` and FEEDBACK.
- **Not:** the CLI's judge, `verify.rs`, `mcp.rs`; the IR, eda, the generators; `scenarios/`, `fab/`; workflows.
- **Running beside you:** the backlog wave in roadmap §5 (`4300j`, `1323`, `4408`, `1916` now). Merge `main` before READY.
- One PR, left unmerged. Write `~/.cache/aet-2312/pr-title-body.md`.

## 5. Adversarial pass, full

- **A record set that shows a failure and still replays as a pass:**
  - for each kind;
  - for each status spelling (`timeout_*`, `invalid`);
  - for a mismatch under a zero mask, which must show nothing.
- **A verdict inflated** past its records for an exact kind, or deflated for a lower-bound kind.
- **A run that did not boot** whose verdict names a booted-read finding.
- **`replay` and `attribute` disagreeing** on any planted archive.
- **B6's duplicate-record check** (`replay.rs:76`, `:118-128`): remove it only if no refusal's kind or index moves.

## 7. Flag-and-stop conditions

- **After §1.2:** stop with the table and the rulings it needs.
- **Any committed or CLI-made verdict moves.** Stop with the diff.
- **A trace or scenario format bump is needed.** Stop with why.
- **`locked_peripheral_access` needs `svd-hal` in chaos.** Stop: the dependency is a ruling.
- **The records and the judge disagree on a committed scenario.** Stop: one of them is wrong.
