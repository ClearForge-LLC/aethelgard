# AET-WO-4412 FEEDBACK — P4 slice B: foreign-firmware dynamics

**Builder:** Claude, worktree `~/aethelgard-4412`, branch `wo/aet-4412-foreign-dynamics`, PR #410. `main` `ba49e146` (4411, #395) merged in.
**Cadence:** a stop after (1), then (2)–(4) to the 10:43Z rulings. Stops along the way: D1's scope (11:07Z), D4's laziness (12:15Z), the re-simulation flip (13:40Z, 15:13Z) and the `defaults.parm` collision (17:17Z). Each was ruled on the bus.
**First commit:** `TIER2_MIN` 205 → 209 (`1919` F11).

## 1. Part (1): what happened before the build

The table, the DMA-request reading and the triage are in `spike/4412/RESULTS.md`. Its predictions were committed (`d790f3e8`) before any Renode run, and §5 grades them.

- **DShot over MAVLink did not exhaust memory. It hung.** Renode 1.17.0's `STM32_Timer` costs about 25 s of wall per virtual second at DShot600's bit rate, and the run hit the 120 s guard as a `ToolError` (2 of 2). The stock DMA moved the whole stream at `EN` into a `DMAR` the timer does not model, so no DShot bit left the twin.
- **A reboot was judged a crash** (PC 0xEFFFFFFE, RCC back on HSI). One reset macro without `LoadELF` fits it inside the cap: about 0.5 GB, parameters persisting.
- **Timer DMA requests (RM0433 Rev 9 Table 124, p. 697):** TIM12–14 have none. No committed DShot board sits on them.
- **Triage:** R2–R6 go after P4, R7 and R8 are closed, and the RC baud row is closed (`RESULTS.md` §3).

## 2. What was built

- **(2) The board.**
  - **D1 (a):** generated runs ignore `mcu.foreign_clock`. `generate` writes `<project>.foreign.repl` beside `<project>.repl`. `validate` holds a declared clock to ArduPilot's tree for the declared crystal (`aethelgard-ir` `data/ardupilot-h7-clock.tsv`, from `stm32h7_mcuconf.h` sha256 67f48e5f). `simulate --foreign` holds it for an ELF naming `AP_HAL::panic` (D1 (b)).
  - **D2:** `judge_foreign` judges PLLSRC = HSE at the crystal's declared fast ppm. See §6 row 1 for the 16 clocks.
  - **Sentinel-03 declares its derived clock** (`spike/4412/py/derive.py`: USARTs 100 MHz, timers 200 MHz) and nothing else. Its generated files differ from main's only in the specification sha256.
  - **CI builds Sentinel-03's ELF** in the generated matrix. Its `hwdef.dat` and `defaults.parm` are held to fixtures, `pinout` judges it, and the `foreign` job runs it.
  - **`MOT_PWM_TYPE`:** the hwdef emits it for a DShot board in `<stem>.defaults.parm` (`AP_Motors_Class.h` L360–371: DShot300 = 5, DShot600 = 6). The pinned build embeds that file; this was shown by measurement, `MOT_PWM_TYPE` = 6 read back over MAVLink on ELF 1cfc5e02. A board mixing motor protocols is refused (`MotorProtocolsMixed`).
- **(3) The motors (D4 (a)).**
  - **Our own models,** each pinned by sha256 and declared only on a foreign platform, only for DShot outputs' timers, by cancel-and-restate:
    - `models/renode/AethelgardSTM32Timer.cs` (frozen at 1e6242ef). It is eager while UDE, CCxDE, UIE or CCxIE is set, and lazy otherwise.
    - `AethelgardSTM32DMA.cs` (frozen at 4aed9519, then a530046c after the review below). It paces a stream by its DMAMUX request and bursts into CCRx through DCR and DMAR.
  - **The judge reads each frame** from the DMA model's line for each paced transfer (`aethelgard-twin` `dshot.rs`): throttle, telemetry bit and checksum. The verdict says where TIM1 was read. There is no end-of-window stop.
  - **The solver** offers a DShot timer only with an update request (`NoDshotDmaRequest`).
  - **A circular paced stream** is refused, as `foreign_dma_stream_unbounded`.
