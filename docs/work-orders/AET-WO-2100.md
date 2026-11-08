# AET-WO-2100 — The twin boots: a Renode platform from the solved graph, and generated firmware running headless

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-2100-twin-boot`.
**Author:** Claude (architect) · **Builder:** a **strong** builder for the spike; the build tier is set at the ruling. Targeted adversarial pass (`docs/roadmap.md` §6). A fresh session on Furnace.
**Phase:** P2, the first work order. **Serves:** P2's twin. Every later P2 work order (2101's transport, 2200's analog bridge, 2300's fault campaigns) runs firmware on this platform.

**Grounds:**
- `docs/architecture.md` §2.2: Renode 1.17.0's `stm32h7.repl` instantiates 4 I2C controllers (`STM32F7_I2C`) but **only one SPI instance, `spi4`**, although the `STM32H7_SPI` model exists. The generated description must add the instances the solved topology uses.
- `docs/architecture.md` §2.4 and §2.9: Renode exits 0 even when a load fails, and never logs a platform failure at `[ERROR]`. So `simulate` judges by two witnesses: a listed CPU, and the absence of `There was an error executing command`. Any run verdict needs the same care.
- `docs/architecture.md` §2.7: `STM32F7_I2C` is transaction-level and models no I2C error flag. That matters for P2's faults, not here.
- `docs/architecture.md` `RUL-04` as amended, and §2.13: the board crate `boards/sentinel-01` is dependency-free, with typestates from `SolvedTopology`. Its `unsafe` is confined to the real-address implementation.
- `docs/work-orders/AET-WO-1400.FEEDBACK.md` S2 and F3 (ruled R2). The board crate compiles for `thumbv7em-none-eabihf` in an isolated toolchain. Adding that target to the image is one `rustup-init` flag, about 72 MB installed and 1 s, **deferred until P2's first firmware work order needs it. This is that work order.**
- `docs/architecture.md` §2.2 on `hilt` 0.2.0: a reference for booting an ELF in Renode from tests, not a dependency.
- The seams:
  - `crates/aethelgard-cli/src/renode.rs` (`simulate`);
  - `aethelgard-svd-hal`'s generator;
  - `boards/sentinel-01`;
  - `container/Dockerfile`: no image change without a ruling.

> **What this is:** the solved graph lowers to a third projection, a Renode platform description, beside the board and the board crate. Firmware written against the generated board crate boots on it headless and reaches a checkable point.
>
> It is NOT:
> - the lock-step analog transport (`2101`);
> - fault injection (`2300`);
> - sensor models beyond what booting needs;
> - a change to the board crate's API.

**Cadence:** spike first, then STOP for the architect's ruling, then build, all in one session. One PR for both halves, left unmerged.

## 0. Spike — the questions it must answer

Record every answer in `docs/work-orders/AET-WO-2100.FEEDBACK.md` under "Spike", with the command or CI run that measured it. Then STOP. Probes go under `spike/2100/`, and the build deletes them.

Renode and the isolated `thumbv7em` toolchain both exist on Furnace (`AET-WO-1600`'s and `AET-WO-1400`'s kits under `~/.cache`). So the spike can run end to end there. CI is the witness only once the ruling settles the image.

- **S1 The platform description.**
  - Emit a `.repl` from the reference `SolvedTopology`: include `stm32h753.repl`, add each SPI instance the topology uses at RM0433's address, with `STM32H7_SPI`, and name each solved peripheral as Renode does.
  - Load it through today's `simulate`, and record its peripheral listing.
  - Record which names, addresses and interrupt lines come from where. They must trace to the pinned ST header (`AET-WO-1400`) or Renode's own files, never to memory.
- **S2 The firmware.**
  - Write the smallest firmware that uses the generated board crate: configure the solved I2C and SPI pins and clocks through the typestates, then reach a fixed point that Renode can observe.
  - Measure the runtime options:
    - a hand-written vector table and linker script in the firmware crate, dependency-free;
    - against `cortex-m-rt` (MIT OR Apache-2.0), vendored or fetched.
  - For each option record:
    - its licence;
    - whether it builds offline;
    - its `unsafe` (allowed outside the workspace, as in the board crate);
    - its ELF size.
- **S3 The observable point.** Choose how Renode tells that the firmware ran. Measure each candidate for determinism:
  - a UART line;
  - a write to a reserved RAM word read through the monitor;
  - a GPIO toggled on a solved pin;
  - the register image after N virtual milliseconds.

  The verdict must not rest on Renode's exit code (§2.4).
- **S4 Headless and deterministic.**
  - Run the boot for a fixed virtual time, five times, and record the wall time and the observed value.
  - Show the result does not depend on host speed.
  - Record what Renode's time framework gives: 100 MIPS default, 1 ns resolution.
- **S5 Where it runs in CI.**
  - The image has no `thumbv7em` target. Propose the image change and its size, but do not make it: `container/` needs a ruling.
  - Also measure the alternative: build the ELF in `gates`, which has rustup and the network, and hand it to `tools` as an artifact. That alternative is a workflow change, so it also needs a ruling.
  - Say which keeps `INV-01` (the ELF a function of the source) and `INV-03`.
- **S6 The CLI surface.** Should this be `simulate --firmware <elf> --until <condition>`, or a new leaf? Propose the verdict's shape, which becomes an MCP tool automatically, and the refusals.

**STOP** when §0 is answered: FEEDBACK's Spike section written, the PR open, every answer cited. The architect rules on the PR and gives the build goal in the same message.

## 1. Scope — the build half, after the ruling

Shaped by the spike. Its intent:
1. **Emission:** a deterministic `.repl` emitter from `SolvedTopology`, byte-identical across runs.
2. **The firmware:** a reference firmware crate for `sentinel-01`, outside the workspace like the board crate, that uses only the board crate's API.
3. **The CLI:** `simulate` boots firmware and judges by the ruled observable, with no trust in Renode's exit code.
4. **Tests:** the platform loads with every solved peripheral listed; the firmware boots to the observable; two emissions are byte-identical; two boots give the same observation.

## 2. Invariants and fences

- `INV-01`, `INV-03`, `INV-06`. No `unsafe` in any workspace crate. Generated or reference firmware is outside the workspace, like `boards/`.
- **Protected surfaces diff to empty:**
  - the steering documents, `README.md`, `LICENSE-*`, `docs/inputs/**`;
  - `container/` and `.github/workflows/`, with any change stopped for a ruling;
  - `aethelgard-ir`'s types;
  - the board crate's API.
- One PR, left unmerged. **Push and revoke by the FUR-WO-0301 F7 route.**

## 3. Acceptance

1. Every §0 answer is in FEEDBACK with its measurement.
2. After the build, §1.4 passes where the ruling puts it.
3. CI is green on the first push after the ruling.

## 5. Adversarial pass — targeted

1. Make Renode report a boot that did not happen: a load that half-failed, or an observable that could be satisfied by reset values.
2. Make two boots disagree.
3. Make the emitted platform name a peripheral the topology did not solve, or miss one it did.

## 5a. Reduction pass — diff-sized

Run the `reduction-pass` skill on the finished diff. One line in FEEDBACK's gates.

## 6. Upward-feedback directive

`docs/work-orders/AET-WO-2100.FEEDBACK.md`, same schema. It carries:
- the S1 to S6 measurements;
- the image or workflow proposal;
- the gates line.

## 7. Flag-and-stop conditions

- A change under `container/` or `.github/workflows/` (stop for the ruling S5 asks for).
- `unsafe` in a workspace crate.
- A boot verdict that rests on Renode's exit code alone.
- A licence that does not fit `RUL-11`.
- A test that can only go green by being skipped, ignored, or weakened.
- Any step that needs `sudo`, a system package, or a secret beyond the push route above.

## 8. Kickoff prompt

```
/goal AET-WO-2100 spike: every §0 question has a measured answer or a recorded reason it cannot be, cited in FEEDBACK, on an open unmerged PR from wo/aet-2100-twin-boot, and the build half has not started.
```
