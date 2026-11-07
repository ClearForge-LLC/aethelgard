# AET-WO-1400 — FEEDBACK

**Branch:** `wo/aet-1400-board-crate` · **Base:** `1b4fe5f` (the merge of PR #37), as the kickoff set it. Live `main` is now `ad27d9e` (AET-WO-0300d, `run.rs`'s guardian retry); it touches no file here and is merged before the build half, with 1200. · **Builder:** Claude Code (Opus 5.5) as `forge` on Furnace (x86_64, kernel 7.0.0-38). · **Cadence:** spike half only. §0 is answered below and the branch is **stopped for the architect's ruling**. Nothing from §1 is built. No §7 condition arose: no image change, no target added to the project, nothing under `container/` or `.github/`, no `unsafe` in a workspace crate, no vacuous proof, no `assume` that excludes a seeded bug.

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

The harness drives every sequence the typestate API admits for this topology:
- both clock orders;
- both admitted speeds;
- 0–2 bytes written, `writes <= 2` assumed (443).

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

**CI, the second witness:** the `tools` job of this PR, `ANSWER 1400-S1:` lines. Run: CI_RUN.

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

**What the proof covers, and what it does not.** It proves the typestate API's register effects over a harness-owned image, from any starting state, for every admitted sequence. It does not prove the silicon. `mmio`, at the real addresses, is never exercised; `architecture.md` §2.2 already says so. It does not prove the harness right either: the harness is the specification restated, and a wrong constant in it is a wrong proof. That is why `INV-07` puts it on the specification side.

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
| F13. In this toolchain libtest captures `io::stderr()` as well as `eprintln!`, so a passing tier-2 test hides any measurement it prints. A `/dev/stderr` handle opened as a file is not captured, which is how `ANSWER 1400-S1` reaches the log without a deliberate failure (0300b's route) | `spike_1400_tier2.rs` | info | CI technique | Reuse it in later spikes | no |

## The `OQ-04` answer, for `architecture.md` §8

A Kani proof over a harness-owned register image, substituted for the base address through a register trait, **catches** a seeded wrong AF number, a missing clock enable and a write past the block. Each is refuted with the specification rule it breaks named, and the corrected crate proves from an arbitrary starting image, all through `aethelgard prove`, on Furnace and in CI (S1). The proof is of the typestate API's register effects, not of the silicon. The useful question is answered yes, provided the crate under proof has no dependencies (F4) and the harness is generated from the specification and pinned by regeneration (F10).

## Gates

Furnace, on the spike diff: `cargo fmt --all -- --check` ok; `cargo clippy --workspace --all-targets --locked -- -D warnings` ok, with `--features aethelgard-cli/with-tools` too; `cargo test --workspace --locked` GATES_TEST. Both spike crates are `cargo fmt`-clean and pass `clippy --all-targets -D warnings` (`--manifest-path`). Tier 2 is accepted from CI: CI_RUN. The adversarial pass (§5) and the reduction pass (§5a) close the build half and are not run on a spike.

## Promote-or-needs-none pass over `AET-WO-1200`'s FEEDBACK

**Deferred to the build half.** `AET-WO-1200`'s FEEDBACK is on PR #32, unmerged, and not on this base. It is done after the ruling, when `origin/main` with 1200 is merged in.

## Asked of the architect (one message, with the build goal)

1. **F4:** amend `RUL-04` to "beside embassy, dependency-free", and say where register offsets come from (the vendored ST data, or a new SVD vendoring).
2. **F3:** no image change in 1400? Or add the thumb target now?
3. **F9/F10:** generated crates at `boards/<project>/`, excluded; the trybuild host; the shape-check changes; and the harness guarded by clause 4 rather than injected by `prove`.
4. **F8:** if trybuild cannot run offline in `tools`, may clause 3 be accepted from `gates`?
