# AET-WO-1919 — CI headroom (2): Sentinel-03's bench package inside its limit with room, a platform load under the hang guard, and an empty input refused

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1919-ci-headroom-2`.
**Author:** Claude (architect) · **Builder:** strong tier (the `4400j` builder, who found the load's missing bound), with a targeted adversarial pass.
**Phase:** P4. Written 2026-11-26. **Cadence:** a measurement, a stop, then the build.

**Grounds:**
- **Roadmap §7.1, "Planned, not written":** a CI work order that profiles Sentinel-03's `verify` and brings `bench-package-s03` back under the shards' wall time.
  - 16 min on Furnace (`4300k`);
  - 70.4 min in CI after `1918`'s split (run 45620143376), against the job's 90-min limit;
  - `4411` adds the WiFi module and its `uart4` harness to Sentinel-03.
- **`4400j` FEEDBACK F15** (#403):
  - `RUL-13` bounds `simulate --firmware` only.
  - A hung `simulate --platform` (a load) ran until the job's 60-minute timeout on CI run 45709090931: two shards were lost, and three tests waited on a lock behind it.
  - `4400j`'s `quit` line removed that cause. It did not bound the next one.
- **`4400j` FEEDBACK F11:** an input with `"frames": []` is accepted, so a run that sends nothing is admitted to the 12 s input window (D3).
- `INV-04`, `INV-06`, `RUL-09`, `RUL-13`, `RUL-15`.

> **What this is:**
> - **(1) Where `bench-package-s03`'s time goes.** Measure first, with predictions committed in `spike/1919/`:
>   - every step of the job on CI, at least three runs, with each runner's CPU model;
>   - inside `verify`, each gate's time, and within `prove` each harness's wall time and peak, by name;
>   - the same on Furnace;
>   - the same with `4411`'s branch if it is pushed.
>   - **Then propose ways to bring the job under 45 minutes on the slowest runner measured, keeping every proof and every gate.** For example:
>     - proving harness subsets in parallel jobs, under `1918`'s 12 GiB ceiling each, then one job that composes their results;
>     - caching what is identical between runs, keyed by its inputs' hashes;
>     - removing a repeated build.
>   - **Stop with the table and the options.** A change to what `verify --against-fab` derives is a verdict change, and is ruled first.
> - **(2) A platform load under the hang guard.**
>   - Measure every load the CLI makes (`simulate --platform`, the MCP `simulate` tool, and every tier-2 test that loads): its wall time on CI and on Furnace.
>   - **Then build:** every Renode invocation the CLI makes, load or boot, is killed at `1918`'s 120 s hang guard. The kill is a `ToolError`, never a finding, and never a verdict.
>   - If any measured load comes within 5× of 120 s, stop with it.
>   - Draft the `RUL-13` amendment's text in FEEDBACK. The architect lands it at merge.
> - **(3) An empty input refused.**
>   - `"frames": []` is refused with a JSON error and a nonzero exit, naming the field (`INV-06`).
>   - If any committed input or test relies on an empty one, stop with it.
> - **Each refusal and each bound comes with a planted test, and each test with a mutant it kills.**
>
> It is NOT:
> - dropping, skipping or weakening any test, proof or gate;
> - a larger runner, a self-hosted runner or Quench (Aethelgard is public);
> - the no-input 5 s window (`4400j` F8: a row);
> - Renode's memory cap (`4400j` F4: re-measured by a later row, not here);
> - the steering documents.

## 1. Build

1. **Measure (1) and (2),** predictions first. **Stop** with:
   - (1)'s table and options;
   - (2)'s load times against 120 s;
   - whether (3) finds an empty input committed anywhere.
2. **Build (2) and (3),** and (1) as ruled. Merge `main` before READY.

## 2. Acceptance

- **(1) as ruled:**
  - `bench-package-s03` under its target on CI's slowest runner, read from the job's own log;
  - every harness and every gate still runs, shown by name against `main`'s list;
  - both committed packages pass `verify --against-fab` unchanged, or move only as ruled.
- **(2):**
  - a planted Renode stuck at its prompt during a *load* is killed at the guard as a `ToolError`, as `4400j`'s boot test is;
  - no load test runs longer than its measured time plus margin;
  - the caller's own timeout is never what ends a run.
- **(3):** the planted empty input is refused, with its field named.
- CI is green. Every new test runs in a shard at its measured weight.

## 3. Fences

- **Fence:**
  - `ci/` and `.github/workflows/ci.yml`;
  - `aethelgard-cli`'s `run.rs`, `renode.rs`, `main.rs`'s `simulate` dispatch and the foreign input's reader;
  - `verify.rs`'s prove gate (for (1), only as ruled);
  - `spike/1919/` and `docs/work-orders/AET-WO-1919.*`.
- **Not:**
  - any committed board or package, except a dispatch the ruling names;
  - a test's assertions, except the new ones;
  - the steering documents.
- **Running beside you:** `4411` (Sentinel-03 with the module; its write dispatch follows its build). Merge `main` before READY.
- One PR, left unmerged. Write `~/.cache/aet-1919/pr-title-body.md`.

## 5. Adversarial pass, targeted

- **A proof that stops running:** a harness dropped in a split, or a cached result whose key misses an input.
- **A bound that judges the host:** a load guard tight enough to fire on a slow runner, or a kill reported as a finding.
- **An empty input by another name:** frames present but each with nothing to send, or an input that arms nothing.

## 7. Flag-and-stop conditions

- **After the measurement** (the one planned stop).
- **(1) cannot meet its target without dropping or weakening a proof.** Stop with the numbers.
- **A load within 5× of the guard,** or any committed verdict or package that would move.
