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
| The vendor register description gives `AET-WO-1100` the microcontroller package and its alternate-function table (`roadmap.md` §5, from Argon's plan) | A CMSIS-SVD file describes peripherals and their registers. Its schema has no element for a package, a pin or an alternate function. Argon found this at its 2026-10-29 review, and the format agrees. The pin and alternate-function tables come from ST's own MCU database, which the embassy project's `stm32-data` already turns into JSON. *Resolved 2026-11-07 (§2.8):* the pin data is taken from ST's own `STM32_open_pin_data`, BSD-3-Clause, rather than from `stm32-data`'s generated JSON |
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

### 2.7 Read from Renode's source (2026-10-29, the architect)

Read, not run. The file is `src/Emulator/Peripherals/Peripherals/I2C/STM32F7_I2C.cs` in `renode-infrastructure` at `066a7f13`, the commit that Renode v1.17.0's `src/Infrastructure` submodule pins (`git ls-tree v1.17.0 src/Infrastructure`). It was compared with that repository's `master` on the same day.

**The model is transaction-level.** `STM32F7_I2C` is a `SimpleContainer<II2CPeripheral>`. A transfer calls the addressed child's `Write(byte[])` or `Read(count)` once; nothing toggles a pin, and nothing has bit timing.

**At 1.17.0 it models no I2C error at all.** In the status register, `NACKF` (bit 4), `BERR` (8), `ARLO` (9), `OVR` (10) and `TIMEOUT` (12) are `WithTag` placeholders, which read as 0 and have no storage. The interrupt logic carries `//TODO: implement NACKF`. A transfer to an address with no device logs a warning and returns, setting no flag. `ErrorInterrupt` is created and reset, and never set.

**On `master`, after 1.17.0, `NACKF` is implemented.** An absent device sets it with `STOPF`, and it can raise the event interrupt. `BERR`, `ARLO`, `OVR` and `TIMEOUT` are still placeholders, and `ErrorInterrupt` is still never set.

**Consequence.** A bridge cannot surface an analog fault by setting a status flag in the stock model, because the flags it would set do not exist. `OQ-02` now has to choose how a fault reaches the firmware (§8).

### 2.8 Where the pin table comes from (2026-11-07, the architect)

Read, not run.

**`stm32-data`** (embassy-rs) is MIT OR Apache-2.0 as code. Its generated JSON lives in `stm32-data-generated`, which carries no licence file. The data comes from ST's CubeMX database.

**ST's own `STM32_open_pin_data`** is BSD-3-Clause, copyright STMicroelectronics. Its README calls it a subset of the STM32CubeMX database, published for tools that do not use CubeMX. At commit `7d1f1514` (2026-06-29) it holds:
- `mcu/STM32H753ZITx.xml` (sha256 `d167c088…`): the LQFP144 package, 144 pins, 114 of them I/O. It names its GPIO block `STM32H747_gpio_v1_0`.
- `mcu/IP/GPIO-STM32H747_gpio_v1_0_Modes.xml` (sha256 `49f51ae2…`): each pin's signals with their alternate-function numbers. For example, PB8 carries `I2C1_SCL` as `GPIO_AF4_I2C1`.
- A board configuration for the Nucleo-H753ZI (`boards/…NUCLEO-H753ZI…ioc`).

**Ruling.** `AET-WO-1101` vendors those two XML files, pinned by that commit and their hashes, together with ST's licence text, and parses them. The primary source with a clear licence wins over a derivative without one. BSD-3-Clause is compatible with `RUL-11`: redistributing the files requires keeping ST's notice, and the engine's own licence is unchanged.

### 2.9 Measurements from AET-WO-0300b (2026-11-07, GitHub Actions and Furnace)

The architect re-read each value from the `tools` run's `ANSWER 0300b-1.1` lines (run 42406190783); the record is `docs/work-orders/AET-WO-0300b.FEEDBACK.md`.

**Renode 1.17.0 never reports a platform failure at its `[ERROR]` log level.** Clean loads of the Nucleo-H753ZI board and of the STM32H753 print no `[ERROR]` or `[WARNING]` line.

Eight broken platforms were tried, among them an unknown peripheral type, an unknown property, a duplicate registration, overlapping or misaligned memory, an unregistered interrupt target, a bad CPU type, and a failing `init:` command. Each reported itself only through the monitor's "There was an error executing command" line, which `simulate` already reads as a failed load or a `ToolError`.

One case reports nothing at all: two overlapping tags in `init:` list a CPU and print nothing. The `[ERROR]` check `AET-WO-0300b` added is therefore a fail-closed backstop that no real input has yet reached. Argon's RT-02 described a path that Renode, as measured, does not take.

### 2.10 Measurements from AET-WO-0400 (2026-11-07, Furnace and GitHub Actions)

The record is `docs/work-orders/AET-WO-0400.FEEDBACK.md`. These are plumbing numbers for synthetic events, not `OQ-03`.

**Determinism across machines.** The golden 10,000-event trace was computed on Furnace: 191,873 bytes, sha256 `51c6193f…896d`, `state_hash` `535db225…adac4`. CI's `gates` job reproduced both unchanged. Encoded bytes per event, by kind:
- register write: 17.0;
- pin level: 13.1;
- interrupt: 9.7;
- analog sample: 22.3;
- fault injected: 33.8.

The mean is 19.2 bytes.

**Cost.** Writing and committing the stream, every `sync_all` included, takes 0.213 s in release (46,888 events/s) and 1.334 s in debug. Replay takes 0.195 s in release.

**What the adversarial pass measured:**
- **Integrity holds, as intended.** 3 million mutation-fuzz iterations found no input the decoder accepts that fails to round-trip. A forged trace with a recomputed manifest replays, because the manifest proves integrity, not authenticity.
- **Replay as built is quadratic** in distinct state keys: 6.5 s for 20,000 distinct registers.
- **Memory is not yet bounded by shape:** one crafted 16.8 MB event reached a peak of 578 MB.

Both are fixed by `AET-WO-0400a`, before any trace from outside is replayed.

### 2.11 Measurements from AET-WO-1200 and the AET-WO-1400 spike (2026-11-08, Furnace and GitHub Actions)

**The pin search (`RUL-02`, as measured).** The pure-Rust search meets the 1 s bound by about four orders of magnitude, so Z3 was not measured.
- Plain forward checking is exponential on infeasible input: one case took 329 s.
- Instance matching plus pin matching before every node makes every case found sub-millisecond. On Furnace, `aethelgard solve` takes 2.4 to 3.0 ms end to end, release, the median of 21 runs. The reference specification takes 7 nodes and the stress specification 42.
- **A 10 M-node budget is the backstop** (`BudgetExceeded`, naming a bus).
- **The gap `AET-WO-1201` closed (PR #48).** Some infeasible specifications used to spend the whole budget, in 5.6 to 6.6 s release, and name the first bus of the colliding set. It happened when two instances' only remaining pin for one signal was the same pin, because pin matching unions over instances.
  - **The fix:** a third check. When a kind's unplaced buses equal the instances they can still take, every instance is forced, and its (instance, signal) rows must match to distinct free pins.
  - **The result:** both reproducers now refuse `Infeasible` in 198 and 186 nodes, about 4 ms. A differential test over 3,000 fixed-seed specifications, and a one-off 450,000, found no answer changed except `BudgetExceeded` becoming `Infeasible`.

**How the solver uses the pin table (§2.8).**
- Reserved pins: `PA13` and `PA14` (SWD; debugging is assumed SWD only), and `PC14`, `PC15`, `PH0` and `PH1` (the oscillators).
- The table's key order is byte order (`PA0, PA1, PA10, …`), and it decides which free pin a chip select takes.

**Kani over a register image (`OQ-04`), Kani 0.68.0 and CBMC 6.11.0.** The probe is one GPIO pin pair taken to AF4 and one I2C peripheral behind its RCC enable bit, all through a `Registers` trait. On Furnace, through `aethelgard prove`:

| Board | Verdict | Time |
|---|---|---|
| correct | pass | 46 s |
| wrong AF number | fail, rule named | 40 s |
| missing clock enable | fail, rule named | 50 s |
| write past the block | fail, rule named | 0.5 s |

Peak RSS was 2.3 GB.
- Non-vacuity has three witnesses: the cover point is satisfied, each seed fails only on its own assertion, and an under-unwound loop fails rather than hides.
- In edition 2024, `assert!` messages reach Kani's report only as a placeholder, so the harness uses `kani::assert`.
- Comparing a `[u32; 256]` with `==` is a 1,024-iteration `memcmp` under CBMC, so the harness compares register images word by word.

**Where the board crate compiles.** On the host, so trybuild and Kani run in `gates` and `tools`. It also compiles for `thumbv7em-none-eabihf` in an isolated toolchain. Adding that target to the image is one `rustup-init` flag, about 72 MB installed and about 1 s. It is deferred to P2.
- trybuild snapshots survived a different `rustc` (1.100 nightly) unchanged.
- `cargo fmt` on the crate under test broke 4 of 6 snapshots, so they are a function of the generated text, which clause 4 pins.

### 2.12 Measurements from the AET-WO-1300 spike (2026-11-08, GitHub Actions, the tools image)

These were measured with `kicad-cli` 10.0.7 and the image's libraries, in four `tools` runs whose ANSWER lines are committed with the work order.

- **Libraries.** Every part of the reference probe has a symbol and a footprint except the W25Q64JV. It uses `Memory_Flash:W25Q32JVSS` (same SOIC-8 pinout) with Value `W25Q64JVSS`. The libraries are CC-BY-SA 4.0 with a design exception, and the image ships no copy of that licence.
- **ERC-clean emission.** The emitter must add:
  - a no-connect on every unused pin;
  - a `PWR_FLAG` per rail;
  - a net per `power_out` pin;
  - a resistor between a bidirectional pin and a flagged rail.

  UUIDs derived from content paths survive KiCad, and emission is byte-identical across runs and runners. ERC also needs a library configuration, which a copied schematic alone lacks.
- **The gate's rules (`RUL-03`).**
  - **The `.kicad_dru` step is load-bearing** for net-class and rule-level loosening.
  - **The setup block's `clearance_min` sets the board minimum** and beats the project's.
  - **A local pad or footprint clearance is judged only against that minimum.**
  - **Before the build, `check drc` judged by KiCad's defaults**, because it copied only the board.
- **Placement and routing.**
  - `kicad-cli` 10.0.7 has no Specctra DSN or SES. The image has `pcbnew` Python, which `RUL-12` forbids, and no Java.
  - DRC refills zones only with `--refill-zones`, and saved fills are not byte-deterministic.
  - A two-layer board placed and routed on a grid from Rust passes `pcb drc --schematic-parity` with exit 0, nothing reported, and 0 unconnected with vias.
  - Parity matches footprints by reference. It requires the library id, the value, every symbol field and KiCad's net names, with root-sheet labels prefixed `/`.
- **Escaping.** S-expression quoting (`\"`, `\\`) is sufficient for every character `RUL-14` admits. `/`, `{`, `}`, `$`, `[` and `]` are not: KiCad reinterprets them, hence `RUL-14`'s narrowing.

### 2.13 Measurements from the AET-WO-1300 and AET-WO-1400 builds (2026-11-08, Furnace and GitHub Actions)

**The schematic emitter (`AET-WO-1300`, PR #40).**
- **Output.** `aethelgard emit` writes the reference probe's `.kicad_sch` and BOM byte-identically across processes, environments and runners. ERC passes on it in the tools image.
- **The board minimum.** The project step is load-bearing once the gate's setup block carries no clearance token: the board's own project gave 0 violations, the gate's gave 1.
- **The critical finding (A1).** The gate's S-expression reader disagreed with KiCad's in three lexical ways, and together they let a board pass with a 0.05 mm violation under `"rules": "specification"`.
- **Two inputs the emitter must not trust:**
  - ST's pin table and KiCad's symbol name package pins 28 and 29 differently (`PC2`/`PC2_C`), so the emitter draws by package position, never by name.
  - A reference starting with `#` is a virtual symbol to KiCad, so `emit` refuses it.

**The board crate (`AET-WO-1400`, PR #42).**
- **The P1 gate clauses.**
  - **Clause 3:** 11 trybuild compile-fail cases and one valid driver, offline in `tools`.
  - **Clause 4:** the whole committed `boards/sentinel-01` regenerates byte for byte, and a hand-edited harness fails the comparison.
  - **Clause 5:** `prove` passes the generated block and fails each seeded bug, at 3.5 GB peak per proof.
- **The register facts** come from ST's `cmsis-device-h7` at `8f922cd` (v1.10.7, Apache-2.0), hash-checked.
- **A vacuous proof (E1).** A harness whose `kani::assume` cuts every path exits 0 with `** 0 of 1 cover properties satisfied`, and `prove` used to say `pass`. **`prove` now fails any exit 0 whose cover summary is short (`cover_unsatisfied`),** and the generated harness covers each branch: nine covers.
- **What else the adversarial pass reached (E2–E8):** a foreign `Registers` impl, raw register access, a swapped `TIMINGR`, stray bits in `CR1`, `CFG2` and `BSRR`, a transient MODER glitch, and a second board. Each is now refused or checked.

### 2.14 Measurements from the AET-WO-1302 build (2026-11-08, Furnace and GitHub Actions)

**The router (`AET-WO-1302`, PR #60), and P1 gate clause 1.**
- **The clause holds** as one `tools` test. From `examples/sentinel-01-minimal.json`, `emit`, then `check erc`, then `check drc --spec --schematic` all exit 0 with no finding of any kind, unconnected items included. The board of 1300's S5 names passes the same way.
- **The router** is a deterministic, integer-only, two-layer grid router in `aethelgard-eda`:
  - a 0.125 mm grid, which divides the LQFP's 0.5 mm pitch;
  - A* with via and bend costs;
  - ground stitched first into an unfilled GND zone on B.Cu, then the nets in ascending pad-count order;
  - at most 500 rip-ups.

  Before it returns, its own clearance and connectivity judges run. A board it cannot complete is refused `Unroutable`, naming the net.
- **The reference board:** 348.5 mm of track in 360 segments, 70 vias and 37 rip-ups, routed in 1.0 s in a release build.
- **Placement fed routing.** At 0.5 mm, two IC pads faced each other across a channel one track wide. A 1 mm gap between ICs only fixed it, and every 1301 placement test still passes.
- **Routability is data, not a promise.** Some congested specifications fail closed, naming the net (B10). That is the safe direction.
- **What the gate does not judge (A2, A3).** KiCad gave exit 0 to copper with false net labels, and to a floating loop on a net no pad carries. The geometry was right and only the labels lied. Hence the `RUL-03` amendment: consumers derive nets from copper.

### 2.15 Measurements from the AET-WO-2100 build (2026-11-08, Furnace and GitHub Actions)

**The twin boots (`AET-WO-2100`, PR #56).**
- **The platform.** `aethelgard-twin` emits the `.repl` from a `SolvedTopology`. It includes Renode's `stm32h753.repl` and adds only the SPI instances Renode lacks, each with its base address and interrupt from the ST header. Renode lists every solved peripheral.
- **The firmware.** It is generated, dependency-free and built offline in the image in 0.05 s.
- **The boot.** Each boot reaches the fixed point, and two boots give identical observations. 10 ms of virtual time executes exactly 1,000,000 instructions.
- **Renode's RCC resets with every clock already on,** so the boot writes the gating registers cold first. Renode then refuses a write to a locked peripheral.
- **What the adversary reached, and what closed it:**
  - **The magic loaded rather than written:** in a loadable section, an extra segment, or `.data`. Closed by `FirmwareLoadsBootWord`.
  - **No vector table at the boot address.** Closed by setting `VectorTableOffset` and by `pc_outside_firmware`.
  - **Two ELFs with one identity.** Closed by hashing each segment's addresses and sizes into `image_sha256`.
  - **A jump into a hole,** which hung the call while Renode grew at about 55 MB/s. Closed by B1, the bounds in `RUL-13`.
- **The residual (B2).** The boot word witnesses that the solved configuration happened and that the core kept executing in the firmware's own code. It does not witness that `main` returned.

### 2.16 Read at source for `OQ-02` (2026-11-08, the architect's desk spike)

Nothing was run. The pins are Renode `v1.17.0` (`ab721d88`, infrastructure `066a7f13`) and ngspice `ngspice-44.2` (`80d22fc8`). Every claim is cited in `docs/work-orders/AET-WO-2101.desk.md`.
- **ngspice steps on the caller's thread.**
  - `ngSpice_Init_Sync` takes three callbacks:
    - `GetVSRCData` supplies a `V… external` source's value;
    - `GetISRCData` does the same for a current source;
    - `GetSyncData` fires on each accepted or rejected step. It can rewrite the next delta, or redo the step by returning 1.
  - A plain `run` blocks the caller, and the callbacks fire on that same thread.
  - Debian's 44.2 is built `--with-ngshared`.
  - The manual's 5-parameter `GetSyncData` is stale; the header's has 7. So the lead in §8 holds, through inversion of control. The FFI still needs `unsafe`.
- **Renode offers no external I2C hook.** External Control (TCP, a C client) carries ADC, GPIO, SPI, CAN and bus access. Co-simulation (TCP, or an in-process native library) is bus-mapped. Neither carries I2C, and the sandbox refuses sockets anyway.
- **What does exist inside Renode:**
  - `Mocks.DummyI2CSlave`, with Python handlers on `ReadRequested`, `DataReceived` and `TransmissionFinished`. Upstream uses this pattern in a test.
  - C# compiled by Roslyn when it is `include`d.
  - A stock `BME280` I2C model. TMP117 and TMP102 have none, and TMP108 is a stub.
- **`STM32F7_I2C` never signals NACK.** An absent device leaves the transfer incomplete: TXIS is never set, so the write phase never starts (`AET-WO-2101` F4). A short read completes early. An empty receive queue reads 0. So a bus fault reaches the firmware only as a timeout, a short transfer or wrong data. The trace must record the measured rise time separately from what the firmware saw (§8).

### 2.17 Measurements from the AET-WO-2300 build (2026-11-09, Furnace and GitHub Actions)

**The campaign, the archive and replay (`AET-WO-2300`, PR #74), measured on the tree merged with `AET-WO-2203`.** The record is `docs/work-orders/AET-WO-2300.FEEDBACK.md`.
- **Both reference scenarios match their declared expectation.** `pullup-2k2` passes. `pullup-47k` fails for exactly `analog_rise_exceeds_ceiling` and `i2c_timeout`. Its archive holds the rise (3,982,301 ps on both lines, against a 300,000 ps ceiling), the bus fault and U2's TXIS timeout as three separate events, in that order.
- **Replay needs only the run directory.** Two `replay` calls print identical bytes, and the recomputed state-sequence hash equals the committed one. A flipped byte is `HashMismatch`. A flipped value with the manifest re-sealed is `ReplayDiverged`. Two campaigns give identical run ids, hashes and traces.
- **Trace bytes (`OQ-03`, static campaigns).** Each scenario boots for a fixed 100 ms of virtual time.

  | Run | Events | `trace.cbor` | Trace bytes per simulated second |
  |---|---|---|---|
  | `pullup-2k2` | 15 | 907 B | 9,070 B/s |
  | `pullup-47k` | 16 | 1,335 B | 13,350 B/s |
  | The campaign (2 runs, 0.2 s simulated) | 31 | 2,242 B | 11,210 B/s |

  These bytes do not grow with virtual time, because a boot's records are read once, at its end, so the per-second figure is a ratio, not a rate. Full-fidelity recording of a static campaign costs about 1 KB per scenario. Nothing is sampled.
- **The dynamic case is estimated, not measured.** A per-quantum log at 10 µs quanta would be about 100,000 events per simulated second, at 30 to 60 B each: 3 to 6 MB/s. The reader's 256 MiB bound would then hold 45 to 90 s of simulated time per run. `AET-WO-2301` measures it.
- **`aethelgard-chaos` no longer depends on `aethelgard-ir`.** Applying a scenario's overrides to a `Specification` lives in the CLI's runner, so the chaos crate never names a `Specification` (`RUL-16`).

### 2.18 Measurements from the AET-WO-3100 build (2026-11-09, Furnace and GitHub Actions)

**Attribution from the archive (`AET-WO-3100`, PR #79).** The record is `docs/work-orders/AET-WO-3100.FEEDBACK.md`.
- **Measured rises on the reference's I2C bus,** 30 to 70 %, against the 300 ns ceiling:

  | Case | Rise |
  |---|---|
  | 2.2 kΩ | 186.40 ns |
  | 3.3 kΩ | 279.60 ns |
  | 3.9 kΩ | 330.44 ns |
  | 4.7 kΩ | 398.23 ns |
  | 10 kΩ | 847.30 ns |
  | 47 kΩ | 3,982.30 ns |
  | 100 kΩ | 8,472.98 ns |
  | 220 pF at 2.2 kΩ | 410.09 ns |
  | 470 pF on SDA at 2.2 kΩ | 876.11 ns |

  The rise is linear in R × C to within the bridge's resolution. 3.3 kΩ passes with 7 % margin and 3.9 kΩ fails by 10 %.
- **The confident wrong answer is real and measured.** Renode's controller leaves `CR2` holding the write phase and `TXDR` at 0 on any TXIS timeout, including the unmutated 47 kΩ run. Read without the transfer phase, every timeout looks like a firmware defect. Attribution counts `SADD` whenever `CR2` was written, `NBYTES` once the read phase began, and `TXDR` once the write phase ran (F2).
- **Trace bytes with kind 15** (`OQ-03`, static): 993 B (2.2 kΩ) and 1,421 B (47 kΩ) per run at 100 ms, which is 86 B per run more than §2.17.
- **`simulate --firmware` is unchanged by the trace format change:** byte-identical to `main` over four runs.

### 2.19 Correction, 2026-11-12: what the analog records are

The Mythos 5.2 review (`docs/reviews/2026-11-12-mythos-5-2.md`, A1 and A2) reproduced what §2.15 to §2.18 did not say:
- **Every rise time in §2.17 and §2.18 is computed from declared values.** The netlist is built from the pull-up and the net constraint's `max_capacitance`, a bound used as a value. A 47 kΩ pull-up with a declared 5 pF, and 10 MΩ with 1 fF, both pass the campaign.
- **Nothing bounded the pull-up from below.** 24 Ω, 50 Ω and 100 Ω passed, though 24 Ω asks the driver to sink 137 mA against UM10204's 3 mA. `AET-WO-1203` adds the floor to `solve`.
- The words "measured" in §2.15 to §2.18 and in the record kind `analog.rise_time_measured` are kept as written, beside this correction, per this section's habit. They mean "computed by ngspice from the declared R and C". The record kinds are renamed with `AET-WO-2301`'s trace version bump (`RUL-19`).

### 2.20 Measurements from AET-WO-2204 (2026-11-14, Furnace and GitHub Actions)

The boot judge was rebuilt to judge what Renode records, not what the firmware writes (PR #111, `79bdfe1`).
- **No `I2C1` register witnesses a transfer.** Renode 1.17.0's `STM32F7_I2C` leaves a real read's controller at its reset state plus `CR2` and `TXDR`, both writable. The firmware consumes `RXNE` and `RXDR`, and the board's `ICR` write clears `STOPF`. A firmware writing every register by hand read identically and passed (review A11.1).
- **The witness is the device model's own log.** At `Noisy`, `BME280.cs` logs `Write D0` and `Read 60` only when the controller calls it. No store produces these lines, and every forgery measured lacked them.
  - They are matched as whole lines after a validated host timestamp, and per device in the records' order.
  - A model with no known line format is a named failure (`i2c_witness_unavailable`).
  - A log past the 8 MiB budget leaves every read unwitnessed.
- **Renode's console interleaves below line level when logging is asynchronous.** Under a 62 MB flood, the `BEGIN` marker was scattered across log lines in 8 of 10 runs, and values split in 7 of 10. `[general] use-synchronous-logging = true` removed this in 10 of 10 runs, at +9 % wall time on that flood and none measurable on the reference.
  - Every Renode the CLI starts now gets `--config`, the droop's boot through `bridge --renode` included.
  - The file states all 19 keys Renode reads. Renode rewrites a configuration file that lacks a key, and the CLI's files are sealed read-only for the tool's uid (`AET-WO-1601`), so a one-key file aborted every boot.
- **The core parked.** `ICSR.VECTACTIVE` must be 0 (thread mode), and the final `PC` must be on a `yield; b.n` self-loop. The loop is read from the ELF's executable segments at their physical address (virtual must equal physical) and read back from the machine after the run. The ELF that `aethelgard build` hands `verify` has no symbols, so "the generator's fixed point" is not knowable from it without a generator change; the judge does not claim it.
- **Reserved names.** The pinned platform chain declares 97 instances (68 of them all-lowercase), two builtins and 13 keywords. A reference whose lowercased name collides with one redeclares it, and Renode refuses the load. `aethelgard_ir::RESERVED_REFERENCES` (81 names) is refused by `validate`.
- **The trace is at version 5.** Kind 13 is one record per identity read, carrying the device's expected and observed lines. Kind 19's probe is witnessed the same way. Version-4 archives are refused by name.

### 2.21 Measurements from AET-WO-4100 and the AET-WO-4300 spike (2026-11-14, Furnace and GitHub Actions)

**Stackups** (`4100`, PR #120, `d00c965`):
- **KiCad 10.0.7's layer table at 4 layers is kept as written.** Inner copper takes ids 4 and 6 (`In1.Cu`, `In2.Cu`). A board needs no `(stackup …)` block, and neither DRC nor the Gerbers read one. The gate discards the board's `(setup …)` anyway, so the stackup lives only in the specification and `FAB-NOTES.md`.
- **DRC reports a seeded inner-layer clearance violation and an inner short.** Inner Gerbers carry the two clock lines at lines 2 and 9, the places `RUL-03`'s deletion rule names; five exports are byte-identical once stripped. Under the gate's rules, a plane keeps the declared clearance (0.2005 mm measured) from a foreign via. The 0.5005 mm first measured came from an unprepared copy.
- **Extraction's bound over a near plane.** The trial gains an elliptical cap below a piece whose next copper layer lies at a depth `h` smaller than the clearance `S`: `lower(A, p, h) = A/h + πph/(8S) + πpS/(8h) + 4πh/9 + 2πS²/(9h)`.
  - It equals today's cap at `h = S`, and the integer path is unchanged whenever `h ≥ S`.
  - It was checked against a quadrature of the trial, and it lies above the exact stripline (Cohn).
  - On OSH Park's 0.179 mm prepreg, the 4-layer fixture's I2C lines bound at 45.9 and 48.8 pF, against the declared 100 pF.
- **Routing.** Four layers (S-P-P-S) route the 1302 reviewer's 24-device board `D1`, which fails closed at 2 layers. The `D3` board, with 12 SPI flashes added, fails at 4 layers on the MCU's adjacent supply and ground pads; that needs escape planning. Six layers measured no better than four.

**Flight I/O, read and measured** (the `4300` spike, PR #123):
- **Timers.** `STM32_Timer` drives a channel's pin through the AF route.
  - **The model's conventions:**
    - the period is `ARR` ticks, where silicon counts `ARR + 1`;
    - the first edge comes one period late;
    - `MOE` and `CCxP` are tags and do not gate or invert the output;
    - the clock is the platform's fixed `frequency` (250 MHz), not the firmware's clock tree.
  - **Two clocks in the log.** With virtual timestamps, a line a timer event causes carries its exact time, in 10 ns steps; a line the CPU causes carries the start of its 100 µs quantum. So a store can drive a pin, but only on the grid.
  - **DShot.** DShot600's bit timing is resolvable. DMA cannot drive a timer, because the DMA enables are tags and `DMAR` is undefined, so a DShot frame's content cannot be witnessed.
- **UARTs.** `STM32F7_USART` delivers every transmitted byte to a file backend and accepts injected bytes. No byte carries baud time or framing; `M0`, `TXINV` and `RXINV` are tags.
- **USB.** `STM_USB` (DWC2) enumerates as CDC-ACM under Renode's `USB_UART` host, measured with Antmicro's Zephyr firmware. It is not registered in `rcc`, so its enable bit reads 0.
- **The I/O compensation cell is not modelled.** `SYSCFG_CCCSR` drops `EN`, and `READY` reads 0, so a firmware that waits on `READY` hangs. DS12117 Rev 10 Table 149 gives speeds `10` and `11` only with the cell enabled. Ruled (`4300` D1): those speeds are refused, and SPI uses `01`.

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

Eight workspace members, and `bridge/` outside the workspace for its `unsafe` FFI (`RUL-17`); corrected 2026-11-12 (review A8.4), the table below lists the original seven:

| Part | Owns | Never touches |
|---|---|---|
| `crates/aethelgard-ir` | the specification schema and the solved topology, as separate types (`RUL-16`), validation, serialization | external processes, network |
| `crates/aethelgard-eda` | schematic, layout, bill of materials, the strict KiCad reader, capacitance extraction, the fabrication notes' text | firmware |
| `crates/aethelgard-svd-hal` | vendor register description ingestion, generated board crate | board files |
| `crates/aethelgard-twin` | simulator platform descriptions (the Renode `.repl`, emitted from a `SolvedTopology` and `aethelgard-svd-hal`'s register facts), analog model, the lock-step bridge | the `Specification`, which it never reads (`RUL-16`) |
| `crates/aethelgard-chaos` | fault scenarios, the run rules, trace archive, verdicts (the campaign *runner* is the CLI's `campaign.rs`; corrected 2026-11-12, review A8.3) | suppressing a panic or a timeout |
| `crates/aethelgard-evolve` | attribution from traces, mutation proposals | merging anything, bypassing a gate |
| `crates/aethelgard-cli` | the `aethelgard` binary: subcommands over the crates and the external tools (`verify --fab-out` writes the fabrication package), and the `mcp` subcommand that exposes the same surface over stdio | raw credentials in source or logs |

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
| `RUL-02` | Pin-mux, voltage-domain, and timing resolution | A constraint solver. **Spike first** (`AET-WO-1200`), measuring both candidates on the chosen part's full pin table. *Amended 2026-10-29, prior inverted:* the prior is now **a pure-Rust constraint search** (backtracking with propagation, in a fixed variable and value order), and SMT via the `z3` crate is the alternative the spike must justify | Heuristic or model-guessed pin selection is exactly the hallucination `INV-01` and `INV-03` exist to catch. The inversion, at Argon's 2026-10-29 review: the P1 gate demands byte-identical regeneration, and a search with a fixed order returns the same assignment by construction, whereas the model an SMT solver returns can change between Z3 versions. Z3 also puts a C++ library into every build. The STM32H753ZI has up to sixteen alternate functions per pin, AF0 to AF15, a table small enough that the spike must show the search is too slow before Z3 earns its cost |
| `RUL-03` | Board artifact format | Emit KiCad S-expressions (`.kicad_sch`, `.kicad_pcb`) directly; validate with `kicad-cli` passing `--exit-code-violations` and `--format json`. *Amended 2026-10-29:* the design rules DRC judges against come from the specification, never from the emitted files. In its scratch copy, before DRC runs, the gate itself supplies every file that carries rules. It overwrites the board's `(setup …)` block, and writes the project's design settings and a `.kicad_dru` from the specification; the emitter's own copies are never used. **Proven in `AET-WO-1300`** by a seeded violation that a looser emitted rule would have hidden. *Amended 2026-11-08* (the `AET-WO-1300` spike, ruled on PR #40). In its scratch copy the gate writes a setup block with no clearance tokens, a `.kicad_pro` that holds the board minimum and ignores ERC's library checks, its own `.kicad_dru`, and project-local `fp-lib-table` and `sym-lib-table` files pointing at the vendored libraries; it never uses an environment variable for these. A board carrying a local `(clearance …)` on a pad, a footprint or a zone is refused by name, because a local override is judged only against the board minimum and would escape a per-net rule. So is an in-board net class, and so is every token that waives a check for an item: `net_tie_pad_groups`, `allow_soldermask_bridges`, `jumper_pad_groups`, `duplicate_pad_numbers_are_jumpers yes`, `allow_missing_courtyard` (ratified at the `AET-WO-1300` build, A2). The gate's reader must tokenise exactly as KiCad does: KiCad skips `#` lines, reads NUL as whitespace and keeps a `"` inside an atom, and each difference once let a board hide its rules from the gate (A1, critical, fixed). Without `--spec`, the same files hold KiCad's defaults and the verdict says `"rules": "kicad-default"`. **No path reads a rule from the board under judgement.** Zones are emitted unfilled, and DRC runs with `--refill-zones`, because KiCad's saved fills are not deterministic. **The gate judges copper by geometry, as KiCad does, and never by a track's `(net …)` label.** A board whose labels lie is still the board its copper says it is. KiCad gave such boards exit 0 (`AET-WO-1302` A2, A3), so no consumer of a board trusts a label: each one derives nets from copper connectivity to pads (amended 2026-11-08, PR #60). *Amended 2026-11-13 (`AET-WO-1306`):* every gate reads a caller's KiCad file through one strict reader, `aethelgard_eda::strict`. Where KiCad would keep the last of a repeated child or field, or decode an escape other than `\" \\ \n \r \t`, the gate refuses the file rather than guess. Only the strict `Node` type is read, so a first-match read of caller input does not compile. *Amended 2026-11-13 (`AET-WO-3300`, ruling (a) on PR #103, ratified by Scotty):* aethelgard passes a tool's output through as the tool wrote it, with one exception: where the tool cannot be made deterministic by its own options, the named lines that carry the nondeterminism may be deleted, never rewritten. Each line is matched exactly at its measured place, any departure is a `ToolError` with nothing written, a test pins the rule against the tool's real output, and the package says what was removed. The only such case is KiCad 10.0.7's two clock lines in every Gerber (`%TF.CreationDate`, `G04 Created by … date`) *Amended 2026-11-14 (`AET-WO-4100`, PR #120):* the rules are the specification's own `board.rules`: clearance and track width at or above the engine floor, with via drill and diameter fixed to the engine's. The verdict labels them `"specification"`. With `--spec`, `check drc` first refuses, by name, a board whose copper layers, planes, thickness or via types are not the declared board's (`LayersDisagree`, `PlanesDisagree`, `ThicknessDisagrees`, `ViaNotThrough`). | Text format, diffable, headless checks exist. The flag is mandatory because of the exit-code finding in §2.1. The amendment is `INV-07` applied to DRC. KiCad stores a board's rules in the board's own project files, so a DRC that reads them judges the artifact by rules the artifact chose. An emitter that loosened a clearance would pass its own gate. |
| `RUL-04` | Firmware hardware interface | A generated `#![no_std]` board crate whose pin, clock, and peripheral states are typestates. **Amended 2026-11-08** (the `AET-WO-1400` spike, ruled on PR #42). The crate has **no dependencies** and sits **beside `embassy`, not above it**, because `aethelgard prove` cannot judge a crate with a dependency: a registry dependency gives `ToolError`, and a vendored one breaks `RUL-13`'s file cap. Every register access goes through one trait, whose real-address implementation is the crate's only `unsafe`. The register facts trace to a pinned ST source, with its licence and hash recorded, as §2.8 does for the pin data. The generated Kani harness implements the trait over a harness-owned image (`OQ-04`) and is pinned by byte-identical regeneration. Embassy may drive other peripherals in P2 firmware, never one the board crate owns. `stm32-metapac` is not a dependency of anything: it ships no licence text and carries the `stm32-data` provenance that §2.8 declined | `INV-02` is enforced by `rustc` at zero runtime cost only if illegal states are unrepresentable; Kani is the only Rust tool that proves absence of a class of UB rather than testing for it |
| `RUL-05` | Digital twin | Renode for instruction-level execution, `ngspice` for the analog nets that matter (bus rise time, rail droop), lock-stepped through a bridge whose transport is spiked (`OQ-02`). **Named as instruction-level, not cycle-accurate** **Corrected 2026-11-12 (review A8.2):** what runs today is a static analog pre-pass before the boot (`RUL-17`, `cli/src/analog.rs`); in-run lock-step is `AET-WO-2301`, reserved. | Renode is the only open simulator with the target family's peripherals modelled; the analog solver supplies the physics Renode idealises away. Claiming more than instruction fidelity would violate the honesty `INV-06` demands of measurements |
| `RUL-06` | Trace format and storage | **Local-first**: the archive of record is on disk, and Cloudflare R2 is an optional mirror. **Departure from Argon's plan**, which streamed to R2 as the primary store. *Amended 2026-10-29, replacing Cap'n Proto:* the format is **deterministic CBOR** (RFC 8949 §4.2.1, core deterministic encoding) in a profile of unsigned and negative integers, byte strings, UTF-8 text strings and arrays only, each with a definite length and its shortest-form argument; no maps, floats, tags or simple values. The reader refuses anything outside the profile, so an accepted trace has exactly one encoding. The encoder and the strict decoder are written in `aethelgard-chaos` (`AET-WO-0400`). The archive has one writer, the Rust orchestrator; Renode's Python hooks send it events over the bridge and never write the archive. **Corrected 2026-11-12 (review A8.1):** no Python hook exists or is generated; `RUL-17` replaced that route, and the bridge is the `aethelgard-bridge` library called by the CLI. | `INV-04` must hold with no network and no credentials; a replay that needs a cloud account is a replay that fails on the day it is needed. R2 remains attractive for cost once volume is measured (`AET-WO-0400`). The amendment's reasons: Cap'n Proto's Rust code generator needs the C++ `capnp` schema compiler, installed nowhere this workspace builds. Cap'n Proto was chosen so Python inside Renode (`RUL-12`) could write the same format. That reason is dropped, on Argon's point that the archive should have one writer. What remains is readers: CBOR is an IETF standard with libraries in Python, for analysis, and in TypeScript, for the viewer `RUL-12` reserves. `postcard` is simpler here and documents its wire format, but it is not self-describing. A reader in another language must mirror the Rust types' field order by hand, and a reordered Rust field changes the format silently. Integers and arrays alone make the bytes a function of the values, with no map-key order to canonicalise and no float to normalise. Most CBOR libraries decode non-shortest forms leniently, which lets two byte streams carry one trace; the strict decoder makes "one trace, one encoding" checkable, and the profile is small enough to cost less than a dependency |
| `RUL-07` | What the evolution loop may do | Propose a specification diff with an attribution record; never apply it to `main`, never alter a gate | `INV-05`, `INV-07`, and northstar `NG-04` together: the loop proposes, the gates judge, a human merges |
| `RUL-08` | First reference target | **Ruled 2026-10-19 (`OQ-01`, delegated to Claude by Scotty):** the STM32H753ZI on the Nucleo-H753ZI board, with the STM32H743 treated as the same target (identical silicon minus the crypto block). RP2350 remains the planned second target, precisely because it is a different vendor; it gets a hand-written platform description, since Renode ships none | §2 and §2.2: the only candidate with a Renode platform (4 I2C, DMA, ADC, EXTI, RCC, flash, 9 UARTs), a board file, a vendor-published physical reference board with a digital twin, an `embassy-stm32` HAL, and a `hilt` harness entry. RP2350 and ESP32-S3 have none of these in Renode. The one caveat, a single SPI instance in the platform file, is cheap to fix in the generated description. A second vendor later is what keeps the IR from becoming one vendor's register map (northstar §6) |
| `RUL-09` | Where gates are enforced | In the chaos runner and the CI job, never in the proposing agent | `INV-07`; a check the proposer performs on itself is not a boundary |
| `RUL-10` | Tool server language and placement | **Ruled 2026-10-19 (`OQ-05`, delegated to Claude by Scotty; revised the same day before merge, see Amendments):** the contract is the `aethelgard` command-line binary, one Rust executable with JSON on stdin and stdout and documented exit codes (`AET-WO-0300`). *Amended 2026-10-21 by `AET-WO-0300`:*
  - Tool subcommands take their inputs as path flags, because board files are not JSON.
  - stdout is still exactly one JSON line.
  - Exit codes `1` (the tool ran and the verdict is fail), `4` (the tool, a variable it needs, or the sandbox is unavailable) and `5` (the tool ran but its output cannot be trusted, never read as pass or fail) extend `0`, `2`, `3`, `64` and `70`.
  - Exit `6` (amended 2026-11-12, `AET-WO-3200` F2): `propose` wrote a generator report, not a specification proposal. Only `propose` exits 6; MCP counts 0, 1 and 6 as completed calls.
  - The full contract is `crates/aethelgard-cli/README.md`. The MCP server is the **`aethelgard mcp` subcommand of that same binary**, built on `rmcp`, the official MCP Rust SDK (Apache-2.0, stdio transport, streamable-HTTP and OAuth features available), speaking stdio and binding to nothing (`AET-WO-1500`). The tool list is generated from the command tree, never hand-written. **Departure from the ClearForge fleet standard's TypeScript default**, on Scotty's instruction that language follows the layer | A first version of this ruling chose a TypeScript wrapper because the fleet standard said so. On merits: a second runtime (Node) in the container buys nothing when the engine is already a binary; a wrapper that lists tools by hand is a second copy of the command surface, which is the drift `INV-01` exists to forbid at the design level; `rmcp` is an official SDK, not a framework wrapper, so the standard's real rule (official SDK, never a wrapper) still holds. A local stdio tool is not an exposed node and needs no OAuth. The day it is exposed it becomes a fleet server and gets its own ClearSeal work order; the standard's controls are language-agnostic by its own statement, and `rmcp` carries the auth and HTTP transports that work would need |
| `RUL-11` | License | **Ruled 2026-10-19 (delegated):** every crate, the CLI included, is `MIT OR Apache-2.0`, with both texts at the repository root. **Generated artifacts (board files, board crates, platform descriptions, fabrication packages) carry no license obligation from this project**, stated here the way compiler runtimes state it, so a user's proprietary firmware is unaffected. A hosted campaign service or a curated component-model library, if either is ever built, may carry its own terms; nothing in this repository will depend on them | Matches `embassy`, `svd2rust`, `hilt`, and the Rust norm, so contributors already know the deal; Apache carries a patent grant, MIT carries the fewest questions. The dependency inventory is compatible: KiCad GPL-3 (invoked as a separate process, never linked), Renode MIT, ngspice BSD-3 (linked as a library; its `XSPICE` and `tclspice` parts are not used), Z3 MIT, Kani Apache-2.0 OR MIT, ST's `STM32_open_pin_data` BSD-3-Clause (vendored data files keep ST's notice, §2.8). KiCad 10.0.7's symbol and footprint libraries are CC-BY-SA 4.0 with a design exception. The few the emitter uses are vendored verbatim into `aethelgard-eda` with KiCad's licence beside them, so that crate's `license` is `(MIT OR Apache-2.0) AND CC-BY-SA-4.0`. Designs emitted from them fall under the exception (amended 2026-11-08, `AET-WO-1300` D1). **Amended again 2026-11-08, at Scotty's direction:** the engine is for anyone to use, whoever they are and whoever they work for, so no crate carrying the engine's own logic should carry a share-alike term. The vendored files move into a data-only crate, `aethelgard-kicad-lib` (`CC-BY-SA-4.0`), and `aethelgard-eda` returns to `MIT OR Apache-2.0`, keeping the files' sha256 pins on its side (`AET-WO-1304`, merged in PR #62). Cap'n Proto (MIT) was in this list until `RUL-06` replaced it on 2026-10-29. A copyleft license on the engine would have blocked exactly the contributions the northstar §6 says the project dies without: other people's sensor models |
| `RUL-12` | Which language at which layer | **Ruled 2026-10-19 (delegated):** the engine, the CLI, the analog bridge, and the generated firmware are **Rust**. Hooks that run *inside* Renode (fault injection at a peripheral, custom peripheral stubs) are **Python**, because that is Renode's embedded extension language, kept to the smallest snippets the Rust orchestrator can generate and never hand-maintained. Shell is for container build and CI glue only. **C++ is not written** here: KiCad, ngspice, and Z3 are consumed as a subprocess or through `bindgen`, never patched. **Go has no layer** in this project. **TypeScript and React are reserved for a browser viewer** of traces and verdicts, which no phase schedules; if one is built it is a separate package that reads the trace archive and never holds engine logic **Corrected 2026-11-12 (review A8.1):** the Renode-hook row is void; no Python is generated anywhere. | Language follows the layer, per Scotty. Rust is forced at the bottom by `INV-02` (the generated board crate is `no_std` Rust and the generator needs Rust AST tooling) and earns the rest by being one toolchain, one binary, and memory-safe while parsing untrusted vendor files. Python inside Renode is a platform constraint, the same kind of exception the fleet standard makes for the S25. Go has no `no_std` story, no Rust-codegen tooling, and `cgo` makes the `libngspice` boundary worse. A browser is the one place TypeScript is the native language, so that is the one place it is used |
| `RUL-13` | Bounds on what the CLI reads | **Ruled 2026-10-19 (AET-WO-0100 F3, F9):** input is capped at 16 MiB and refused unread above it (`InputTooLarge`, exit 3). *Amended 2026-11-07 by `AET-WO-0300b`:* the same 16 MiB caps every tool input's copy, summed across a crate's files and enforced while copying, and a crate is also capped at 4,096 files. The CLI sets no timeout of its own: a `--file` naming a pipe nobody writes to blocks, and the caller's timeout ends it. **Amended 2026-11-08 for `simulate --firmware` only** (`AET-WO-2100` B1, PR #56): the reaper kills Renode's process tree at a 30 s wall-clock bound or a 1 GiB resident-memory cap. Both are measured constants, at about 10× the slowest reference boot and about 2.5× its peak. The memory cap is enforced by polling, because CoreCLR will not start under any `RLIMIT_AS`. A killed boot is a `fail` naming the bound: `wall_bound_exceeded`, `memory_cap_exceeded` or `cpu_aborted` **Corrected 2026-11-12 (review A8.6, A8.7):** the 30 s / 1 GiB bound also applies to the bridge (`cli/src/analog.rs`); an archive read is capped at 256 MiB (`chaos/src/archive.rs`), not 16 MiB. | The cap stops a hostile document exhausting memory before parsing. A 120,000-net document is 13.7 MB and validates in 0.17 s, and the reference probe's specification is kilobytes; raise the cap by amendment when a real specification passes a quarter of it. Refusing non-regular files would also refuse `--file /dev/stdin` and process substitution, and every caller in §5 and §6 already runs the binary under a bounded sandbox, so the timeout belongs there |
| `RUL-14` | What a well-formed specification must contain | **Ruled 2026-10-19 (AET-WO-0100 F4, F5); built in `AET-WO-1100`:** every identifier (project name, domain id, net name) is printable ASCII `0x21`–`0x7E` with no whitespace inside it. **Narrowed 2026-11-08** (`AET-WO-1300` D6): no identifier may contain `/`, `{`, `}`, `$`, `[` or `]`. The spike measured that KiCad reinterprets each as a sheet path, an escape token, a variable or bus syntax, giving a collision, an expansion or an ERC error, while S-expression quoting handles every other character. A specification with no voltage domain is refused. An empty `net_constraints` stays valid **Corrected 2026-11-12 (review A8.8):** identifiers are also capped at 64 bytes. **Narrowed by measurement 2026-11-13 (`AET-WO-1102`):** one function, `aethelgard_ir::check_identifier`, holds the rule for every kind. **The base set** for every identifier also refuses `"` (KiCad does not escape it in a placement row) and `%`, `*`, `,` and `\` (KiCad rewrites each as `\uXXXX` in a Gerber net attribute; refusing them keeps every output byte-for-byte, so no fabricator's tool must decode an escape), 11 characters in all, and any identifier holding a date shape `dddd-dd-dd` or a time shape `dd:dd:dd`. **A project name** is also a Cargo crate name of at most 58 bytes (`generate` names the board crate `<project>-board`). **A device reference**, I2C or SPI, is also a Renode identifier (`[A-Za-z][A-Za-z0-9_]*`). Every output was measured with every admitted character, through `verify --fab-out` in CI. `validate` now refuses what `generate` refused, with exit 2 and the rule broken; scenario names are checked as they are read **Version 2, 2026-11-14 (`AET-WO-4100`, PR #120):** a specification carries a required `board`. It has copper layers (2, or 4 as signal, plane, plane, signal with exactly one `GND` plane), each plane's net, the stackup as bounds each with its `source`, and the rules. A declared minimum or maximum must be one its source supports, never a nominal value, and minima round down and maxima round up. A version-1 file is refused by name at every entry. | Two ids a human cannot tell apart, such as `VDD_3V3` with a Cyrillic `В`, or one carrying a zero-width space, would let the board and the firmware disagree on which rail is which with neither tool noticing, which is the drift `INV-01` exists to forbid. KiCad's own net names fit the set once KiCad adds its own prefixes: a specification's `SDA` becomes KiCad's `/SDA`. `+3V3` and `Net-(U1-Pad3)` fit as they stand. A board with no rail cannot be powered, so an empty domain list is absence under `INV-06`. A net constraint is an optional bound, not a required part, so its absence is not |
| `RUL-15` | The tools image: how it is built, published and consumed | **Ruled 2026-10-20 at review of `AET-WO-0200` (PR #7, findings F1, F2, F4, F5):** a Dockerfile `FROM kicad/kicad:10.0.7` pinned by index digest. Every apt package is pinned to an exact version from the base distribution's normal archive, every download carries a sha256 written in the Dockerfile, and the Rust and Kani versions are numbered. The default user is root, with tool state under `/opt`. The image is published to `ghcr.io/clearforge-llc/aethelgard-tools`, tagged with the commit, **only** by the `publish` job on a push to `main`, the one job holding `packages: write`, which pushes the very image it has just smoke-tested under `--network none`. Consumers pin it by digest from a committed `container/IMAGE`, which a pull request updates from the digest in the publish run's summary. CI never writes to the repository. Until that file exists, a consumer builds from the Dockerfile | Nix lacks Kani and the pinned Rust, so it would need a second install mechanism beside it (§2.4). KiCad 10 exists only as the official image. Exact archive pins fail loudly when a version leaves the archive (`INV-06`), and the published digest is what makes a consumer's image reproducible, so a snapshot mirror would add a slow dependency to every cold build to buy what the digest already gives. Root, because GitHub `container:` jobs mount a runner-owned workspace that a non-root default user cannot write. Publishing from `main` alone keeps write permission off every pull request from a fork, whose token GitHub caps at read. A branch pushed to this repository can still edit the workflow and request `packages: write` on its own pull-request run, and branch protection does not stop that, because the run happens before any merge. The boundary is therefore who holds write access to the repository: write access is publish access, which is accepted because the same people can already change what `main` builds. Branch protection on `main` (a pull request plus a passing `gates` check) guards `main`'s history, not the registry |
| `RUL-16` | What a specification is, and what the solver adds | **Ruled 2026-10-29 at Argon's review; built in `AET-WO-1100`.** `aethelgard-ir` holds two distinct types: `Specification`, the declared intent and bounds a person writes (today's `TopologyGraph`, renamed), and `SolvedTopology`, the solver's assignments. No API accepts a `SolvedTopology` where a `Specification` is expected, and the reverse holds too. Every keyed collection in either type is a `BTreeMap`, or a `Vec` sorted and checked for duplicate keys. Neither type contains a `HashMap` **Scope, 2026-11-09 (PR #73 F2):** the rule binds every generator, platform, firmware and judge input. `aethelgard-eda`'s schematic emitter may read the `Specification`, because a schematic draws declared intent. **Amended 2026-11-12 (`AET-WO-1205`):** `SolvedTopology` is at schema version 2 and carries `i2c_speeds`, each I2C bus's declared speed, so the generators and `validate` hold to intent without reading the `Specification`. A test pins its readers at exactly two. **Amended 2026-11-12 (`AET-WO-1204`):** the fence is compiled. A specification's device lists are private to `aethelgard-ir`; `cli`, `svd-hal` and `twin` each carry a `clippy.toml` that disallows the two accessors; a guard test fails the build if either is renamed; and `eda`'s reader is pinned at `schematic.rs`. Since `AET-WO-2203`, `SolvedTopology.i2c_devices` carries the I2C devices, and no other consumer learns about devices from the `Specification` | One type holding both halves would let a hand-written specification carry a "solved" pin and skip the solver, so a gate would judge an assignment nothing derived (`INV-03`). It would also let solver output be fed back in as if it were intent. The P1 gate's byte-identical regeneration needs serialization order to be a function of content. A `HashMap`'s iteration order is randomised per process, so a `HashMap` would fail that gate intermittently, which is worse than failing it every time |
| `RUL-17` | The lock-step bridge (`OQ-02`) | **Ruled 2026-11-08 on PR #66, ratified by Scotty 2026-11-09.** `libngspice` runs in-process and is stepped synchronously (`ngSpice_Init_Sync`): `SetBkpt` at the edges for a static fault, inversion of control at about 10 µs for a dynamic one. The `unsafe` FFI lives only in the `ffi` module of `bridge/`, a crate excluded from the workspace. It is hand-declared against a vendored, hash-pinned `sharedspice.h` and checked by a test. The CLI runs the `aethelgard-bridge` binary as a sandboxed tool, and the bridge may own Renode as its child. A bus fault reaches the firmware as the sensor's absence from the platform, using stock models only. The trace records the measured rise time and the firmware's effect as separate records, and labels the effect as the model's rendering of a NACK. A static fault's analog pass runs before the boot. Renode's monitor history goes to `/dev/null`. The bridge's solver trait has the FMI 3.0 Co-Simulation shape, without the dependency. *Amended 2026-11-13 (`AET-WO-2301`), ratified by Scotty 2026-11-14:* a dynamic fault whose analog side does not depend on the firmware (a declared rail droop) is solved once before the boot and applied as scheduled events at its exact crossings (to 10 ns), not by inversion of control per quantum: measured, per-quantum lock-step at 10 µs missed a 2.15 µs crossing of `VBOR3` and was seven times slower. Per-quantum lock-step stays the route for the first analog load that depends on what the firmware does. A brownout is rendered as a held reset (`machine Reset`, the cold writes and vector table re-applied, held, released with a 10 ns nudge), recorded as the model's rendering, because Renode 1.17.0 models no BOR, PVD or reset cause | `AET-WO-2101` measured three ngspice routes. Only this one runs under the unchanged sandbox; a process per transient needs `/tmp`, and a long-lived process needs a pty. It is the fastest (4.0 ms per transient), and its repeats are byte-identical. Renode offers no I2C hook, and its NACK is unimplemented. A research pass on 2026-11-09 found no open tool that couples Renode to a SPICE solver; every lock-step bridge it found is HDL-to-SPICE at pin level. This is a workspace exception in the manner of `RUL-04`'s `mmio`, not a `RUL-12` amendment |
| `RUL-18` | Who proposes a change after a failure (`OQ-06`) | **Ruled by Scotty 2026-11-12, option C.** Deterministic rules propose first. A language model may be consulted only where no rule matches, and its proposal is labelled as model-made. Either way, the same gates judge every proposal and a human merges it (`RUL-07`). A **software** attribution in generated firmware is a generator defect: it yields a structured generator bug report, never a specification diff. A proposal never changes a constraint the gates judge (`INV-07`). `AET-WO-3200` builds the rules and the proposer interface; a model proposer is a later work order | Rules are free, replayable and explainable, and they meet the P3 gate's 47 kΩ case exactly (the rise is linear in R × C, §2.18). A model earns its cost only on failures no rule anticipates, so it comes after real unmatched failures exist. The firmware is generated from the specification, so a firmware defect the specification did not cause can only be fixed in the generator; changing the specification to work around it would hide the defect (`INV-05`) |
| `RUL-19` | What a verdict claims, today | **Ruled 2026-11-12, from the Mythos 5.2 review (A1, A4, A10.2, A10.3):** (1) **The analog records are computed from declared values.** *Amended 2026-11-12 (`AET-WO-1305`):* `aethelgard extract` now bounds each I2C line's capacitance from above from the routed board's copper (Dirichlet's principle, ε at FR-4's IPC maximum, plus UM10204's maximum `Ci` per pin), and `verify` runs it as a gate. When every line's bound is at most its declared value, the campaign's declared-value rise time bounds the board's, within the stated conditions (bare copper, part bodies excluded, a DRC-clean board). The rise time is ngspice arithmetic on a netlist built from the specification's pull-up and its `max_capacitance` bound; no property of the routed board reaches the campaign until extraction (`AET-WO-1303`; a first estimate before `AET-WO-3300`). Every report says "computed from declared values", never "measured on the board". (2) **Each gate binds only its own input.** `check drc --spec`, `check erc` and `prove` each judge the bytes they are handed; none proves those bytes came from one specification, and `prove` passes a crate's own harness. Only `aethelgard verify` (`AET-WO-3301`) binds a pair, by regenerating it and naming every hash in one verdict; the P3 fabrication clause is met through it (`AET-WO-3300`: `verify --fab-out`). **Amended 2026-11-12 (`AET-WO-3301` F1):** `verify` binds what each gate reports judging, not what the gate's tool reads. A process of the same uid that writes into a gate's private scratch between the gate's hash and the tool's read changes what is judged, and on Linux only a uid boundary stops it. Until `AET-WO-1601`, every `verify` verdict carries that residual, and the P3 fabrication clause waits for `1601` as well. (3) **`replay` proves integrity, not reproduction:** it re-hashes and re-checks a static run's records; determinism is shown by running the campaign twice (`INV-04` as worded). (4) **An MCP client acts with the user's filesystem identity:** every path flag is a plain path, and only a scenario's `base` is confined. **Amended 2026-11-12 (`AET-WO-1601`, `AET-WO-3301` F1):** on Linux the boundary against a process that races a gate's scratch is the uid. A verdict's `sandbox.uid_boundary` says whether that boundary held for the run; when it is `false`, a process of the same uid (or the owner of a rootless user namespace) could have changed what the tool read, and the verdict claims nothing beyond the bytes it hashed. **Amended 2026-11-14 (`AET-WO-2204`, §2.20):** (5) **A boot passes** when, for each identity read, Renode logged the device model's own transaction lines, whole and in the records' order, with the part table's value, and the core ended in thread mode on a self-loop after writing the boot word. **It does not claim** that the core is at the generator's fixed point, that no interrupt runs after the boot word (review A2, open until P4's firmware enables one), or that the record's value was derived from the read rather than written as a constant (A3) | A gate that over-claims is worse than one that under-claims, because the next decision rests on the claim. The P2 gate stays passed as its clauses were written, and this ruling says what the pass is evidence of, so neither Scotty nor an announcement repeats more than the code shows. **Public wording, ruled by Scotty 2026-11-12:** "The firmware and the specification's declared electricals hold under seeded faults; board-level electrical evidence arrives with extraction." |

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

**Trust model for MCP clients (`RUL-19`, 2026-11-12).** An MCP client acts with the user's filesystem identity: every path flag is a plain path the client chooses, `emit` may create directories and overwrite its own three file names, and each verdict returns the sha256 of any file it can read. Only a scenario's `base` is confined. A tool's scratch directory is under the system temporary directory and is bounded by the caller's cgroup, not by the tool (review A12.1; `RLIMIT_FSIZE` is backlog).

**The sandbox as built (`AET-WO-0300`, rulings on PR #12).** Each tool call goes through these steps:

1. **Copy first.** The CLI copies the input into a private scratch directory and hashes the copy. *Amended 2026-11-12 (`AET-WO-1601`):* scratch lives in `TMPDIR` if set, otherwise `/var/lib/aethelgard` (root) or `~/.cache/aethelgard`; every ancestor must be root's or the caller's, writable by no group or other, and spelled in `[A-Za-z0-9/._-]`, or the call refuses with exit 4, `ScratchUnsafe`. `/tmp` is refused, so no other user can plant a `.cargo` configuration above it (`AET-WO-3301` F13). The tool judges that copy, so the verdict's sha256 is exactly the bytes judged (`INV-03`), and the original is never written.
2. **A guardian.** It is started with the temporary directory and removes it if the CLI dies first.
3. **A reaper.** It runs in its own process group and is registered as a child subreaper. It starts a Landlock trampoline in a second group, which confines itself and becomes the tool. When the tool ends, or the CLI dies by any signal, the reaper kills the tool's group and every remaining descendant.
4. **An empty environment,** apart from `HOME` and `TMPDIR`, which point at the temporary directory. Kani alone also gets `KANI_HOME`, `RUSTUP_HOME`, `PATH` and `CARGO_TARGET_DIR`; Renode also gets `DOTNET_EnableDiagnostics` (corrected 2026-11-12, review A8.5). `build` (`AET-WO-3301` F10) gets `RUSTUP_HOME`, `PATH`, `CARGO_TARGET_DIR` and the caller's `RUSTUP_TOOLCHAIN`. stdin is `/dev/null`.
5. **A uid boundary when the CLI runs as root** (`AET-WO-1601`, ruled on PR #96). The trampoline empties the bounding set, clears groups and becomes uid 65534, gid 65533, before Landlock and seccomp, and reads every id back; a failure is exit 4, `SandboxUnavailable`, never a run as root. The copy is root's, 0444; the tool's directory is root's, group 65533, mode 1770. `RLIMIT_FSIZE` is 64 MiB; a tool that reaches it ends as exit 5. Every verdict's `sandbox.uid_boundary` is `true` only when the tool ran under that uid, the CLI's user namespace is the initial one, and every path the tool may read beyond the system set is root's and writable by no other uid or group but root's; otherwise `false`, and `uid_boundary_unmet` names the first condition that failed (`tool_uid`, `user_namespace` or `toolchain`). The tools image is built to meet the third (`AET-WO-1602`, `container/smoke.sh` check 10).
6. **Landlock's filesystem rights through ABI V3 are required.** Below that floor the call refuses with exit 4, `SandboxUnavailable`, and there is no flag or variable that runs a tool unconfined. TCP rules (ABI V4 and later) and signal and abstract-socket scoping (ABI V6 and later) are applied where the kernel has them, and reported in the verdict as `sandbox.tcp_confined` and `sandbox.scoped`.

**Socket gap closed by `AET-WO-0300a` (§2.6).**

Two more steps now run in the trampoline before the tool starts:

1. **Before Landlock, the descriptor check.** It refuses the call when the tool would inherit any descriptor above 2 without close-on-exec, with exit 4, `SandboxUnavailable`, `id` `descriptors`. A leaked connected socket is a channel no filter sees, so callers must not leak. Our own CI closes the runner's leaked descriptors in the step's shell.
2. **After Landlock, a required seccomp filter.** It applies to the tool and to everything the tool starts. It is default-allow (`AET-WO-1600` measured a default-deny allowlist as unstable across environments), and it does the following:
   - kills any non-x86_64 or x32 system call, and any `int 0x80` call;
   - refuses `socket` for every family, and `socketpair` other than an `AF_UNIX` stream or seqpacket pair;
   - refuses `io_uring`;
   - **since `AET-WO-1600`:**
     - refuses with `EPERM`: System V IPC, `unshare`, `setns`, `clone` with any `CLONE_NEW*` bit, the `pidfd_*` calls, `ptrace`, `process_vm_readv`/`writev`, `kcmp`, the keyring calls, `bpf`, `perf_event_open`, `userfaultfd`, and POSIX message queues;
     - allows `prlimit64` and `sched_setaffinity` only for pid 0 or the tool itself (an outside `prlimit64` could set another process's CPU limit and kill it, past Landlock's signal scoping);
     - refuses the user-wide forms of `setpriority` and `ioprio_set`;
     - returns `ENOSYS` for `clone3`, so glibc falls back to `clone`, whose flags the filter can read. `EPERM` there broke every tool.

   If the filter cannot be installed, the call refuses with `SandboxUnavailable`, `id` `seccomp`. There is no escape hatch.
3. **Capabilities drop before the tool starts** (`AET-WO-1600`). In order:
   - the bounding set is emptied when `CAP_SETPCAP` is effective, which in practice means root in the image;
   - the ambient set is cleared;
   - the effective, permitted and inheritable sets are cleared;
   - all of it is verified, with `no_new_privs` set.

   Failure refuses with `SandboxUnavailable`, `id` `capabilities`. An unprivileged user cannot empty the bounding set, and under `no_new_privs` a full one grants nothing.

**The host is a supported boundary for tool subcommands** (named 2026-11-08, at the close of `AET-WO-1600`, PR #51). The non-socket channels this section listed as open on a host are closed: System V shared memory, `unshare(CLONE_NEWUSER)` and `pidfd_open`. So are the further channels its adversarial pass found, at no cost to any tool: tier 2 passed 16/16, then 20/20 with 1300 and 1301's tests.

**Two residuals are accepted and stated:**
- **A2:** a tool can lower another same-uid process's scheduling or I/O priority through the per-pid forms of `setpriority`, `ioprio_set` and `sched_setscheduler`, `setparam` and `setattr`. That is interference only, with no kill, no read and no write. BPF cannot tell the caller's own threads from other pids, and KiCad and Renode use these calls on their own threads.
- **A4:** below Landlock ABI 6, signals to other same-uid processes are not scoped. The verdict reports this as `sandbox.scoped: false`.

The published image remains the stronger boundary. Docker adds its own IPC namespace and profile on top.

**The lifecycle at every instant (`AET-WO-0300c`, rulings on PR #34).**
- The reaper and the guardian take the CLI's pid as an internal argument, `--cli-pid`, and never sample their parent.
- The guardian starts before the scratch directory exists.
- **Once the CLI is gone, the guardian retries removing its directory every 20 ms, for up to 10 s, and exits 70 if the directory is still there** (`AET-WO-0300d`). One `remove_dir_all` can lose its final `rmdir` to the trampoline, which may still be creating `status-*`. CI measured that leak about once in three runs before the fix.
- Under `aethelgard mcp`, which names itself in the internal variable `AETHELGARD_INTERNAL_PARENT_PID`, each CLI arms `PR_SET_PDEATHSIG` and then checks its parent against that pid. The variable never reaches a tool, because tool environments are cleared. Only `mcp` sets it, and it spawns every CLI from its main thread.
- **The signal is armed nowhere else.** Linux sends it when the parent's *thread* ends, so arming it for every CLI would kill the CLI under any multi-threaded caller. `mcp` never arms it on itself.

Measured: an immediate-cancel storm of 500 calls left 0 survivors, three runs in a row, against 8 before the fix. 100 `mcp` servers SIGKILLed while calls were starting left 0, against 52 before. 11,319 timed kills of the CLI left 0.

**The residual, accepted for P0:** a direct caller that dies in the CLI's first instant leaves that call to run to its own end. A direct caller owns its child's lifetime, as `RUL-13` already rules for timeouts.

**A rule for whoever builds the `mcp` subcommand.** The internal first arguments that start the reaper, the trampoline and the guardian execute the argument vector they are given. They are safe only because they are not in the command tree. The `mcp` subcommand must therefore map every tool to a `clap` subcommand and never forward raw arguments.

## 7. Traceability

None. This project has no governing standard. DO-178C and ISO 26262 appear in the northstar's beneficiary list as standards *users* may answer to; Aethelgard does not claim conformance to either. Recorded explicitly so a reader knows the question was asked.

## 8. Open questions

Each names a decider and what it blocks. None is a placeholder.

- **`OQ-01` Which microcontroller anchors the reference probe?** **Ruled 2026-10-19, see `RUL-08`.** Scotty delegated the ruling to Claude; the evidence is in §2.2. No longer blocks anything.
- **`OQ-02` How do the digital simulator and the analog solver exchange state in lock-step?** Candidates: a shared-memory ring buffer or a Unix domain socket, as Argon listed; a third is driving `ngspice` as a library through its shared-library interface rather than as a process. **Ruled and ratified: see `RUL-17`** (PR #66, ratified by Scotty 2026-11-09). The library route held. No longer blocks anything.

  **Leads from Argon's 2026-10-29 review.** The third has been read at source (§2.7). The others are unverified until read at source by a desk spike, which needs no builder, before `AET-WO-2101` measures anything:
  - **The FFI needs `unsafe`, and no workspace crate may contain it.** Calling `libngspice` in-process therefore means either a crate outside the workspace with its own audit, or ngspice as a sandboxed child process driven over a pipe. The sandbox (§6) refuses every `socket()` call, so a Unix socket on a named path is already ruled out for any tool that runs under it. That leaves pipes, `AF_UNIX` stream or seqpacket pairs, and a shared-memory ring only as a file both sides map inside the run's temporary directory.
  - **ngspice should be stepped synchronously, not on its background thread.** Its shared library is reported to offer a synchronous interface (`ngSpice_Init_Sync`, with callbacks that supply source values and confirm each time step) beside `bg_run`. Stepping on the caller's thread is what makes a co-simulated run deterministic, which `INV-04` needs.
  - **Renode's STM32 I2C model is transaction-level, and at 1.17.0 it has no error flags (§2.7).** Argon proposed a Python hook that sets `BERR`, `ARLO` or `NACKF` when the bridge measures a fault. That cannot work on the stock model. The spike weighs three ways for a fault to reach the firmware:
    - **The sensor's own model,** which this project writes, refuses to acknowledge or returns corrupted data. The firmware then sees what a real bus with slow edges produces at the far end.
    - **A replacement controller model** with real error flags, loaded at run time. It is C#, so it is a new language layer under `RUL-12`.
    - **A Renode upgrade,** which buys `NACKF` only.

    Whichever is chosen, the trace records the bridge's measured rise time as the evidence, and the effect the firmware saw as a separate event. Neither is ever disguised as the other.
  - **The bridge's channel must pass the descriptor check.** The sandbox refuses a call that would hand a tool an inherited descriptor above 2 (§6). So a pre-opened pipe or `socketpair` to Renode is refused by design, and Renode's stdout is the capture its verdict is judged from. A named FIFO created in the run's temporary directory needs neither a socket nor an inherited descriptor, and Landlock already allows that directory. The alternative is a ruling that admits one declared descriptor.
- **`OQ-03` What is the trace volume of one fault campaign, and does full-fidelity recording survive it?** Decider: measured by `AET-WO-0400`, ratified by Scotty if sampling is ever proposed (which touches `INV-04`). **Answered for static campaigns, 2026-11-09 (§2.17):** about 1 KB per scenario, independent of virtual time, so full fidelity survives with room to spare. **Answered for declared droops, 2026-11-13 (`AET-WO-2301`); Scotty confirmed 2026-11-14 (D8) that this is not sampling:** a droop adds its declared rail, its crossings, its resets and one observation per release, 140 B to 686 B per run whatever the virtual time; nothing per quantum exists, so nothing is sampled, and `MAX_RECORDS` stays 65,536. Per-quantum lock-step, when a firmware-dependent load needs it, reopens this. Previously: still open for dynamic campaigns, whose per-quantum records are estimated at 3 to 6 MB per simulated second; `AET-WO-2301` measures them and either answers this or brings a sampling proposal to Scotty. No longer blocks P2; blocks `AET-WO-2301`'s acceptance.
- **`OQ-04` Can Kani prove anything useful about volatile register access, or only about stubs?** **Narrowed 2026-10-19** (§2.2): only over a harness-owned register image, never over the real base address. **Answered yes, 2026-11-08** (§2.11, the `AET-WO-1400` spike).
  - A proof over a harness-owned, fully nondeterministic image, run through `aethelgard prove`, refutes a wrong AF number, a missing clock enable and a write past the block. Each refutation names the specification rule broken, and the corrected crate proves.
  - It proves the typestate API's register effects, not the silicon.
  - It holds only for a dependency-free crate (`RUL-04`) whose harness is generated from the specification and pinned by regeneration (`INV-07`).
- **`OQ-05` Tool server language: Rust via `rmcp` in the workspace, or TypeScript per the ClearForge fleet standard?** **Ruled 2026-10-19, see `RUL-10` and `RUL-12`.** Rust, as an `mcp` subcommand of the engine binary on the official Rust SDK. No longer blocks anything.
- **`OQ-06` Who proposes mutations in the evolution loop: deterministic rules, a language model, or both in sequence?** Decider: Scotty and Argon, informed by P3's spike. Blocks `AET-WO-3200`. Northstar `NG-04` fixes that the proposer never judges; it does not fix who proposes. **Ruled 2026-11-12 by Scotty: see `RUL-18`.** No longer blocks anything.

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
| 2026-10-29 | §2.1, §3, §4, §8 | Added a §2.1 row: SVD has no pin or alternate-function table. Amended `RUL-02` (the prior inverted to a pure-Rust search), `RUL-03` (DRC rules from the specification) and `RUL-06` (deterministic CBOR replaces Cap'n Proto). Added `RUL-16` (`Specification` and `SolvedTopology`), and the `ir` row in §3 to match. `OQ-02` gained three unverified leads, and its spike is now `AET-WO-2101`. Added §2.7, read from Renode's 1.17.0 source: its I2C model has no error flags, which turned one `OQ-02` lead into a fact. A fourth lead covers the bridge's channel. `RUL-03` now has the gate supply every rule-bearing file, and `RUL-06` records that the archive has one writer | Rulings on Argon's review of `main` at 9818c4f, triaged by the architect. Argon's own text is in `docs/inputs/argon-session/` as captures 06 and 07. Two of its red-team findings were overstated and are recorded in the roadmap where they changed a work order: Kani's exit 0 cannot be forged, and what matters is that the harness comes from the specification side (`AET-WO-1400`); tampering with a verdict after the tool has spoken needs `ptrace` or `/proc/<pid>/fd` access to the CLI, which sits outside the tool's Landlock domain and is refused, so blocking `ptrace` outright is cheap defence in depth for `AET-WO-1600`. The Cap'n Proto compiler problem was `AET-WO-0400`'s first spike question, and is answered here instead, so that work order no longer needs a spike |
| 2026-11-07 | §2.1, §2.8, §4 | Added §2.8: the pin table comes from ST's BSD-3-Clause `STM32_open_pin_data`, vendored by `AET-WO-1101`. The §2.1 row's licence question is resolved, and `RUL-11`'s inventory names the source | The architect read both candidates' licences and ST's files for the STM32H753ZI. The primary source has a clear licence; the derivative's generated data has none |
| 2026-11-07 | §2.9, §4 | Added §2.9 measurements from `AET-WO-0300b`. Amended `RUL-13` with the tool-input copy cap and the 4,096-file crate cap | Rulings at review of PR #18, promoting its FEEDBACK's two items. The measurement showed that the `[ERROR]` path Argon's RT-02 named does not occur in Renode 1.17.0. The check stays as a backstop |
| 2026-11-07 | §2.10 | Added §2.10 measurements from `AET-WO-0400` | Rulings at review of PR #21. Its adversarial findings 1, 2 and 3 became `AET-WO-0400a`: a per-event delta hash chain (manifest version 2), shape-bounded decoding, and a mirror crash that leaves the run readable. Findings 6 and 7 and the three scope questions were accepted as built and recorded there |
| 2026-11-07 | §6 | Added the sandbox lifecycle as `AET-WO-0300c` built it, the internal variable `AETHELGARD_INTERNAL_PARENT_PID`, and the accepted residual for direct callers | Rulings at review of PR #34. The parent-death signal fires on the parent thread's exit, so it is armed only under `mcp`, which spawns from its main thread |
| 2026-11-08 | §2.11, §4, §6, §8 | Added §2.11 (the pin search and the Kani probe). Amended `RUL-04` to a dependency-free board crate beside `embassy`. Answered `OQ-04`. Added the guardian's bounded retry to §6 | `AET-WO-1200` merged (PR #32). The `AET-WO-1400` spike was ruled on PR #42: `prove` cannot judge a crate with any dependency. `AET-WO-0300d` merged (PRs #39 and #41) after CI measured a leaked directory about once in three runs |
| 2026-11-08 | §2.12, §4 | Added §2.12, the KiCad measurements from the `AET-WO-1300` spike. Amended `RUL-03`: every rule-bearing and library file is gate-written, local clearance overrides are refused, and no path reads a board's rules. Amended `RUL-11`: the vendored KiCad libraries are CC-BY-SA 4.0. Narrowed `RUL-14`: no identifier contains `/`, `{`, `}`, `$`, `[` or `]` | The spike was ruled on PR #40 (D1, D3–D6). It measured that a local override escapes a per-net rule, and that KiCad reinterprets those six characters |
| 2026-11-08 | §2.13, §4 | Added §2.13 (the 1300 and 1400 builds). `RUL-03` widened to refuse waiver tokens and to require a KiCad-exact reader. `prove` fails a short cover summary | PRs #40 and #42. The adversarial passes found a critical reader bypass (A1) and a vacuous-proof pass (E1) |
| 2026-11-08 | §6 | The seccomp filter's `AET-WO-1600` denials and the capability drop added. The host named a supported boundary for tool subcommands, with residuals A2 (priority interference) and A4 (signals below Landlock ABI 6) stated | PR #51. Measured by refusal against every tool, with a census by `strace`. The adversarial pass found that `prlimit64` could kill another process |
| 2026-11-08 | §3, §4 | The twin's row: it emits the `.repl` from a `SolvedTopology` and `aethelgard-svd-hal`'s register facts, and never reads a `Specification`. `RUL-11` amended: KiCad's files move into their own `CC-BY-SA-4.0` crate | The `AET-WO-2100` spike was ruled on PR #56 (R6). Scotty set the licensing intent on 2026-11-08 |
| 2026-11-08 | §2.14, §4 | Added §2.14, the `AET-WO-1302` measurements; P1 gate clause 1 holds. `RUL-03`: the gate judges copper by geometry, never by a net label, so consumers derive nets from copper | PR #60, A2 and A3 |
| 2026-11-08 | §2.15, §4 | Added §2.15, the `AET-WO-2100` measurements. `RUL-13` amended for `simulate --firmware`: a 30 s wall bound and a 1 GiB resident cap on Renode, both measured. `RUL-11`'s split is done | PRs #56 and #62 |
| 2026-11-08 | §2.16 | Added §2.16, the `OQ-02` desk spike read at source: ngspice's synchronous interface, Renode's I2C extension points, and the controller's fault surface | `AET-WO-2101`'s precondition (§8 leads) |
| 2026-11-09 | §2.16, §4, §8 | Added `RUL-17`: the lock-step bridge, ruled on PR #66 and ratified by Scotty. `OQ-02` is closed. §2.16 corrected: an absent device leaves TXIS unset | `AET-WO-2101`'s measurements and the research pass of 2026-11-09 |
| 2026-11-09 | §2.17, §4, §8 | Added §2.17, the `AET-WO-2300` measurements. `OQ-03` answered for static campaigns and narrowed to dynamic ones (`AET-WO-2301`). `RUL-16` scope note: the schematic emitter may read the specification | PRs #73 and #74; P2's exit gate |
| 2026-11-09 | §2.18 | Added §2.18, the `AET-WO-3100` measurements: the rise sweep, the TXIS-timeout controller state that forces phase-gated evidence, and the trace bytes with kind 15 | PR #79 |
| 2026-11-12 | §4, §8 | Added `RUL-18`: rules first, a model only where no rule matches, and software attributions as generator bug reports. `OQ-06` is closed | Scotty's ruling of 2026-11-12, on the options the architect put after `AET-WO-3100` |
| 2026-11-12 | §2.19, §3, §4, §6 | Added §2.19 (the analog records are computed from declared values; no pull-up floor existed) and `RUL-19` (what a verdict claims today). Corrected `RUL-05`, `RUL-06`, `RUL-12`, `RUL-13`, `RUL-14`, §3 and §6 against the code. Added the MCP trust model | The Mythos 5.2 review of 2026-11-12 (`docs/reviews/`), verified at source by the architect |
| 2026-11-12 | §4 | `RUL-10` amended: exit 6, a generator report (`AET-WO-3200` F2) | PR #86 |
| 2026-11-12 | §4 | `RUL-16` amended: the fence is compiled (`AET-WO-1204`) | PR #90 |
| 2026-11-12 | §4, §6 | `RUL-19` (2) amended: `verify` binds what each gate reports, and a same-uid race is its residual until `AET-WO-1601`; §6 gains `build`'s environment | PR #91 (F1, F10) |
| 2026-11-12 | §4 | `RUL-16` amended: the solved topology carries the declared I2C speed, schema version 2 (`AET-WO-1205`). `RUL-19`: the public P2 wording, ruled by Scotty | PR #94; Scotty, 2026-11-12 |
| 2026-11-12 | §4, §6 | §6 gains the private scratch root, the uid boundary and `RLIMIT_FSIZE` (`AET-WO-1601`); `RUL-19` (1) gains extraction's first step (`AET-WO-1305`); `RUL-19` (4) gains `uid_boundary` | PRs #96, #97, #98, #99 |
| 2026-11-14 | §2.21, §4 | Added §2.21 (stackups measured, extraction's bound over a near plane, and the flight-I/O models read and measured). `RUL-03` amended (the rules are the specification's; `check drc --spec` judges the declared board). `RUL-14` amended (specification version 2, the `board` section, bounds with sources) | PRs #120, #123 |

New sections continue the sequence and never reuse a number.

## 11. Provenance

Written 2026-10-14 by Claude Code in session `claude/argon-repo-docs-synthesis-rjrb0d`. Rests on `northstar.md`, the founding Google Doc and the five Argon session captures under `docs/inputs/argon-session/`, and the measurements in §2 and §2.2, taken in the Claude Code cloud container on 2026-10-14 and 2026-10-19. Sources read for §2: Renode's supported-boards and time-framework documentation, Antmicro's 2026-07-16 STM32H7 reference platform post, the KiCad 9.0 CLI reference, the Kani paper and the Kani book's feature-support page, Renode's `stm32h7.repl`, `nucleo_h753zi.repl`, and `CHANGELOG.rst` from the googlesource mirror at 1.17.0, the `hilt` and `paprika` crate pages on docs.rs, and the `clearforge-mcp-standard` skill. §2.3 rests on `docs/work-orders/AET-WO-0100.FEEDBACK.md` and the architect's review of PR #4.
