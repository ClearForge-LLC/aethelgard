# AET-WO-4241 — The bring-up firmware reads every device with an identity, and the verdict names the ones the twin cannot witness

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-4241-read-every-identity`.
**Author:** Claude (architect) · **Builder:** strong tier, with a full adversarial pass.
**Phase:** P4 (generated firmware and the twin's judge). Written 2026-11-20. **Cadence:** a measurement, then a build with one stop. **After `4240v`** (#259, merged).

**Grounds** (`4600` F19, #245; `3403` F6 and U1; roadmap §7.1):
- **The twin decides what the firmware does.** `svd-hal/src/board/firmware.rs` `reads()` (about L400) skips every device that `answers` rejects: "the twin has a model for it".
  - So Sentinel-01's TMP117 is never read. Its only bench witness is an address probe by the debugger (`BENCH.md` B9; `3403` F6, U1).
  - `4600` measured the effect: removing a TMP117 left the ELF byte-identical (`170258…`).
- **That inverts `INV-07`'s direction:** the twin's model set shapes the artifact the twin then judges. The firmware should follow the specification. The judge should say what it cannot witness.

> **What this is:**
> - the generated firmware makes the identity read of every device whose part row has an identity, modelled or not;
> - the twin's judge, for a device it has no model for, records the read as **not witnessable in the twin**, by name, never as a pass and never as a failure;
> - the bench package and `BENCH.md`'s expectations moved by tooling and by the architect.
>
> It is NOT:
> - a new device model;
> - configuration writes or data reads for unmodelled parts (identity only);
> - a change to what a modelled device's record must hold.

## 1. Build

1. **Measure first,** in `spike/4241/`, with predictions committed before the runs.
   - For every committed specification: which devices have a part row with an identity, and which of those the twin models.
   - What the twin does today with a read to an unmodelled address or chip select: the I2C status, the "Unknown slave" line (`3305`'s per-step attribution), and the SPI bytes.
   - **Stop and report** if the twin's response to an unmodelled read cannot be told apart from a modelled device that failed. Name what the judge would need.
2. **The firmware.** `reads()` takes every device whose part has identity steps. The `answers` filter is gone for identity steps and kept for configuration and data steps. Record layout and order are as today (`aethelgard_twin::boot::identity`).
3. **The judge.** A record for a device with no twin model gets a named outcome (for example `identity_not_witnessable`, with the part and the reason), counted apart from passes and failures.
   - It is listed in `verify`'s verdict, beside the ledger's unwitnessed rows.
   - It never fails a gate, and it never counts toward a gate's `judged`. Coordinate with `2306`'s `judged` fields if it has merged.
4. **Re-judge** every committed specification. Each verdict either is unchanged or moves only by the new named outcomes. Any other move is a stop.
5. **The bench package moves:** Sentinel-01's ELF gains the TMP117 read. Say so on the bus, and the architect dispatches `write=true`. You commit the artifact byte for byte (the fence is widened to it).
6. **`BENCH.md`:** write the TMP117 read's expected record (status, value, the datasheet's ID register and its cited value) in FEEDBACK. The architect places it.

## 2. Acceptance

- **The measurement table** is in FEEDBACK, with predictions beside it.
- **Every identity read is made.** `4600`'s pair (with and without a TMP117) now gives two different ELFs.
- **The new outcome appears exactly for unmodelled devices.**
  - **Mutant:** report it as a pass, and a test fails.
  - **Mutant:** keep the `answers` filter for identity steps, and a test fails.
- **No modelled device's record or verdict moves.**
- **The CLAUDE.md gates pass, and CI is green.** `TIER2_MIN` rises by your own tier-2 count.

## 3. Invariants and fences

- **Invariants:** `INV-07`, `INV-03`, `INV-06`.
- **Fence:**
  - `crates/aethelgard-svd-hal/src/board/firmware.rs` (the selection only);
  - `crates/aethelgard-twin` and `crates/aethelgard-cli/src` (the judge's outcome and the verdict line);
  - tests and goldens moved by tooling;
  - `spike/4241/`;
  - FEEDBACK;
  - `fab/sentinel-01-bench/package/`, byte-for-byte commit only.
  - **Not:** device models, the IR, specifications, `BENCH.md` (the architect places rows), workflows, or `ci/` (except a weight row P1/P2 asks for).
- **Running beside you:**
  - `2306` (#269), `cli` verdict lines and `judged`;
  - `2305` (#271), `renode.rs`'s witness and the BOR probe. Both are ahead of you in the bench-package queue.
  - `4405` (pincheck) and `1209c` (IR validate): no shared file.

  Merge `main` after each of `2306` and `2305` lands, and before READY.
- **One container of yours at a time,** under `capped-run`, `--mem 8G` for `prove` and 16G for `verify`, at `nice -n 10`.
- One PR, left unmerged. Write `~/.cache/aet-4241/pr-title-body.md`.

## 5. Adversarial pass, full

- **A modelled device misclassified as unmodelled,** so its failure hides as "not witnessable".
- **An unmodelled device at a modelled device's address,** which F-COLLIDE describes.
- **A board whose only identity devices are unmodelled:** does any gate then pass having judged nothing? Say how `2306`'s rule sees it.
- **A read to an unmodelled SPI device** that changes bus timing for a modelled one after it.

## 7. Flag-and-stop conditions

- **The twin cannot tell an unmodelled read from a failed modelled one.** Stop with the bytes and lines.
- **A modelled device's record or verdict moves.** Stop with the diff.
