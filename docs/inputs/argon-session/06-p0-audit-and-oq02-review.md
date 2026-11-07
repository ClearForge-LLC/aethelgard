# Argon session capture — response 6 ("P0 red-team audit, OQ-02 lock-step, P1 readiness")

**Source:** Argon (Gemini), replying to the architect's briefing on `main` at 9818c4f, which was sent with a plain-text dump of the repository.
**Captured:** 2026-10-29. Scotty pasted it into the architect's Claude Code session. It first arrived truncated, in four parts; this is the complete copy, which arrived afterwards. Argon's text is reproduced verbatim, including its closing `git` commands and PR summary, which nobody executed.
**Fidelity notes:** Math is kept as Argon wrote it, in LaTeX `$…$`. These are Argon's claims, not measurements:
- Where the architect accepted one, overstated it, or checked it at source, see `docs/architecture.md` §2.7 and the 2026-10-29 amendments in both steering documents.
- §1.2's plan to set `BERR` and `ARLO` from a Python hook is contradicted by Renode 1.17.0's source (`architecture.md` §2.7).

Treat this file as an input artifact, not a steering document.

---

# Argon Audit: P0 Red-Team, OQ-02 Lock-Step Architecture, and P1 Readiness

**Date:** 2026-10-29  
**Target Commit:** `9818c4f4fd0b328174a91e72bc3553bdb59579ed` (`main`)  
**Author:** Argon (Gemini, Co-Architect & Red Team)  
**For:** Scotty (Owner) and Claude (Co-Architect)

---

## 0. Ratification of Departures (`docs/architecture.md` §2.1 & §4)

Every departure from the founding Argon drafts recorded in `docs/architecture.md` §2.1 and §4 is ratified as an improvement grounded in empirical measurement:
1. **Instruction-level vs. cycle-accurate (`INV-04`, `RUL-05`):** Renode models CPUs at an integer instruction rate per virtual microsecond with a 1 ns virtual-time resolution (`docs/architecture.md` §2). Naming it instruction-level aligns the contract with physical reality.
2. **Local-first trace archive (`RUL-06`):** Making disk the archive of record and R2 an optional mirror guarantees `INV-04` and `INV-06` hold under `--network none` and without cloud credentials.
3. **CLI-first contract & `rmcp` subcommand (`RUL-10`, `RUL-12`):** Eliminating a separate TypeScript MCP server removes a schema-drift boundary (`INV-01`) and keeps Node.js out of the runtime container.
4. **STM32H753ZI on Nucleo-H753ZI (`RUL-08`):** Verified by the Renode 1.17.0 platform and changelog audit (`docs/architecture.md` §2.2). Holding RP2350 as the second-vendor target prevents the IR from collapsing into an STM32 register map (`docs/northstar.md` §6).
5. **`INV-07` (Gates are never computed from the artifact they judge):** Essential safeguard in `docs/northstar.md` §4 to prevent `aethelgard-evolve` from reward-hacking its own pass criteria.

---

## 1. `OQ-02` Desk Spike: Lock-Stepping Renode and `ngspice` (`docs/architecture.md` §2.2, §8)

**Verdict on the `docs/architecture.md` §2.2 Prior:** Driving `libngspice.so.0` in-process is the right transport for the analog solver, **provided** you resolve three structural traps before `AET-WO-2100` starts.

### 1.1 Three Traps in the Current Prior

1. **In-process with Aethelgard $\neq$ In-process with Renode:**
   `aethelgard` is a Rust binary (`aethelgard-twin` / `aethelgard-cli`), whereas Renode is a separate .NET/Mono process (`/opt/renode/renode`, `container/Dockerfile:31-36`). Linking `libngspice.so.0` into `aethelgard-twin` removes the IPC boundary between Rust and `ngspice`, but the process boundary between Rust and Renode remains.
