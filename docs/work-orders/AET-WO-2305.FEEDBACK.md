# AET-WO-2305 — FEEDBACK (build, one measured stop)

Branch `wo/aet-2305-bor-recovery`, from `main` at `2fda6e13`. The work order was read from the docs branch (copy at `~/.cache/aet-2305-wo.md` on Furnace); it is not committed here.

Every figure is labelled **measured**, **read at source** or **inferred**. The predictions (`spike/2305/PREDICTIONS.md`, commit `f08ad2b9`) were committed before the runs (`b4510253`). One trial came before them: Sentinel-01 minimal, run once to learn the log's format. Its rows are marked *seen*.

## How it was measured

- **The CLI:** `main`'s, built in the pinned tools image from a scratch copy carrying one measurement-only patch, never committed. The patch writes Renode's whole log and the composed monitor commands to files (`AET2305_LOG`, `AET2305_CMDS`).
- **The runs:** one campaign per board, with its undisturbed boot first (`spike/2305/scenarios/*-undisturbed.json`, no override), then its droops. `spike/2305/sh/run-all.sh` runs them under `capped-run` (8 GiB, swap 0, read back).
- **The analysis:** `spike/2305/sh/analyze.py` reads the archive (kinds 13, 18, 19) and the log.
  - **The boot word's instant** is exact: the boot's start plus the `AETHELGARD-BOOTWORD` hook's instruction count × 10 ns. The start is the release + the 10 ns nudge, or 0 undisturbed.
  - **A device line's instant** is Renode's stamp, which is good only to a quantum (100 µs).
- **The results:** `spike/2305/out/measure-main.jsonl`, `measure-phase.jsonl` and `measure-timebase.jsonl`.

## §1.1 The measurement table (measured on `main` 2fda6e13), predictions beside

| Run (board, droop) | Release (ns) | Boot word after the boot's start | First identity line (stamp, after start) | Witness lines at the probe | First device's lines by the boot word | Verdict | Predicted |
|---|---|---|---|---|---|---|---|
| S-01 minimal, `droop-bor3` | 1,231,690 | 25.70 µs | 0 | 2 (U2: `Write D0`, `Read 60`) | 2 | pass | *seen* |
| S-01 minimal, `droop-tolerated` | no crossing, no reset | 25.70 µs | 0 | — | 2 | pass | *seen* |
| S-01 bench (`examples/sentinel-01.json`), `droop-bor3` | 1,231,690 | 28.13 µs | 0 | 2 | 2 | pass | 1.22–1.24 ms, 25–40 µs, 2, pass: **met** |
| `motors-dshot600`, `dshot600-droop` | 331,690 | 21.75 µs | 0 | 2 | 2 | pass | 0.32–0.34 ms, 25–60 µs: **boot 3.25 µs earlier than the range**; the rest met |
| `four-layer`, `four-layer-droop` (BOR 0, 3.1 V) | no crossing | 16.70 µs | 0 | — | 2 | pass | ~1.23 ms release, 25–40 µs: **missed. No BOR0 crossing, so no reset; my prediction overlooked BOR level 0** |
| `spi-only`, `spi-only-droop-bor3` | 1,231,650 | 15.24 µs | 0 | 8 (U4, SPI) | 9 | pass | "2 (U3's identity)": **missed. The first check is U4 on SPI, not U3**; boot 20–40 µs missed by 4.76 µs |
| Sentinel-03, `droop-tolerated` (3.0 V) | no crossing | 39,127.18 µs | 66.17 µs (U7) | — | 3 | pass | no crossing, pass: **met** |
| **Sentinel-02-shaped, `s02-droop`** (`2303` F11) | 332,670 | **39,072.41 µs** | 28.42 µs (U2) | 2 | 2 | **fail, `brownout_unrecovered`** (boot word 0 at +100 µs) | 39.0–39.5 ms and the fail: **met**; "4–8 lines by the boot word": **missed, 2.** U2 (BME280) has only its identity step |
| **`4600`'s logger, `droop-bor3`** | 1,231,850 | **19,522.27 µs** | 22.17 µs (U20) | 2 (U20's identity **and** configure: the witness already fails) | 3 | **fail, `brownout_unrecovered`** | 19.5–19.7 ms and the fail: **met**; "U10's identity": **missed. The first check is U20 (BMP581)**; 4–8 lines: missed, 3 |
| **`sentinel-01-ext-bmp581`, `droop-bor3`** | 1,231,770 | **19,523.76 µs** | 0 (U2) | 2 | 2 | **fail, `brownout_unrecovered`** | met, except 4–8 lines (2) |

**Undisturbed boots (measured):** on all nine boards, the undisturbed boot word counted from 0 equals the recovered boot word counted from the boot's start, to the instruction. **Predicted: met.**

**Release phase (measured):** the release was moved through one quantum, by six `start_ns` offsets from 0 to 99,990 ns, on Sentinel-01 minimal and on the logger. The recovered boot word did not move: 25,700 ns and 19,522,270 ns each time, a spread of 0.

## §1.1 The twin's time base

- **Measured, exactly:** I doubled the BMP581's `settle_us` in a scratch copy of `parts.json` (10,000 → 20,000; never committed) and ran ext-bmp581's undisturbed boot.
  - Boot word: 19,523,760 → 39,011,460 ns, so Δ = 19,487,700 ns.
  - Spins: 649,590 → 1,299,180.
  - **So 30.000 ns per spin:** three instructions at Renode's 100 MIPS.
