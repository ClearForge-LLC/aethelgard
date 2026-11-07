# AET-WO-1400 — The board crate: typestates generated from the solved topology, and what Kani can prove about them

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main` after `AET-WO-1200` has merged (named in the kickoff; verify live HEAD before branching).
**Branch:** `wo/aet-1400-board-crate`.
**Author:** Claude (architect) · **Builder:** a **strong** builder, with a full adversarial pass (`docs/roadmap.md` §6). A fresh session on Furnace.
**Phase:** P1 · **Gate clauses owned:**
- **Clause 3:** a `trybuild` suite of at least five firmware snippets that touch an unconfigured pin, an ungated peripheral clock, or a bus speed above the solved rise-time ceiling fails to compile, and one valid driver compiles (`INV-02`).
- **Clause 4, board-crate half:** regenerating twice gives byte-identical output.
- **Clause 5:** `cargo kani` proves the harness on the generated register block, or `architecture.md` §8 records under `OQ-04` why it cannot and what replaces it.

**Grounds:**
- `docs/architecture.md` §4:
  - `RUL-04`: a generated `#![no_std]` board crate whose pin, clock and peripheral states are typestates, built on the `embassy` ecosystem. Any `unsafe` register block is bounded-model-checked;
  - `RUL-16`: the generator reads a `SolvedTopology`, never a `Specification`;
  - `RUL-12`.
- `docs/architecture.md` §2.2 and §8 `OQ-04`, narrowed: a proof is possible only over a harness-owned register image, never over the real base address. The question is whether such a proof catches a seeded typestate bug.
- `docs/architecture.md` §3: `aethelgard-svd-hal` owns "generated board crate".
- `docs/northstar.md` §4: `INV-01`, `INV-02` and `INV-07`. The proof harness comes from the specification side, never from the crate under proof (roadmap §5's 1400 line, from Argon's RT-01).
- The seams:
  - `crates/aethelgard-svd-hal`, with the pin table;
  - `crates/aethelgard-ir` (`SolvedTopology`, with the `gpio` map from `AET-WO-1200`);
  - `crates/aethelgard-cli` (`prove`, `--exact`);
  - `container/Dockerfile`, which installs Rust with the minimal profile and **no** `thumbv7em-none-eabihf` target.

> **What this is:** a generator, in `aethelgard-svd-hal`, that turns a `SolvedTopology` into the source of a `no_std` board crate. In that crate:
> - each solved pin and peripheral is a typestate, so misuse fails to compile;
> - register access goes through one trait a Kani harness can implement over a register image it owns;
> - the generator also writes the proof harness, from the specification side.
>
> It is NOT:
> - drivers for the sensors;
> - clock-tree solving beyond enabling the peripherals the topology uses;
> - firmware that runs anywhere, which is P2;
> - an `aethelgard build` subcommand. If the spike shows one is needed, that is a ruling.

**Cadence:** spike first, then STOP for the architect's ruling, then build, all in one session. One PR for both halves, left unmerged.

## 0. Spike — the questions it must answer

Record every answer in `docs/work-orders/AET-WO-1400.FEEDBACK.md` under "Spike", with the command or CI run that measured it. Then STOP. Probes go under `spike/`; the build deletes them.

- **S1 `OQ-04`: does a proof catch a seeded bug?**
  - **Build the probe:** a minimal, hand-written version of what the generator will emit. That is one GPIO pin taken through `Unconfigured → AlternateFunction<4>`, and one I2C peripheral that needs its RCC clock-enable bit set before use. All register access goes through a `Registers` trait.
  - **Write a Kani harness** that implements `Registers` over a harness-owned array, and proves that every reachable call sequence leaves the register image in a state the specification allows. That means the right AF number in `AFRL` or `AFRH`, the mode bits, and the clock enabled before the peripheral's first write.
  - **Seed three bugs**, one at a time: a wrong AF number, a missing clock enable, and a write past the block. For each, record whether Kani fails the seeded version and proves the fixed one, with its output lines and runtime.
  - Run it through **`aethelgard prove`**, not bare `cargo kani`, so the CLI's verdict and witnesses are what is measured.
- **S2 Where the crate compiles.**
  - Does the typestate API compile on the host target, so `trybuild` and Kani can run in `gates` and `tools`?
  - Does it compile for `thumbv7em-none-eabihf`? The tools image has no such target.
  - Record what an image change would be: `rustup target add`, with its size and time. **Do not make it.** It touches `container/`, so it is a ruling.
- **S3 `embassy`.** Can the generated crate build on `embassy-stm32` 0.6.0 (MIT OR Apache-2.0) for the STM32H753ZI, or on its PAC `stm32-metapac`, without network at build time inside the image?
  - Record each option: what it pulls in, whether its HAL already owns the pins in a way that fights the generated typestates, and whether the generated crate should sit **above** it (using its drivers) or **beside** it (using only the PAC).
