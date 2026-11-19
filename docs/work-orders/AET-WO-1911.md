# AET-WO-1911 — CI that waits less: docs-only PRs skip the heavy jobs, the shards share one build, and the shard count is measured and set

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main` **after `AET-WO-1910` merges**, named in the kickoff.
**Branch:** `wo/aet-1911-ci-wait`.
**Author:** Claude (architect) · **Builder:** strong tier (the `1910` builder, reused), with a targeted adversarial pass.
**Phase:** infrastructure. Written 2026-11-19. **Cadence: build, with one measured stop (§1.3).**

**Grounds:**
- **Scotty, 2026-11-19:**
  - "Let's do all that and then see the results before worrying about batching": docs-only routing, build once and test many, and the shard count;
  - "measure it in 1911 and set the shard count". This replaces his 2026-11-14 rule that `ci/SHARDS` stays 8.
- **Aethelgard is a public repository, so GitHub's standard hosted runners are free.** CI costs wait time, not money. Quench (the self-hosted runner on the Precision) serves the private repositories. A self-hosted runner for a public repository is out: a fork's PR could run code on it. Roadmap §6's "Quench has eight slots" premise is corrected in the docs PR that writes this order.
- **Two limits stay real:**
  1. **The account's concurrent-job limit.** One push already starts about 25–30 jobs across six workflows, and four builders push.
  2. **Each shard's fixed setup:** 190–235 s measured by `1910`, pulling the image and building the workspace. That is the part more shards cannot divide.
- **Docs-only PRs pay the full suite.** #230, #231 and #234 each ran 60–70 minutes of CI for Markdown. Some tests read documents (for example `ledger_tier1`, which checks a reference table in `docs/`), so "docs only" is not "nothing to test".

> **What this is:**
> - a PR whose diff touches only documents runs the gates and every test that reads a document, and nothing heavier;
> - the workspace's tests built once per CI run and shared by every shard;
> - the shard count measured and set.
>
> It is NOT:
> - batching or a merge queue. That waits for these results (Scotty);
> - a self-hosted runner;
> - a weakened test or gate, or a test skipped by path on a PR whose code it could judge;
> - caching test *results* across runs.

## 1. Build

1. **Docs-only routing.**
   - **Classify the diff,** in a first job: the changed paths, against the PR's base.
   - **"Docs-only" is the positive list:** `docs/**`, `*.md` at the root and under `fab/**`, `README*`, `LICENSE*`. Anything else is code, including `examples/`, `scenarios/`, `boards/`, `fab/**/package/**`, `ci/`, workflows and `container/`. Unknown means code.
   - **On docs-only, run:**
     - fmt;
     - every test that reads a file under a docs-only path, found by inventory rather than by name list;
     - and nothing else.
   - **The inventory is checked by a guard:** it greps the test sources for paths under the docs-only list, and refuses by name a test that reads one and is not in the docs-only set.
   - **On `main`'s push, always the full suite.** `bench-package` keeps its every-PR run for code PRs (`1910` D8).
   - The routing is one step whose decision is printed: which paths, which class, which jobs run.
2. **Build once, test many.**
   - One job builds the workspace's test binaries in the pinned image, once per run.
   - Every shard downloads them and runs its assigned tests without compiling.
   - **The method is yours,** for example a `cargo test --no-run` artifact with its binaries' paths, or nextest's archive. `1909` §0 kept nextest out as a runner. If you choose it only to archive, say why that is not a new runner. Changing what decides a pass is a §7 stop.
   - **Prove sameness.** On one run, every shard's set of test names and their outcomes equals the same commit's set under the current per-shard build. Record both lists' hashes.
3. **The shard count, measured, then STOP.**
   - With §1.1 and §1.2 in place, run the full suite at **8, 12 and 16 shards** (more if 16 still pays), each planned by `ci/shard.sh` from today's weights. Record for each:
     - a lone PR's push-to-green wall time;
     - each shard's setup and test time;
     - the queueing seen when **three PRs push at once.** Use three scratch PRs of your own, closed after.
   - Read the account's concurrent-job limit as GitHub reports it. If you cannot read it, say so; do not assume it.
   - **STOP** with the table and a recommended count, with its reason: the point where more shards stop buying wall time, or where queueing eats the gain.
4. **After the ruling:**
   - set `ci/SHARDS`;
   - refit by the REFIT RULE;
   - rewrite REFIT RULE 4 in `ci/shard.sh` to cite this order and the ruling;
   - record before-and-after wall times over three runs.

## 2. Acceptance

- **A docs-only PR is green in minutes,** with every document-reading test run. A code PR runs everything it did.
- **The shared build produces the same tests with the same outcomes** as the per-shard build (§1.2's proof).
- **The shard count is set from the measured table,** as ruled.
- **`TIER2_MIN` is exact,** and the CLAUDE.md gates pass.

## 3. Invariants and fences

- **Invariants and rulings:** `INV-04`, `INV-06`; `RUL-12`.
- **Fence:**
  - `.github/workflows/*.yml` (the routing, the build job and the shard matrix);
  - `ci/`;
  - test-harness changes the shared build needs, none to an assertion;
  - FEEDBACK.
  - No production code.
- **Running beside you:** `4400g`, `3304` and `4201`. Take main before building, and again before READY. Your workflow changes reach their CI on their next push, so say on the bus when they land.
- One PR, left unmerged. Write `~/.cache/aet-1911/pr-title-body.md`.

## 5. Adversarial pass, targeted

- **A code change routed as docs-only:** a renamed file, a symlink, a path under `docs/` that a build reads (`include_str!`).
- **A document-reading test missing from the docs-only set.**
- **A shard that runs a stale binary,** built from another commit, or that compiles anyway.
- **A test that passes in the shared build and fails in a per-shard build,** or the reverse.

## 7. Flag-and-stop conditions

- **§1.3.**
- **The shared build changes a test's outcome:** stop with the test.
- **The concurrency limit cannot be read, and the measured queueing is ambiguous:** stop with the numbers.

<!-- AET-WO-1911 scratch: a docs-only change, never merged -->
