# Aethelgard — architecture

**Status:** ratified 2026-10-19 by merge of pull request #1 and Scotty's delegation of `OQ-01` and `OQ-05` to Claude the same day; first drafted 2026-10-14. **Supersedes** the unmerged architecture draft inside `docs/inputs/argon-session/05-bootstrap-script-v2-repo-genesis.md`. Read `northstar.md` first.
**Author:** Claude Code, synthesizing the Argon (Gemini) planning session with Scotty · **Gate:** Scotty.

---

## 1. How to use this document

This is the source of truth for what is being built and why each choice was made. Implementation detail belongs in work orders; this document constrains them. The roadmap tracks sequence; this tracks shape.

**The divergence rule:** any deviation from this document is recorded as a dated amendment in §10 with its rationale. Undocumented drift is a defect, not a shortcut.

Invariants are **not** defined here. They live in `northstar.md` §4 and are cited below as `INV-01` through `INV-07`.

## 2. Ground truth — measured, not assumed

Measured on 2026-10-14 in the Claude Code cloud container that produced this document, before anything below was designed. Commands are cited so a reader can re-run them. This container is not the build host; the deterministic build environment is `AET-WO-0200`'s job, and its measurements supersede these.

**Repository.** One commit on `main`, a twelve-byte `README.md`, no code (`git log --oneline`, `ls -la`).

**Toolchains present.** `cargo 1.97.0`, `rustc 1.97.0`, `python3 3.13.16`, `node v22.22.0`, `docker 29.8.2`, `nix 2.34.6` (`<tool> --version`).

**Toolchains absent.** `kicad-cli`, `renode`, `ngspice`, `kani` are not on `PATH` (`which`). Nothing in this document has been run against them.

**Crates exist on crates.io** for every library Argon's plan names (`cargo info <crate>`): `z3`, `svd-parser`, `svd2rust`, `capnp`, `embassy-stm32`, `embassy-rp` (tagged `rp2040`, `rp235x`), `kani-verifier`, `rmcp`, `esp-hal`. Existence only; none was compiled.

**Renode.** Its supported-boards page lists `stm32h7`, `stm32h743`, `stm32h747`, `stm32h753`, and `nucleo_h753zi` platforms. No RP2040, RP2350, ESP32, or ESP32-S3 platform appears in the first 100,000 characters of that page; the remaining 447,752 characters were not read, so their absence is **unverified**. Antmicro published an STM32H753 Renode reference platform with a physical devkit and a digital twin on 2026-07-16, running Zephyr. Renode's time framework models a CPU as an integer number of instructions per virtual microsecond (default 100 MIPS) with a virtual-time resolution of one nanosecond. The page never uses the words "cycle accurate."

**KiCad 9.0 CLI.** `kicad-cli sch erc` and `kicad-cli pcb drc` exist and accept `--format json` and `--exit-code-violations`, which makes the exit code 5 when violations are found. The documentation does not describe the exit code when the flag is omitted; a third-party source reports it is 0 even with violations. **Unverified here**; `AET-WO-0200` tests it. *Settled in §2.4: it is 0.*

**Kani.** Kani stubs functions it cannot analyze (inline assembly, FFI) with hand-written mocks that are not checked against the real implementation, and it does not detect data races or all provenance-related undefined behaviour. How it treats volatile memory-mapped register access was not confirmed from a primary source. **Unverified**; `AET-WO-1400` spikes it.

### 2.1 Corrections to the planning conversation

The wrong belief is the instructive half, so both columns stay.

| Assumed in the Argon session | Measured |
|---|---|
| Renode gives a "cycle-accurate" digital twin, and traces are indexed by "simulation picosecond" | Renode is instruction-level: an integer instruction count per virtual microsecond, with one-nanosecond virtual-time resolution. There is no public RTL for these microcontrollers, so nothing cycle-accurate is available. `INV-04` is now worded "at the simulator's own time resolution" and every "picosecond" in Argon's drafts is read as "trace timestamp" |
| `kicad-cli sch erc` and `pcb drc` "pass with 0 errors" is a usable exit gate | Without `--exit-code-violations` the documented nonzero exit does not apply, so a gate written as "exit 0" can pass with violations present. Every KiCad gate in the roadmap passes the flag and expects exit 5 on an injected violation |
| STM32H7, RP2350, and ESP32-S3 are equal candidates for the reference probe | Only STM32H7 has documented Renode platforms, plus a vendor-published reference board with a digital twin. The other two are unverified in Renode and the Xtensa core in ESP32-S3 has only a sample controller listed |
| Argon could push a branch to GitHub | It could not; both of its bootstrap scripts were never executed. The second script also has a bug: an unquoted heredoc containing backticks around `${CRATE}` would execute each crate name as a shell command |
| The steering documents were "Ratified" | Nobody ratified them. These drafts say so in their status lines |
| The seven-crate workspace "compiles" | It was never compiled anywhere. The workspace is built and measured by `AET-WO-0100` |
| Verilator is an alternative to Renode for "RISC-V/ARM cores", and Miri an alternative to Kani for the driver patch (founding doc, capture 00) | Verilator simulates RTL, and no RTL exists for STM32, RP2350, or ESP32 silicon; it applies only to open cores such as VexRiscv, which would be a different reference target. Miri interprets ordinary Rust tests and cannot execute `no_std` firmware against memory-mapped registers. Neither is a drop-in alternative; both are recorded here so they are not re-proposed as one |
| Planning documents belong in `docs/genesis/GENESIS.md` and `STEERING.md` | The ClearForge `repo-genesis` contract places them at `README.md`, `docs/northstar.md`, `docs/architecture.md`, `docs/roadmap.md`. Argon corrected this itself in capture 05; this document keeps that correction |

