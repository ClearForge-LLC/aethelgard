# AET-WO-4400e3 — FEEDBACK (build)

Branch `wo/aet-4400e3-rettobase`, from `main` at `7d52d29e`. The work order is read from `~/.cache/aet-4400e3-wo.md` (docs PR #216). It was ordered by the ruling on #215 (comment 6961943794, fix (1)).

Every answer is labelled **measured**, **read at source** or **inferred**.

## Sources (read at source)

- **Armv7-M Architecture Reference Manual,** ARM DDI 0403E.b (ID120114), §B3.2.4 "Interrupt Control and State Register, ICSR", p. B3-657. The PDF is from `documentation-service.arm.com/static/5f8fef3af86e16515cdbf816`, sha256 `2d4af213859ed4650066b28937e3fc7a62974e521d0ff6df965bb3766d171f2c`, recorded in `~/.cache/aet-ds/SHA256SUMS`. The text:
  > "RETTOBASE, bit[11]. In Handler mode, indicates whether there is an active exception other than the exception indicated by the current value of the IPSR: 0 There is an active exception other than the exception shown by IPSR. 1 There is no active exception other than any exception shown by IPSR. In Thread mode the value of this bit is UNKNOWN."
- **Renode 1.17.0** (the pinned image's build `1.17.0+20260907gitf1dd1b4af`; `renode-infrastructure` `cd4b002a`), `src/Emulator/Cores/Arm-M/NVIC.cs` L1079: `RETTOBASE` = `activeIRQs.Intersect(SystemException).Count() <= 1`, which counts system exceptions only.
- **Upstream `e7a8584c`** (2026-10-07, "[GH#941] NVIC: Make ICSR.RETTOBASE take external interrupts into account"): `activeIRQs.Count() <= 1`, which counts every active exception. It is in no release (`4400e2` FEEDBACK).
- **The pin:**
  - The CLI already reads `renode --version`: it refuses any version but v1.17.0 (`main.rs` `renode_pinned`) and parses the `build:` line (`renode::build`).
  - The image installs Renode from `container/Dockerfile` L42–50: `renode-1.17.0.linux-portable.tar.gz`, sha256 `4ba7c68b…`.

## The design (read at source; nothing run yet)

- **The hook:** one monitor command, `sysbus SetHookAfterPeripheralRead sysbus.nvic "…"`.
  - On a read at offset `0xD04` (`ICSR`), it sets bit 11 to `activeIRQs.Count <= 1`. It reads the NVIC model's own active-exception stack, which is upstream's expression exactly. It passes every other bit through.
  - **Why the model's stack, not IABR plus SHCSR (the `4400e2` spike's choice):** the architecture defines `RETTOBASE` from every active exception. HardFault and NMI have no active bit in `SHCSR` or `IABR`, so the spike's count missed them.
- **Where:** it goes in every boot's monitor commands, right after the platform loads, while `renode --version` names the pinned buggy build.
  - A build measured to carry the fix gets no hook. That list is empty: no Renode carrying the fix has been measured here.
  - Any other build is refused by name before the boot (`INV-06`): whether its `RETTOBASE` is architectural is a fact the twin does not have.
- **The pin tests:**
  - **tier 1:** the Dockerfile's Renode tarball and version equal the constants the hook is tied to;
  - **tier 2, in the image:** the image's `renode --version` build is the buggy one; the bug is present without the hook (a test named for it); and the hook corrects it.
- **The regression firmware:** a dependency-free fixture, like `forge-log`.
  - It gives TIM5 (IRQ 50) priority `0x70` and DMA2 stream 1 (IRQ 57) `0xA0`, the priorities `4400e2` measured. It enables both and pends IRQ 57 from thread mode.
  - IRQ 57's handler pends IRQ 50, which preempts it.
  - It records `ICSR` in thread mode, in the outer handler and in the inner one, then disables both and parks.

## Predictions — written before any run

| id | What | Prediction |
|---|---|---|
| P1 | The regression firmware **without** the hook | the inner handler reads `RETTOBASE` **1** (the bug); the outer reads 1; thread mode reads 1. `VECTACTIVE` is 66 inner and 73 outer |
| P2 | **With** the CLI's hook | inner **0**; outer 1; thread 1. Every other bit of each word equals the run without the hook |
| P3 | `4400e`'s ELF under the CLI's newly emitted script (only the ELF swapped) | no fault; reaches `Init Gyro`; polls `FIFO_COUNT` |
| P4 | `4400c2`'s repro with the hook from the CLI's emitted script | no fault past 1.6016 s; answers the `COMMAND_LONG` |
| P5 | Every generated-firmware verdict (the reference and the bench board at 100 ms; every scenario's campaign line) | **byte-identical** to `main`'s, outside the `memory` sample. The generated firmware enables no interrupt, so no `ICSR` read ever differs, and the judge's own `ICSR` read happens in thread mode |
| P6 | The hook's cost on a generated boot | below the noise of the 100 ms boot's wall time |
| P7 | The reflection on a private field | works in Renode 1.17.0's IronPython (`GetField(..., NonPublic \| Instance)`) |

## Results — measured after the predictions (`c6b27884`), on the final commit `f5e414e3`

### What was built

- **`crates/aethelgard-cli/data/rettobase-hook.monitor`:** the one monitor command, `sysbus SetHookAfterPeripheralRead sysbus.nvic "…"`. On a read at `0xD04` it returns the model's value with bit 11 set to `activeIRQs.Count <= 1`, the NVIC model's own active-exception stack read through reflection, which is upstream's expression. It returns every other offset unchanged.
- **`src/renode.rs`:**
  - `RETTOBASE_HOOK` (`include_str!` of that file);
  - its doc carries the citations: Armv7-M ARM DDI 0403E.b §B3.2.4, p. B3-657; `NVIC.cs` L1079 at `cd4b002a`; `e7a8584c`; `chcore.c` L315;
  - `BootPlan::commands` emits it in **every boot**, right after `machine LoadPlatformDescription` (the NVIC exists from then) and before `sysbus LoadELF`. It is a peripheral hook, so it survives a droop's `machine Reset`.
- **The pin, tested both ways:**
  - **unit** `the_rettobase_correction_is_tied_to_the_dockerfile_s_renode`: `container/Dockerfile` pins exactly the Renode v1.17.0 asset with sha256 `4ba7c68b…`; the build constant starts with the pinned version; the tier-2 test checks the same build;
  - **tier 2** `the_pinned_image_s_renode_is_the_build_whose_rettobase_the_cli_corrects`: the image's `renode --version` names `1.17.0+20260907gitf1dd1b4af`;
  - **tier 2** `renode_1_17_0_reads_rettobase_1_in_a_handler_nested_over_an_external_interrupt`: **the bug, by name.** Without the hook, the nested handler reads `RETTOBASE` 1, with `IABR1` showing both IRQs active. When the pinned Renode stops having the bug, this fails, and the hook must go.
  - **tier 2** `the_cli_s_correction_reads_rettobase_0_nested_1_alone_and_moves_no_other_bit`: with the CLI's hook the nested read is 0; the lone handler's read is 1; thread mode is unchanged; the two runs differ in nothing but the nested read's bit 11.
- **The hook cannot vanish silently:** the stubs of `tests/boot_tier1.rs` and `tests/campaign_tier1.rs` now accept a boot's command line only with exactly `data/rettobase-hook.monitor` between the platform load and the ELF load. All 52 and 67 of their tests run through it.
- **The regression firmware** `tests/fixtures/rettobase`:
  - no dependencies, built offline in the image like `forge-log`;
  - it sets `AIRCR` `PRIGROUP` 3; TIM5 (IRQ 50) at `0x70`; DMA2 stream 1 (IRQ 57) at `0xA0`;
  - it pends 57; 57's handler pends 50, which preempts it;
  - it records `ICSR` in thread mode, outer and inner, plus `NVIC_IABR1` in the inner handler, so the nesting is witnessed rather than assumed (adversarial M8, below);
  - then it disables both and parks.
- **Docs and CI:** the CLI README's *Booting firmware* step 2 and a new paragraph, *The `RETTOBASE` correction*. `TIER2_MIN` 180 → **183**, for the three tier-2 tests in `twin_tier2.rs` (`ci/shard.sh`'s `TIER2_RE` counts that binary).

**A design decision, named.** The kickoff asked for the hook "emitted only while the pinned … Renode is the one with the bug, read … as the CLI already reads it". I first tied the emission to the runtime `build:` line, refusing any other build by name. **That would have broken `AET-WO-2205` §1.1's ruled test** (`campaign_tier1.rs`: a stub `renode` reporting `9.9.9+20991231gitdeadbeef0` must still run and archive that build). So the tie is static instead:
- the unit test binds the Dockerfile's asset;
- the tier-2 test binds the image's build;
- the bug test binds the bug itself.

**The hook is idempotent on a fixed Renode:** it computes exactly upstream's fixed expression. So a 1.17.0 development build outside the image gets a correction that changes nothing. On a build where the private field `activeIRQs` were renamed, the hook would raise a Python error per `ICSR` read. That residual is named; the pin tests force a re-measurement first.

### §2 Acceptance, measured (tools image, under `capped-run`; each repro under the CLI's own emitted script, captured from `/proc` with the CLI built from this commit, only the ELF swapped)

| Acceptance | Measured | |
|---|---|---|
| The CLI emits the hook | the captured `renode` argv holds `sysbus SetHookAfterPeripheralRead sysbus.nvic …` once, between `LoadPlatformDescription` and `LoadELF` | held |
| `4400e`'s ELF (`a799dfb6…`, CI run 44148995663) | **no fault through 4.0 s** (one `RunFor`, no observation hook): `Init Gyro` on the console; ICM-42688-P identity at **3.569253830 s**; **975 `FIFO_COUNT` reads** (`0x00 0x00`, the FIFO is `4220c`'s); idle (`PC` `0x08103C3C`, `ICSR` `0x800`) at 4.0 s; 10.1 s wall, 500 MiB | held |
| `4400c2`'s repro (`c8a2c9fa…`, platform `p2-spi-usart`, three `COMMAND_LONG` at 1.5 s) | **past 1.6016 s with no fault:** idle (`ICSR` `0x800`) at 1.0, 1.5 and 2.0 s; it answers (`AUTOPILOT_VERSION` with `fb773567`, `d103983f`); 7.3 s wall, 500 MiB | held |
| The regression witness | bare: thread `0x800`, outer `0x849`, inner **`0x842`**, `IABR1` `0x02040000`. With the hook: inner **`0x042`**, every other word the same | held |
| **No generated-firmware verdict moves** (one firmware build each, judged by `main` `7d52d29e`'s CLI and by this commit's; each side run from its own tree): both boards' `simulate --for 100ms`; campaigns of all 25 scenarios (the reference's 4 and 12 seeded, the bench's 5, `motors/` 2, `motors-pwm` 1, `serial/` 3) | **the campaigns' 54 archive files are byte-identical.** Their printed lines are identical but for `run`, the archive's own path. The two `simulate` verdicts are identical in all 556 and 586 fields but `input.path` (`/cache/main-src/boards/…` against `/repo/boards/…`, the same file in the two checkouts) and `memory` (the RSS sample, which the README excludes from findings and archives) | held |
| The pin test | as above, unit and tier 2 | held |
| The gates | `cargo fmt --all -- --check` and the bridge's clean; `cargo clippy --workspace --all-targets -D warnings` clean, and with `--features with-tools`; `cargo test --workspace`: **132 suites, 2,178 passed, 0 failed**; the package gate as CI runs it packages `data/rettobase-hook.monitor` | held locally; CI below |

