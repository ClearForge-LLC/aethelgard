# AET-WO-4400h FEEDBACK

**STOPPED on §7, "the cap fails before 10 s,"** with the curve (`spike/4400h/RESULTS.md` §1). The decisions are in RESULTS §5 (D1–D5).
- Part (1) is measured.
- Parts (2)–(4) are predicted and harnessed, not run.
- The branch commits only `spike/4400h/` and this file.

## 0. Gates and passes

- **Gates**, on the branch in the tools image `1e0ab105…`:
  - `cargo fmt --all -- --check`: 0;
  - `cargo clippy --workspace --all-targets -- -D warnings`: 0;
  - `cargo test --workspace`: see §4.

  No crate changes on the branch. The window patch is `spike/4400h/patches/window.patch`, measured in a detached worktree and never committed to a crate.
- **Adversarial pass (targeted, WO §5):**
  - **A window that holds only on a quiet host: it does.**
    - Main's own 5 s window was killed by the 30 s wall bound at load 22–28 (881‰). At load 7 it took 9.8 s (514–526‰).
    - 8 s passed only at load 9, with the log at 97 % of its budget.
    - Two cores at 5 s raise the share from 514–526‰ to 600–624‰.
    - This decided the proposal: keep 5 s.
  - **Arms that are not arms, and the injection's own traffic in the queue:** not reached, since the stop came first. The controls are built and committed:
    - zero-frame padding of the same byte count;
    - the parameters without the arm;
    - no injection;
    - monitor accesses carrying no PC fall out as `judge` rows.

    The board has no motor output, so any arm on it is by construction "armed with outputs disabled" (`PREDICTIONS.md` P2.7).
  - **My own cause claim:** the 768 MiB heap-limit replay shows that at least 768 MiB of managed memory is live 2 s after the IMU starts. It does not show that all the growth is managed; RESULTS §1.3 says so.
- **Reduction pass (diff):** −4 committed files (generated plan JSON; `py/plans.py` is their one source), 1 duplicate removed (`ten-cap.sh` folded into `ten.sh`), 0 dead paths removed, 0 protected kept, 1 finding below (F8).

## 1. Findings

| # | finding | where | proposed owner |
|---|---|---|---|
| F1 | **Main's 5 s foreign window fails on Furnace under load.** The 30 s wall bound killed it at load 22–28 (wall 30.88 s, 881‰). CI's runners are not Furnace, but 4400f F1's caution (RSS follows wall time) holds at today's bound | RESULTS §1.1 `w05-a` | the architect: a watch row |
| F2 | **The foreign log budget is spent by per-transaction lines** of the IMU's SPI traffic: about 2.17 MB per virtual second from 4.4 s. The parts are the device and mux NOISY lines the judge enables, the CLI's own `AETHELGARD-CFG2` hook, and Renode's SPI slave-mode ERROR. Each is needed once. The window cannot pass 8 s until they are summarised like `Unhandled write` | RESULTS §1.3; `flood-kept.txt` | D2 (after 2314 merges: `foreign.rs`) |
| F3 | **Renode's RSS grows about 100–120 MiB per virtual second with the IMU's traffic, and at least part of it is live managed memory.** It is not the log, not the hooks, not the GC lagging, and not our ICM model's buffers. What holds it is not found. This bounds every foreign window, and any campaign that runs ArduPilot long | RESULTS §1.3 | D2 (b): a heap dump |
| F4 | **A window's end can land on a firmware transient.** At exactly 6.000 s, GPIOB PB13 reads output/AF0 (`register_mismatch`) with and without the log. 5, 7 and 8 s read clean. A new bound needs its end placed, or the read repeated | RESULTS §1.4 | D2 |
| F5 | **The twin cannot deliver bytes to a foreign firmware's DMA-received USART.** Renode's USART `ReceiveDmaRequest` is never wired to `dmamux1`, and `IDLE`/`IDLEIE` are tags. SERIAL0 has no `OPTIONS` parameter, so ArduPilot cannot be told to avoid DMA | RESULTS §2 | D3 |
| F6 | **No CI-built ELF has motors and an IMU,** so TIM1/TIM2 cannot be reached by arming any ELF CI has. `sentinel-02-own` has `motor_outputs: []`. `serial-dshot-9600` has no IMU. | bus, 2026-11-24 | D5 |
| F7 | `docs/work-orders/AET-WO-4400h.md` was not on main at `29b3a97d`, though the kickoff said it was. It landed in `ef3b43d9` (#391) during the spike, identical to `~/.cache/aet-4400h-wo.md`, the copy I worked from. Main is merged into the branch | — | none |
| F8 | `spike/4400h/sh/window.sh` writes `log` and `stop` fields that are always `[]` and `null`. They were kept because the committed rows were produced by this script as it stands | `sh/window.sh` | none (a spike file) |
| F9 | The WO says "four builders"; the kickoff says about eight. Measured load ran from 2 to 28 on 20 cores during this spike, so a quiet-host number is a time-of-day number | `out/win/*/load.txt` | the architect |

## 2. What I would do next, in order

1. **D4:** run (2)–(4) in replay now. The arming path, its failed checks and persistence are independent of the bound, and they shape D3. The bank-2 result is `TVI-WO-1304`'s.
2. **D2:** the window's cost, then a ruled bound.
3. **D3:** the USART receive, then an armed foreign window in the judge itself.
4. **D5:** a motored board, when ruled, for TIM1/TIM2 and DShot.

## 3. Provenance

| what | value |
|---|---|
| ELF | `5eace4ca…` (run 45410032458) |
| Renode infrastructure source | `cd4b002a` (tarball sha256 `83e6af97…`, checked) |
| ArduPilot source | `fb773567`, copied from the pinned ardupilot image `e1809b90…` |
| Tviburi readers | main `d22dad8` by `git show` (`out/tviburi-readers.sha256`) |
| Source readings | three read-only subagents; every line a prediction rests on was spot-checked |

## 4. The test gate

(filled from the gate run)