- **S4 `trybuild`.** On the S1 probe, write the five compile-fail snippets the gate names: an unconfigured pin, an ungated clock, a bus too fast, plus two more that the spike chooses. Record whether each fails to compile with a stable diagnostic, and whether the valid driver compiles. Note that `trybuild` snapshots `rustc`'s text; say how the pinned toolchain keeps it stable.
- **S5 The unsafe boundary.**
  - The generated crate will contain `unsafe`, for volatile access at real addresses. It sits outside the workspace (`RUL-04`), so the workspace's `forbid(unsafe_code)` does not apply to it.
  - Confirm that the generator, inside the workspace, contains no `unsafe`.
  - Say where generated crates are written, and how the tests compile them without adding them to the workspace.

**STOP** when §0 is answered: FEEDBACK's Spike section written, the PR open, every answer cited. The architect rules on the PR and gives the build goal in the same message.

## 1. Scope — the build half, after the ruling

This is shaped by the spike. Its intent:

1. **The generator** in `aethelgard-svd-hal`: `SolvedTopology` + `PinTable` → crate source, written deterministically (`INV-01`), with no `unsafe` in the generator itself.
2. **The generated crate:**
   - typestates for every solved pin and peripheral;
   - a register trait;
   - the `unsafe` real-address implementation, kept to one module;
   - a generated Kani harness that implements the trait over a register image.
3. **The gate's clauses 3, 4 and 5:**
   - the `trybuild` suite;
   - a byte-identical regeneration test;
   - the proof through `aethelgard prove`, passing on the generated block. If it cannot pass, `OQ-04` in `architecture.md` §8 is amended by the architect, from the FEEDBACK.

## 2. Invariants and fences

- **`INV-02`:** illegal use fails to compile; a runtime check is not a substitute.
- **`INV-07`:** the harness is generated from the specification side, and never accepted from the crate under proof.
- **No `unsafe` in any workspace crate.** The generated crate is not a workspace crate.
- **Protected surfaces diff to empty:**
  - the steering documents, `README.md`, `LICENSE-*`, `docs/inputs/**`;
  - `container/` and `.github/workflows/`, with any image change stopped for a ruling;
  - `aethelgard-ir`'s types, except as the ruling allows.
- One PR, left unmerged. **Push and revoke by the FUR-WO-0301 F7 route:** a `grep -o` redirect from your transcript into `gh-write prep`'s file, never the Write tool.

## 3. Acceptance

1. Every §0 answer is in FEEDBACK with its measurement.
2. After the build, the gate's clauses 3, 4 and 5 pass, or have a recorded `OQ-04` outcome, in `gates` and `tools`. Tier 2 is proven in CI (`docs/roadmap.md` §6).
3. CI is green on the first push after the ruling.

## 4. Scope fence

Sensor drivers, clock-tree solving, flashing, running firmware, an image change, other parts.

## 5. Adversarial pass — full

1. Write firmware that misuses hardware and still compiles.
2. Make the generated harness prove something weaker than it claims: vacuity, an `assume` that excludes the bug, or an unwound loop that hides a write.
3. Make two regenerations differ.

## 5a. Reduction pass — diff-sized

Run the `reduction-pass` skill on the finished diff. One line in FEEDBACK's gates.

## 6. Upward-feedback directive

`docs/work-orders/AET-WO-1400.FEEDBACK.md`, same schema. It carries:
- the spike's measurements for `architecture.md` §2;
- the `OQ-04` answer;
- the image-change proposal, if any;
- the gates line;
- a promote-or-needs-none pass over `AET-WO-1200`'s FEEDBACK.

## 7. Flag-and-stop conditions

Park the branch and say so, in FEEDBACK and in the session, if any of these appears:
- An image change, a new target, or anything under `container/`.
- `unsafe` in a workspace crate.
- A proof that passes vacuously, or one that can only pass with an `assume` that excludes the seeded bug.
- A harness taken from the crate under proof.
- A test that can only go green by being skipped, ignored, or weakened.
- Any step that needs `sudo`, a system package, or a secret beyond the push route above.

## 8. Kickoff prompt

**Step 1, in its own turn:** send `/goal` alone, read what is in force, and dismiss it.

**Step 2, in its own turn, set the goal for the spike half:**

```
/goal AET-WO-1400 spike: every §0 question has a measured answer or a recorded reason it cannot be, cited in FEEDBACK, on an open unmerged PR from wo/aet-1400-board-crate, and the build half has not started.
```

Confirm that the "Goal set:" echo matches.

**Step 3, the kickoff message:** carried by the architect at kickoff, with the live base. The architect gives the build goal in the same message as the ruling.
