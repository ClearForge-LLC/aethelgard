# AET-WO-2204 — The boot judge a firmware cannot fool: an unforgeable I2C witness, one per transaction, the core at its fixed point, every log line read

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-2204-boot-judge`.
**Author:** Claude (architect) · **Builder:** strong tier, with a full adversarial pass (`docs/roadmap.md` §6).
**Phase:** P2 hardening, before P4 adds a second sensor bus. **Cadence: spike (§1), then STOP for the architect's ruling, then build (§2).**

**Grounds, each measured or read before this work order:**
- **W1, review A11.1, measured in `AET-WO-1900` §1.10.** A hand-written firmware that writes the identity record and stores the reference boot's final `CR2` (`0x020104EC`) and `TXDR` (`0xD0`) straight into `I2C1`, calling no `transfer`, is judged `pass`. Renode 1.17.0 stores both writes.
  - The pinning test is `twin_tier2::a_firmware_that_writes_the_witness_registers_by_hand_passes_today_a_named_residual`. It is to be **inverted, not deleted**.
- **W2, `AET-WO-2202`.** Only the last transaction on each bus is witnessed. Each bus has one modelled device today; P4's IMU bus will have more.
- **W3, `AET-WO-2301` F9, D9.** A firmware parked in its fault handler passes the boot judge: the boot word and record are written, the instruction count holds, and `PC` lies inside an executable segment. The judge requires only "inside the firmware", not "at the fixed point".
- **W4, `AET-WO-2301` B10, measured.** The static boot's `RunFor; logLevel 3` tail drops queued log lines: Renode prints asynchronously, and the level filter applies at print time. Through a pipe, 165 to 183 lines were lost, varying run to run. A `locked_peripheral_access` or `renode_error_log` line printed just before the reads can vanish. The droop probes already avoid this (2301 A2); the static tail does not.
- **W5, `AET-WO-1102` A3, inferred.** A device reference named for a Renode instance the platform's stock list omits (`nvic`, `sysbus`, `usart1`…) or for a platform keyword (`using`, `init`) may redeclare that instance in the emitted `.repl`.

> **What this is:** the boot judge (`aethelgard-twin::boot`, the CLI's `renode.rs`), made to judge what the firmware did, not what it wrote down.
>
> It is NOT:
> - the archive's naming and version hygiene (`AET-WO-2205`, reserved);
> - attribution rules;
> - a change to any static verdict on the reference firmware, which must still pass, byte-identical, wherever the judge's new fields allow.

## 1. Spike

Probes live under `spike/2204/`. **Predictions go into FEEDBACK before any run.**

**S1. An observable a plain store cannot set (W1).**
- **Read at source** (Renode 1.17.0's `STM32F7_I2C` model, pinned commit) which `I2C1` registers a firmware write changes and which only the model's own transfer logic changes. Candidates:
  - `ISR`: `TXE`, `TXIS`, `RXNE`, `TC`, `STOPF`, `NACKF`, `BUSY`;
  - `RXDR`: the last byte received;
  - any transfer counter.
- **Measure in the pinned image,** with probe firmwares:
  - the reference read;
  - a hand-written forgery storing every candidate register's reference value;
  - a firmware that transfers to the wrong address.
- **Report** which candidates hold a value only a real transfer to the right device produces, and which the forgery can set.

**S2. One witness per transaction (W2).**
- **Find a witness that survives a later transaction on the same bus.** Candidates:
  - the BME280 model's own state (its register pointer, or a read count), read through the monitor;
  - Renode's peripheral access log for `I2C1`, read only between markers and bounded;
  - a per-transaction record the firmware writes, judged against the controller state captured after each transfer by a monitor hook.
- **Prefer what Renode stores** over what the firmware claims.
- **Measure:**
  - two reads to two modelled devices on one bus, with one of them forged;
  - the cost per transaction in wall time and in output bytes.

**S3. The fixed point (W3).**
- Find the generated firmware's fixed point: the symbol, or the address range of the `loop {}`.
- Find how the judge can know it from the ELF alone, without trusting the firmware: a linker-placed symbol the generator emits, or the entry's reachable tail.
- **Measure:** the reference boot's final `PC`, and that of the `systick-before` fault-parked firmware from `2301`.

**S4. One console level through the reads (W4).**
- Measure the static boot with the console kept at its level through the reads, skipping whole log lines between markers as the droop probes do.
- **Show that** every log line the firmware caused reaches the judge, ten runs in a row, and that the reads stay byte-exact.

**S5. Renode's reserved names (W5).**
- Read the pinned platform file and Renode's monitor grammar for every instance name and keyword a `.repl` declaration could collide with.
- **Measure:** one emitted platform per colliding reference loaded by `simulate`, with the outcome of each.

**Stop** with the table, a recommendation for each of S1 to S5, and the decisions needed.

## 2. Build, after the ruling

Expected, to be confirmed by the ruling:
1. **The witness** reads the unforgeable observable S1 found.
   - The A11.1 pinning test is inverted: the forgery now **fails** with a named finding.
2. **Per-transaction witnessing** as S2 ruled. A second device on the reference bus may be needed as a test fixture only, not a change to the reference specification.
3. **The boot judge requires `PC` at the fixed point** S3 identified. The `systick-before` firmware fails with a named finding, and the reference passes.
4. **The static boot's reads keep one console level** (S4). `main`'s static verdicts stay byte-identical, except where a previously lost log line is now read; any such change is named in FEEDBACK with the run that shows it.
5. **Device references colliding with a reserved Renode name are refused** where the name set is known: in `validate` if the set is static and pinned, otherwise in `generate`, before any output (S5).
6. **Records and the trace version.** If the witness's record shape changes (kind 13), the trace version rises once, and old archives stay readable or are refused by name. Coordinate with `AET-WO-2205`, which owns the record renames: **one bump per work order**, and say so in FEEDBACK.

## 3. Invariants and fences

- `INV-04`, `INV-05`, `INV-06`, `INV-07` (the expectation comes from the solved topology and the part table, never from the firmware); `RUL-13`, `RUL-17`, `RUL-19`. No `unsafe` outside `bridge/src/ffi.rs`.
- **Fence:**
  - `crates/aethelgard-twin` (boot, platform);
  - the CLI's `renode.rs`, `verdict.rs` and `main.rs`'s `simulate`;
  - `crates/aethelgard-chaos` only for kind 13 and the trace version;
  - `crates/aethelgard-evolve` only where it reads the witness (`witness_wrong`, `witness_fields`);
  - `aethelgard-ir` only for S5's refusal;
  - tests and their fixtures;
  - `ci.yml` (`TIER2_MIN`) and `ci/test-weights.tsv`;
  - this work order's FEEDBACK.
- **The reference firmware and the committed generated files stay byte-identical.** A generator change S3 needs (a symbol for the fixed point) is a §7 stop for a ruling.
- One PR, left unmerged. **Push and revoke by the FUR-WO-0301 F7 route.**

## 4. Acceptance

- **The spike table, ruled.**
- **Each forgery fails** with a named finding: the A11.1 forgery, the second-device forgery, and the fault-parked firmware. The reference passes, and its verdict is byte-identical where the fields allow.
- **Static log lines are no longer lost:** a test shows that a line the old tail dropped now reaches the judge.
- **Reserved names** are refused before any output.
- **The CLAUDE.md gates pass,** and CI is green with every shard under 15 minutes.

## 5. Adversarial pass — full

1. **Forge a pass** against the new witness: any register, any order, any timing, a second device, a write after the transfer.
2. **Make a firmware fail** that does everything right. Look for false negatives from timing or log interleaving.
3. **Make the fixed point ambiguous:** a firmware that loops elsewhere but looks like the fixed point.
4. **Lose a log line again.**

## 7. Flag-and-stop conditions

- No register in Renode 1.17.0 is both unforgeable and present after a real transfer (S1).
- The fixed point cannot be known without a generator change (S3).
- A static verdict on the reference must change for a reason other than a log line that was previously lost.
