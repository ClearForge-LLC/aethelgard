# AET-WO-1400 — FEEDBACK

**Branch:** `wo/aet-1400-board-crate` · **Base:** `1b4fe5f` (the merge of PR #37), as the kickoff set it. `origin/main` at `7e3ce87` (AET-WO-0300d and its follow-up, #39 and #41) is merged in with a merge commit, `59a7e16`, before the first push, as the architect directed. It touches no file here. For the build, `origin/main` was merged three more times, each with a merge commit: `a87de62` (`3f3924e`, AET-WO-1200), the merge of `425347e` (AET-WO-1202), and the final one, recorded under Build. · **Builder:** Claude Code (Opus 5.5) as `forge` on Furnace (x86_64, kernel 7.0.0-38). · **Cadence:** spike, stopped and ruled on PR #42 (R1 to R4, in `~/.cache/aet-1400-ruling.md`, and the ceiling ruling: option 2, AET-WO-1202), then build. §7 in both halves:
- no image change, no target added to the project, nothing under `container/` or `.github/`;
- no `unsafe` in a workspace crate;
- no harness taken from the crate under proof;
- no `assume` that excludes a seeded bug;
- no test skipped, ignored or weakened.

The committed proof was never vacuous: its cover was satisfied at every head. But the adversarial pass (E1) showed that `prove` could not have told a vacuous harness from a real one. That is the §7 "vacuous proof" class. The ruling in session was to fix `prove`'s judge in 1400 rather than park, and that is done (Build, E1).

Entry schema: `finding · where · severity · type · recommendation · decision-needed`.

## What the spike added (the build deletes all of it)

- `spike/s1-probe/`: the hand-written stand-in for generator output. It is `#![no_std]`, has its own `[workspace]` and **no dependencies**, which `prove` requires (S3). It contains:
  - `Registers`, the one door to the hardware;
  - `mmio`, the only `unsafe`: volatile access at RM0433's base addresses;
  - the typestates, written once in `board!` and instantiated four times (`good` and three seeded bugs, each differing in one argument at lib.rs:276–299);
  - `proofs`, the Kani harness, behind `#[cfg(kani)]`.
- `spike/s4-trybuild/`: its own `[workspace]`, path-dependent on the probe, holding `trybuild` and the snippets. A separate crate keeps the dependency out of the crate under proof.
- `crates/aethelgard-cli/tests/spike_1400_tier2.rs`: the tier-2 path. Under `with-tools`, it runs `aethelgard prove` on the four harnesses inside the image and writes one `ANSWER 1400-S1` line per harness to the job log. It is a new file, so it cannot conflict with 1200's command-tree work.

Scratch measurements live outside the repository in `~/.cache/aet-1400-spike/` (Furnace).

## Spike

The spike's files are cited as they stood at commit `6419150`. The build deleted `spike/` and `crates/aethelgard-cli/tests/spike_1400_tier2.rs`, as the ruling directs.

### S1 `OQ-04`: a proof over a harness-owned image catches all three seeded bugs

**Answer: yes.** Through `aethelgard prove`, the correct board proves and each seeded bug fails, each with a `proof_failed` finding naming the assertion it broke. Kani 0.68.0, CBMC 6.11.0.

**The probe** (`spike/s1-probe/src/lib.rs`):
- PB8 and PB9 go `Unconfigured → AlternateFunction<4>` (`I2C1_SCL`, `I2C1_SDA`; `GPIO_AF4_I2C1` in ST's vendored GPIO file).
- I2C1 goes `Gated → Clocked → Enabled<Speed>`, with port B `Gated → Clocked`.
- Every access is through `Registers::{read, write, modify}` on `(Block, byte offset)`, generic and monomorphized, not `dyn`.

The constants are RM0433's: `RCC_AHB4ENR` 0x0E0 bit 1, `RCC_APB1LENR` 0x0E8 bit 21, MODER 0x00, AFRL/AFRH 0x20/0x24, I2C CR1/TIMINGR/TXDR 0x00/0x10/0x28. Every one was cross-checked against `stm32-metapac` 21.0.0's `rcc_h7rm0433.rs`, `gpio_v2.rs` and `i2c_v2.rs`, which agree.

**The harness** (`proofs`, lib.rs:300–491) restates the specification with its own constants and imports only the boards' public API. A wrong constant in a board is therefore not mirrored in its oracle.

The register image `Image { rcc: [u32; 256], gpiob: [u32; 10], i2c1: [u32; 11] }` starts **fully nondeterministic**. It is the oracle as well as the storage:
- any access with an offset not word-aligned or past the block's last register fails (lib.rs:342);
- a GPIOB write before `GPIOBEN`, or an I2C1 write before `I2C1EN`, fails at the write (356, 361);
- `TIMINGR` written while `PE` is set fails (366).

The harness drives a chosen set of sequences, **not every interleaving the API admits**:
- two orders: port clock then pins then the I2C clock, or the I2C clock then port clock then pins;
- both admitted speeds;
- 0–2 bytes written, `writes <= 2` assumed (443).

Interleavings it does not drive include the I2C clock between PB8 and PB9, and PB9 before PB8. The oracle's checks are insensitive to order, except clock before write, which the types force. Its doc comment ("every call sequence") overstates this. The build's generated harness chooses the interleaving nondeterministically from the specification.

It then checks the final state:
- MODER and AFRH fields for PB8/PB9;
- no other pin's MODER or AFR field changed, AFRL and the other GPIOB registers unchanged;
- RCC equal to before plus exactly the two enable bits, word by word;
- `PE` set.

A `kani::cover!` marks the end (481). `#[kani::unwind(257)]` covers the 256-word RCC loop.

**Seeds, one per module.** Each one's fix is `good`:

| Seed | One-argument difference | Failed check (`prove` finding) |
|---|---|---|
| wrong AF number | `af_value = knobs::af_off_by_one` (writes 5 for 4) | `PB8/PB9 AFRH field is not AF4` (lib.rs:392) |
| missing clock enable | `enable_i2c1_clock = false` | `I2C1 written before RCC_APB1LENR.I2C1EN is set` (lib.rs:361) |
| write past the block | `afr_offset = knobs::afr_offset_by_pin` (pin 8 → 0x40, past AFRH 0x24) | `register access past the end of its block` (lib.rs:342) |

**Measured through `aethelgard prove`** on Furnace (`target/debug/aethelgard prove --crate spike/s1-probe --harness proofs::<board>::every_valid_sequence`, with `KANI_HOME`, `RUSTUP_HOME` and `PATH` from the local Kani 0.68.0 kit, and inherited descriptors above 2 closed as `ci.yml` does). Every verdict carries `"sandbox":{"landlock_abi":8,"tcp_confined":true,"scoped":true,"sockets_denied":true}`.

| Board | Exit | Verdict | Wall (s), one at a time | Wall (s), four in parallel (tier-2 binary, final source) |
|---|---|---|---|---|
| `good` | 0 | `pass`, `findings: []` | 46.36 | 54.09 |
| `bug_wrong_af` | 1 | `fail`, one `proof_failed` as above | 40.32 | 42.75 |
| `bug_no_clock` | 1 | `fail`, one `proof_failed` as above | 50.03 | 53.08 |
| `bug_past_block` | 1 | `fail`, one `proof_failed` as above | 0.53 | 0.57 |

The tier-2 test binary (`spike_1400_tier2`, libtest threads) runs the four in parallel in 54.09 s wall on Furnace, with peak RSS 2.3 GB for the largest process (`/usr/bin/time -f %M`). The one-at-a-time column is the earlier source, before `cargo fmt`; formatting changed no verdict.

**CI, the second witness:** [run 42427459474](https://github.com/ClearForge-LLC/aethelgard/actions/runs/42427459474), job `tools` (128568699413), on `df0ef5d`, inside the image pinned by `container/IMAGE`, with the four in parallel on a 4-CPU runner. Its `ANSWER 1400-S1:` lines give the same verdicts and findings as Furnace, and every one carries `"landlock_abi":8,"tcp_confined":true,"scoped":true,"sockets_denied":true`:

| Board | Exit | Finding | Wall (s) |
|---|---|---|---|
| `good` | 0 | `[]` | 104.81 |
| `bug_wrong_af` | 1 | `proof_failed`: `PB8/PB9 AFRH field is not AF4` | 85.18 |
| `bug_no_clock` | 1 | `proof_failed`: `I2C1 written before RCC_APB1LENR.I2C1EN is set` | 103.42 |
| `bug_past_block` | 1 | `proof_failed`: `register access past the end of its block` | 1.23 |

`spike_1400_tier2`: `test result: ok. 4 passed` (23:58:05 to 23:59:50). `gates` passed in 1m35s and `tools` in 3m55s.

**Bare `cargo kani`**, on a copy, for the raw lines (final run, `--harness proofs::<board>::every_valid_sequence --exact`):

| Harness | Exit | Kani's lines | Verification Time | Failed check location |
|---|---|---|---|---|
| `proofs::good::every_valid_sequence` | 0 | `** 1 of 1 cover properties satisfied`, `VERIFICATION:- SUCCESSFUL`, `Complete - 1 successfully verified harnesses, 0 failures, 1 total.` | 45.50 s | none |
| `proofs::bug_wrong_af::…` | 1 | `0 of 1 cover properties satisfied (1 unreachable)`, `Failed Checks: PB8/PB9 AFRH field is not AF4`, `VERIFICATION:- FAILED` | 36.31 s | `src/lib.rs:392:9 in function proofs::check_final` |
| `proofs::bug_no_clock::…` | 1 | `1 of 1 cover properties satisfied`, `Failed Checks: I2C1 written before RCC_APB1LENR.I2C1EN is set`, `VERIFICATION:- FAILED` | 45.45 s | `src/lib.rs:361:21 in function <proofs::Image as Registers>::write` |
| `proofs::bug_past_block::…` | 1 | `0 of 1 cover properties satisfied (1 unreachable)`, `Failed Checks: register access past the end of its block`, `VERIFICATION:- FAILED` | 0.08 s | `src/lib.rs:342:13 in function proofs::Image::slot` |

The `bug_no_clock` cover is satisfied because the image starts arbitrary: on paths where `I2C1EN` was already 1, the sequence completes. Kani still fails it on the paths where it was 0, which is the case a reset part is in.

**`bug_past_block` is fast for a reason.** `afr_offset_by_pin(8)` is a constant 0x40, so the bounds check is false on every path and CBMC refutes it almost at once (0.08 s verification). The other two need the solver: the wrong AF fails only in the final check, and the missing clock fails only on paths whose arbitrary initial `I2C1EN` is 0.

**Non-vacuity, three witnesses:**
1. The cover is `1 of 1 cover properties satisfied` on `good`.
2. Each seed fails on exactly the assertion meant for it, at the expected line.
3. A short unwind fails rather than hides. The first `good` run compared the RCC arrays with `==`, which lowers to a 1 KiB `memcmp` loop, and failed with `memcmp.unwind.0` "unwinding assertion loop 0". Unwinding assertions are on, so an under-unwound loop makes the proof fail. The comparison is now word by word.

The one `assume` (`writes <= 2`) bounds the byte count; no seed depends on it.

**Two things Kani 0.68.0 did that the build must know:**

| finding | where | severity | type | recommendation | decision-needed |
|---|---|---|---|---|---|
| F1. In an edition-2024 crate, `assert!(cond, "literal")` reaches Kani's report only as "This is a placeholder message; Kani doesn't support message formatted at runtime". `prove`'s finding then names nothing. `kani::assert(cond, "literal")` reports the text | probe, first run | medium | tool behaviour | The generated harness uses `kani::assert` for every specification check, so each `proof_failed` finding names the rule | no |
| F2. `==` on a `[u32; 256]` is a 1,024-iteration `memcmp` under CBMC, so it needs `unwind(1025)` or fails | probe, first run | low | tool behaviour | The generated harness compares register images word by word, with `unwind` sized to the largest block plus one | no |

**What the proof covers, and what it does not.** It proves the typestate API's register effects over a harness-owned image, from any starting state, for the sequences listed above. It does not prove the silicon. `mmio`, at the real addresses, is never exercised; `architecture.md` §2.2 already says so. It does not prove the harness right either: the harness is the specification restated, and a wrong constant in it is a wrong proof. That is why `INV-07` puts it on the specification side.

### S2 Where the crate compiles

- **Host, yes.** The probe builds and passes `clippy -D warnings` on `x86_64-unknown-linux-gnu` with the pinned 1.99.1. trybuild runs there (S4), and Kani runs there: Kani compiles with its own `nightly-2026-08-21` toolchain under `KANI_HOME`, so the crate needs no target beyond the host.
- **`thumbv7em-none-eabihf`, yes.** It was built in an isolated toolchain (`RUSTUP_HOME=~/.cache/aet-1400-spike/rustup`, installed as the Dockerfile installs it, then the target added; neither Furnace's `~/.rustup` nor the image was changed). `cargo build --offline --target thumbv7em-none-eabihf`: dev 0.23 s, release 0.06 s; `libaet_1400_s1_probe.rlib` is 111,914 bytes.
- **The image change, recorded and not made.** Measured on Furnace by the S2/S3 subagent; the lead re-read the sizes and the log lines.
  - `rustup target add --toolchain 1.99.1 thumbv7em-none-eabihf`: 1.01 s wall.
  - The rust-std directory is 72,172,351 B installed (`du -sb …/lib/rustlib/thumbv7em-none-eabihf`). `RUSTUP_HOME` goes from 646,008,767 to 718,181,705 B (+11 %). The compressed download size was not measured, because rustup deletes its downloads.
  - The change is one flag on `container/Dockerfile:49`'s `rustup-init`: `--target thumbv7em-none-eabihf`. The existing `chmod -R a+rX /opt/rustup` covers it.

| finding | where | severity | type | recommendation | decision-needed |
|---|---|---|---|---|---|
| F3. Clauses 3 and 5 need only the host target: trybuild compiles on host, and Kani uses its own toolchain. The thumb target is needed when firmware is linked for the part, which is P2 | S2 | info | measurement | **Proposal: no image change in 1400.** Add the target when P2's first firmware work order needs it, or when the architect wants CI to prove that the generated crate stays `no_std`-clean for the part. That is one flag, about 72 MB installed, about 1 s | **yes**: image change now, later, or never |

### S3 `embassy`

Both options were measured by the subagent in a fresh `CARGO_HOME` (`~/.cache/aet-1400-spike/{A,B}`); the lead re-read the lock counts, sizes, build logs and source citations. `embassy-stm32` 0.6.0 depends on `stm32-metapac` **21.0.0**.

| | A: `embassy-stm32 =0.6.0`, feature `stm32h753zi` | B: `stm32-metapac =21.0.0`, `stm32h753zi`, no default features |
|---|---|---|
| Crates (Cargo.lock, excluding the root) | 75 | 20 |
| `cargo vendor` size | 265,027,853 B | 244,422,441 B (`stm32-metapac` alone is 240,414,939 B, all chips) |
| Offline build, `thumbv7em`, empty `CARGO_HOME`, vendored source | succeeds, 7.45 s, 793 MB RSS | succeeds, 1.98 s, 468 MB RSS |
| `cargo check` on host | succeeds, 4.48 s | succeeds, 0.36 s |
| Licences | all MIT/Apache-compatible; `unicode-ident` adds Unicode-3.0 | the same |
| Build scripts | no network code; `embassy-stm32/build.rs:2620` runs `rustfmt` on its output | reads its features only |

The offline claim rests on `--offline`, source replacement, an empty `CARGO_HOME` and reading every build script. Furnace cannot cut a process's network: unprivileged `unshare -rn` is refused.

**Who owns the pins.** `embassy_stm32::init` takes the `Peripherals` singleton, configures RCC, and enables **every GPIO port's clock** (generated `init_gpio`, `_generated.rs:3815–3821`). `I2c::new` takes `Peri<'d, I2C1>` and pins bounded by `SclPin<I2C1>`/`SdaPin<I2C1>`. Those traits are generated from stm32-data's AF table, so embassy already refuses an unsolved AF at compile time. The constructor writes AFR, PUPDR, OTYPER, OSPEEDR and MODER itself (`gpio.rs:680–690`), and enables and resets the I2C clock through a reference count (`i2c/mod.rs:223–224`, `rcc/mod.rs:299–311`). Its drop guard disables the clock and resets the pins again.

**Above or beside.**
- **Above** (typestates wrapping embassy drivers): the generated layer cannot write GPIO or RCC for those peripherals without becoming a second writer that bypasses embassy's reference count, or needing `steal()`. Its typestates would restate checks embassy already makes. And the register writes Kani should judge happen inside embassy, where no harness-owned `Registers` can reach them.
- **Beside on the PAC** (B): `stm32-metapac` is a const raw pointer plus `read_volatile`/`write_volatile` (`common.rs:58,65`, `chips/stm32h753zi/pac.rs:644` I2C1 = 0x4000_5400). It has no ownership model and no trait to substitute.

**The measurement that decides it: `prove` cannot judge a crate with any dependency.**
- `spike/s1-probe` with a single `stm32-metapac = "=21.0.0"` dependency, through `aethelgard prove`: exit **5**, `ToolError`, "cargo kani exited 1 but its output matches none, or more than one, of: failed proof, compile error, missing harness".
- The cause, reproduced outside the sandbox with an empty `CARGO_HOME`, offline: `error: Failed to get cargo metadata.: … no matching package named stm32-metapac found`. The sandbox gives the tool `HOME` = its temporary directory, an empty cargo home and no sockets.
- Vendoring inside the crate is refused before the tool runs: `prove --crate ~/.cache/aet-1400-spike/B` is exit **3**, `InputTooLarge`, "holds more than 4096 files" (`RUL-13`). The vendored tree is 326 MB.

| finding | where | severity | type | recommendation | decision-needed |
|---|---|---|---|---|---|
| F4. The crate `prove` judges must have no dependencies. Neither embassy option can be the crate under proof, and vendoring breaks `RUL-13`'s caps | S3 | high | measured constraint | **Beside, and dependency-free:** the generated board crate depends on nothing. It takes register offsets and bit positions from the vendor data `aethelgard-svd-hal` already owns, or from ST's SVD added by a work order, with metapac as a test-time cross-check at most. Embassy may still drive *other* peripherals in P2 firmware, never one the board crate owns. This **amends `RUL-04`**, whose text says "built atop the `embassy` ecosystem" | **yes**: amend `RUL-04` |
| F5. A `ToolError` from `prove` carries no line of the tool's output. A crate whose dependency cannot resolve gives no hint why. The text above came from a re-run outside the sandbox | `crates/aethelgard-cli/src/kani.rs`, `judge` | low | diagnostics | When the outcome is unrecognised, put the first `error:` line in the `ToolError` message. That is a CLI change, outside 1400's scope | yes: a later CLI work order, or none |
| F6. `stm32-metapac` and `embassy-stm32` declare MIT OR Apache-2.0 in `Cargo.toml` but ship no licence text. metapac's data comes from `stm32-data` (its README), which is the provenance §2.8 chose not to depend on | S3, step 5 | info | licence | Moot if F4 is ruled as proposed. Otherwise, a licence check before any dependency | no |

### S4 `trybuild`

`spike/s4-trybuild/tests/ui.rs` uses trybuild 1.0.122. With the pinned `rustc 1.99.1 (58cf8f9c3 2026-10-13)`, `cargo test` passes all 7 cases, in 0.50 s warm and 4.8 s cold, `.stderr` snapshots committed:

| Case | Why | First diagnostic |
|---|---|---|
| `pass/valid_driver` | port and peripheral clocked, both pins AF4, `Fast400k`, one byte | compiles (`firmware` is compiled and not called: trybuild runs a passing case, and on the host these are not the STM32's registers; the first run crashed for exactly that reason) |
| `fail/unconfigured_pin` (gate) | an `Unconfigured` PB8 handed to `init` | `error[E0308]: mismatched types` |
| `fail/ungated_clock` (gate) | `init` on `I2c1<Gated>` | `error[E0599]: no method named init found for struct …I2c1<Gated>` |
| `fail/bus_too_fast` (gate) | `init::<FastPlus1M>` over a 400 kHz ceiling | `error[E0277]: the trait bound FastPlus1M: AtMostCeiling is not satisfied` |
| `fail/unsolved_af` (spike's choice) | PB8 into AF6, which ST's table gives it for `I2C4_SCL` and the solver did not | `error[E0277]: …Pin<8, Unconfigured>: SolvedAf<6> is not satisfied` |
| `fail/swapped_pins` (spike's choice) | SCL and SDA swapped, both correctly configured | `error[E0308]: arguments to this method are incorrect` |
| `fail/forged_typestate` (one more, for §5.1) | a configured pin built by a struct literal | `error[E0451]: field _mode … is private` |

The speed ceiling is a **sealed** trait implemented only for speeds at or under the ceiling, not `const { assert!(N <= CEIL) }`. The trait refuses at type-check time with E0277 and a stable message. A const assertion fails only after monomorphization, and its wording carries more compiler internals.

**Stability, measured both ways:**
- **Toolchain.** All 7 cases pass unchanged under a different compiler, `rustc 1.100.0-nightly (8925ea358 2026-08-20)` (Kani's own toolchain), with the 1.99.1 snapshots. Within one minor version, `rustc`'s text did not move them. The pin in `rust-toolchain.toml` (1.99.1, the same in the image and in `gates`) is what holds it across larger steps, since a crate under the repository root inherits it.
- **The crate's own text.** `cargo fmt` on the probe, with no semantic change, broke 4 of the 6 compile-fail snapshots. Each snapshot quotes the dependency's source lines, under `$AET_1400_S1_PROBE/src/lib.rs` with line numbers stripped, and a macro invocation that moved from one line to six changed every one that pointed into it.

| finding | where | severity | type | recommendation | decision-needed |
|---|---|---|---|---|---|
| F7. trybuild snapshots depend on the generated crate's exact text, so any generator change that moves a line regenerates them | S4 | medium | measured | Generate snapshots with `TRYBUILD=overwrite` and commit them with the regenerated crate. The byte-identical regeneration test (clause 4) makes that text a function of the topology, so snapshots move only when the generator does. Emit plain items rather than a macro, so a diagnostic points at the item and not at a whole invocation | no |
| F8. In the `tools` job, tests run under `--network none`, and only the networked `--no-run` step fetches crates, from the workspace lock. trybuild, its dependencies and its nested cargo build were **not** measured offline in the image | S4 | medium | not measured | The build makes trybuild a dev-dependency of a workspace member, so the networked step fetches it, and measures the nested build in `tools` on the first push. If it cannot run offline, clause 3 runs in `gates`, which has the network | yes, if it fails |

### S5 The unsafe boundary

- **The workspace has no `unsafe`.** `grep -rn '\bunsafe\b' crates --include=*.rs`, minus comments and `unsafe_code`, finds 0 lines. All seven members state `#![forbid(unsafe_code)]` at their root and inherit `[workspace.lints.rust] unsafe_code = "forbid"`. The new tier-2 test file is a target of `aethelgard-cli`, which inherits the lint. The 9 `unsafe` lines in this branch are all under `spike/`: `mmio`'s two volatile accesses and its constructor, plus the snippets' `unsafe { Mmio::new() }`. The build's generator, in `aethelgard-svd-hal`, writes `unsafe` as text and contains none.
- **Where generated crates go, measured** in a scratch workspace (`~/.cache/aet-1400-spike/ws-exp`):
  - **Measured:** a workspace member's path dev-dependency on a crate that declares its own `[workspace]`, under the repository root, is refused: `error: multiple workspace roots found in the same workspace`.
  - **Measured:** without `[workspace]`, the crate silently becomes a workspace member (`cargo metadata` lists it), which would put it under `clippy --workspace` without the forbid.
  - **Measured:** with `exclude = ["boards"]` in the root `Cargo.toml`, the path dev-dependency builds and tests pass; `cargo metadata` lists one workspace member, and its `unsafe` compiles because it does not inherit the lint.
  - **Proposal:** commit the reference board's generated crate at `boards/<project>/`, a top-level directory excluded from the workspace, with no `[workspace]` key. Then:
    - clause 4 regenerates it in memory in a workspace test and compares every byte with the committed tree;
    - clause 3's trybuild suite lives in a workspace member's `tests/`, with `trybuild` and `boards/<project>` as dev-dependencies;
    - clause 5 runs `prove --crate boards/<project> --harness <full path>`. `prove` copies the crate to a temporary directory, where it is its own root.
  - **Not here:** the generated harness lives in that committed tree, so it is generated from the specification side (`INV-07`). The regeneration test is what makes a hand-edited harness fail the gate.

| finding | where | severity | type | recommendation | decision-needed |
|---|---|---|---|---|---|
| F9. The proposal touches the root `Cargo.toml` (`exclude`). It also needs `workspace_shape.rs`'s `svd_hal_has_only_its_known_files_keys_and_dependencies`, which fixes svd-hal's dev-dependencies at `serde_json` and its files at `src/*.rs`, `tests/*.rs` and the data, to admit `trybuild` and the `boards/` dev-dependency. Roadmap §6: graduating is stated, not implied | S5 | medium | ruling input | Rule the location (`boards/<project>/` excluded) and the trybuild host (svd-hal, or the CLI's tests), and the build names the shape checks it changes | **yes** |
| F10. `prove` judges a harness inside the crate it is given. For `INV-07`, the harness is generated beside the crate and guarded by clause 4. The alternative is `prove` injecting a specification-side harness into its copy, which is a CLI change | S5 | medium | ruling input | The regeneration guard, which needs no CLI change | **yes** |

## Findings outside the five questions

| finding | where | severity | type | recommendation | decision-needed |
|---|---|---|---|---|---|
| F11. The probe models what the gate names and no more. Real I2C pins also need open-drain (`OTYPER`), and the GPIO port's clock is itself a gate. The probe makes the port clock a typestate (`GpioB<Gated>` has no pins) and asserts it in the oracle. `OTYPER` is only held unchanged by the frame check | S1 | low | scope | The generator emits each pin's full configuration from the solved role (mode, AF, type, speed, pull), and the harness checks each field the specification fixes | no |
| F12. `Board::take()` refuses a second call at run time (`Option`), not at compile time; `INV-02` cannot reach a singleton | S1 | info | design | Keep `take()`; document it as the one runtime check | no |
| F13. libtest (1.99.1) captures `eprintln!`, so a passing tier-2 test hides a measurement printed that way. **Corrected after the first push:** the first version also claimed that `io::stderr()` is captured. That was not measured: those runs had fd 2 on `/dev/null` because of the builder's own shell loop. Re-measured with the loop fixed (`~/.cache/aet-1400-spike/cap-exp`), `io::stderr().write_all` and a `/dev/stderr` handle both reach the log, and only `eprintln!` is captured. The test as shipped uses the `/dev/stderr` handle, and CI shows its lines. Its doc comment repeats the wrong claim, and the file is deleted in the build | `spike_1400_tier2.rs` | info | CI technique | Write to `io::stderr()` or a `/dev/stderr` handle, never `eprintln!`, for `ANSWER` lines in a passing test | no |

## The `OQ-04` answer, for `architecture.md` §8

A Kani proof over a harness-owned register image, substituted for the base address through a register trait, **catches** a seeded wrong AF number, a missing clock enable and a write past the block. Each is refuted with the specification rule it breaks named, and the corrected crate proves from an arbitrary starting image, all through `aethelgard prove`, on Furnace and in CI (S1). The proof is of the typestate API's register effects, not of the silicon. The useful question is answered yes, provided the crate under proof has no dependencies (F4) and the harness is generated from the specification and pinned by regeneration (F10).

## Gates (spike)

Furnace, on the merged tree (`59a7e16`): `cargo fmt --all -- --check` ok; `cargo clippy --workspace --all-targets --locked -- -D warnings` ok, with `--features aethelgard-cli/with-tools` too; `cargo test --workspace --locked` ok, 309 passed and 0 failed across 29 test targets in 17.3 s, with inherited descriptors above 2 closed as `ci.yml` does. Both spike crates are `cargo fmt`-clean and pass `clippy --all-targets -D warnings` (`--manifest-path`). Tier 2 is accepted from CI: run 42427459474 on `df0ef5d`, `gates` and `tools` green. The adversarial pass (§5) and the reduction pass (§5a) close the build half and are not run on a spike.

## Asked at the spike (ruled on PR #42 as R1 to R4; the build below follows the rulings)

1. **F4:** amend `RUL-04` to "beside embassy, dependency-free", and say where register offsets come from (the vendored ST data, or a new SVD vendoring).
2. **F3:** no image change in 1400? Or add the thumb target now?
3. **F9/F10:** generated crates at `boards/<project>/`, excluded; the trybuild host; the shape-check changes; and the harness guarded by clause 4 rather than injected by `prove`.
4. **F8:** if trybuild cannot run offline in `tools`, may clause 3 be accepted from `gates`?

## Build (after the ruling on PR #42)

### What was built

- **R1, the register facts: ST's CMSIS device header.**
  - Source: `STMicroelectronics/cmsis-device-h7` at `8f922cdc7cc6de2344e75ddd657889f4ff761790` (tag `v1.10.7`, "Release v1.10.7", 2026-02-16).
  - Licence: Apache-2.0, verified three ways:
    - GitHub's licence field;
    - `LICENSE.md` is the Apache License 2.0 text (sha256 `49d37f1f…20df`);
    - the header itself reads "© 2019 STMicroelectronics … licensed under terms that can be found in the LICENSE file in the root directory of this software component".
    The repository has no NOTICE file.
  - Vendored byte for byte at `crates/aethelgard-svd-hal/data/st-cmsis-device-h7/`: `Include/stm32h753xx.h` (2,118,971 B, sha256 `4f6def439a9fa94705303fe227d34308048c44bd36263d311aa54cc94f8398f5`), `LICENSE.md` and `SOURCE.md`.
  - `cmsis::register_facts()` checks the hash before reading a byte. It is not a preprocessor, and refuses anything outside three shapes (README): a `#define` that is a literal, a name or a sum; a `uint32_t`-only `typedef struct`, whose offsets come from field order, cross-checked against every `Address offset:` comment; and the one `RCC_<reg>_<periph>EN_Pos` that exists.
  - Field widths are derived from consecutive positions (`MODE1_Pos − MODE0_Pos`).
  - The ruling's offsets are pinned by `the_probe_offsets_come_out_of_the_header`: RCC 0x0E0 and 0x0E8, GPIO 0x00, 0x20 and 0x24, I2C 0x00, 0x10 and 0x28. The bases, the clock gates and an alias chain are pinned beside them.
  - **Machine cross-check:** the spike measured every one of those constants against `stm32-metapac` 21.0.0 (`rcc_h7rm0433.rs`, `gpio_v2.rs`, `i2c_v2.rs`), which agree. The header's own offset comments are a second, in-file check.
  - The fallback route, a transcribed RM0433 table, was not needed.
- **The generator, `aethelgard_svd_hal::board::generate(crate_name, &SolvedTopology, &PinTable)`.**
  - It reads no `Specification` (`RUL-16`). It runs `SolvedTopology::validate` first, refuses the solver's `RESERVED_PINS`, and cross-checks every solved AF against the pin table.
  - It refuses whatever it cannot emit: a peripheral other than `I2Cn` or `SPIn`, an unknown signal, a peripheral missing a role, a bad crate name, ceilings that do not number the I2C peripherals. The README lists every refusal.
  - Its output is a `BTreeMap<path, text>`: no clock, no environment, no host name. The provenance in each file is the input hashes.
  - Two emitters share the plan and nothing else. `emit_lib` writes the typestates. `emit_proofs` writes the harness and computes its expectations itself, labelled `belt-and-suspenders` at the site.
  - It contains no `unsafe`, being in a workspace crate under `forbid(unsafe_code)`, and emits it only inside `mmio`, which a unit test checks.
- **The reference board, `boards/sentinel-01/`** (R3), generated from `examples/sentinel-01-minimal.json`. That solves to I2C1 on PB6/PB7 at AF4, SPI1 on PA5/PA6/PA7 at AF5, `FLASH_CS` on PA0, and a ceiling of 400 kHz. It holds five files: `Cargo.toml` (no dependencies, no `[workspace]`), `Cargo.lock`, the `src/lib.rs` shell, `src/board.rs` and `src/proofs.rs`.
  - The root `Cargo.toml` gains `exclude = ["boards"]`, and `trybuild` and the board in `[workspace.dependencies]`.
  - The board is never a workspace member (`cargo metadata` lists the seven).
  - It builds for `thumbv7em-none-eabihf` in the isolated toolchain (S2), with dev build 0.07 s.
- **The register door is sealed (adversarial E2, E3).** `Registers: sealed::Sealed`, and every method takes a `sealed::Token` that only the board crate can make. Firmware therefore can neither implement it nor call it around the typestates. Only the generated API and the in-crate harness can.
- **`prove` detects vacuity (adversarial E1; ruled in session to fix in 1400).** `crates/aethelgard-cli/src/kani.rs` reads Kani's cover summary. On exit 0, any declared cover never reached is a fail with finding `cover_unsatisfied`, and two summaries are a `ToolError`. README contract rows for exit 1 and `prove` amended. Three tier-1 tests and one tier-2 seed.
- **The typestates (F11, F12).**
  - Each solved pin has exactly one method, into its solved function: `into_i2c1_scl`, `into_output`. Each writes the role's full configuration, OTYPER, OSPEEDR, PUPDR and AFR or BSRR, then MODER last.
  - Port pins exist only from `GpioX<Clocked>::split`. A peripheral's `init` takes only its own configured pins, from `Clocked`.
  - `i2c1_speed::AtMostCeiling` is sealed and implemented only for modes at or under the solved ceiling. `TIMINGR` and the SPI frame settings are placeholders until a clock tree is solved.
  - `Board::take` is the one runtime check, and is documented as such.
  - Pin-role electrical settings are generator policy (`Role::policy`: I2C open-drain, SPI push-pull at high speed, outputs starting high), not register facts.
- **The generated harness (F1, F2).**
  - A nondeterministic scheduler runs 13 steps. At each step it takes any action whose inputs exist: two port clocks, two peripheral clocks, seven pin configurations, two inits.
  - That is every interleaving the typestate API admits. The `kani::assume` on the chosen action admits only those orders; the types forbid the rest at compile time. It never excludes an order the types allow.
  - Each `init` drives each admitted speed, and zero or one byte. The output pin is driven zero or one time.
  - The image is fully nondeterministic at the start, sized from the header's structs: RCC 76 words (0x130, not the probe's 1 KiB), GPIO 10, I2C 11, SPI 21. `unwind(77)` is the largest block plus one, and unwinding assertions are on.
  - The oracle checks:
    - bounds on every access;
    - the clock before any read or write of a gated block;
    - `TIMINGR` and SPI `CFG2` written only while disabled;
    - every field the specification fixes for every solved pin (mode, type, speed, pull, AF) and the frame condition on every other pin's field and every other register;
    - RCC equal to before plus exactly the solved gates, word by word;
    - each peripheral enabled, `TIMINGR` one of the admitted speeds' values, SPI master.
  - After the adversarial pass, also:
    - the chosen speed's exact `TIMINGR`;
    - `CR1` and `CFG2` beyond their own bits;
    - every BSRR write and output level, and that `write_byte` leaves outputs alone;
    - every MODER transition: final mode only, once the pin's other fields are final (E4 to E7).
  - Every check is a `kani::assert` naming its rule. Nine `kani::cover!`s mark each branch and the end of a complete schedule, and the scheduler asserts that it started every peripheral.
- **The `rustfmt` problem, found and fixed.** `cargo fmt --all` reaches the board through svd-hal's path dev-dependency. That made the regeneration test and the gates' `fmt --check` contradict each other, and the regeneration test caught it on the first reformat.
  - An inner `#![rustfmt::skip]` is refused on 1.99.1 (`E0658: custom inner attributes are unstable`, measured).
  - So `src/lib.rs` is a shell already in rustfmt's form, holding `#[rustfmt::skip] mod board;` and `#[cfg(kani)] #[rustfmt::skip] mod proofs;`, and the generator alone owns those modules' bytes.
- **The ceiling (the ruling on option 2).** `SolvedTopology.i2c_ceilings` (AET-WO-1202, `425347e`) is keyed by bus id, and the topology does not say which peripheral serves which bus. The generator therefore requires one ceiling per I2C peripheral (`CeilingMismatch`) and gives every I2C peripheral the lowest. That is exact for sentinel-01's single bus, and never above any bus's ceiling for more. A unit test proves the filter at 100 kHz, 400 kHz and 1 MHz, and the minimum with two buses.
- **Deleted:** `spike/` and `crates/aethelgard-cli/tests/spike_1400_tier2.rs`.

### Shape-check changes (roadmap §6: graduation is stated)

In `crates/aethelgard-cli/tests/workspace_shape.rs`:
1. `svd_hal_has_only_its_known_files_keys_and_dependencies`:
   - `DATA` gains `data/st-cmsis-device-h7/{LICENSE.md, SOURCE.md, stm32h753xx.h}`;
   - the file allowlist admits `tests/ui/**/*.stderr` (`.rs` under `tests/` was already allowed);
   - the dev-dependencies are exactly `sentinel-01-board`, `serde_json` and `trybuild`, each still `workspace = true`.
2. New, `boards_are_excluded_and_dependency_free`: the root `exclude` is exactly `["boards"]`, every `boards/*/Cargo.toml` sets only `[package]` and `[lints]` (no `[workspace]`, no dependency table of any kind), and at least one board exists.
3. `members_are_exactly_the_seven_parts` and every other check are unchanged.

### Gate clauses

All three were measured on Furnace and in CI. The first CI run was [run 42429539794](https://github.com/ClearForge-LLC/aethelgard/actions/runs/42429539794) on `b62026b`: `gates` 1m56s, `tools` 9m42s. **The final run, on the head with every adversarial fix, is [run 42433045643](https://github.com/ClearForge-LLC/aethelgard/actions/runs/42433045643) on `748190a`: `gates` 1m28s, `tools` 17m44s, all green.** In its `tools` log:
- clause 3: `valid_driver [should pass] ok` and 11 `[should fail to compile] ok`, under `--network none`;
- clause 4: `the_committed_board_crate_is_exactly_the_generated_one`, `two_generations_are_byte_identical` and `a_hand_edited_harness_fails_the_comparison` all ok;
- clause 5: the five `ANSWER 1400-C5` lines below.

The commit after `748190a` changes this FEEDBACK only.

| Clause | Test | Furnace | `tools` (b62026b) |
|---|---|---|---|
| 3: misuse fails to compile, a valid driver compiles | `aethelgard-svd-hal/tests/board_ui.rs`: `pass/valid_driver` plus 11 `fail/` cases (below) | ok, 0.5 s warm | ok on `b62026b` (the 8 then present) and on `748190a` (all 11), under `--network none`. **R4 measured: trybuild's nested build runs offline in `tools`**, so clause 3 is accepted from `tools`, and `gates` runs it too |
| 4: regenerating twice is byte-identical | `aethelgard-svd-hal/tests/board.rs`: the whole committed tree equals the generator's output (missing, differing and extra files all reported); two generations equal; the negative control | ok | ok |
| 5: Kani proves the generated block | `aethelgard-cli/tests/board_tier2.rs` (with-tools): `prove --crate boards/sentinel-01 --harness proofs::every_valid_sequence`, plus three seeded bugs and, since `a7f0b56`, a vacuous seed that must be `cover_unsatisfied`, run serially (3.5 GB each against the runner's 15 GiB) | board pass (≈70 s); seeds fail in 41–71 s; 250 s in all, peak RSS 3.53 GB | `ANSWER 1400-C5` on `748190a`: board `exit=0`, `findings: []`, 209.39 s; `seed-wrong-af` exit 1 `GpioB word 8: a solved pin's field is not what the specification fixes`, 153.03 s; `seed-no-clock` exit 1 `Spi1 accessed before RCC_APB2ENR bit 12 is set`, 219.99 s; `seed-past-block` exit 1 `register access past the end of its block`, 222.34 s; `seed-vacuous` exit 1 `cover_unsatisfied` `** 6 of 9 cover properties satisfied (3 unreachable)`, 96.94 s. Every verdict has `landlock_abi 8`, `sockets_denied true`. On `b62026b`, before the stronger oracle, the board took 117 s |

**Clause 3 cases** (first diagnostic line; snapshots committed, regenerated with `TRYBUILD=overwrite` with the board, F7):

| Case | Diagnostic |
|---|---|
| `unconfigured_pin`, an unconfigured PB6 into `init` | `E0308 mismatched types` |
| `ungated_peripheral_clock`, `init` on `I2c1<Gated>` | `E0599 no method named init … I2c1<Gated>` |
| `ungated_port_clock`, `split` before the port clock | `E0599 no method named split … GpioB<Gated>` |
| `bus_too_fast`, `init::<FastPlus1M>` over 400 kHz | `E0277 FastPlus1M: AtMostCeiling is not satisfied` |
| `unsolved_function`, PB6 as SPI SCK | `E0599 no method named into_spi1_sck` |
| `swapped_pins`, SCL and SDA swapped | `E0308 arguments to this method are incorrect` |
| `output_unconfigured`, chip select driven before it is an output | `E0599 no method named set_low … PA0<Unconfigured>` |
| `forged_typestate`, a struct literal | `E0451 field _mode … is private` |
| `foreign_speed`, firmware implementing the speed trait | `E0277 Turbo: i2c1_speed::sealed::Sealed` |
| `foreign_registers`, firmware implementing `Registers` (E2) | `E0277 Fake: …sealed::Sealed is not satisfied` |
| `raw_register_access`, firmware calling `Mmio::write` (E3) | `E0603 module sealed is private` |

`bus_too_fast` was written while the ceiling was pending and joined `fail/` when 1202 merged. `foreign_registers` and `raw_register_access` came from the adversarial pass. On `b62026b` CI ran the first eight. The final head's run, cited above, runs all eleven.

### Adversarial pass (§5), by a fresh subagent on the frozen commit `b62026b`

A fresh Opus subagent ran the pass in its own worktree, detached at `b62026b`, and changed nothing in it. Its scratch, every seed's log and every firmware snippet, is in `~/.cache/aet-1400-adv/` on Furnace. The lead spot-checked the s14 log and `prove` JSON (`UNREACHABLE`, `"verdict":"pass"`) and `fw/examples/a01_fake_registers.rs` before acting on the report.

**Escaped at `b62026b`, all fixed in `a7f0b56`.** After each fix the lead re-ran the same seed through `aethelgard prove` (`scratchpad/seeds.py`, four in parallel in a 32 GB cgroup):

| # | Attack (§5) | Severity | Fix | Re-run after the fix |
|---|---|---|---|---|
| E1 | `kani::assume(false)` in generated code: Kani exits 0 with `** 0 of 1 cover properties satisfied (1 unreachable)`, and `prove` said **pass**. The checker could not see vacuity (§7's class; ruled in session: fix the judge in 1400) | high | `prove`'s judge reads Kani's cover summary. On exit 0 with S < N, the verdict is a fail with finding `cover_unsatisfied` (README contract rows for exit 1 and `prove` amended). Two cover summaries are a `ToolError`. The generated harness covers every branch, nine in all: each speed, each start, each write, each output level, and the end. Tier 1: three new tests. Tier 2: a vacuous seed in `board_tier2.rs` | `e1-vacuous`: fail, `cover_unsatisfied` "** 6 of 9 … (3 unreachable)". `e1-partly-vacuous` (only Standard100k cut): fail, "** 7 of 9 … (2 unreachable)" |
| E2 | Firmware implements `Registers` on a stand-in, so the typestates advance while the real part stays gated | high (`INV-02`) | `Registers: sealed::Sealed`, with `sealed` `pub(crate)` | trybuild `foreign_registers`: `E0277 Fake: …sealed::Sealed is not satisfied` |
| E3 | Firmware calls `Mmio::write` directly: any register, any offset, around every typestate rule, with no further `unsafe` | high | Every `Registers` method takes a `sealed::Token` that only the board crate can make | trybuild `raw_register_access`: `E0603 module sealed is private` |
| E4 | `Standard100k`'s `TIMINGR` replaced by another admitted mode's: proved | medium | The schedule records the chosen mode's `TIMINGR`, a generator literal and not the crate's constant, and `check_final` asserts equality | `e4-timingr-swap`: `I2c1 TIMINGR is not the chosen speed's` |
| E5 | Extra bits set in I2C `CR1`, or SPI `CFG2` overwritten: proved | medium | Frame checks: `CR1` beyond its enable bit, `CFG2` beyond `MASTER` | `e5-i2c-cr1-extra`: `I2c1 CR1 changed beyond its enable bit`. `e5-spi-cfg2-overwrite`: `Spi1 CFG2 changed beyond MASTER` |
| E6 | BSRR unconstrained: `set_high` drove low, the chip select started low, SPI `write_byte` toggled CS: all proved | medium | Every BSRR write may touch only a solved output's set or reset bit. `set_low` and `set_high` are checked against BSRR. An output may enter output mode only once its level is high. `write_byte` must leave every BSRR word unchanged | `e6-set-high-drives-low`: `PA0 set_high did not set it`. `e6-starts-low`: `PA0 entered its mode before its other fields were final`. `e6-spi-write-toggles-cs`: `SPI1 write_byte touched a GPIO BSRR` |
| E7 | Only the final image was checked: a transient output glitch, and MODER set before AFR, both proved | low–medium | Every MODER write: a solved pin may move only to its final mode, only once OTYPER, OSPEEDR, PUPDR and AFR (or BSRR) are final. Unsolved pins' MODER fields never move | `e7-output-glitch`: `PA5's mode moved to one its role never takes`. `e7-moder-before-afr`: `PA5 entered its mode before its other fields were final` |
| E8 | A second board (`boards/evil`, with a `build.rs` and a safe `Mmio::forged`), and the root's board path redirected to it: the shape checks and clause 4 stayed green, while clause 3 judged the wrong crate | medium | `boards_are_excluded_and_dependency_free` now pins `workspace.dependencies.sentinel-01-board.path` to `boards/sentinel-01`, refuses `build.rs`, `package.build` and `package.links` in any board, and requires the boards present to be exactly `["sentinel-01"]`, the ones clause 4 regenerates. `.gitignore` gains `/boards/*/target/` | Faults injected one at a time: `build.rs`, `package.build`, a redirected path and a second board are each caught |

With every fix in, the unmodified board passes `prove` in 123.7 s, four in parallel.

**Caught at `b62026b`, no change needed:**
- `Default`, `Clone`, struct literals and a second `Rcc`: E0277, E0599, E0451.
- Clocking a port twice, use after move, reconfiguring a pin: E0382, E0599.
- Pins from the wrong peripheral: E0308.
- `Mmio::new` without `unsafe`: E0133.
- In the oracle: a clobbered field of another pin, a wrong OTYPER, OSPEEDR or PUPDR, an extra RCC bit, the wrong peripheral's register, SPI not master, `TIMINGR` written while enabled.
- A loop longer than the unwind: `unwinding assertion loop 0`. Unwinding assertions are on, and `prove` passes no flag that turns them off.
- The schedule's `assume`s mirror the type preconditions exactly. Every surviving path is a full permutation.

**Determinism:** the agent found none of `HashMap`, an environment read, a clock, a path or a float in `generate`. `cargo fmt --all`, `clippy` and `test` write nothing into `boards/` (measured by snapshot).

**What the proof cannot catch, by design** (the agent's s17 and s18): a register fact wrong in *both* emitters, such as `TIMINGR` at 0x14 or `I2C1EN` at bit 22. Both emitters read the same `Facts`, so the proof checks the crate against the generator's facts and policy, not against silicon.

Those facts are guarded by the header hash, by the in-file `Address offset:` cross-check, and by the pinned-offset tests. The spike checked those against RM0433 and `stm32-metapac`. The SPI offsets and bits the agent found unpinned are now pinned too: `CR1` 0x00, `CFG2` 0x0C, `TXDR` 0x20, size 0x54, `APB2ENR` 0xF0, `BSRR` 0x18, `PE`/`SPE` bit 0, `MASTER` bit 22.

**Latent, left unfixed (low: the header is hash-pinned and contains none of these shapes, which the agent measured):** the parser would accept, with a value C does not give:
- a `#define` inside `#if 0` or inside a multi-line comment;
- an octal-looking literal;
- text after a comment on a `#define` line;
- two fields on one line;
- a field inside a comment.

The agent's fix list goes with any future re-vendoring: strip block comments across lines, and refuse any `#if` other than the include guard and `__cplusplus`, any octal, and two fields on one line.

### Reduction pass (§5a) and mutants

Reduction pass (diff), over the build diff and again over the adversarial fixes: −7 net in the first pass and +5 net in the second (a helper replacing five copies) · 2 duplicates removed:
- the `I2C`/`SPI` prefixes, now derived from `Kind::header`;
- five inline copies of the oracle's field-mask arithmetic, now `oracle_mask`. The emitted bytes are unchanged, which the regeneration test confirms.

1 dead path removed: `PinUsedTwice`, unreachable behind `SolvedTopology::validate`'s `DuplicatePin`. 2 protected kept, both labelled `belt-and-suspenders`: `CeilingTooLow` behind `validate`, and the oracle restating `emit_lib`'s mask arithmetic, which `INV-07` requires. 0 findings below.

Mutants, one per guard this work order adds (`scratchpad/mutants.py`, with each file restored), re-run on the final code. 14 of 14 are killed by their own suite:
- the header hash check, the offset-comment cross-check, the `uint32_t`-only field check and the clock-gate uniqueness check (`--lib cmsis`);
- the AF cross-check, the reserved-pin refusal, `validate`, the ceiling count and the ceiling filter (`--lib board`);
- the regeneration compare, by bytes and by extra files (`--test board`);
- `prove`'s cover guard (`--test tools_tier1 prove`);
- the two new shape checks (`--test workspace_shape`). Each catches its fault unmutated, and lets it through when mutated. The faults: an extra `exclude` entry; a `[dependencies]`, `[target.*.dependencies]` or `[build-dependencies]` table in the board; and, after E8, a `build.rs`, `package.build`, a redirected path and a second board.

The generated harness's own checks are mutated by the 11 seeds above. The original three are in tier 2, plus the vacuous seed.

### Gates (build)

Furnace, on the final merged tree (`cd88b67`, `origin/main` `6525506` merged):
- `cargo fmt --all -- --check` ok;
- `cargo clippy --workspace --all-targets --locked -- -D warnings` ok, and with `--features aethelgard-cli/with-tools`;
- `cargo test --workspace --locked` ok: 396 passed, 0 failed, across 32 test targets, in 21.8 s, with inherited descriptors above 2 closed as `ci.yml` does.

The generated board passes `clippy --all-targets -D warnings` by its own manifest, and builds for `thumbv7em-none-eabihf` in the isolated toolchain. Tier 2 is accepted from CI (below).

### Promote-or-needs-none pass over `AET-WO-1200`'s FEEDBACK (now on the base)

- **S4.2, `gpio` records no direction:** needs none here. The generator treats every `gpio` entry as an output, which is all the solver emits. A direction field would reach the generator as an unknown key, refused by `SolvedTopology`'s `deny_unknown_fields`, never guessed.
- **Reserved and special pins** (PA15 JTAG, PA11/PA12 USB, PC14/PC15/PH0/PH1): **promoted into the generator**, which now refuses `RESERVED_PINS` itself (`ReservedPin`), so a hand-built topology cannot get a board that drives SWD or the crystal. PA15, PA11 and PA12 stay assignable, as ruled for 1200.
- **B1 to B8, A1 to A6:** needs none for 1400. Determinism (A5) is matched here by the regeneration test.
- **1101 finding 1, the five AF entries not yet checked against a datasheet:** still open, and still to promote. 1400 emits AFs from that table, so a wrong entry would be a wrong board. The pin-table cross-check catches a solver/table mismatch, not a table/silicon one.

### Findings (build)

| finding | where | severity | type | recommendation | decision-needed |
|---|---|---|---|---|---|
| B1. `i2c_ceilings` is keyed by bus id, and nothing in `SolvedTopology` names which peripheral serves which bus. The generator applies the lowest ceiling to every I2C peripheral: sound, and exact for one bus, but it under-admits on a board with two buses of different ceilings | `board::generate`, AET-WO-1202 | medium | interface gap | A 1202 follow-up keys `i2c_ceilings` by peripheral, or carries the bus id on each I2C `PinAssignment`; then the generator maps exactly | **yes** |
| B2. `cargo fmt --all` formats path dependencies, which made the gates' fmt check and clause 4 contradict each other. Fixed with the shell-plus-`#[rustfmt::skip]` layout; an inner `#![rustfmt::skip]` is unstable on 1.99.1 (measured) | the generated crate | medium | measured | Keep the layout for every generated crate; the regeneration test enforces it | no |
| B3. The bless route needs `boards/<project>/` to exist: cargo cannot load the workspace without svd-hal's path dev-dependency. A first board is bootstrapped by creating its `Cargo.toml` by hand, then blessing | `tests/board.rs` | low | ergonomics | Documented in the README. A second board would want an `aethelgard` subcommand, which is a ruling (WO "is NOT") | no |
| B4. The `tools-image` workflow ran an `image` job on this PR's pushes although the branch touches nothing under `container/` or `.github/` (`git diff --stat origin/main...HEAD -- container .github` is empty). It passed (5m2s). This is a path-filter question for that workflow, not 1400's to change | `.github/workflows/tools-image.yml` | low | observation | Architect's call | no |
| B5. `I2C_TIMINGR` and SPI `CFG1` are placeholders, and the oracle accepts exactly the placeholder `TIMINGR` values. When a clock tree is solved, both emitters must take the solved values | `board.rs` | info | scope | A P2 clock-tree work order | no |