2. **Workspace `#![forbid(unsafe_code)]` Collision:**
   Root `Cargo.toml` sets `unsafe_code = "forbid"`, and `crates/aethelgard-cli/tests/workspace_shape.rs:124-131` enforces `#![forbid(unsafe_code)]` on every workspace member. Meanwhile, `docs/architecture.md` `RUL-04` restricts the `unsafe` carve-out strictly to *generated* board crates outside the workspace. Calling `libngspice` C APIs (`ngSpice_Init`, `ngSpice_Init_Sync`, `ngSpice_Command`, `ngGet_Vec_Info`) via `bindgen` requires `unsafe extern "C"` and raw pointer callbacks.
   - **Action needed:** Either amend `RUL-04` / `workspace_shape.rs` to permit a tightly-scoped FFI module in `aethelgard-twin`, or isolate the `libngspice` C bindings in a dedicated non-workspace helper crate/binary.
3. **Do NOT use `libngspice`'s Background-Thread Interface (`bg_run`):**
   `libngspice`'s `bg_run` spawns an asynchronous pthread inside `libngspice` that races ahead of caller time; pausing it via `bg_halt` introduces OS thread-scheduling jitter, violating `INV-04` (deterministic replay).

### 1.2 Exact APIs for Deterministic Co-Simulation

* **`libngspice` Side — Synchronous Step Callbacks (`sharedspice.h`):**
  - Initialize on the main thread with `ngSpice_Init` and `ngSpice_Init_Sync(SendStat, ControlledExit, SendData, InitData, GetVSRCData, GetISRCData, Ident, userData)`.
  - Run a synchronous transient simulation (`ngSpice_Command("tran <tstep> <tstop>")`).
  - At every internal SPICE time step, `libngspice` synchronously invokes `GetVSRCData(double *v, double time, char *node, ...)` / `GetISRCData` on the **same thread** to sample external digital drive states, and `SendData(pvecvaluesall, ...)` to emit solved node voltages (`v(sda)`, `v(scl)`, `v(vdd)`). Zero threads, 100% deterministic (`INV-04`).
* **Renode Side — Virtual Time Quantum & Transaction Hooking:**
  - Renode's `EmulationManager.Instance.CurrentEmulation.MasterTimeSource` advances virtual time in steps of `Quantum` at a 1 ns resolution (`docs/architecture.md` §2).
  - **Critical Hardware-Model Trap:** In `platforms/cpus/stm32h7.repl`, Renode's `STM32F7_I2C` is a **transaction-level peripheral model**, not a cycle-stepped GPIO pin toggle model (`docs/architecture.md` §2.2). When firmware writes to `I2C_TXDR`, `STM32F7_I2C` sends a byte array (`Write(byte[] data)`) across the `II2CPeripheral` C# interface instantaneously in virtual time—it never toggles physical `SDA`/`SCL` GPIO pins at 400 kHz.
  - **How to bridge them in `AET-WO-2100` (Event-Windowed Hybrid Lock-Step):**
    1. When firmware initiates an I2C transfer at virtual timestamp $T_0$, a Renode Python peripheral hook (`RUL-12`) intercepts the transaction parameters (bus clock frequency from `I2C_TIMINGR`, payload bytes, address).
    2. The hook emits a compact synchronous event to `aethelgard-twin` over a pre-established `AF_UNIX` `SOCK_STREAM` `socketpair` (allowed by `crates/aethelgard-cli/src/run.rs:554`) or stdio pipe. *(Note: `socket()` is blocked by `run.rs:545`, so Renode cannot open a TCP or pathname Unix socket inside the sandbox).*
    3. `aethelgard-twin` steps `libngspice` synchronously across the byte window using the board's extracted $R_{\text{pullup}}$, $C_{\text{bus}}$, and rail voltage $V_{\text{DD}}(t)$.
    4. If `libngspice` measures $t_{\text{rise}} > t_{\text{max}}$ (e.g., 300 ns for 400 kHz Fast-Mode with a 47 k$\Omega$ pull-up) or rail droop below $V_{\text{IH}}$, `aethelgard-twin` returns an electrical fault verdict for that transaction, and the Python hook sets the exact hardware fault status bits (`I2C_ISR.BERR`, `ARLO`, or `NACKF` / timeout) in the `STM32F7_I2C` peripheral register map at $T_0 + \Delta T$.

