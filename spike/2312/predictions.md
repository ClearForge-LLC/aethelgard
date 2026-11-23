# AET-WO-2312 §1.1 — predictions, written before any measurement

Written 2026-11-23 10:2xZ, on `wo/aet-2312-replay-records` at main `7dec1d7b`, before any sweep, probe or timing
output existed. They are read at source only. `results.md` checks each one.

The "before" side throughout is main `7dec1d7b`'s CLI. The archives are made in the pinned tools image
(`container/IMAGE`, `e5fc7ddb…`) under `capped-run`, by 2310's sweep (`sh/measure.sh`, unchanged).

## The archives

- **P1. Committed archives.** There are 5 (`git ls-files 'spike/*/out/**'`): `spike/2309/out/flood` (trace 18)
  and the four `spike/3305/out/archives/*` (trace 14).
  - `replay` and `attribute` refuse all five by version (`UnknownVersion`), exit 3. Nothing else is read.
  - So for these, "replays as before" means "refused by version, as before".
- **P2. CLI-made archives.** 2310's groups at main give 64 runs: the committed scenario groups, the seeded
  mutants, and the two-device cases.
  - `replay` exits 0 on each, with the campaign line's `verdict`, `expect`, `matched` and `findings`.
  - `attribute` gives an `Attribution` for each, never `NotACampaignRun`.

## The derivation (1), kind by kind

- **P3. Counts.** For every kind (1) derives, the count from the records equals the judge's count, on all
  64 CLI-made archives. The derivation follows the judge (`cli/src/renode.rs`), not `select`:
  - the analog, held-low, boot-word, `pc_outside_firmware`, `cpu_halted`, brown-out, flash, motor and serial kinds,
    as `check_run` binds them today;
  - `register_mismatch`: kind 11 with `(observed ^ expected) & mask != 0`. Exact, at any boot;
  - `{i2c,spi}_timeout`: kind 12 or 21 with status `timeout_*`. Exact, at any boot. The judge names a timeout
    before the boot word too (`timeouts_without_the_boot_word`);
  - the six `{bus}_{step}_mismatch` kinds: status `ok`, the value failing the judge's `Pass` rule, named by step.
    Exact, once booted, and none before;
  - `{bus}_record_invalid`: status `invalid`, once booted. **Lower bound only.** An unmodelled device's read is
    not archived as kind 12 or 21, but the judge names its invalid status by the same kind
    (`unmodelled_finding`);
  - `{bus}_transaction_unwitnessed`: stays a lower bound (`2310` item 7). Archiving is lossy
    (`archived_lines`, `fit`): an `[ERROR]` line past `MAX_WITNESS_LINES` turns a `step_error` into what reads
    as a differing witness. `select` counts every `ok` read whose witness differs, exactly;
  - `locked_peripheral_access`: the judge names it at most once, from Renode's log (`LOCKED`), which is not
    archived. The records do not show it in either direction:
    - an RCC enable bit left off need not mean an access was made;
    - a clock enabled after an access leaves RCC as solved.
    So it can be bound only as "at most 1". `select`'s RCC-enable-bit events stay attribute's evidence.
- **P4. Exercised kinds.** The CLI-made archives exercise `register_mismatch`, `i2c_timeout`, the identity
  mismatch (seeded), `boot_word_missing`, the analog, held-low, motor and serial kinds, `cpu_halted` (droop runs)
  and unwitnessed (two-0x77). No archive names `locked_peripheral_access` or a `record_invalid`. Status words seen
  are `ok` and `timeout_*` only (2310 P4). So "equal on all" is vacuous for the kinds that are never exercised.
  Their rows are classified from the judge's source.
- **P5. Gating.** One CLI-made archive is read and not booted (`seeded-no-boot-word`). Its kind 12 statuses are
  `ok` with matching values, or `timeout_*`. The gated and ungated sets are equal there, so gating moves nothing
  measured.

## B6: `locked_peripheral_access` without `svd-hal`

- **P6.** On every kind 11 of every CLI-made archive, the name prefix `RCC.` selects exactly the records whose
  address is in `svd-hal`'s RCC range (`register_facts().base("RCC")` plus its layout size). So the derivation
  needs no `svd-hal` in chaos.
  - Every kind 11 name is `PERIPHERAL.REGISTER` from the judge's image, as `chaos/src/record.rs:521` reads it.

## Attribution under (3)

- **P7.** Attribute's index sets under the prototype derivation equal `select`'s, on all 64 archives, for every
  kind that a verdict names or an event shows. So no `Attribution` moves:
  - not its rule, domain, evidence, symptoms or masked;
  - not its `unexplained`.
  The prototype differs from `select` only on records no archive holds: an unbooted run's invalid or mismatching
  read, a witness with an `[ERROR]` line, and a step with no witness format.

## The cost (4)

- **P8.** The reviewer's `big_fail` (all reads time out) and `big_pass`, K buses × 128 devices, re-made at trace 19:
  - K = 20, 100 and 200 give about 0.5, 2.5 and 5 MB;
  - `replay` takes under 0.5 s on each;
  - `attribute` grows about cubically: under 1 s at 0.5 MB, 20–60 s at 2.5 MB, over 300 s at 5 MB on `big_fail`;
  - `big_pass` is fast. With no failing kind, nothing is scoped.
  - **The hotspot** is `decide` calling `faults(b)` over every bus per scoped symptom: buses × records ×
    symptoms. `check_of` and `bus_of_device` add a linear factor per event.
  - **Lean for (4):** index the records by bus and by `(reference, step)` once, so attribute is near-linear. No
    refusal by size.

## The four shapes

- **P9.** Re-made at trace 19 by the reviewer's mutations on the `archived` helper's header:
  - `reg_unnamed`, `timeout_unnamed` and `idmismatch_unnamed` replay `pass`, `matched: true`, exit 0 at main.
    `attribute` says `I1` on each;
  - `halted_unnamed` is already refused at main, by `cpu_halted`'s binding (`2310`);
  - each is under 8 KiB, far under 256 KiB, so all four are committed as fixtures.

## B6: the fold's duplicate-record check

- **P10.** `replay.rs`'s `ONCE_KINDS` check cannot go. It refuses at the fold, with `DuplicateRecord { index, kind }`,
  before `check_run`, and `check_run` names no index. Removing it moves every duplicate refusal's kind and index.
  - Kinds 16 and 20 are held to once by `check_run` too, but with a different message.

## Stops

- **P11.** None of the §7 stops fires:
  - no committed or CLI-made verdict moves;
  - no format bump is needed;
  - `svd-hal` is not needed (P6);
  - records and judge agree on every committed scenario.
  The rulings needed are `locked_peripheral_access`'s bound (P3) and `record_invalid`'s lower bound.