The Renode correction is the load-bearing one. A plan that promises cycle accuracy and picosecond replay will be believed, and the first time a timing bug slips through simulation the project's credibility goes with it. Promising instruction-level fidelity with an honest resolution is a claim the tool can keep.

**Blocked, not faked.** Three numbers Argon's roadmap used as gates have never been measured: the solver's time on a realistic pin table ("< 1000 ms"), the trace volume of a fault campaign, and Kani's runtime on a generated register block. `AET-WO-1200`, `AET-WO-0400`, and `AET-WO-1400` produce them. No estimates are recorded in their place.

### 2.2 Measurements added 2026-10-19

Taken to rule `OQ-01` and `OQ-05`. Same discipline: commands and sources cited, nothing estimated.

**Renode 1.17.0 (released 2026-09-06), STM32H7 platform.** `platforms/cpus/stm32h743.repl` and `stm32h753.repl` both include `platforms/cpus/stm32h7.repl` (read from the `opensecura.googlesource.com` mirror of the Renode repository, `refs/heads/master`). That file instantiates: 4 I2C controllers (`STM32F7_I2C` model), 2 DMA controllers plus BDMA and DMAMUX, 2 ADCs (added in 1.17.0), 9 UARTs, 14 timers, 5 low-power timers, EXTI, RCC, the flash controller, QuadSPI, SDMMC, Ethernet with a PHY, 2 FDCAN, USB, RNG, CRC, hardware semaphore, independent and window watchdogs. **Only one SPI instance, `spi4`, is instantiated**, although the `STM32H7_SPI` model exists (added 1.14.0); the generated platform description must add the other SPI instances itself or the reference probe must put its SPI device on SPI4. `nucleo_h753zi.repl` adds three LEDs, the user button, and an Ethernet PHY on top, with an 8 MHz HSE.

**Renode changelog, whole file (139,683 bytes).** `grep -i` finds zero entries for `rp2040` or `rp2350`. ESP32 has "translation support for Espressif ESP32 chips" (1.14.0, 2023-08-08) and an experimental Xtensa assembler (1.16.1), but no SoC platform with peripherals. The Nucleo-H753ZI board landed in 1.15.0 (2024-03-18) "with a range of ST peripherals". This closes the gap left open in §2: the RP2350 and ESP32-S3 absence is now verified, not assumed.

**`hilt` 0.2.0 (2026-09-28, MIT OR Apache-2.0).** A Rust harness that boots an ELF in Renode from `cargo test`, using the official `antmicro/renode` container. Built-in platforms include `stm32h7()` (the Nucleo-H753ZI) and `rp2040()` with its own `.repl`. Two releases, one maintainer; it is a reference for `AET-WO-2100`, not a dependency to lean on.

**Kani feature support page.** Inline assembly: unsupported, and any unsupported feature is compiled to `assert(false); assume(false)`, which fails verification if reachable. Raw pointer types: partial. Dereferencing unaligned raw pointers and breaking aliasing rules: not detected. There is no row for volatile access; `read_volatile` falls under "compiler intrinsics: partial". **Consequence for `OQ-04`:** a proof over a real peripheral base address is not possible, because the address is not memory the harness owns. The proof has to run over a harness-owned register image substituted for the base address, which proves the typestate and driver logic, not the silicon. That is still worth having, and the gate wording in `roadmap.md` P1 already allows it.

**Analog bridge.** No Rust binding to `libngspice` exists on crates.io; the `ngspice` crate is a 0.0.1 name reservation and `paprika` is an unrelated command-line parser. The shared-library C interface (`ngSpice_Init`, `ngSpice_Command`, `ngGet_Vec_Info`, with callbacks for output and background-thread completion) is documented in the ngspice manual and reachable with `bindgen`. This is the third candidate named in `OQ-02` and it is now the prior: in-process library calls avoid a transport entirely.

**Crate versions on 2026-10-19** (`cargo info`): `rmcp 3.5.1`, `embassy-stm32 0.6.0`, `stm32h7xx-hal 0.16.0`, `svd2rust 0.37.1`, `hilt 0.2.0`.

**ClearForge MCP standard** (`clearforge-mcp-standard` skill, ClearSeal v0.6 on MCP `2026-07-28`): fleet servers are TypeScript on the official MCP TypeScript SDK; the only sanctioned exception is a platform constraint, and the server library must be the official SDK, never a framework wrapper. The standard applies to nodes that are exposed; a local stdio tool is not a node.

### 2.3 Measurements from AET-WO-0100 (2026-10-19, on Furnace)

Measured by the builder as `forge` on Furnace and re-checked by the architect in the cloud container; the record is `docs/work-orders/AET-WO-0100.FEEDBACK.md` §6.