### Predictions beside results

| id | Predicted | Result | |
|---|---|---|---|
| P1 | without the hook: inner `RETTOBASE` 1, outer 1, thread 1; `VECTACTIVE` 66, 73 | `0x842`, `0x849`, `0x800` | held |
| P2 | with it: inner 0; nothing else moves | `0x042`; the rest identical | held |
| P3 | `4400e`'s ELF reaches `Init Gyro` | yes; 975 polls | held |
| P4 | `4400c2`'s repro passes 1.6016 s and answers | yes | held |
| P5 | every generated verdict byte-identical outside `memory` | archives byte-identical; lines identical but the archive path and the checkout path | held |
| P6 | cost below a 100 ms boot's noise | not separated: both sides' boots are within run-to-run variation, as the shard weights already state | not measured apart |
| P7 | reflection works in Renode 1.17.0's IronPython | yes | held |

### Adversarial pass, targeted (§5): one mutant each, against the tests that should kill it (`~/.cache/aet-4400e3/mutants.sh`, on the final code)

| Mutant | Guarded case | Killed by |
|---|---|---|
| M1: the hook also clears bit 0 (`& ~0x801`) | another bit of `ICSR`, `VECTACTIVE` | tier 2, `…moves_no_other_bit` |
| M2: `Count <= 0` | 0 where the architecture says 1 (one active exception) | tier 2, `…moves_no_other_bit` (the lone handler must read 1) |
| M3: every offset rewritten | a register other than `ICSR` | unit, `…follows_the_platform_and_precedes_the_firmware` |
| M4: the hook's line removed from `commands()` | silently lost | `boot_tier1` (every stubbed boot exits 99) |
| M5: the hook after `LoadELF` | misplaced | unit, and the stubs |
| M6: the Dockerfile's Renode sha256 changed | a Renode upgrade the hook outlives | unit, `…tied_to_the_dockerfile_s_renode` |
| M7: the image's build differs (the tier-2 constant moved) | the same, in the image | tier 2, `the_pinned_image_s_renode_is_the_build…` |
| M8: the inner IRQ's priority below the outer's (no nesting) | a regression witness that witnesses nothing | tier 2, the bug test (`IABR1` shows IRQ 57 not active beneath) |

