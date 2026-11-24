# AET-WO-2314 — A failure names its own cause: the campaign's undisturbed boot, a fault vector caught before the memory cap, and the foreign judge's log trimmed

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-2314-failures-name-their-cause`.
**Author:** Claude (architect) · **Builder:** strong tier, with a full adversarial pass.
**Phase:** P4, backlog: the twin and the foreign judge. Written 2026-11-24. **Cadence:** a measurement, then a build with one stop.

**Grounds** (the rows, from `main`):
- **`4410` FEEDBACK §6, Q15 (ruled 05:05Z): the campaign's misattributed refusal.** `crates/aethelgard-cli/src/campaign.rs:1224` `word_of` reads the undisturbed boot's `boot_word_ns` alone. So `PlantedEventUnplaced` ("the undisturbed boot wrote no boot word") names no cause when the boot:
  - ended at a bound (`main.rs:2220-2228`);
  - dropped log lines past the budget (`renode.rs:2515`);
  - wrote a fault word (`renode.rs:2523-2527`);
  - parked before the word.

  `simulate`'s `undisturbed_boot_word_ns` (`renode.rs:4239-4262`) names the cause. The same row records `bor_recovery::the_bmp581_extension_recovers_after_its_settle` failing 3 of 4 CI runs and none on Furnace, the 30 s wall bound suspected and unproven.
- **`1609` FEEDBACK §6:**
  - **R1:** a fault-vector hook (HardFault, MemManage, BusFault and UsageFault at the ELF's VTOR), named `foreign_fault`, before the log budget or the memory cap can stand in for it.
  - **R3:** the HardFault that fails only as `memory_cap_exceeded`. Under `2311`'s access log it peaks at 2.59 GB (1.10 GB without the log); under `2313`'s 1.5 GiB it still fails for the wrong reason.
  - **R4:** the decoy panic symbol.
- **`2311` A5:** the foreign judge's access-log filter keeps `TDR` writes it never reads. Dropping them cuts the log that R3 measured.
- **`INV-06`** (cite it; do not restate it): a refusal names what failed.

> **What this is:**
> - **(1) The campaign names the cause.** When the undisturbed boot yields no boot word, `PlantedEventUnplaced` carries that boot's own end: the bound and its value, the dropped-line count, the fault word, or the park code. These come from the same verdict `simulate` builds. A test plants each of the four causes and checks the refusal names it.
>   - Then **re-run the BMP581 BOR-recovery test under shard 11's packing** (the job's `shard.tsv`, 4 CPUs) until it fails or 10 runs pass. Report the cause it names. If it is the wall bound, propose the fix with its numbers; it is a ruling, not part of this build.
> - **(2) A fault vector is caught by name.** Measure first whether a hook on the four fault vectors (read from the ELF's VTOR) can pause the machine before the memory cap or the log budget ends the run, on the planted HardFault of `1609` §1.2. If it can:
>   - `simulate --foreign` fails `foreign_fault`, naming the vector and the PC;
>   - the R3 run fails `foreign_fault`, not `memory_cap_exceeded`.
>
>   If it cannot, stop with the measurement.
> - **(3) `2311` A5:** drop the `TDR` writes from the foreign judge's access-log filter. Show that no rule reads them (by search and a test), and measure the R3 run's peak before and after.
> - **(4) R4, measured:** whether the panic hook's address can be cross-checked against the ELF's call sites to `AP_HAL::panic` cheaply. Build it if the measurement is small and sure; otherwise it stays a row, with the numbers.
> - **Each refusal comes with a test, and each test with a mutant it kills.**
>
> It is NOT:
> - `1609` R2 (liveness: timing-dependent, a row);
> - the supervisor as the twin's reset floor (`4300k` D6's row, after `4300k` merges);
> - the motor witness's period convention (`TVI-WO-1302` Q1's coordinated row);
> - any bound's value (the wall or memory constant), unless ruled after (1).

## 1. Build

1. **Measure first.** Commit predictions in `spike/2314/`, then record:
   - the four causes reaching `word_of`, each reproduced;
   - (2)'s hook feasibility and timing on the planted HardFault, and on ArduPilot's normal 5 s boot (no false `foreign_fault`);
   - (3)'s readers of `TDR`;
   - (4)'s cost.

   **Stop** with them.
2. **Build** to the ruling. Merge main before READY.

## 2. Acceptance

- **Each of the four planted causes** is named by the campaign's refusal.
- **The planted HardFault** fails `foreign_fault` with its vector, under both `simulate --foreign` and R3's access-log run. ArduPilot's normal boot does not.
- **`ci/foreign-judge.sh`'s cases** pass unchanged, apart from cases this work order adds.
- **The CLAUDE.md gates pass, and CI is green.**

## 3. Fences

- **Fence:**
  - `crates/aethelgard-cli/src/campaign.rs`, `renode.rs`, `renode/foreign.rs`, `run.rs` (reporting only, no bound's value);
  - `crates/aethelgard-twin` where the verdict's end is carried;
  - their tests and goldens;
  - `ci/foreign-judge.sh` (added cases only);
  - `spike/2314/`, `docs/work-orders/AET-WO-2314.*`.
- **Not:**
  - `verify.rs`'s ran table or `validate` (`AET-WO-1610`'s and `4300k`'s);
  - `aethelgard-eda` (`1324`'s);
  - the IR (a new verdict field is a stop);
  - `fab/`, or the steering documents.
- **Running beside you:** `AET-WO-4300k`, `1324` and `1610`. Add, never rewrite another's code, and merge main before READY.
- One PR, left unmerged. Write `~/.cache/aet-2314/pr-title-body.md`.

## 5. Adversarial pass, full

- **A cause still unnamed:** a boot that ends in two ways at once, or the bound hit during the planted run rather than the undisturbed one.
- **A false `foreign_fault`:** an ELF that installs its own fault handler and returns, or a VTOR moved at run time.
- **A filter change that hides a finding:** a rule that read `TDR` by another name.

## 7. Flag-and-stop conditions

- **After §1.1,** with the measurement.
- **(2)'s hook cannot pause before the cap.** Stop with the timing.
- **A committed verdict or golden changes** other than the refusal texts this work order adds. Stop with the diff.