**Toolchain.** Rust is pinned to `1.99.1` (`58cf8f9c3 2026-10-13`) in `rust-toolchain.toml`, with `cargo 1.99.1` and `rustup 1.29.1`. Edition 2024 is accepted. Rust is installed in `forge`'s home with no `sudo`, from `static.rust-lang.org` with its `.sha256` checked. It is not yet on a fresh shell's `PATH` (see `RUL-13`'s note).

**Build times.** `cargo test --workspace` takes 2.61 s from a clean target directory with the registry already fetched, and 0.11 s warm (`/usr/bin/time`). Furnace is x86_64 with 20 cores and 60 GiB.

**Egress.** `static.rust-lang.org` and `crates.io` (41 packages) were reached with no proxy or special configuration. Nothing else was fetched.

**Push path.** The per-push `clearforgekey` installation token for this repository carries the workflows permission, so a push touching `.github/workflows/` is accepted. The risk named at kickoff did not occur.

**Correction found by the build.** serde's derived `Deserialize` with `deny_unknown_fields` still accepts a positional JSON array (`["p",[],[]]` validated as `"ok":true`). `deny_unknown_fields` is not "objects only". Every specification type now deserializes through the `object_only!` wrapper in `aethelgard-ir`, and any new one must.

### 2.4 Measurements from AET-WO-0200 (2026-10-20, GitHub Actions)

Measured in CI, because Furnace has no container runtime. The builder recorded them, and the architect re-read each value from the run's own output lines. The record, with every run linked, is `docs/work-orders/AET-WO-0200.FEEDBACK.md`.

**`kicad-cli pcb drc` without `--exit-code-violations` exits 0 with violations present.** KiCad 10.0.7 and 9.0.8 both do this, with and without the network, on a board carrying a 0.05 mm clearance violation. With the flag it exits 5, and a clean board exits 0 either way. KiCad's own demo boards agree. A gate written as "exit 0" passes a broken board, which is why `RUL-03` makes the flag mandatory.

**KiCad 10 ships only as the official image.** `kicad/kicad:10.0.7` is Debian 13 (trixie) and amd64-only. Ubuntu 26.04's archive carries 9.0.8. KiCad 10 lists `--exit-code-violations` and `--format` for `pcb drc`.

**Nix supplies three of the four tools, not Kani and not the pinned Rust.** At nixpkgs `09063bd1`, it has KiCad 10.0.7, Renode 1.17.0, ngspice 45 and CBMC 6.11.0, but no `kani` attribute and only `rustc` 1.99.0. That is why the image is a Dockerfile (`RUL-15`).

**Renode 1.17.0** ships no `-dotnet` variant. The portable tarball runs headless with `DOTNET_SYSTEM_GLOBALIZATION_INVARIANT=1` and no extra packages, and loads `nucleo_h753zi.repl` in about 2 s. **A failed platform load also exits 0** when stdin is at EOF, which is always the case under Docker. It prints `There was an error executing command … Could not find file` with no `[ERROR]` tag, and `System bus created` appears before the platform is read, so neither string tells success from failure. Only the listed CPU (`cpu (CortexM)` from `peripherals`) distinguishes a loaded board.

**Kani 0.68.0, without `cargo kani setup` at build time,** tries to download its bundle from GitHub on first run, and under `--network none` it fails with exit 1. Pre-installed, it proves a harness and fails a seeded overflow with no network.

**ngspice 44.2** (trixie) measures an RC step's 10–90 % rise at `2.197224e-03` s against 2.2·R·C = `2.2e-03` s. `libngspice.so.0` is registered with the loader.

**Cost.** The image is 4.8 GB. A cold build takes about 210–260 s, and a warm one, with the layers from the Actions cache, about 50–100 s. Both candidates' layers took 2.6 GB of the repository's 10 GB cache. Smoke runs in 8 s, and the workspace gates inside the image in 17 s. An `ubuntu-latest` runner had 86 GB free with no cleanup, x86_64, 4 CPUs and 15 GiB.

**GHCR from a pull request.** With `contents: read`, `docker login ghcr.io` succeeds and the package API returns 404. What a PR cannot measure is a push; the first `main` publish is that measurement (`RUL-15`). **Confirmed 2026-10-20 by that publish,** [run 39363778781](https://github.com/ClearForge-LLC/aethelgard/actions/runs/39363778781). The `GITHUB_TOKEN` with `packages: write` created the package with no new secret and pushed `ghcr.io/clearforge-llc/aethelgard-tools@sha256:74f69bd36cda772187aea76e633e1eebd9e82bd3c07c82a16ae0c46e1bba206c`, tagged `166b746…`, after smoke passed on that same image under `--network none`. The digest is now in `container/IMAGE`. A package an org's Actions create is private by default, and an anonymous pull is refused until its visibility is set to public.

### 2.5 Measurements from AET-WO-0300 (2026-10-21, GitHub Actions and Furnace)

Measured inside the published image in CI, with Landlock and process behaviour cross-checked on Furnace's own kernel. The architect re-read each value from the runs' `ANSWER` lines; the record is `docs/work-orders/AET-WO-0300.FEEDBACK.md`.

**`kicad-cli sch erc` lies like `pcb drc`.** Without `--exit-code-violations` it exits 0 with violations present. With the flag, any finding exits 5, a warning alone included. Across 8 fixtures and 16 of KiCad's own demo boards and schematics (0 to 578 findings each), the report's finding count and the flag's exit code never disagreed. Each report names its judge: `kicad_version`, plus `included_severities`, which defaults to `["error","warning"]`.

**`cargo kani` exits 1 for three different outcomes.** A failed proof, a crate that does not compile, and a harness name that matches nothing all exit 1; only the printed lines tell them apart. It also writes `Cargo.lock` into the crate it is given, and it fails outright on a read-only one, so a judge must work on a copy.

**Landlock is available but drifts.** Under Docker's default seccomp profile on `ubuntu-latest`, Landlock enforced fully, at ABI V7 on one run and V8 on the next, because the runner kernel changed between runs (6.17 to 7.0). Furnace enforces V8. Renode needs read access to `/proc` and to `/dev/urandom`, and Kani needs `/dev/null`, beyond a read-only system and a read-write temporary directory. **Landlock has no rule for connecting to a pathname Unix socket or for UDP**, at any ABI up to 8. A sandboxed `build.rs` on Furnace reached the user's D-Bus session bus and tmux's socket.

**Children outlive a killed caller.** With no mechanism in place, Renode, and Kani's three-process tree (`cargo-kani` → `kani-driver` → `cbmc`), all survive `SIGTERM` and `SIGKILL` to their parent. A parent-death signal kills only the direct child. A double-forked `setsid` daemon escapes even a process-group kill. What works is a reaper in its own process group, registered as a child subreaper, that kills the tool's group and every remaining descendant (§6).

**Cost.** A cold `docker pull` of the image by digest on a fresh runner takes 55–58 s. A tool's `--version` costs 30–120 ms per call. One real run costs 0.3–0.6 s for DRC or ERC, 2.6–3.3 s for Renode loading the Nucleo, and 0.13–0.3 s for a fixture Kani proof.

### 2.6 Measurements from AET-WO-0300a (2026-10-21, GitHub Actions and Furnace)

The architect re-read each value from the runs' own output and the Furnace before-and-after attack output; the record is `docs/work-orders/AET-WO-0300a.FEEDBACK.md`.

**Landlock leaves real channels open on a host.** On Furnace, under Landlock alone, a sandboxed process could still:
- `connect` to the user's D-Bus session bus and to tmux's socket;
- `sendto` over UDP;
- open `io_uring`.

A seccomp filter installed by the Landlock trampoline refused all four with `EPERM` at `socket()`. It installs unprivileged, under Docker's default profile too, in 25–323 µs.

**`seccompiler`'s rule compiler cannot express an x32-safe default-allow filter.** It matches system-call numbers under `AUDIT_ARCH_X86_64`, which x32 shares. So a rule on `socket` never sees x32's `socket` (`0x40000000|41`). The filter is therefore hand-assembled BPF that tests the x32 bit itself. Neither measured kernel enables x32, so no test could have shown the hole.

**A datagram `socketpair` reaches outside; a stream or seqpacket pair does not.** An `AF_UNIX` `SOCK_DGRAM` pair can be re-`connect`ed, or used to `sendto`, any pathname datagram socket. From inside the sandbox it wrote into the systemd journal. Stream and seqpacket pairs give `EISCONN`. Kani needs stream and seqpacket pairs, and no tool needs any `socket()` family.

**`io_uring` is blocked by Docker's default profile inside the image, but open on Furnace's host kernel.** On a host, the filter is the only barrier to it.

**The GitHub Actions runner leaks two pipe descriptors (142 and 145, without close-on-exec) into every step's shell on the `ubuntu-latest` host.** They are not present behind `docker run`, nor under Furnace's `cargo test`.

## 3. Shape — one representation, two projections, one loop

```
                 specification (declarative)
                           │
                           ▼
                 ┌──────────────────┐
                 │  aethelgard-ir   │  solved graph: pins, nets, domains, timing bounds
                 └───────┬──────────┘
            ┌────────────┴─────────────┐
            ▼                          ▼
 ┌──────────────────┐        ┌───────────────────┐
 │  aethelgard-eda  │        │ aethelgard-svd-hal│
 │ schematic, layout│        │ no_std typestate  │
 │ BOM, fab package │        │ board crate       │
 └────────┬─────────┘        └────────┬──────────┘
          │ parasitics (R, L, C)       │ firmware image
          └─────────────┬─────────────┘
                        ▼
              ┌──────────────────┐
              │  aethelgard-twin │  digital simulator + analog solver, lock-stepped
              └────────┬─────────┘
                       ▼
              ┌──────────────────┐
              │ aethelgard-chaos │  fault scenarios in, trace archive + verdict out
              └────────┬─────────┘
                       ▼
              ┌──────────────────┐
              │ aethelgard-evolve│  attribute, then propose a specification change
              └────────┬─────────┘
                       │ proposal (never a merge)
                       └──────────────► back to the specification
```

Seven parts, in one Rust workspace:

| Part | Owns | Never touches |
|---|---|---|
| `crates/aethelgard-ir` | the specification schema, the solved graph, validation, serialization | external processes, network |
| `crates/aethelgard-eda` | schematic, layout, bill of materials, fabrication package emission | firmware |
| `crates/aethelgard-svd-hal` | vendor register description ingestion, generated board crate | board files |
| `crates/aethelgard-twin` | simulator platform descriptions, analog model, the lock-step bridge | the specification (read-only) |
| `crates/aethelgard-chaos` | fault scenarios, campaign runner, trace archive, verdicts | suppressing a panic or a timeout |
| `crates/aethelgard-evolve` | attribution from traces, mutation proposals | merging anything, bypassing a gate |
| `crates/aethelgard-cli` | the `aethelgard` binary: subcommands over the crates and the external tools, and the `mcp` subcommand that exposes the same surface over stdio | raw credentials in source or logs |

**Seams that are security or blast-radius boundaries**, named because a module boundary and a trust boundary are maintained differently:

- **Tool server → external CLIs** (`kicad-cli`, `renode`, `ngspice`, `cargo kani`) is a **trust boundary**. Arguments are passed as arrays, never shell strings; each process runs in a resource-bounded sandbox; a missing binary is an error (`INV-06`). This is the seam an injected specification would try to pivot through.
- **Evolve → specification** is a **blast-radius boundary**. A proposal is a diff on a branch with its attribution record attached. Nothing in the loop can merge (`INV-05`, northstar `NG-04`).
- **Chaos → trace archive sink** is a **credential boundary**. The local archive needs no credentials; the optional remote archive is referenced by environment-variable name only and its absence halts the upload step, not the run (`INV-06`).
- **IR → every other crate** is a module boundary only. It is the thing `INV-01` protects, but it is enforced by the type system, not by a trust mechanism, and it should be said plainly that it is not a security boundary.

## 4. Ratified rulings

Each carries its reason. Departures from Argon's plan are labelled as departures so they are not later mistaken for drift.

| # | Question | Ruling | Reason |
|---|---|---|---|
| `RUL-01` | Implementation language and layout | One Rust Cargo workspace; the tool server lives in the same workspace | One type system carries the solved graph from solver to simulator to tool server with no marshalling layer in which `INV-01` could drift |
| `RUL-02` | Pin-mux, voltage-domain, and timing resolution | A constraint solver, with SMT via the `z3` crate as the prior. **Spike first** (`AET-WO-1200`): the prior is held loosely, because a pin-mux table may be small enough for a pure-Rust constraint search and the Z3 system dependency is a real cost in the container | Heuristic or model-guessed pin selection is exactly the hallucination `INV-01` and `INV-03` exist to catch |
| `RUL-03` | Board artifact format | Emit KiCad S-expressions (`.kicad_sch`, `.kicad_pcb`) directly; validate with `kicad-cli` passing `--exit-code-violations` and `--format json` | Text format, diffable, headless checks exist. The flag is mandatory because of the exit-code finding in §2.1 |
| `RUL-04` | Firmware hardware interface | A generated `#![no_std]` board crate whose pin, clock, and peripheral states are typestates, built atop the `embassy` ecosystem for the chosen target; any `unsafe` register block is bounded-model-checked. **Spike first** (`AET-WO-1400`): how Kani sees volatile register access decides whether the proof is real or a stub | `INV-02` is enforced by `rustc` at zero runtime cost only if illegal states are unrepresentable; Kani is the only Rust tool that proves absence of a class of UB rather than testing for it |
| `RUL-05` | Digital twin | Renode for instruction-level execution, `ngspice` for the analog nets that matter (bus rise time, rail droop), lock-stepped through a bridge whose transport is spiked (`OQ-02`). **Named as instruction-level, not cycle-accurate** | Renode is the only open simulator with the target family's peripherals modelled; the analog solver supplies the physics Renode idealises away. Claiming more than instruction fidelity would violate the honesty `INV-06` demands of measurements |
| `RUL-06` | Trace format and storage | Cap'n Proto trace schema, **local-first**: the archive of record is on disk, and Cloudflare R2 is an optional mirror. **Departure from Argon's plan**, which streamed to R2 as the primary store | `INV-04` must hold with no network and no credentials; a replay that needs a cloud account is a replay that fails on the day it is needed. R2 remains attractive for cost once volume is measured (`AET-WO-0400`) |
| `RUL-07` | What the evolution loop may do | Propose a specification diff with an attribution record; never apply it to `main`, never alter a gate | `INV-05`, `INV-07`, and northstar `NG-04` together: the loop proposes, the gates judge, a human merges |
| `RUL-08` | First reference target | **Ruled 2026-10-19 (`OQ-01`, delegated to Claude by Scotty):** the STM32H753ZI on the Nucleo-H753ZI board, with the STM32H743 treated as the same target (identical silicon minus the crypto block). RP2350 remains the planned second target, precisely because it is a different vendor; it gets a hand-written platform description, since Renode ships none | §2 and §2.2: the only candidate with a Renode platform (4 I2C, DMA, ADC, EXTI, RCC, flash, 9 UARTs), a board file, a vendor-published physical reference board with a digital twin, an `embassy-stm32` HAL, and a `hilt` harness entry. RP2350 and ESP32-S3 have none of these in Renode. The one caveat, a single SPI instance in the platform file, is cheap to fix in the generated description. A second vendor later is what keeps the IR from becoming one vendor's register map (northstar §6) |
| `RUL-09` | Where gates are enforced | In the chaos runner and the CI job, never in the proposing agent | `INV-07`; a check the proposer performs on itself is not a boundary |
| `RUL-10` | Tool server language and placement | **Ruled 2026-10-19 (`OQ-05`, delegated to Claude by Scotty; revised the same day before merge, see Amendments):** the contract is the `aethelgard` command-line binary, one Rust executable with JSON on stdin and stdout and documented exit codes (`AET-WO-0300`). *Amended 2026-10-21 by `AET-WO-0300`:*
  - Tool subcommands take their inputs as path flags, because board files are not JSON.
  - stdout is still exactly one JSON line.
  - Exit codes `1` (the tool ran and the verdict is fail), `4` (the tool, a variable it needs, or the sandbox is unavailable) and `5` (the tool ran but its output cannot be trusted, never read as pass or fail) extend `0`, `2`, `3`, `64` and `70`.
  - The full contract is `crates/aethelgard-cli/README.md`. The MCP server is the **`aethelgard mcp` subcommand of that same binary**, built on `rmcp`, the official MCP Rust SDK (Apache-2.0, stdio transport, streamable-HTTP and OAuth features available), speaking stdio and binding to nothing (`AET-WO-1500`). The tool list is generated from the command tree, never hand-written. **Departure from the ClearForge fleet standard's TypeScript default**, on Scotty's instruction that language follows the layer | A first version of this ruling chose a TypeScript wrapper because the fleet standard said so. On merits: a second runtime (Node) in the container buys nothing when the engine is already a binary; a wrapper that lists tools by hand is a second copy of the command surface, which is the drift `INV-01` exists to forbid at the design level; `rmcp` is an official SDK, not a framework wrapper, so the standard's real rule (official SDK, never a wrapper) still holds. A local stdio tool is not an exposed node and needs no OAuth. The day it is exposed it becomes a fleet server and gets its own ClearSeal work order; the standard's controls are language-agnostic by its own statement, and `rmcp` carries the auth and HTTP transports that work would need |
| `RUL-11` | License | **Ruled 2026-10-19 (delegated):** every crate, the CLI included, is `MIT OR Apache-2.0`, with both texts at the repository root. **Generated artifacts (board files, board crates, platform descriptions, fabrication packages) carry no license obligation from this project**, stated here the way compiler runtimes state it, so a user's proprietary firmware is unaffected. A hosted campaign service or a curated component-model library, if either is ever built, may carry its own terms; nothing in this repository will depend on them | Matches `embassy`, `svd2rust`, `hilt`, and the Rust norm, so contributors already know the deal; Apache carries a patent grant, MIT carries the fewest questions. The dependency inventory is compatible: KiCad GPL-3 (invoked as a separate process, never linked), Renode MIT, ngspice BSD-3 (linked as a library; its `XSPICE` and `tclspice` parts are not used), Z3 MIT, Kani Apache-2.0 OR MIT, Cap'n Proto MIT. A copyleft license on the engine would have blocked exactly the contributions the northstar §6 says the project dies without: other people's sensor models |
| `RUL-12` | Which language at which layer | **Ruled 2026-10-19 (delegated):** the engine, the CLI, the analog bridge, and the generated firmware are **Rust**. Hooks that run *inside* Renode (fault injection at a peripheral, custom peripheral stubs) are **Python**, because that is Renode's embedded extension language, kept to the smallest snippets the Rust orchestrator can generate and never hand-maintained. Shell is for container build and CI glue only. **C++ is not written** here: KiCad, ngspice, and Z3 are consumed as a subprocess or through `bindgen`, never patched. **Go has no layer** in this project. **TypeScript and React are reserved for a browser viewer** of traces and verdicts, which no phase schedules; if one is built it is a separate package that reads the trace archive and never holds engine logic | Language follows the layer, per Scotty. Rust is forced at the bottom by `INV-02` (the generated board crate is `no_std` Rust and the generator needs Rust AST tooling) and earns the rest by being one toolchain, one binary, and memory-safe while parsing untrusted vendor files. Python inside Renode is a platform constraint, the same kind of exception the fleet standard makes for the S25. Go has no `no_std` story, no Rust-codegen tooling, and `cgo` makes the `libngspice` boundary worse. A browser is the one place TypeScript is the native language, so that is the one place it is used |
| `RUL-13` | Bounds on what the CLI reads | **Ruled 2026-10-19 (AET-WO-0100 F3, F9):** input is capped at 16 MiB and refused unread above it (`InputTooLarge`, exit 3). The CLI sets no timeout of its own: a `--file` naming a pipe nobody writes to blocks, and the caller's timeout ends it | The cap stops a hostile document exhausting memory before parsing. A 120,000-net document is 13.7 MB and validates in 0.17 s, and the reference probe's specification is kilobytes; raise the cap by amendment when a real specification passes a quarter of it. Refusing non-regular files would also refuse `--file /dev/stdin` and process substitution, and every caller in §5 and §6 already runs the binary under a bounded sandbox, so the timeout belongs there |
| `RUL-14` | What a well-formed specification must contain | **Ruled 2026-10-19 (AET-WO-0100 F4, F5); built in `AET-WO-1100`:** every identifier (project name, domain id, net name) is printable ASCII `0x21`–`0x7E` with no whitespace inside it. A specification with no voltage domain is refused. An empty `net_constraints` stays valid | Two ids a human cannot tell apart, such as `VDD_3V3` with a Cyrillic `В`, or one carrying a zero-width space, would let the board and the firmware disagree on which rail is which with neither tool noticing, which is the drift `INV-01` exists to forbid. KiCad's own net names (`+3V3`, `/SDA`, `Net-(U1-Pad3)`) fit the set. A board with no rail cannot be powered, so an empty domain list is absence under `INV-06`. A net constraint is an optional bound, not a required part, so its absence is not |
| `RUL-15` | The tools image: how it is built, published and consumed | **Ruled 2026-10-20 at review of `AET-WO-0200` (PR #7, findings F1, F2, F4, F5):** a Dockerfile `FROM kicad/kicad:10.0.7` pinned by index digest. Every apt package is pinned to an exact version from the base distribution's normal archive, every download carries a sha256 written in the Dockerfile, and the Rust and Kani versions are numbered. The default user is root, with tool state under `/opt`. The image is published to `ghcr.io/clearforge-llc/aethelgard-tools`, tagged with the commit, **only** by the `publish` job on a push to `main`, the one job holding `packages: write`, which pushes the very image it has just smoke-tested under `--network none`. Consumers pin it by digest from a committed `container/IMAGE`, which a pull request updates from the digest in the publish run's summary. CI never writes to the repository. Until that file exists, a consumer builds from the Dockerfile | Nix lacks Kani and the pinned Rust, so it would need a second install mechanism beside it (§2.4). KiCad 10 exists only as the official image. Exact archive pins fail loudly when a version leaves the archive (`INV-06`), and the published digest is what makes a consumer's image reproducible, so a snapshot mirror would add a slow dependency to every cold build to buy what the digest already gives. Root, because GitHub `container:` jobs mount a runner-owned workspace that a non-root default user cannot write. Publishing from `main` alone keeps write permission off every pull request from a fork, whose token GitHub caps at read. A branch pushed to this repository can still edit the workflow and request `packages: write` on its own pull-request run, and branch protection does not stop that, because the run happens before any merge. The boundary is therefore who holds write access to the repository: write access is publish access, which is accepted because the same people can already change what `main` builds. Branch protection on `main` (a pull request plus a passing `gates` check) guards `main`'s history, not the registry |

## 5. Runtime and deployment

Everything runs headless in one pinned container image, `ghcr.io/clearforge-llc/aethelgard-tools`, built from `container/Dockerfile` by `AET-WO-0200` and used identically by a local builder and by CI. How it is built, published and consumed is `RUL-15`. Its smoke test, `container/smoke.sh`, is the definition of "the tools work", and `container/check-pins.sh` refuses a Dockerfile that can drift.

Operational facts worth writing down now, so the first builder does not rediscover them:

- `kicad-cli` must be given `--exit-code-violations`, or a gate passes with violations present (§2.4, measured).
- Renode is run headless (`--disable-gui --console --plain -e "…; quit"`); a run that waits on an interactive monitor looks like a hang. **Its exit code does not report a failed platform load** (§2.4), so every caller asserts on what the loaded machine reports, such as its CPU in `peripherals`, never on the exit code alone.
- `ngspice` runs in batch mode; interactive mode blocks on a prompt that never comes.
- `cargo kani` installs its own backend on first run, so the image pre-installs it with `cargo kani setup --use-local-bundle` and `KANI_HOME=/opt/kani`. Without that, the first offline run fails as a download error, not a proof failure (§2.4, measured).
- The image is amd64-only, because the official KiCad 10 image is. An arm64 build host would need a different KiCad source and revisits `RUL-15`.

## 6. Security posture

The `aethelgard` binary is the only part that executes external programs, and its `mcp` subcommand exposes that surface, so it carries the posture: argument arrays with no shell, a sandbox per invocation, tool-scoped capabilities, and secrets by environment-variable name only (`AETHELGARD_R2_BUCKET`, `AETHELGARD_R2_ACCESS_KEY_ID`, `AETHELGARD_R2_SECRET_ACCESS_KEY`), absent means the remote mirror is skipped with a logged error. Generated firmware is never executed on the host; only inside the simulator. The ClearForge fleet MCP standard (`clearforge-mcp-standard` skill) is the reference posture if this server is ever exposed beyond localhost; see `OQ-05`.

**The sandbox as built (`AET-WO-0300`, rulings on PR #12).** Each tool call goes through these steps:

1. **Copy first.** The CLI copies the input into a private temporary directory and hashes the copy. The tool judges that copy, so the verdict's sha256 is exactly the bytes judged (`INV-03`), and the original is never written.
2. **A guardian.** It is started with the temporary directory and removes it if the CLI dies first.
3. **A reaper.** It runs in its own process group and is registered as a child subreaper. It starts a Landlock trampoline in a second group, which confines itself and becomes the tool. When the tool ends, or the CLI dies by any signal, the reaper kills the tool's group and every remaining descendant.
4. **An empty environment,** apart from `HOME` and `TMPDIR`, which point at the temporary directory. Kani alone also gets `KANI_HOME`, `RUSTUP_HOME` and `PATH`, passed through by name. stdin is `/dev/null`.
5. **Landlock's filesystem rights through ABI V3 are required.** Below that floor the call refuses with exit 4, `SandboxUnavailable`, and there is no flag or variable that runs a tool unconfined. TCP rules (ABI V4 and later) and signal and abstract-socket scoping (ABI V6 and later) are applied where the kernel has them, and reported in the verdict as `sandbox.tcp_confined` and `sandbox.scoped`.

**Socket gap closed by `AET-WO-0300a` (§2.6).**

Two more steps now run in the trampoline before the tool starts:

1. **Before Landlock, the descriptor check.** It refuses the call when the tool would inherit any descriptor above 2 without close-on-exec, with exit 4, `SandboxUnavailable`, `id` `descriptors`. A leaked connected socket is a channel no filter sees, so callers must not leak. Our own CI closes the runner's leaked descriptors in the step's shell.
2. **After Landlock, a required seccomp filter.** It applies to the tool and to everything the tool starts. It does three things:
   - kills any non-x86_64 or x32 system call;
   - refuses `socket` for every family, and `socketpair` other than an `AF_UNIX` stream or seqpacket pair;
   - refuses `io_uring`.

   If the filter cannot be installed, the call refuses with `SandboxUnavailable`, `id` `seccomp`. There is no escape hatch.

**Still open on a host:** non-socket channels to other processes with the same uid, namely System V shared memory, `unshare(CLONE_NEWUSER)` and `pidfd_open`. The filter is default-allow by design (§4 of the work order). Inside the published image, Docker gives the tool its own IPC namespace and refuses unprivileged `unshare`, so **the image is the supported boundary for tool subcommands.** A host-side system-call allowlist is `AET-WO-1600`, which must land before the P1 launch clause puts a crates.io-installed binary on hosts.

**A rule for whoever builds the `mcp` subcommand.** The internal first arguments that start the reaper, the trampoline and the guardian execute the argument vector they are given. They are safe only because they are not in the command tree. The `mcp` subcommand must therefore map every tool to a `clap` subcommand and never forward raw arguments.

## 7. Traceability

None. This project has no governing standard. DO-178C and ISO 26262 appear in the northstar's beneficiary list as standards *users* may answer to; Aethelgard does not claim conformance to either. Recorded explicitly so a reader knows the question was asked.

## 8. Open questions

Each names a decider and what it blocks. None is a placeholder.

- **`OQ-01` Which microcontroller anchors the reference probe?** **Ruled 2026-10-19, see `RUL-08`.** Scotty delegated the ruling to Claude; the evidence is in §2.2. No longer blocks anything.
- **`OQ-02` How do the digital simulator and the analog solver exchange state in lock-step?** Candidates: a shared-memory ring buffer or a Unix domain socket, as Argon listed; a third is driving `ngspice` as a library through its shared-library interface rather than as a process. **Prior since 2026-10-19: the library route** (§2.2), held loosely. Decider: the spike in `AET-WO-2100`, ratified by Scotty. Blocks P2's work orders beyond the spike.
- **`OQ-03` What is the trace volume of one fault campaign, and does full-fidelity recording survive it?** Decider: measured by `AET-WO-0400`, ratified by Scotty if sampling is ever proposed (which touches `INV-04`). Blocks the P2 exit gate.
- **`OQ-04` Can Kani prove anything useful about volatile register access, or only about stubs?** **Narrowed 2026-10-19** (§2.2): only over a harness-owned register image, never over the real base address. The spike in `AET-WO-1400` now has to show that such a proof catches a seeded typestate bug, which is the useful question. Decider unchanged.
- **`OQ-05` Tool server language: Rust via `rmcp` in the workspace, or TypeScript per the ClearForge fleet standard?** **Ruled 2026-10-19, see `RUL-10` and `RUL-12`.** Rust, as an `mcp` subcommand of the engine binary on the official Rust SDK. No longer blocks anything.
- **`OQ-06` Who proposes mutations in the evolution loop: deterministic rules, a language model, or both in sequence?** Decider: Scotty and Argon, informed by P3's spike. Blocks `AET-WO-3200`. Northstar `NG-04` fixes that the proposer never judges; it does not fix who proposes.

## 9. What this document does not cover

Work-order-level implementation detail, and sequence. Sequence lives in `roadmap.md`; this document constrains the order without setting it. Argon's crate-level type sketches (`TopologyGraph`, `VoltageDomain`, `NetConstraint`) are preserved in `docs/inputs/argon-session/` and are input to `AET-WO-0100`, not rulings.

## 10. Amendments

| Date | Section | Change | Rationale |
|---|---|---|---|
| 2026-10-14 | — | Initial draft | Seed. `RUL-06` local-first and `RUL-07` through `RUL-09` are departures from or additions to Argon's draft, each with its reason in §4 |
| 2026-10-19 | status, §2.2, §4, §8 | Ratified by merge of PR #1. Added §2.2 measurements. Ruled `OQ-01` (`RUL-08`) and `OQ-05` (`RUL-10`), added `RUL-11` license, narrowed `OQ-04`, set a prior on `OQ-02` | Scotty delegated `OQ-01` and `OQ-05` to Claude and chose open source on 2026-10-19. Renode's platform file and changelog, read in full, replaced the "unverified" absence of RP2350 and ESP32 with a verified one. The ClearForge MCP standard leaves no preference-based room for a Rust server |
| 2026-10-19 | §3, §4, §6 | `RUL-10` revised before merge from a TypeScript wrapper to an `mcp` subcommand on `rmcp`; `RUL-12` (language per layer) added; the tool-server row in §3 became the `aethelgard-cli` crate | Scotty withdrew the fleet standard's TypeScript default for this project: language follows the layer. The first ruling had followed the standard rather than the problem, and said so; this row records the correction rather than hiding it |
| 2026-10-19 | §2.3, §4 | Added §2.3 measurements from `AET-WO-0100` and the serde positional-array correction; added `RUL-13` (CLI input bounds) and `RUL-14` (identifier set and minimum content, to build in `AET-WO-1100`) | Rulings on the work order's decision-needed findings F3, F4, F5 and F9, made at review of PR #4. The build found what the plan could not, and this is where it lands |
| 2026-10-20 | §2, §2.4, §4, §5 | Added §2.4 measurements from `AET-WO-0200`. Marked §2's KiCad exit-code question settled. Added `RUL-15` (the tools image). Rewrote §5's opening, its Renode and Kani bullets, and the amd64-only constraint from measurement | Rulings made at review of PR #7. The spike showed the DRC flag is load-bearing. The build showed a failed Renode load exits 0, which neither the plan nor the review's suggested log strings anticipated |
| 2026-10-20 | §4 | `RUL-15`'s last reason corrected: branch protection does not stop a same-repository branch's pull-request run from requesting `packages: write`; write access to the repository is the boundary | The first wording, taken from the builder's B7, named the wrong guard. The run that could publish happens before any merge, so a rule on `main` cannot reach it |
| 2026-10-21 | §2.5, §4, §6 | Added §2.5 measurements from `AET-WO-0300`. Amended `RUL-10`: path flags, exit codes 1, 4 and 5. Added the sandbox as built to §6, with its socket gap and the `mcp` argv rule | Rulings at review of PR #12. The builder's two-process-group reaper replaced the one-group design the spike ruling specified, which leaked a temporary directory on every kill. Its adversarial pass found that Landlock leaves pathname Unix sockets open, which became `AET-WO-0300a` |
| 2026-10-21 | §2.6, §6 | Added §2.6 measurements from `AET-WO-0300a`. §6's socket gap marked closed, with the seccomp filter, the descriptor refusal, the non-socket channels still open on a host, and the image as the supported boundary | Rulings at review of PR #15. The architect's spike ruling that `socketpair` "reaches nothing outside" was wrong for datagram pairs, and the adversarial pass proved it by writing to the systemd journal from inside the sandbox |

New sections continue the sequence and never reuse a number.

## 11. Provenance

Written 2026-10-14 by Claude Code in session `claude/argon-repo-docs-synthesis-rjrb0d`. Rests on `northstar.md`, the founding Google Doc and the five Argon session captures under `docs/inputs/argon-session/`, and the measurements in §2 and §2.2, taken in the Claude Code cloud container on 2026-10-14 and 2026-10-19. Sources read for §2: Renode's supported-boards and time-framework documentation, Antmicro's 2026-07-16 STM32H7 reference platform post, the KiCad 9.0 CLI reference, the Kani paper and the Kani book's feature-support page, Renode's `stm32h7.repl`, `nucleo_h753zi.repl`, and `CHANGELOG.rst` from the googlesource mirror at 1.17.0, the `hilt` and `paprika` crate pages on docs.rs, and the `clearforge-mcp-standard` skill. §2.3 rests on `docs/work-orders/AET-WO-0100.FEEDBACK.md` and the architect's review of PR #4.
