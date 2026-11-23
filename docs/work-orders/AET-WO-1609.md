# AET-WO-1609 — A check that did not run is as loud as one that failed: one "ran or not" table in every verdict, and the foreign judge's late panic

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1609-ran-or-not`.
**Author:** Claude (architect) · **Builder:** strong tier, with a full adversarial pass.
**Phase:** P4, from the round-3 review. Written 2026-11-23. **Cadence:** a measurement, then a build with one stop. **After `1608`** (both touch `crates/aethelgard-cli`'s verdict and MCP surface).

**Grounds:**
- **Review 3** (`docs/reviews/2026-11-23-mythos-5-2.md`) B5 ("each gate listed as `judged n`, or `off: <the key that would turn it on>`"), C2 ("a check that did not run is as loud as one that failed"), and A15's foreign-judge item: "The foreign judge can pass a run in which ArduPilot panics after its last witness (`crates/aethelgard-cli/src/renode/foreign.rs:1265-1300`). Partly known (`foreign.rs:51`). Unverified: no Renode on the host." Owners in `….response.md` (rows B5, C2, A15).
- **`INV-03`** (cite it; do not restate it). **`RUL-14`**, whose rule on optional keys `4300k` and `4410` apply to `drop_limits`, `fab_tolerances` and A12. This work order makes the class visible in every verdict, rather than closing instances.
- **`2306`** listed the gates that judged nothing; A2 (`drop_limits`) showed a gate that never started and stayed silent.

> **What this is:**
> - **(1) The table.** Every verdict `verify` writes (and every other subcommand's verdict the measurement finds with gates of its own) carries one table: each gate by name, with `judged n` (the count of things it judged), or `off: <the specification key that would turn it on>`, or `not reached: <why>`. A gate with `judged 0` is listed as such, never omitted. The table is derived from the gates' own results, never from a hand-kept list: a new gate that is not in the table fails a test by name.
> - **(2) The verdict format bump** this needs, by the repo's own versioning, with goldens moved by their generators. Readers that parse the verdict (the MCP tools, `replay`, `attribute`, CI scripts) read the new field or are shown not to need it.
> - **(3) The foreign judge's late panic.** **Verify it in the twin first:** plant a run where ArduPilot panics after its last witness (an ELF or `var_info` patch, as `2311`'s CI does), and record whether `simulate --foreign` passes it. If it passes, it fails by name (`foreign_panic_after_witness` or a better name from the measurement), with the evidence the judge read. If it does not pass, record why, and the planted case becomes a regression test.
> - **Each refusal comes with a test, and each test with a mutant it kills.**
>
> It is NOT:
> - making any optional key required (`4300k` and `4410` own those);
> - a new gate, or a change to what any gate judges;
> - `replay`'s binding to the records (`2312`, merged).

## 1. Build

1. **Measure first.** Commit predictions in `spike/1609/`, then record:
   - every gate in every verdict, where its on/off decision is made, and which key turns it on;
   - the verdicts' readers;
   - the planted late panic's outcome under `simulate --foreign`, with the run's evidence.
2. **Stop with the measurement,** the table's shape, and the panic's outcome.
3. **Build to the ruling.** If the package's verdict moves, ask for the dispatch.

## 2. Acceptance

- **Every committed board's verdict** carries the table, and every gate in it reads `judged n`, `off: <key>` or `not reached: <why>`.
- **A planted gate added without a table row** fails a test by name.
- **A specification with `drop_limits` removed** shows that gate as `off: drop_limits` in its verdict.
- **The planted late panic** fails by name, or the measurement shows the judge already refuses it, with a regression test either way.
- **The CLAUDE.md gates pass, and CI is green.**

## 3. Fences

- **Fence:** `crates/aethelgard-cli` (the verdict, `verify.rs`, `src/renode/foreign.rs`, the MCP snapshot if a tool's output changes), the verdict's readers where they must read the new field, `ci/foreign-judge.sh` (the planted case), goldens moved by their generators, `spike/1609/` and FEEDBACK.
- **Not:** the IR, any generator, eda, `fab/`, a gate's own logic.
- **Running beside you:** `4240e` (generator and IR), `1917` (CI paths and shard scripts; you share `ci/` only at `foreign-judge.sh`). Merge `main` before READY.
- One PR, left unmerged. Write `~/.cache/aet-1609/pr-title-body.md`.

## 5. Adversarial pass, full

- **A gate that reports `judged n` without having judged:** a count taken before the gate's loop, or of inputs rather than judgements.
- **A table built from a list** that a new gate can miss.
- **A panic the judge still passes:** one after the last read, one in a thread, a `HardFault` that the exception check misses, a watchdog reset.

## 7. Flag-and-stop conditions

- **After §1.2,** with the measurement.
- **A gate's on/off decision cannot be named by a key** (it is implicit in the data). Stop with the gate.
- **The verdict bump breaks a reader outside the fence.** Stop.