- **(4) The reboot (D5).** The reset macro runs without `LoadELF`, from a cold reset with VTOR. RCC_RSR's software-reset flags are set by hooks (RM0433 Table 56 row 4, p. 333). `boot.resets_ns` and `foreign_reboot` are reported, and parameters are judged before and after the reset.
- **The `defaults.parm` admission** (ruled 17:35Z, (a) made strict). This is `4406`'s deferred "the `defaults.parm` work order", done here. It isn't a route around `ParamDefaultsOverride` (`4400h`): the guard learns one exact file.
  - **How:** `check pinout` regenerates `generate hwdef`'s file for the topology. pincheck inflates the ROMFS copy (raw deflate) and compares it byte for byte. Equal admits it, and `load_defaults_file` with it; on ChibiOS that function loads only `@ROMFS/defaults.parm` (`Util.h:43-45`, `AP_Param.cpp:1599-1632`). Anything else is refused, naming the first differing line, and every other signature still refuses.
  - **What it proves:** this judgement is regeneration equality, so it shows that what runs is what we wrote. The MAVLink read-back shows the value.
  - **The dependency:** `miniz_oxide` 0.8.9 (MIT OR Zlib OR Apache-2.0, `forbid(unsafe_code)`), with `adler2` 2.0.1 (0BSD OR MIT OR Apache-2.0). pincheck still reaches only `aethelgard-ir`.
- **(ii), the re-simulation finding (ruled 15:27Z).**
  - **The two unstable fields:** on a DShot foreign run the verdict names `boot.not_rerun_stable` = [`boot.instructions`, `boot.pc`], with the reason. ExecutedInstructions differs by 6 in 6 of 23 dshot12 runs. The end PC did not move in the shipping configuration; it moved only in one diagnostic variant.
  - **Nothing committed binds them:** `boot_tier1` `no_committed_file_binds_a_not_rerun_stable_field`. CI holds the list as present with its reasons, never by field name.
  - **The ISR before the flip** is TIM5's (IRQ 50, ChibiOS's system timer; `spike/4412/proto/NOTES.md` §11).
  - **The judged figures** were identical across the 23 runs:

  | judged figure | across 23 dshot12 runs |
  |---|---|
  | transfer lines: count, first EN, last end | identical (6,986; 6180670750; 11999183297) |
  | every transfer word, every EN and end stamp | identical |
  | frame decode per channel | identical |
  | USART1/2/3 bytes and stamps | identical |
  | end reads: PC 0x810883c and every line but one | identical |
  | end reads: ExecutedInstructions | 0x18305D92 in 6, 0x18305D8C in 17 |

- **INV-04:** a foreign input run's archive (armread12, 6265f15f) replayed twice gives the same `state_sequence_hash` (f7566598…). Its limit is a row (§6).

## 3. Acceptance

- **Sentinel-03's ELF is built and checked in CI:** the generated row, `pinout` (judged set 10) and the `foreign` job's three runs. **The CI wall figure** (14:09Z (b)): on 43bed846 the arm run passed the flat 120 s hang guard and was killed as `ToolError`, which was the stop (21:43Z). Measured alone, three times each, on CI's runners (ruled 21:47Z (c); ardupilot-image run 45901122965, a throwaway job reverted in 622a7fc2):

  | run (12 s window) | runner CPU, nproc 4 | wall, three runs |
  |---|---|---|
  | arm (armread12) | AMD EPYC 7763 | 145.85, 144.65, 144.23 s |
  | reboot (reboot12) | Xeon 6973P-C | 78.91, 78.35, 78.74 s |
  | DShot (dshot12) | AMD EPYC 9V45 | 65.26, 66.16, 70.65 s |

  The worst, 145.85 s, is under the ruled 300 s stop. **The scaled guard (21:47Z (a)):**
  - `renode::foreign_boot_wall` = `max(120 s, 5 × 12.155 s × the window)`, 729.3 s for 12 s, by `1918` (2)'s own rule.
  - Its plants: a 125 s Renode passes in a 3 s window, and one that never returns is a `ToolError` at 182.325 s.
  - The flat-120 s mutant is killed.
  - **In the foreign job under it** (1e951a5d, ardupilot-image run 45911215499), all three on an AMD EPYC 7763: arm 140.96 s, DShot 112.98 s, reboot 104.03 s, each a pass. The worst is still arm's, and still under the 145.85 s the guard is derived from.
  - **Its cost (named, 22:29Z):** a foreign run that truly hangs now holds a CI runner until its guard, up to about 12 minutes (729.3 s) for a 12 s window, against 2 minutes before.