- **Read at source:** `settle_spins` sizes the loop at `ceil(us × hsi_fastest / 10⁶)`, and HSI's fastest is 64.959 MHz (`svd-hal` `board.rs`, `AET-WO-4230` amendment 1).
- **Inferred from both:** the twin runs the settle **1.9488 × the declared time** (3 × 64.959 / 100), the same on every board.
  - The "about 1.95×" in `2303`, `4600` and `4202` is this constant plus the board's own bring-up:
    - logger 34.57 µs;
    - ext-bmp581 36.06 µs;
    - S-02-shaped 97.01 µs;
    - S-03 151.78 µs.
  - That is the boot word less 30 ns × spins.
- **The time base is a constant per board,** and one undisturbed run fixes it. §7's third condition does not fire.

## The stop (§7 condition 2) and the fork it raises

**Measured or read at source:**
- **The campaign makes no baseline run.** `cli` `campaign.rs` `run_one` boots each scenario alone, and the runs share nothing. A campaign need not hold a no-fault scenario at all.
- **No record holds a boot word's instant.** Kind 10 holds the word, not its time. Kind 19's `at_ns` is the probe's instant.
- **Replay pins the constant.** `chaos` `campaign.rs` `check_run` requires each probe at release + `RECOVERY_BOUND_NS`, and `droop_end_ns` adds that constant to the span check before any run.

**So a derived bound needs all four:**
- a boot the judge makes;
- a value the archive does not hold today;
- a trace version (17, after `3305`'s 16);
- a replay rule.

§1.2 allows "a run the judge makes and archives"; §7 stops on "a change to what the campaign archives". I reported this on the bus (2026-11-20T04:50:44Z) with a proposal, and I am holding §1.2 for the ruling.

**Proposal, if ruled in:**
- **The baseline boot:** inside the droop scenario's own `simulate`, the same prepared boot with no resets: the same firmware copy, booted platform, models, stimuli and other overrides. Then the droop boot.
- **Archive and replay:** one new record with the undisturbed boot word's instant, the margin and the bound. Kind 19 probes at release + nudge + bound. Replay checks bound = instant + margin and each probe's place.
- **Failures by name:**
  - a baseline with no boot word is `brownout_unrecovered`, naming the missing bound (`INV-06`);
  - the span check moves after the measurement and refuses by name.
- **The margin:** one quantum (100 µs) is recommended. That is the twin's one timing freedom: peripherals synchronise with the core at quantum edges. The measured spread is 0. The alternative is T/8.

## Built so far (holds under either ruling)

- **§1.3 The witness (`chaos` `power::witness_recovers`, used by the CLI judge and by replay):**
  - The lines from the release must begin with the first device's identity lines.
  - Every later line must continue that device's own later steps, in the script's order, each step whole.
  - **A repeated identity read fails:** that is a boot that started over, not a recovery. I narrowed "extra lines after it are allowed" to the device's own later steps so a reset loop cannot pass (adversarial pass).
  - A wrong identity value still fails `brownout_unrecovered`, by its word and by its line.
  - An archive written before this build still replays: an exact one-step witness is the rule's first case.
- **§1.4 Coverage (`cli` `verify.rs` `uncrossed`):**
  - On the MCU's supply, at the specification's `mcu.bor_level` > 0, a droop below the band whose loaded floor stays at or above that level's falling threshold covers nothing.
  - Its row names the crossing it lacks, for example: ``rail `VDD_3V3` (`x` crosses none: at BOR level 3 a droop must cross VBOR3's falling level, 2610 mV; its loaded floor is 2850 mV)``.
  - The level is the board's, not the droop's field, so no scenario declares its way past it.
  - It covers nothing for the whole scenario, as an eased override does today, so it fails closed.
  - The L697 loaded-floor test moved from Sentinel-01 (level 3) to `four-layer` (level 0), where the band alone decides.
- **Mutants, unit level (measured, each restored by its inverse edit):**
  - an exact one-step witness: killed;
  - a wrong identity value passing: killed;
  - a non-crossing droop covering: killed.
  - The tier-2 forms the WO names (a constant bound on a waiting board; the exact witness on the logger) wait on §1.2.

## Findings

- **F1 · §1.4 leaves Sentinel-03's MCU rail unmet** (measured, by the rule; asked on the bus). Its one droop, `droop-tolerated`, is at BOR level 3 with a loaded floor of 2.95 V, so it crosses nothing. A crossing droop for Sentinel-03 is a new scenario file, outside this fence, and it recovers only under a derived bound (a 39.1 ms boot).
- **F2 · The predictions' misses** (above): BOR level 0 renders no reset at 3.1 V; `spi-only`'s first check is SPI; the logger's first check is U20, not U10. The judge's first check is the image's first, and each board's differs.
- **F3 · Close to `INV-07`** (inferred). A bound read from the generated firmware's own undisturbed boot is computed from the artifact under test. It judges that a droop leaves the bring-up as it was, not that the bring-up is fast enough. What caps it is the margin and the boot judge's own checks on the undisturbed boot. No declared requirement bounds a bring-up's length today.
