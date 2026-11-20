# AET-WO-2307 — The acknowledgement refusal names each register and class, and hands a person the entries to fill

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-2307-ack-entries`.
**Author:** Claude (architect) · **Builder:** standard tier, with a targeted adversarial pass.
**Phase:** P4. Written 2026-11-20. **Cadence:** a measurement, then a small build.

**Grounds** (`4600` F12; roadmap §7.1, the `4600` F12 and `2304b` rows):
- `verify --fab-out` refuses `UnwitnessedFieldNotAcknowledged` when a ledger row of an unwitnessed class has no entry in `acknowledge_unwitnessed` (`cli/src/verify.rs`, about L1468–1480).
- The refusal lists row ids only. A person must then work out each register and class, and the fields one entry has to cover. That costs one refusal per mistake: `4600` wrote 41 entries this way, 75 % of its specification.
- The easy path, copying the ledger's own `judge` text into `reason`, makes an acknowledgement say nothing the artifact did not (northstar §6; `INV-07` in spirit).
- Sentinel-03 has 280 such rows across 35 registers (`2304b`). Its acknowledgements are a person's work: this work order makes that work one pass, and does not do it.

> **What this is:**
> - The refusal gains `error.entries`: one object per (register, class) that lacks an entry, `{register, class, fields, reason: ""}`.
>   - Its fields are exactly the unacknowledged rows' fields, sorted. Entries are sorted by register, then class.
>   - It is the shape `acknowledge_unwitnessed` takes, so a person pastes it and writes each `reason`.
> - The message also states the reason rule: 1–200 bytes of printable ASCII, at most one entry per register and class.
> - An entry that already exists for a register and class but misses fields is reported as that entry, completed with the missing fields, and its existing `reason` is kept. Say how `validate` and the stale check treat the result.
>
> It is NOT:
> - writing any reason. `reason` is always the empty string, which `validate` refuses (`InvalidSource`), so a pasted entry cannot pass unread.
> - any acknowledgement in a committed specification;
> - any change to which rows need an acknowledgement, to the ledger's classes, or to the stale rule.

## 1. Build

1. **Measure first,** with predictions committed before the runs, in `spike/2307/`:
   - The current refusal on Sentinel-03 (`--fab-out`, scratch output): its row count, and the (register, class) pairs and field counts the new `entries` should hold.
   - **Prediction:** 280 rows and 35 registers, as `2304b` measured. Say how many (register, class) pairs.
   - A board built as `4600`'s M was, if its specification is held in `spike/4600/`. Its committed acknowledgements, with their reasons emptied, must equal `entries` field for field.
2. **The refusal:**
   - `error.entries`, added to the refusal's body only for this kind. Every other refusal's bytes stay as they are. Check the JSON error contract in `cli/README.md`, and update it.
   - The message names the count, the reason rule, and that `entries` holds the skeleton.
3. **Tests:**
   - A specification with no acknowledgements gives one entry per (register, class), each `reason` "", fields sorted, and the order stable.
   - Those entries pasted in with each reason filled by a test string pass the acknowledgement step. Pasted with reason "", they are refused by `validate` as `InvalidSource`, by name.
   - A partial entry is reported completed, its reason kept, and the result passes.
   - No entry's reason ever equals a ledger row's `judge` text, by test over Sentinel-03's ledger.
   - **Mutants:** drop a field from an entry; merge two classes into one entry; copy `judge` into `reason`. A test kills each.

## 2. Acceptance

- **The measurement** is in FEEDBACK, with Sentinel-03's (register, class) count.
- **Every other refusal's bytes are unchanged.** Run every refusal test, and say so.
- **The CLAUDE.md gates pass, and CI is green.** Say whether `TIER2_MIN` moves (Sentinel-03's run is tier 2).

## 3. Fences

- **Fence:** `crates/aethelgard-cli/src/{verify,ledger}.rs` and their tests, `crates/aethelgard-cli/README.md`'s refusal rows, `spike/2307/`, FEEDBACK.
- **Not:** `aethelgard-ir` (the acknowledgement's shape and `validate` stay), any specification, the ledger's classes, workflows, or `ci/` (except a weight row).
- **Running beside you:**
  - `4240b` (IR clock, specification 23, in the package queue);
  - `1211b` (generator, solved 16);
  - `1211d` (eda);
  - `1913` (CI weights);
  - `1211e` (IR `validate`, specification 24).
- Merge `main` before READY. One PR, left unmerged. Write `~/.cache/aet-2307/pr-title-body.md`.

## 5. Adversarial pass, targeted

- **A path by which an entry passes without a person writing its reason:** an empty or whitespace reason, a default, or a copy of `judge` or of another entry's reason.
- **An entry that covers more fields than the rows that need it,** so the acknowledgement claims rows that were never unwitnessed.

## 7. Flag-and-stop conditions

- **Sentinel-03's count differs from `2304b`'s 280 rows across 35 registers.** Stop and report it.
- **Any other refusal's bytes would move.** Stop and report it.