- **The armed input run** on Sentinel-03 arms inside the cap and the window, with DShot engaged and judged. Locally on the merged ELF c9182b88: pass, armed, TIM1 at request 15 with 0 malformed, `MOT_PWM_TYPE` 6 read back, 778 MB (495 ‰ of the cap).
- **Planted firmware and boards:**
  - a DShot timer with no request is refused at the solver (`NoDshotDmaRequest`);
  - a circular paced stream is refused by name (`foreign_dma_stream_unbounded`);
  - a reboot's second boot is judged (`foreign_reboot`, parameters read back after it);
  - a `defaults.parm` other than the topology's is refused, naming its line.
- **INV-08,** against main `ba49e146`: 18 committed specifications generate a platform, and 16 are byte-identical.
  - **Sentinel-03** differs only by its specification sha256.
  - **`sentinel-02-own`** differs by 92 lines: its generated `.repl` now runs at the solved clock (HSI 64 MHz), where main's ran at its `foreign_clock` (200 MHz). That is D1 (a) as built and ruled. Its foreign platform keeps 200 MHz, and no package or golden is bound to the generated one.
- **CI and packages:** the write dispatch on 43bed846 (run 45897665687) is green, and its artifacts are committed unchanged.
  - **`fab/sentinel-03/package`:** two files moved.
    - `MANIFEST.json`: `specification_sha256`, `topology_sha256`, `verdict_sha256`, and `FAB-NOTES.md`'s sha256.
    - `FAB-NOTES.md`: the specification sha256 line.
    - Both follow from the specification's `foreign_clock` block. The ELF (66d9b7f6) and every fabrication file are byte-identical.
  - **After the review's DMA fix (01:37Z ruling):** `bench-package-s03 --against-fab` on f69235d6 found `MANIFEST.json` alone differing, with no finding. The write dispatch on f69235d6 (run 45923520354) is committed unchanged. One file moved: `fab/sentinel-03/package/MANIFEST.json`, in its `verdict_sha256` line only (728bf399 → 23dd922a). The DMA model's pin (a530046c) is one of the verdict's tool identities. `FAB-NOTES.md`, the ELF, every fabrication file and `fab/sentinel-01-bench/package` are byte-identical.
  - **`fab/sentinel-01-bench/package`:** byte-identical.
  - **`bench-package-s03`:** 34.1 min, under `1919`'s 45-minute target.
- **Every new test is in a shard at its weight,** checked by `ci/shard.sh build` + `plan` before each push. The two `boot_tier1` tests carry Furnace estimates. The cancel-and-restate test needs no row since the reduction pass (0.3 s).

## 4. Rulings applied