### 1.3 What to Measure First in the `OQ-02` Spike
1. Inspect `STM32F7_I2C` inside `/opt/renode` in `ghcr.io/clearforge-llc/aethelgard-tools` to verify which `I2C_ISR` error bits and interrupts (`ER_IRQ`) are implemented in Renode 1.17.0 versus needing direct register-write injection from the Python hook.
2. Measure the wall-clock time of 1,000 synchronous `libngspice` transient bursts (9 I2C clock cycles each) via `ngSpice_Init_Sync`.

---

## 2. Red-Team Audit of `crates/aethelgard-cli/src/`

### RT-01 (High — False Pass): `kani.rs` stdout/stderr scraping allows `build.rs` or compiler diagnostics to spoof a passing proof
* **Where:** `crates/aethelgard-cli/src/kani.rs:18-62`, `crates/aethelgard-cli/src/main.rs:285-288`.
* **Mechanism:** `kani::judge` concatenates `stdout` and `stderr` via `out.text()`, trims every line, and matches literal lines (`Checking harness <name>...`, `VERIFICATION:- SUCCESSFUL`, `Complete - 1 successfully verified harnesses, 0 failures`). Because `cargo kani` runs `rustc` (including `build.rs` and proc-macros) and prints compiler output to the same streams, any `build.rs` or proc-macro that prints those three lines and exits 0 (or a module suffix match when `--harness` is passed without `::`, where `--exact` is omitted at `main.rs:286`) will fool `kani::judge`.
* **Recommendation:** Always resolve or require `--exact` where supported, or cross-check Kani's machine-readable output / CBMC report files written inside `scratch.tool/target` rather than trusting combined stdout/stderr lines alone.

### RT-02 (Medium — False Pass): `renode.rs` ignores `[ERROR]` log lines if `cpu` instantiated before a partial `.repl` error
* **Where:** `crates/aethelgard-cli/src/renode.rs:20-73`.
* **Mechanism:** `renode::judge` only checks for the literal substring `"There was an error executing command"` (`LOAD_ERROR` at line 9). If a `.repl` file includes `stm32h753.repl` (so `cpu (CortexM)` with `Slot: 0` is registered first) and a subsequent peripheral registration or Python hook inside the `.repl` logs an `[ERROR]` line without triggering the top-level monitor command error string, `cpus.is_empty()` is `false` and `error` is `None`, yielding `"verdict":"pass"` on a broken platform description.
* **Recommendation:** Treat any `[ERROR]` log line in Renode's output during platform load as a failed witness alongside `LOAD_ERROR`.

### RT-03 (Medium — Sandbox Integrity): `socket_filter()` allows `ptrace`, `process_vm_writev`, and `kill` across UID 0 processes when Landlock ABI < 6
* **Where:** `crates/aethelgard-cli/src/run.rs:182-189`, `run.rs:514-565`, `crates/aethelgard-cli/src/main.rs:314-318`, `container/Dockerfile:12`.
* **Mechanism:** Inside the published image, everything runs as `root` (`UID 0`). Landlock signal scoping (`Scope::Signal`) only exists on kernel ABI V6+ (`run.rs:187`). On a host kernel with Landlock ABI 3, 4, or 5 (which passes the required ABI V3 floor in `run.rs:577`), `sandbox.scoped` is `false`. Because `socket_filter()` only blocks `socket`, `socketpair` (non-stream/seqpacket), and `io_uring`, a hostile process inside `prove` or `simulate` (which also has read-only `/proc` at `main.rs:316`) running as `UID 0` can call `ptrace(PTRACE_ATTACH)`, `process_vm_writev`, `pidfd_open`, or `kill(getppid(), SIGKILL)` against the unconfined `reaper` or the parent `aethelgard` CLI process.
* **Recommendation:** Add `ptrace` (101), `process_vm_readv` (310), `process_vm_writev` (311), and `pidfd_open` (434) to the deny list in `socket_filter()` (`run.rs:514`) immediately, or fold into `AET-WO-1600`.

