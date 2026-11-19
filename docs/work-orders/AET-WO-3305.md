# AET-WO-3305 — A fault on a bus with two modelled devices can be detected

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-3305-witness-elsewhere`.
**Author:** Claude (architect) · **Builder:** strong tier, with a full adversarial pass.
**Phase:** P3 (attribution). Written 2026-11-19. **Cadence:** build, with one measured stop. **Before `4500`:** Sentinel-02 has modelled devices that share buses.

**Grounds** (`2304b`'s finding F-MASK, on #254; roadmap §7.1):
- **The bug:** `evolve`'s `witness_fields` (`crates/aethelgard-evolve/src/lib.rs`, about L1006 on `main`) takes `elsewhere = absent.iter().find(|a| a != address)`. That means any address the controller found no device at, other than the witness's own device, counts as "the firmware addressed the wrong device", and that is evidence of a software fault.
- **Why it fires on a faulted bus:** on a faulted bus every device is left out, so device A's witness carries device B's address in `absent`. A's address field is then judged wrong, so the witness is masked as software evidence, and `verify`'s `detection` refuses the run.
- **Measured** on Sentinel-03 at `ba8fb43b`: `held-low-sda` on SENSORS (BME280 at `0x76` and BMP581 at `0x46`) was refused with `attribute masks [{index 77, kind 13}] on the faulted bus: something else fails too`.
  - Sentinel-01 never met it, because its TMP117 is unmodelled and so one device answers.
  - After `4202` removes U2, Sentinel-03's SENSORS has one modelled device too. The bug is still there for any bus with two.

> **What this is:**
> - "elsewhere" means an address **this witness's own transfers** went to, not any address on the bus that went unanswered;
> - a two-device bus fault detected by its named mechanism, with a fixture that fails on `main`;
> - the wrong-address case (`AET-WO-2204`) still caught, with its existing test and a new two-device variant.
>
> It is NOT:
> - a change to what the twin records, the trace format, or any scenario. If the fix needs one, stop (§7);
> - a change to `detection` or coverage rules.

## 1. Build

1. **Read first, and write it down in FEEDBACK before any change:**
   - where `absent` is filled for an `I2cWitness` record (twin, `chaos` or `cli`), and over what window;
   - whether the record already carries enough to tell this witness's own transfers from other devices' transfers in the same window (`step`, timestamps, the transfer's address).
2. **Measure on `main` before the change.** Build a fixture board with two modelled devices on one I2C bus. Sentinel-03 at `main` before `4202` lands is one; if `4202` has landed, use a test specification: two modelled parts on one bus, from the committed rows.
   - Run a held-low-sda (or any bus-wide) fault: it is refused as masked.
   - Run the `2204` wrong-address mutant on the same board: it is caught.
   - Commit both outputs to `spike/3305/` before the fix.
3. **The fix, in `evolve`:** "elsewhere" takes only an address that this witness's own step addressed.
   - Use what the record already carries.
   - If the record cannot tell, stop (§7). Do not guess from the declared device list. "Every declared address except mine" would also hide a firmware that addresses device B's address for device A's step on a healthy bus, and B would ACK there anyway.
   - State in FEEDBACK, with the code path, what the rule can and cannot tell apart on a faulted bus.
4. **Tests:**
   - The two-device bus fault is detected by its named mechanism: the fixture fails on `main` and passes on the branch.
   - The `2204` wrong-address case on a one-device and a two-device board is still caught (existing test unchanged, plus the new variant).
   - **Mutants:**
     - restore the old rule, and the two-device fixture fails;
     - drop the address check entirely, and the wrong-address tests fail.
   - **Every committed campaign trace and verdict reproduces** byte-identically. Name any that move and why: a move means a committed result was masked before.

## 2. Acceptance

- **The two-device bus fault counts as detected,** by its named mechanism, on the fixture and on whichever committed board has two modelled devices on a bus at merge time.
- **The wrong-address evidence still fires** on one-device and two-device boards.
- **Every committed result reproduces,** or each move is explained.
- **The CLAUDE.md gates pass, and CI is green.**

## 3. Invariants and fences

- **Invariants:** `INV-03` (the campaign is a gate), `INV-06`; `RUL-07`.
- **Fence:**
  - `crates/aethelgard-evolve/src`;
  - tests;
  - `spike/3305/`;
  - FEEDBACK.
  - **Not:** the twin, `chaos`, `cli` (except a test), scenarios, specifications, the trace format, workflows, `ci/` or `container/`.
- **Running beside you:**
  - **`2304b`** (#254) bumps the trace and scenario versions and found this bug. Read its FEEDBACK F-MASK.
  - **`4202`** removes U2 from Sentinel-03 and fixes Renode's `run_for` truncation.
  - **`4240v`** changes the generated firmware.
  - Merge `main` after each lands, and re-run the reproduction check.
- **One container of yours at a time,** under `capped-run`, `--mem 16G` for `verify`, at `nice -n 10`.
- One PR, left unmerged. Write `~/.cache/aet-3305/pr-title-body.md`.

## 5. Adversarial pass, full

- **A firmware fault the new rule now hides:** a wrong address that collides with another real device's, on a healthy bus and on a faulted one.
- **A witness window that includes another step's transfer.**
- **A fixture that passes for a reason other than the fix,** such as one device unmodelled.

## 7. Flag-and-stop conditions

- **The record cannot tell this witness's own transfers from another's** without a change to what is recorded. Stop with the record's fields and the change you would need. A trace-format change is the architect's call.
- **A committed result moves** in a way the fix does not explain. Stop with the diff.