- **10:43Z:** D1 (a), D2, D3 (`MOT_PWM_TYPE` last, read back), D4 (a) (own models, RM0433 by page, "lazy, never different", the seam row), D5, D6 (the architect's).
- **11:06Z and 11:30Z:** D1 (b).
- **11:51Z:** D2's fPCLK4, read through RCC_D3CFGR.
- **12:21Z:** the oracle, preload on, the per-transfer line, the ARR tick.
- **13:53Z:** isolate whose flip it is; CARGO_BUILD_JOBS and RUST_TEST_THREADS at 6.
- **14:09Z:** models only in `<project>.foreign.repl`; (a) the circular stream; (b) the CI wall figure.
- **15:27Z:** (ii) (with the architect's INV-04 correction).
- **16:31Z:** (ii) accepted.
- **17:35Z:** the strict admission; the weight rows; tools-image at 90 min.
- **19:38Z:** merge main, commit the spec, measure the wall.
- **20:16Z:** `sentinel-02-own` named in the PR body.
- **21:22Z:** the dispatch.
- **21:47Z:** (c), then (a) by `1918`'s rule; (b) is a row.

## 5. Adversarial pass (full; `spike/4412/ADVERSARIAL.md`)

- **A firmware that passes because the twin stopped it:**
  - A lazy event that had an effect fails the run (`foreign_timer_lazy_differed`).
  - A log past its budget names its frames unwitnessed (`foreign_dshot_unwitnessed`).
  - A stream ends where silicon's does.
  - There is no window stop on the DShot path.
- **A reboot that isn't one:** what Renode's machine reset clears against silicon's software reset, each item tested or named (the table in ADVERSARIAL).
- **A DMA request assumed:** requests come from Table 124 by page. Declared DShot outputs that sent no frame are said (`foreign_dshot_silent`, info).
- **An ELF in CI that isn't the bench's:** the hwdef and `defaults.parm` are held to fixtures, the clock is held at validate and at run time, and the build script is CI's own. CI's ArduPilot ELF for Sentinel-03 (c9182b88, `hwdef.h` c20df41d, run 45897560047) is bit-identical to the local build from the same committed files, so it reproduces.
  - **A correction of my own:** my earlier closure, CI's ELF against the package's, compared two firmwares. The bench flashes the engine-generated ELF (66d9b7f6), which this work order leaves unchanged. ADVERSARIAL now says so.
- **A defaults file on a DShot board:** found by CI on 76113fed. It is a finding against my own D3 work, because I never ran `check pinout` on a `defaults.parm` ELF before pushing. Ruled 17:35Z and built as above.
  - **Evidence:** every DShot ELF from the merged tree (`sentinel-03` c9182b88, `serial-dshot-9600` 4257c9b1, 4411's `sentinel-03-radio`) is admitted and judged, and main's binary refuses each (`spike/4412/out/defaults-pinout/`).

## 6. Rows

1. **D2: ArduPilot's tree is at VOS1's maxima.** `stm32h7_mcuconf.h` 67f48e5f runs every bus and kernel clock exactly at VOS1's maxima. With the crystal's declared +102 ppm, each of 16 is over at its fastest: sys_ck, HCLK, PCLK1–4, the APB timers and the USARTs, as `vos_limit_at_boundary`. Whether the limits include oscillator tolerance is not stated (DS12117 Rev 10 Table 111, pp. 209–210; RM0433 Rev 9 Table 60, pp. 355–356). This is a finding about ArduPilot's tree, not ours.
2. **The Phase 2 seam:** `AethelgardSTM32Timer` against Tviburi's `GuardedSTM32_Timer`. **Ours is the base**: the guard refuses UDE, CCxDE, DCR and every DMAR write, so DShot cannot run under it, and it inherits stock's per-update cost.
   - **To port onto ours:** the guard's refusal records, the §38.4 allow-list, BDTR.LOCK, MOE/OSSI/OISx gating, CCxP polarity, the CCxNE/CCxNP and width refusals.
   - **One conflict:** CCRx at reset. RM0433 says 0; ours reads stock's back.
3. **Renode re-simulation is not bit-stable.** Stock: dma2 +10 ns at 18 accesses in 1 of 20 dshot12 runs. Ours: ExecutedInstructions, 6 of 23. For Tviburi or `1605`.
4. **Generated-path verdict fields that read log stamps,** which can move between re-runs:
   - the device step checks' `observed_ns` (`own_lines`, `renode.rs:1288-1296`);
   - the observer edges behind pin and motor witnesses (`observer_edges`, `:1981`);
   - the timer hooks and anchors (`hooks_of`, `:3230`).

   No committed golden compares a re-run's stamp to another's today, but any future one must allow for this.
5. **INV-04's limit:** the foreign trace holds the input, the stimuli and the firmware's transmission, not the DMA transfers. So a replay does not re-judge `boot.dshot`.
6. **Slice H, at its front (ruled 23:00Z): shard tools-image's unsharded `cargo test`.** It ran 57.8 min on 76113fed (cancelled at 60), then 63.8 min on a8474dda, under the interim 90. On 43bed846, after 4411's merge, it ran 87.6+ min and was cancelled at 90. Now 150 as a second interim; the suite grew about 24 minutes with that merge. Its own work order after #410.
7. **Slice H: faster DShot models** (ruled 21:47Z (b)). Compute a transfer's bursts from the update period, proven against the eager reference. Today the timer is eager while UDE is set, at about 4.8 M wakes per virtual second at DShot600, and the DMA model logs about 8,000 lines per 12 s armed run. The arm run's 145.85 s on CI is the figure to beat.
8. **RUL-13's guard text** needs an amendment row (21:47Z), for the foreign run's scaled hang guard (`renode::foreign_boot_wall`).
9. **For 1802 (passed on at 20:16Z):** the `defaults.parm` header carries `CARGO_PKG_VERSION`. The 0.3.0 bump moves every DShot `defaults.parm` fixture, and the strict compare refuses an ELF built before it.
10. **Slice F:** `simulate --foreign` requires `AP_HAL::panic`, and the clock rule assumes ArduPilot. A kernel of one's own needs a declared firmware ecosystem.
11. **Bidirectional DShot:** the hwdef emits no `BIDIR`, because nothing declares it (R3's open half).

## 6a. The architect's review (2026-11-26T23:39Z, on 1e951a5d): seven fixes and a nit, one push

Each fix has a plant, and each check has a mutant that is killed (`~/.cache/aet-4412/mutants-review.txt`, V1–V7).

1. **A decompression bomb** (`pincheck` `elf.rs`). The stated size is held to the topology's file **before anything is inflated**, and a mismatch is a `ParamDefaultsOverride` naming both sizes.
   - **Plant:** 256 MiB of zeros, deflated to under 1 MiB and stating 256 MiB, is refused by its size, with the process's peak memory grown by under 64 MiB.
   - **The rest held:** contents that don't inflate to exactly the size are still `MalformedRomfs`, and a same-size change still names its line.
2. **Cut-off transfers** (`twin` `dshot.rs`). The DMA model's ` (disabled, NDTR k)` line is parsed, and every position's frame is `Cut`, counted malformed and named. It is never skipped.
3. **The DShot rate** (`foreign.rs` `motor_protocol_unmet`).
   - A timer counting DShot bits (at least half DShot300's rate) is held to its outputs' declared rate **within one prescaler step**: a bit's `(PSC+1)(ARR+1)` cycles within ARR + 1 of `kernel_hz / bit_rate`. ArduPilot sets PSC to the nearest (or the next faster) whole divider of the timer clock (`AP_HAL/RCOutput.cpp:80-97`).
   - **Plant:** DShot300 on a DShot600 board is refused, as `foreign_dshot_rate_unmet`; as ArduPilot sets DShot600, it says nothing; PWM stays the info finding.
   - **Why the rate check matters:** an all-zero frame passes the checksum, so frames decoded at the wrong period could pass on their own. The rate check is what stops that.
   - Sentinel-03's TIM1, at 336 cycles against 333.33, is inside the step.
4. **A re-enabled paced stream** (`AethelgardSTM32DMA.cs`). EN now starts the stream at M0AR and PAR; RM0433 Rev 9 §15.3.16, p. 654, says resuming means rewriting them. **No judged figure and no fabrication byte moves, but Sentinel-03's `MANIFEST.json` does** (corrected after the 01:37Z ruling). My 00:57Z note said no package byte moves, and that was wrong. No generated platform or package loads the model, but its sha256 is pinned in `model.rs`. That pin reaches the verdict's tool identities, so `verdict_sha256` moved, and `bench-package-s03 --against-fab` found `MANIFEST.json` alone differing, with no finding. **A model-hash change moves `verdict_sha256`, so any model edit needs the write dispatch.** The manifest from that dispatch (run 45923520354) is committed unchanged; see the packages row below. Sentinel-03's three runs on ELF c9182b88 were rerun with the new model, and the judged figures are identical to the runs before it: the verdict, the finding kinds, `boot.dshot`'s counts (8,003, 6,785, 1,888), first and last stamps and per-position tallies, the read-back and the reset. ArduPilot never disables a paced stream before its end.
5. **A paced stream aimed at a register other than DMAR** (`AethelgardSTM32DMA.cs`). It is refused and not started (`AETHELGARD-DMA-UNMODELLED`), and the judge fails it as `foreign_dma_stream_unmodelled`.
   - **Why:** the DMA destination "must point to the virtual register TIMx_DMAR" (p. 1586), offset 0x4C of the timer's 1 KiB block (pp. 1616, 1700, 1841). The per-write acknowledgement a CCRx target would need is not modelled. That is the smaller honest fix.
   - ArduPilot's DShot always aims at DMAR.
6. **The CLI defaults test** now asserts exit 1 and a pinout verdict with its `configured` pins, i.e. that the pin judgement was reached.
7. **A missing `TIMx.ARR`:** a timer whose transfers were seen but whose period wasn't read is named, once, as `foreign_dshot_period_unread`.
- **Nit:** `dshot_silent`'s timers are a `BTreeSet`.
- **Named, not fixed (review nits):**
  - The DMA model logs each item at MSIZE, but copies it at PSIZE. ArduPilot's DShot streams use the same size for both.
  - `generate hwdef` writes `defaults.parm` after `hwdef.dat`, non-atomically. A failure between the two writes leaves a hwdef without its defaults, which `check pinout` then refuses, as a mismatch.
  - The timer model's `burstStart` is not cleared by `Reset()`, which clears `burstWords` (L172). Nothing reads it except to set it, so nothing reads a stale value.

## 7. Reduction pass, mutants and gates

- **Reduction pass (diff):** −19 lines net · 1 duplicate removed · 1 mechanism replaced · 0 protected kept · 1 finding below.
  - It ran through a subagent, and I verified each candidate.
  - **The cancel-and-restate test** now emits Sentinel-03's two platforms with the two calls `generate` makes, instead of a whole `generate`: 69 s on CI to 0.3 s, and its weight row is gone. Its before and after loops were one check, written twice.
  - **Kept, named:** the C# models log their marks (`AETHELGARD-DMA-UNBOUNDED`, `lazy: a skipped event at`), and `foreign.rs` holds the same strings to read them. That is the cross-language contract; a test plants each line.
  - **Proof:** mutants on `retype.rs` (a route keeping the cancelled name, an update block on it) are killed by the reduced test.
- **Mutants:** one per guard this work order adds, each against its suite, through `spike/4412/mutants/run.sh` (the file restored and touched).
  - **Every mutant ended killed.** Four survived at first, and each was killed by a test I added: the timer route and GPIO-route mutants, the keep-list mutant (killed by a unit-level keep assert), and the `defaults.parm` size mutant (killed by a stated-longer case).
  - **The `defaults.parm` admission:** 6 of 6 (`~/.cache/aet-4412/mutants-defaults.txt`).
- **Gates:** `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace` pass at each push. `ci/shard.sh build` + `plan` pass in the tools image (guard ok: 3551 names, 20 shards).

## 8. Status

**READY** on PR #410, at the head this file is pushed with. It is the last push, unmerged, and the architect merges.
- **Built:** every part of the work order, and every ruling through 23:00Z.
- **Merged in:** `main` `ba49e146`.
- **Packages:** the write dispatch's artifacts are committed unchanged.
- **Reverted:** the throwaway (no lifted guard remains).
- **Gates:** fmt, clippy, the whole suite, and the shard build and plan pass.
- **CI on this head:** running at READY, and reported on the bus.
- **Tokens:** revoked after each push.

The PR title and body are in `~/.cache/aet-4412/pr-title-body.md`. The rows in §6 are for the architect's docs PR.