### RT-04 (Low — Disk/Memory Exhaustion): `input::crate_dir` has no byte or file-count cap
* **Where:** `crates/aethelgard-cli/src/input.rs:124-147`.
* **Mechanism:** Unlike `validate` (capped at `16 MiB` in `main.rs:38`) and `kicad::read_report` (`16 MiB` in `kicad.rs:116`), `input::crate_dir` copies an arbitrarily large directory tree into `/tmp` before hashing.
* **Recommendation:** Enforce `MAX_INPUT_BYTES` (16 MiB cumulative) in `input::crate_dir` and return `InputTooLarge` (exit 3) when exceeded.

---

## 3. Enforcing `INV-07` in `AET-WO-1300` (KiCad DRC Rules)

Today, `kicad-cli pcb drc` evaluates a `.kicad_pcb` board against the `(setup ...)` design rules stored inside that same `.kicad_pcb` file (`container/fixtures/drc-clean.kicad_pcb:31-33`). To uphold `INV-07` (*"The gates are not computed from the artifact they judge"*):

1. **Specification-Derived `.kicad_dru` at Gate Time:**
   In `AET-WO-1300`, when `aethelgard check drc` judges a generated board against a specification, the CLI should generate the `.kicad_dru` (Design Rules) file and overwrite the `(setup ...)` block in `scratch.tool/input.kicad_pcb` **directly from the validated `TopologyGraph` (`aethelgard-ir`)** immediately before invoking `kicad-cli`.
2. **Why this works:**
   The board artifact (`input.kicad_pcb`) supplies only the physical copper geometry (footprints, pads, tracks, vias, zones); the specification (`aethelgard-ir`) supplies the minimum clearances, track widths, and voltage-domain isolation rules at gate time. Even if a bug or mutation alters the `(setup ...)` header in the `.kicad_pcb` file on disk, the gate overwrites the rules from the specification before judging.

---

## 4. `RUL-06` and `AET-WO-0400`: Drop Cap'n Proto for Pure-Rust Deterministic Framing

`docs/work-orders/AET-WO-0400.md` §0 S1 asks how to compile `.capnp` schemas when `capnpc` (the C++ compiler) is absent on Furnace and in the container, and when `crates/aethelgard-cli/tests/workspace_shape.rs:134-144` explicitly forbids `build.rs` across all workspace crates (`no_member_has_a_build_script`).

**Recommendation: Amend `RUL-06` to replace Cap'n Proto with pure-Rust `serde` + `postcard` (or canonical length-prefixed binary records with CRC32/SHA-256 framing).**

* **Why Cap'n Proto was in the founding draft:** I originally proposed Cap'n Proto assuming polyglot services in different languages would mmap gigabytes of zero-copy traces.
* **Why pure-Rust `serde` (`postcard` or `ciborium`) wins now:**
  1. **Zero C++ toolchain & zero `build.rs`:** `workspace_shape.rs:134` forbids `build.rs`, and `RUL-12` states *"Rust for everything in this repository... C++ is not written here."* Pure-Rust `serde` derives directly on Rust structs in `aethelgard-chaos` / `aethelgard-ir` (`INV-01`).
  2. **The Python-in-Renode counter-argument does not require Cap'n Proto:** `container/Dockerfile` does not have `pycapnp` installed, and `container/check-pins.sh` forbids `pip`. Python hooks inside Renode (`RUL-12`) are tiny peripheral stubs that talk to the Rust orchestrator (`aethelgard-twin`); they should emit simple line-delimited or `struct.pack` events over the bridge pipe to `aethelgard-twin`, and **only the Rust orchestrator** writes the canonical trace archive (`docs/architecture.md` §3).
  3. **Immediate cost savings:** Dropping Cap'n Proto eliminates the `AET-WO-0400` §0 S1 spike entirely, allowing `AET-WO-0400` to run as a single-pass build on a cheaper builder model.

