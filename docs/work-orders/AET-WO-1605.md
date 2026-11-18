# AET-WO-1605 — The tools image's Renode carries upstream's NVIC fixes; the RETTOBASE correction retires by its own pin test; every virtual-time witness is re-measured

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1605-renode`.
**Author:** Claude (architect) · **Builder:** strong tier (the `4400e` builder, reused), with a full adversarial pass.
**Phase:** P4 hygiene (roadmap §7.1, the Renode correction's retirement row, owner "the next tools-image work order"). Written 2026-11-18. **Cadence: build, with one measured stop (§1.1).**

**Grounds:**
- **The pin today** (`container/Dockerfile` L42–44, `container/TOOLS-SOURCES.md` L15–16):
  - Renode 1.17.0, portable release, tag commit `ab721d88`;
  - its `src/Infrastructure` is at `066a7f13`.
- **Two upstream fixes are missing from it** (`4400e2`, `4400e`, measured):
  - **`e7a8584c`:** `ICSR.RETTOBASE` counts external interrupts. `4400e3` works around it with a monitor read hook, tied to the pin both ways.
  - **`7b687540`:** the NVIC's cache-maintenance writes, which today log about 2.7 MB of `Unhandled write` in 4 s of unmodified ArduPilot. `4400e`'s summary absorbs them.
- **Virtual time moves under a later Renode.** `4400e2` measured the barometers at 0.167 s against 0.343 s. So every pinned witness time must be re-measured at the same change, not later.
- **The TIM5 row:** TIM5 runs at a fixed 250 MHz. `4400e` re-owned it, with the foreign window's 4 s bound to re-measure when it is corrected (~4.5 s, [inferred]).

> **What this is:** one pinned Renode that carries both fixes, the `4400e3` hook retired through the test that was written to retire it, and every virtual-time figure the repository pins re-measured against it.
>
> It is NOT:
> - a change to any judge's rules;
> - the TIM5 correction (unless §1.1 shows it is the same change);
> - a model of ours changed to absorb a timing move.

## 1. Build

1. **Measure first, then STOP.**
   - **Which Renode to pin.** Is there a release after 1.17.0 that carries both `e7a8584c` and `7b687540`? Name it, its tag commit and its asset's sha256, read from the release.
   - **If no release carries both,** propose how the image builds Renode 1.17.0 with the two commits applied: from source, at pinned commits, with the patch files committed and cited. Give:
     - the build's cost: image size and build time;
     - its licence obligations (`TOOLS-SOURCES.md`).
   - **On a scratch image with the candidate**, measure:
     - the `4400e3` regression witness: `RETTOBASE` 0 without the hook;
     - the cache-maintenance lines: gone, or what remains;
     - every pinned virtual-time figure in tests and goldens, old against new, with a count of what moves;
     - the generated firmware's verdicts: which bytes move and why;
     - ArduPilot's foreign verdict and its witness times;
     - wall time and peak RSS against today's.

   **STOP** with:
   - the choice (a release, or a patched build), recommended;
   - the moved-figure table;
   - every golden and verdict that would move, each with its cause. A verdict that flips is a stop by itself.
2. **After the ruling:**
   - the image change;
   - the hook's retirement through its own pin test (`4400e3` §1.2): the hook is not emitted for the new pin, and the regression witness passes without it;
   - the `Unhandled write` summary keeps working, on whatever remains;
   - every pinned figure re-measured and re-pinned, with each change cited to the run that measured it;
   - `TOOLS-SOURCES.md` updated, and the §7.1 rows closed.

## 2. Acceptance

- **The regression witness** reads `RETTOBASE` 0 with no hook, and the pin test proves the hook is gone because the bug is gone.
- **No verdict flips.** Each moved figure is re-pinned from a measured run, and named in FEEDBACK.
- **Unmodified ArduPilot passes the foreign judge** on the new image, its witness times recorded.
- **The image builds reproducibly in CI** from pinned inputs. Its smoke test reads the new version.
- **The CLAUDE.md gates pass.** CI is green, and `TIER2_MIN` is exact.

## 3. Invariants and fences

- **Invariants and rulings:** `INV-03`, `INV-06`, `INV-07`; `RUL-05`, `RUL-12`, `RUL-13`, `RUL-16`.
- **Fence:**
  - `container/` (`Dockerfile`, `smoke.sh`, `TOOLS-SOURCES.md`, and any patches);
  - the CLI's monitor script (the hook's retirement);
  - the twin's and the models' tests and goldens, for re-pinned figures only;
  - the workflows that build and publish the image;
  - FEEDBACK.
- **Running beside you:** `1303b` (extraction, `aethelgard-eda`). You share no file with it. One container of yours at a time, under `capped-run`. A scratch image is tagged as scratch and never published.
- One PR, left unmerged. Write `~/.cache/aet-1605/pr-title-body.md`.

## 5. Adversarial pass, full

- **A figure re-pinned to whatever the new Renode says,** with no check that the change has a cause.
- **The hook retired while the bug remains,** or kept while it is gone.
- **An image that is not reproducible:** an unpinned clone, a floating tag, a download without a checksum.

## 7. Flag-and-stop conditions

- **§1.1.**
- **A generated-firmware verdict flips:** stop, with the bytes.
- **The candidate Renode breaks a model of ours** (an API change in a C# model): stop, and say what.
