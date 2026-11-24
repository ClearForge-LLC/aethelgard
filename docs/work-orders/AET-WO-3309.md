# AET-WO-3309 — A witness compares what was sent, byte for byte: counts as well as values, the BMP581 log read whole, and evidence placed by what it says

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-3309-witness-bytes`.
**Author:** Claude (architect) · **Builder:** standard tier, with a targeted adversarial pass.
**Phase:** P4, backlog: attribution. Written 2026-11-24. **Cadence:** a measurement, then a build with one stop.

**Grounds** (roadmap §7.1 rows from `3307`; read `3307` FEEDBACK §3–§5 first):
- **`3307` Adv-1 (pre-existing, fails safe at I4):** `evolve`'s `Write`/`Read` comparison checks values, not byte counts.
  - `write 0x00 <- 0x36 0x40 0x81` in the configure step packs to `0x00364081`, equal to the solved `0x364081`, so it shows as right.
  - The `Write`/`Read` form has the same flaw (`Write 0 D0`). BME280 scripts are identity reads, so it bites first on the BMP581's 3-byte configure.
- **`3307` F1:** `transactions`, `shape`, `surplus` and `sent_elsewhere` (from `3306`) read only `Write`/`Read`, so a BMP581 log is never a surplus.
- **`3307` F2:** `renode.rs` `attribute` hands each step as many lines as it expects, and the last step takes the rest.
  - A model warning (`write to read-only register 0x38 ignored`) can therefore land in another step's witness.
  - ext-cfg37's wrong write landed in the data witness, whose 6-byte read is refused, so a fault stayed I4 (measured).
- **`3307` Adv-3 (latent):** a step's `Write` and `Read` can come from different lines (`find_map`). It is unreachable today.
- **`INV-04`** and **`INV-06`** (cite them; do not restate them).

> **What this is:**
> - **(1) Counts as well as values.** A witness's `Write` and `Read` compare `(value, byte count)` against the solved transaction, in both log forms. Run a **sweep** over every committed attribution, and over `3306`/`3307`'s spike sets where they still build: each verdict is unchanged, or each change is named with its cause.
> - **(2) The BMP581 log in the transaction readers.** `transactions`, `shape`, `surplus` and `sent_elsewhere` read the BMP581 form as `bmp581_bytes` does, so a BMP581 log can be a surplus or sent elsewhere. A planted firmware shows each.
> - **(3) Evidence placed by what it says, not by line count.** Replace `attribute`'s count-based distribution. Each line is given to the step whose transaction it matches, by register and direction. A line that matches no step, such as a model warning, is its own finding with its text quoted, never silently another step's.
>   - ext-cfg37 is the acceptance case: measured I4 today, predicted S1 after.
>   - If (3) moves a committed verdict, stop.
> - **(4) One line per step.** A step's `Write` and `Read` come from the same transaction, never stitched from two lines (Adv-3), with a test that plants the stitch.
> - **Each rule comes with a test, and each test with a mutant it kills.**
>
> It is NOT:
> - the BMP581 model, or any Renode model;
> - the twin's or the campaign's refusals (`AET-WO-2314`);
> - the 4-byte read limit;
> - the IR.

## 1. Build

1. **Measure first.** Commit predictions in `spike/3309/`, then record:
   - every committed attribution's verdict today;
   - each place the count-based distribution decides a witness's lines;
   - ext-cfg37 under (3)'s rule, predicted.

   **Stop** with the sweep's plan and any verdict (1)–(4) would move.
2. **Build** to the ruling. Merge main before READY.

## 2. Acceptance

- **The prepended zero byte** (`write 0x00 <- 0x36 0x40 0x81`) shows wrong, by count.
- **A BMP581 surplus write** is named as a surplus.
- **ext-cfg37** attributes S1, with the wrong write as its evidence. The read-only warning is its own finding.
- **The sweep:** every committed attribution is unchanged, or each change is named and ruled.
- **The CLAUDE.md gates pass, and CI is green.**

## 3. Fences

- **Fence:**
  - `crates/aethelgard-evolve`;
  - `crates/aethelgard-cli/src/renode.rs`, the `attribute` function and its helpers only;
  - their tests and goldens;
  - `spike/3309/`, `docs/work-orders/AET-WO-3309.*`.
- **Not:** the twin, `campaign.rs`, `renode/foreign.rs` (`AET-WO-2314`'s), `verify.rs`, `validate`, the IR, `eda`, the models.
- **Running beside you:** `AET-WO-4300k`, `1610` and `2314`. `2314` also edits `renode.rs`, in other functions: add, never rewrite, and merge main before READY.
- One PR, left unmerged. Write `~/.cache/aet-3309/pr-title-body.md`.

## 5. Adversarial pass, targeted

- **A right verdict that is still wrong:** a byte appended, a pointer widened, or a read of the right value at the wrong length.
- **A line placed in the wrong step:** two steps on the same register, a warning between a write and its read, or a repeated transaction.
- **A finding the firmware can forge:** a log line whose text the firmware controls.

## 7. Flag-and-stop conditions

- **After §1.1,** with the measurement and the sweep's plan.
- **A committed attribution's verdict moves.** Stop with the run and the cause.