---

## 5. P1 Readiness: IR Seams (`AET-WO-1100`) and Pin Solver (`AET-WO-1200`)

### 5.1 Three Seams to Prepare in `crates/aethelgard-ir` (`AET-WO-1100`)
1. **Separate Input Specification from Solved Graph:**
   Do not add optional `Option<SolvedPin>` fields to `TopologyGraph` (`crates/aethelgard-ir/src/lib.rs:20`), as optional solved fields weaken `INV-06`. Keep `TopologyGraph` (or `Specification`) for the declarative input, and introduce `SolvedTopology` as the output of `AET-WO-1200` using **`BTreeMap` / `BTreeSet` (never `HashMap`)** so emitted `.kicad_sch`, `.kicad_pcb`, and board crates are byte-identical across runs (`INV-01`, P1 exit gate clause 4).
2. **Hardware Ground-Truth Correction on CMSIS-SVD (`docs/roadmap.md` §5 `AET-WO-1100`):**
   `docs/roadmap.md` §5 states that `AET-WO-1100` ingests the *"microcontroller package and alternate-function table... from the vendor register description."*
   **Fact check:** ARM CMSIS-SVD files (`STM32H753.svd`) contain **only** memory-mapped peripheral registers, bitfields, and interrupt numbers. **SVD files do NOT contain physical package pinouts (e.g., LQFP144 pin coordinates) or the GPIO Alternate Function mux matrix (`AF0`..`AF15` mapping `PB8` to `I2C1_SCL`).**
   - For STM32, physical pins and alternate-function tables live in `stm32-data` (derived from STM32CubeMX XMLs), whereas register blocks live in `.svd`. `AET-WO-1100` must ingest both (or a unified MCU pin+register descriptor) rather than expecting `.svd` alone to hold pin-mux tables.
3. **Remember `workspace_shape.rs` Graduation:**
   `crates/aethelgard-cli/tests/workspace_shape.rs:26-30` and `:146-182` enforce that `eda`, `svd-hal`, `twin`, `chaos`, and `evolve` contain only `pub use aethelgard_ir::*;` and no files beyond `src/lib.rs`. Every work order that implements one of those crates (`0400`, `1100`, `1200`, `1300`, `1400`) must explicitly authorize removing that crate from `libraries()` in `workspace_shape.rs`.

### 5.2 `AET-WO-1200` Solver Prior (`RUL-02`)
Invert the `RUL-02` prior from Z3 SMT to **pure-Rust backtracking CSP search (AC-3 + MRV over `BTreeMap`)**:
* STM32H753ZI has $\le 114$ GPIOs with $\le 16$ alternate functions each. A pure-Rust finite-domain constraint solver resolves a 20-peripheral board in $< 1\text{ ms}$ in ~250 lines of safe Rust, produces deterministic unsat conflict attributions (`INV-06`), and avoids linking C++ `libz3` (`unsafe` FFI + multi-gigabyte container/build overhead).

---

## 6. Model-Tier Routing & Spend Optimization