**Found and fixed by this pass:**
- **A race in my first tests.** Two tests ran the uncorrected fixture at once in one shared `HOME`, which each deleted (M2 was first killed by the wrong test). Each configuration now runs once per process.
- **A blind spot.** M8's first form showed the bug test passing with no nesting at all. The fixture now records `IABR1`, and the bug test requires both IRQs active.

### Reduction pass

- **Removed:** `BootPlan.rettobase`, a switch production always set to true. Every boot's commands now carry the hook, and the unit test checks it on the plain plan.
- **Kept:** the data file is the single source the CLI, both stubs and the tier-2 test read. No other duplication was added.

### Outside the stated fence, named

- `tests/boot_tier1.rs` and `tests/campaign_tier1.rs`: their stub patterns must name the hook, or every boot test fails (M4).
- `.github/workflows/ci.yml`: `TIER2_MIN` 183, required by the gates.
- `crates/aethelgard-cli/README.md`.
- `spike/4400e2/`: **deleted** after merging `main` `ff17dcf9` (#215 merged as the record, `672882d`). It is cited here and in the code by its work order, and its scripts and evidence stay in `main`'s history at `672882d`. `stubs-mon-fix.txt` was the spike's form of this hook, IABR plus SHCSR; the built hook reads the model's stack instead (Sources).

### Erratum, another order's note

My `4400e2` inbox note to `4220c` said ArduPilot's `INTF_CONFIG0 ← 0xC0` asks for big-endian count and data. `4220c` corrected it: bits 5 and 4 are 0, little-endian (DS-000347 p. 75). Its FEEDBACK holds the right reading.

### Evidence (not in git; names and hashes above)

- `~/.cache/aet-4400e3/`:
  - `accept.log`, `runs/acc-e`, `runs/acc-c2`;
  - `verdicts/` (both sides' outputs and archives), `mutants.txt`;
  - the captured argv `s02own/renode-argv.txt`.
- The ARM PDF: `~/.cache/aet-ds/arm-ddi0403/` (sha256 in `~/.cache/aet-ds/SHA256SUMS`).

### CI and READY

- **CI on `2a2c09b2`** (`main` `ff17dcf9` merged, `spike/4400e2/` deleted): all green.
  - `ci` run 44192149990: `gates`, `shards`, all 8 `tools-shard`s and `tools`. **`tier2_tests_passed=183 floor=183`**, so `TIER2_MIN` is exact.
  - `engine-image` 44192150712 and `ardupilot-image` 44192150110.
- **The overlap with `4220c` (#214):** both PRs move `TIER2_MIN`. Whichever merges second sums them (`main`'s 180, plus 3 from this PR, plus 4220c's), as the architect's note of 22:55 says. This PR does not touch `ci/test-weights.tsv`.

**READY**, 2026-11-17: built to the ruling on #215, to its acceptance. This is the last push unless the architect asks for another. Title and body: `~/.cache/aet-4400e3/pr-title-body.md`.