| Work Order | Builder Tier | Adversarial Tier | Rationale |
|---|---|---|---|
| `AET-WO-0400` (Trace archive) | **Cheap Builder** | **Strong** (1 pass) | Straightforward binary framing + SHA-256 state replay once `RUL-06` drops `capnpc`. |
| `AET-WO-1100` (IR graph + MCU ingest) | **Cheap Builder** | **Cheap** | Data modeling, `object_only!`, and `RUL-14` ASCII validation once architect specifies schema. |
| `AET-WO-1200` (Pin/domain solver) | **Mid/Strong Builder** | **Strong** | Pure-Rust CSP algorithm + exact dual-claimant unsat reporting (~300 LOC). |
| `AET-WO-1300` (KiCad emitter) | **Cheap Builder** | **Mid** | `aethelgard check erc/drc` acts as an automated ground-truth oracle inside the loop. |
| `AET-WO-1400` (Typestate HAL + Kani) | **Strong Builder** | **Strong** | Complex Rust codegen (`syn`/`quote`), sealed typestates (`INV-02`), and Kani register proofs. |
| `AET-WO-1500` (`mcp` subcommand) | **Cheap Builder** | **Strong** (argv rule) | Thin stdio wrapper over existing `clap` tree (`RUL-10`). |
| `AET-WO-1600` (Host syscall allowlist) | **Strong Builder** | **Strong** | Raw BPF / kernel security boundary (`run.rs`). |
| `AET-WO-2100a` (`.repl` emitter) | **Cheap Builder** | **Mid** | Text emission checked by `aethelgard simulate`. Split from `2100b` to reduce Opus spend. |
| `AET-WO-2100b` & `2200` (`OQ-02` bridge) | **Strong Builder** | **Strong** | Highest technical risk (`docs/roadmap.md` §3): synchronous `libngspice` + Renode lock-step. |
| `AET-WO-2300` & `3300` (Faults & Fab) | **Cheap/Mid Builder** | **Mid** | Scenario orchestration and Gerber/BOM packaging. |
| `AET-WO-3100` & `3200` (Attribution & Evolve) | **Strong Builder** | **Strong** | Second highest risk (`docs/roadmap.md` §3): fault attribution accuracy (`INV-05`). |


git checkout -b docs/argon-p0-audit origin/main
# Write the markdown block above to:
# docs/inputs/argon-session/06-p0-audit-and-oq02-review.md
git add docs/inputs/argon-session/06-p0-audit-and-oq02-review.md
git commit -m "docs(inputs): add Argon P0 red-team audit and OQ-02 architecture review"


## Summary
Records Argon's review of `main` at `9818c4f` (`AET-WO-0100` through `AET-WO-0300a`), covering:
1. **`OQ-02` Lock-Step Architecture:** Why `libngspice`'s synchronous `ngSpice_Init_Sync` API (rather than `bg_run`) paired with event-windowed Renode `STM32F7_I2C` transaction hooks is needed, plus the `#![forbid(unsafe_code)]` workspace collision with `libngspice` FFI.
2. **Red-Team Findings (`RT-01`–`RT-04`) in `aethelgard-cli`:**
   - `RT-01`: `kani.rs` stdout/stderr scraping spoofing via `build.rs` or suffix matches.
   - `RT-02`: `renode.rs` missing `[ERROR]` check when a `.repl` fails after instantiating `cpu`.
   - `RT-03`: `run.rs` `socket_filter()` allowing `ptrace`/`process_vm_writev`/`kill` across UID 0 processes on kernels with Landlock ABI < 6.
   - `RT-04`: Unbounded directory copy size in `input::crate_dir`.
3. **`INV-07` in `AET-WO-1300`:** Gate-time `.kicad_dru` / `(setup ...)` injection from `aethelgard-ir`.
4. **`RUL-06` (`AET-WO-0400`):** Proposal to amend `RUL-06` from Cap'n Proto to pure-Rust `serde` (`postcard`) framing, eliminating the C++ `capnpc` and `build.rs` blocker.
5. **P1 (`AET-WO-1100` / `1200`) Ground-Truth Warning:** CMSIS-SVD files lack physical package pins and GPIO Alternate Function (`AF0`..`AF15`) tables (which live in `stm32-data`), plus inverting the `RUL-02` solver prior to pure-Rust CSP search.
6. **Spend & Model-Tier Routing Table:** Tagging all remaining P0–P3 work orders by model tier.
