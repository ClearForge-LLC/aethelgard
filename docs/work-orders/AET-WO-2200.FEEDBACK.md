# AET-WO-2200 — FEEDBACK

**Branch:** `wo/aet-2200-analog-bridge` · **Base:** `ec218d9` (`origin/main`, as the kickoff set it; `main` had not moved when the PR opened, so nothing is merged in) · **Builder:** Claude Code (Opus 5.5) as `forge` on Furnace (x86_64, kernel 7.0.0-38, 20 CPUs). One Sonnet subagent wrote the header signature test (`bridge/src/header.rs`); the lead re-ran it and fixed the mismatch it found in `ffi.rs` (F2). A fresh Opus subagent ran the full adversarial pass, in its own worktree on the build's commit. · **Cadence:** build-only, with one measurement stop (§7), which did not fire. Reviewed on PR #70; the two changes the review asked for are built in the same PR ("Amendments after review", at the end).

**Where everything ran.** The pinned tools image `ghcr.io/clearforge-llc/aethelgard-tools@sha256:2df68259…` (`container/IMAGE`) under rootless podman, capped with `podman run --memory 12g --memory-swap 12g` (2101 F17). The cap was read back from the container's own scope: `memory.max` = 12884901888 (12 GiB). `memory.swap.max` also read 12884901888, so under cgroup v2 `--memory-swap` equal to `--memory` did not hold swap at zero, as it does under v1 (F13): `libngspice0` 44.2 (`44.2+ds-1`, banner `** ngspice-44.2 shared library`), Renode 1.17.0, Rust 1.99.1. Tier 1 also ran on the Furnace host. Nothing was installed.

**§7, held:**
- The measured rises equal S2's (below), well inside both tolerances.
- The bridge runs under the CLI's sandbox with **no** grant beyond the system directories and its own directory, which is less than S5 measured (S5's runs also had `/proc` and `/dev/urandom`).
- Nothing outside the fence changed except as listed under "The fence", each named.
- No test was weakened. Four existing tests changed their topology or their expectation because `validate` now refuses earlier; each is listed under "Tests changed".
- No `sudo`, system package or secret.

Entry schema for findings: `finding · where · severity · type · recommendation · decision-needed`.

## §3, acceptance: one tier-2 test

`crates/aethelgard-cli/tests/twin_tier2.rs` `p2_gate_clause_2_analog_half_47k_fails_and_2k2_passes`, in the image, with the real bridge beside the CLI and the real Renode:

| Pull-up | `rise_30_70_ns`, both lines | S2 (route c, `SetBkpt`) | Ceiling | Fault | Verdict | Boot witnesses |
|---|---|---|---|---|---|---|
| 2.2 kΩ | **186.40139798114484** | 186.4014 | 300 | none (`faults: []`) | `pass`, exit 0 | boot word `0x2100b007`, 1,000,000 instructions |
| 47 kΩ | **3982.300059495949** | 3,982.3001 | 300 | `bridge.fault_state` on `SENSORS`, nets `I2C_SCL`, `I2C_SDA` | `fail`, exit 1, two `analog_rise_exceeds_ceiling` naming the bus, the net, the rise and the 300 ns ceiling | the same, as a separate `boot` record |

- **§3.3:** each case ran twice, and the `analog` and `faults` arrays were byte-identical as JSON text. The same value came from the bridge's own interpolation and from ngspice's `meas` (to its 7 significant figures).
- **Wall time** per `simulate --firmware` call, bridge and boot together: 2.80–3.16 s.
- **"The BME280 is absent from the platform."** Before `AET-WO-2202` lands, the emitted platform declares no I2C device at all, so this holds at both resistances and proves nothing yet. The test asserts it on Renode's own `peripherals` listing, so it starts to bite when 2202's BME280 declaration arrives. What is checked today is that the fault decision reaches the platform Renode loads: its header names the faulted bus (tier 1, `a_line_past_its_ceiling_fails_names_its_bus_and_reaches_the_platform`, whose stub Renode refuses a platform without it). See F5.
- **The firmware's observation** is the boot witnesses, as the work order says for before 2202.

## What was built

1. **`aethelgard-ir`.** `SolvedTopology.analog_lines: BTreeMap<net, AnalogLine { bus, pull_up_ohms, capacitance_femtofarads, ceiling_ns, domain_millivolts }>`, with `unique_keys`, filled by `solve` from the bus (pull-up, domain) and the net constraint (capacitance, rise-time bound). `validate` refuses:
   - a key that is not a solved I2C line (`AnalogLineNotI2c`: no assignment, a GPIO net, or an assignment that is not `I2C<n>`/`I2C<n>_SDA|SCL`);
   - a bus with no ceiling (`UnknownBus`), and two lines of one peripheral naming different buses (`AnalogBusMismatch`);
   - a zero value, by the existing variants (`ZeroPullUp`, `ZeroCapacitance`, `ZeroRiseTime`, `ZeroNominalVoltage`), with `id` the net;
   - a solved I2C line with no entry (`MissingAnalogLine`);
   - a missing field, through serde.
2. **`aethelgard-twin::analog`.** `decks(topology)` gives one `Deck` per line: S2's netlist (R from the rail to `line`, C to ground, `S1` with `vt` at half the rail, `Vctl … external`), S1's `.options` and `.tran` verbatim, and the stimulus (0x7F with NACK, nine 2.5 µs bits, 1 ns edges) as integer picosecond/millivolt points. Every value is written from an integer, so ngspice reads what the topology says. `platform::emit_faulted(topology, buses)` writes the fault decision into the platform's header and is where 2202's device declarations must skip a faulted bus; `emit` is `emit_faulted` with no bus.
3. **`bridge/`**, excluded from the workspace beside `boards`, builds `aethelgard-bridge`:
   - `#![deny(unsafe_code)]` at the root and `#[allow(unsafe_code)]` on `mod ffi` only. `workspace_shape.rs` `the_bridge_keeps_its_unsafe_in_ffi_alone` pins this, the single `serde_json` dependency and `publish = false`.
   - **`ffi`**: the hand declaration. **9 lines carry the `unsafe` keyword** (the `extern` block and eight blocks), and **64 lines sit inside unsafe regions** (the adversary's count, which the lead re-checked: the `extern` block is 28 of them). The module doc mentions the keyword once more. The callbacks receive no Rust pointer: host state is behind one `Mutex` in `host.rs`.
   - **The header:** `vendor/ngspice-44.2/sharedspice.h` (sha256 `4041c4e2…`, the spike's), ngspice's `COPYING`, and `SOURCE.md` pinning both hashes, which `workspace_shape.rs` checks in `gates`. Both files were fetched from SourceForge at the `ngspice-44.2` tag on 2026-11-09. `header.rs` parses the header and `ffi.rs` and checks every function, callback typedef, struct and line citation, with a mutant self-test (11 wrong declarations, each reported).
   - **The solver trait** (`solver.rs`) has the FMI 3.0 Co-Simulation shape: `instantiate`, `set_input`, `do_step(current, step)` returning `early_return` and `last_successful_time`, and `get_output`. `Ngspice` takes one step from 0 per instance; in-run stepping is a later work order's.
   - **The measurement:** `ngSpice_Init_Sync`, a `SetBkpt` at every vertex of the stimulus, a synchronous `run`. The rise is the bridge's own linear interpolation of the accepted points, from the first point at or after the release, which must be below 30 %. It is cross-checked against `meas … TD=<release>`, read back as the vector `meas` leaves, and a disagreement beyond 0.01 ns is an error.
   - **The request** is refused unless it is exactly the documented shape: integers only, a card allowlist (a `* ` title, R/C/S/V elements, `.model`, `.options`, `.tran`, `.end`; no other comment, after A1), one external source, at most 16 MiB.
4. **The CLI.** `simulate --firmware` runs the pre-pass after its input checks and before Renode is looked for (`src/analog.rs`), in a directory of its own (`<scratch>/bridge`), bounded by `RUL-13`'s 30 s and 1 GiB. The verdict gains `analog` and `faults` beside `boot`. The README has a new section, "The analog pre-pass".
5. **Images and CI.** `engine.Dockerfile` fetches and builds the bridge in the one `RUN` and installs it beside `aethelgard`; the engine layer test now requires exactly those two binaries, and `engine-smoke.sh` runs the bridge on `container/fixtures/bridge-request.json` (186.401 ns). `ci.yml`: `gates` checks the bridge's fmt and clippy (neither links); `tools` builds it beside the CLI with the network, then tests it offline, before tier 2. `docs/releasing.md` says it is never published.

## Golden digests that moved (§1.1)

| Digest (`crates/aethelgard-cli/tests/solve_cli.rs`) | Before | After |
|---|---|---|
| Reference solve output (`REFERENCE_DIGEST`) | `9e5b38ff615c734c23c7664486ee6d42476e4fdfd77be5243be4fc40a7991ec9` | `1b98f4c626d7e6cbae798d42514364edcfb8a89aa2cba6a9a19ce878a1cb4049` |
| Stress fixture (`STRESS_DIGEST`) | `77fd96e2d59875ae631b26b2a55085b099779bc62ce7c206653f0eafbb828310` | `d5f2741684970dc6e43738d2522d88d1fa741702f4e174f25a1db43a286d9300` |

The committed `boards/sentinel-01/` regeneration did not move: the board crate and the `.repl` carry the specification's sha256, not the topology's.

## Tests changed

- `aethelgard-ir/tests/validate.rs`: the fixture topology gains two analog lines, and the two hand-written sorted-order strings gain `"analog_lines":{}`.
- `aethelgard-twin/src/tests.rs`: the reference topology gains its analog lines. `what_cannot_be_lowered_is_refused` renamed `SPI1` rather than `I2C1` to reach `UnsupportedPeripheral`, and renamed both the peripheral and its signals for `Undefined`, because an I2C line renamed to a non-I2C name is now refused by `validate` first.
- `aethelgard-svd-hal/src/board.rs` (tests module only): the fixture gains analog lines. A topology with I2C lines and no ceilings is now `InvalidTopology` from `validate`, not the generator's `CeilingMismatch`; the other two cases adjust their analog lines to stay valid.
- `aethelgard-cli/tests/boot_tier1.rs`: every boot now runs a hard-linked copy of the CLI beside a stub bridge (the pre-pass runs before every boot). Five new tests cover the pre-pass.
- `aethelgard-cli/tests/workspace_shape.rs`: `exclude` is `["boards", "bridge"]`, plus the bridge test and its scanner's own test.

## The fence

Inside it as written: `aethelgard-ir` (`lib.rs`, `solve.rs`), `aethelgard-twin` (`analog.rs`, `platform.rs`), the CLI (`analog.rs`, `main.rs`'s `simulate`, `verdict.rs`'s two new records, README), `bridge/`, `container/engine.Dockerfile` and `engine-smoke.sh`, `ci.yml` and `engine-image.yml`, one line of `docs/releasing.md`, tests, this file. Edges, each named:
- **root `Cargo.toml`**: `exclude` gains `bridge`, which §1.3 requires ("excluded from the workspace, as `boards/` is");
- **`aethelgard-twin/src/lib.rs`**: `pub mod analog` and the `UnknownFaultedBus` variant;
- **`container/fixtures/`**: the smoke test's request file, and two sentences in its README;
- **`aethelgard-svd-hal/src/board.rs`**: its tests module only.

Nothing of `AET-WO-2202`'s fence was touched except shared test files (`twin/src/tests.rs`, `boot_tier1.rs`, `twin_tier2.rs`), where the merge will meet.

## Findings

- **F1 · `AnalogLine` carries `domain_millivolts`, a fifth field the §1.1 sketch omits · `aethelgard-ir` · medium · scope · decision-needed (ratify).**
  - §1.2 asks for "R to the domain voltage", and the bridge's 30 % and 70 % thresholds are fractions of it. `SolvedTopology` holds no voltage, and the twin never reads the `Specification` (`RUL-16`).
  - So `solve` copies the bus's domain's `nominal_millivolts` into each line. Hard-coding 3.3 V would have been a default (`INV-06`).
  - Recommendation: ratify the field as built. The alternative, a separate `domain_millivolts` map keyed by bus, adds a map and a cross-check for the same fact.
- **F2 · The header test caught a real mismatch in the first `ffi.rs` · `bridge/src/ffi.rs` · low · fixed.**
  - `ngSpice_Init`'s `SendData*`, `SendInitData*` and `BGThreadRunning*` were declared `*mut c_void`. That is ABI-equal, since null is passed, but the signature differed from the header.
  - The fix declares the three callback types and the four structs they name, exactly. They are never called or read; no line of `unsafe` was added.
  - The spike's `ffi.rs` (2101 S1) had the same three `void*`, so 2101's "declared from the pinned header" was not quite so.
- **F3 · `ngSpice_nospiceinit` segfaults in Debian's 44.2 when called before `ngSpice_Init`, as the header says to call it · libngspice0 44.2+ds-1 · low · measured.**
  - A ctypes probe in the image died with SIGSEGV inside the call (exit 139), before any output.
  - The bridge does not call it. ngspice therefore reads `$HOME/.spiceinit` if there is one, and under the CLI `HOME` is the bridge's own fresh directory, so there is none. Run by hand outside the sandbox, a user's `~/.spiceinit` would be read.
  - Recommendation: none needed under the CLI. Note it for a later in-process route.
- **F4 · A netlist can run shell commands, so the bridge accepts only the cards the twin writes · ngspice 44.2 · medium · measured, guarded.**
  - `ngSpice_Command("shell echo hi")` printed `hi`, from the probe in the image. A `.control` block in a deck runs such commands at load.
  - The request parser allows only a title, comments, R, C, S and V elements, and `.model`, `.options`, `.tran` and a final `.end`. It allows one external source, and printable ASCII only. `bg_` commands are refused in `ffi::command`.
  - The sandbox is the boundary regardless: no socket, Landlock on the filesystem, and its own directory.
- **F5 · Device omission is a mechanism today and a behaviour only after `AET-WO-2202` · `platform.rs` · high for the merge · scope · decision-needed.**
  - Today's platform declares no I2C device, so at 47 kΩ the firmware sees nothing different. Only the platform's header (the fault note) and the verdict change.
  - 2202 adds device declarations to `platform::emit`. Whichever of the two merges second must make those declarations skip every bus in `emit_faulted`'s set. The CLI already passes the recorded fault set, and the tier-2 test already asserts that no `BME280` is in Renode's listing at 47 kΩ.
  - Until then, §3.2's TXIS observation is not reachable. Recommendation: the 2202 merge wires its declarations through `emit_faulted`, and its tier-2 negative becomes this one's 47 kΩ case.
- **F6 · "`PlatformNotEmitted` compares against the emission for the specification plus the recorded fault decision" is circular if read as the `--platform` input · `main.rs` · medium · interpretation · decision-needed.**
  - The caller writes `--platform` before the pre-pass decides anything.
  - Built as two checks:
    1. `--platform` must still be the emission for `--spec` with no fault, as since 2100;
    2. the file the boot loads is written by the CLI as `emit_faulted(topology, faults)`, and just before Renode starts it is read again and compared with an emission computed from the verdict's own `faults` records.
  - Recommendation: ratify. The alternative, making the caller pass the faulted platform, needs a second CLI call to learn the fault first.
- **F7 · A rise that does not complete inside the 22.5 µs window is refused, not failed · `bridge/src/measure.rs`, the CLI · medium · design · decision-needed.**
  - The line has 20 µs after its release to reach 70 %. `t70` = 1.204·RC, so RC above about 16.6 µs cannot be measured: 47 kΩ with more than about 350 pF, or 2.2 kΩ with more than about 7.5 nF.
  - The bridge reports `rise_incomplete`, and the CLI stops with `ToolError` (exit 5). It is never read as unfaulted, but it is not a `fail` verdict either.
  - The adversary's A6 measured the edges in the image through `simulate --firmware`. 47 kΩ / 350 pF fails at 13,938 ns. 47 kΩ / 400 pF exits 5 (`rise_incomplete`, peak 2.16 V). So do pull-ups of `u32::MAX`, capacitance `u64::MAX`, and pull-ups of 1–23 Ω (`line_not_low_at_release`: the 10 Ω switch cannot pull the line below 30 %).
  - None of these is a pass. But a pipeline that reads exit 5 as an infrastructure flake loses a real bus fault, and the refusal blames the tool for what is the specification's fault.
  - Options:
    - (a) keep the refusal;
    - (b) record a fault with a lower bound (`rise > 20 µs − t30`);
    - (c) lengthen the window for slow lines. S1's window is what the work order pins.
- **F8 · `bridge_unavailable` is the one snake_case refusal kind · `src/analog.rs` · low · naming · decision-needed (one word).** Every other refusal kind is CamelCase (`ToolMissing`, `PlatformNotEmitted`); snake_case is the finding convention. It was built literally as §1.4.2 names it. Recommendation: rename to `BridgeUnavailable`, which is one line plus the README.
- **F9 · L6 (Renode's history to `/dev/null`, and the 100,000-command test) is not built · the ruling · info · scope.** In the static path the bridge does not own Renode. The boot is the CLI's single `-e` command line from 2100, not a lock-step monitor, so the history does not grow per quantum. L6 belongs with the in-run lock-step work order.
- **F10 · `meas` stores its result rounded to 7 significant figures · ngspice 44.2 · low · measured.**
  - The vector `meas` leaves holds `1.864014e-07`, not the full double. The 0.01 ns agreement check therefore has 0.0001 ns of `meas` rounding at 186 ns and 0.001 ns at 3,982 ns.
  - Near the window's 20 µs limit the rounding approaches the tolerance, and a near-limit line could be refused as `methods_disagree`. That fails closed.
- **F11 · `cargo install aethelgard-cli` gives a CLI with no bridge · releasing · info.** Outside the engine image, `simulate --firmware` is now `BridgeUnavailable` (exit 4). `docs/releasing.md` says the bridge ships only in the engine image. The README's quickstart does not use `--firmware`.
- **F12 · `cargo build --manifest-path bridge/Cargo.toml` without `CARGO_TARGET_DIR` builds into `bridge/target/` · repository hygiene · low.** That directory is not in `.gitignore`, which is outside the fence. `bridge/.cargo/config.toml` puts a build run from inside `bridge/` in the top-level `target/`, beside the CLI. CI and the engine image always set `CARGO_TARGET_DIR`. Recommendation: add `/bridge/target/` to `.gitignore` at the next docs pass.
- **F13 · `podman run --memory-swap X` with `--memory X` leaves `memory.swap.max` at X under cgroup v2, not 0 · Furnace practice · low · measured.** Read back from `libpod-<id>.scope` (above). The memory cap holds. A container can still swap up to X more. Recommendation for Furnace's contract: read `memory.swap.max` as well, and pass `--memory-swap` as the memory value plus 0 explicitly if podman offers that, or accept swap.
- **F14 · Hand-written topologies can still carry values a specification never yields · `validate` · low · backlog.** The CLI only ever solves a specification, so neither of these is reachable through it (adversarial A8):
  - `ceiling_ns` is not tied to the bus's `i2c_ceilings` Hz, so a hand-written `ceiling_ns: u32::MAX` validates. `solve` refuses it (`RiseTimeTooSlow`).
  - `validate` could require `i2c_ceilings[bus]` to equal the mode of the bus's smallest `ceiling_ns`. That moves several existing fixtures, so it is left for a ruling rather than built.
- **F15 · The CLI does not identify the bridge binary · `src/analog.rs` · low · design.** The CLI checks no version handshake and no hash, only the answer's shape and its `"ngspice":"44.2"`. Locally, `cargo test --features with-tools` does not rebuild the bridge, so a stale one runs silently; CI builds it first. A stub that lies with a well-formed answer passes, which is the trusted-binary assumption every tool here makes.
- **F16 · `booted_is_emitted` is the WO's check, but no test can make it fire · `main.rs` · info · mutation survivor.** It re-reads, just before Renode, a file the CLI wrote moments earlier in a directory no tool can write (the bridge's own directory is separate, A9). Its mutant (`if false`) survives. Swapping the boot plan's platform for the input copy, or emitting without the faults, is killed by tier 1. It is kept as §1.4's literal check.
- **F17 · Two copies of one fact across the crate boundary · `bridge/src/main.rs` `decimal`, `aethelgard-twin` `analog::decimal`; `container/fixtures/bridge-request.json` · low · reduction finding.**
  - The bridge may depend on `serde_json` alone (§1.3), so it cannot share the twin's 9-line formatter, and the CLI and the bridge each hold `"44.2"`.
  - The smoke fixture is the twin's reference request written out; nothing regenerates it, and the smoke's expected 186.401 ns would catch a drift that mattered.

## Adversarial pass (full, §5)

A fresh Opus subagent, in its own worktree detached at `91de9e9` (the build before this section), attacked all five items. It ran the real bridge, Renode and sandbox in the image, from its own runner and cache in `~/.cache/aet-2200-adv/` on Furnace. The lead reproduced A1 and A5 before fixing them.

| | Finding | Severity | Disposition |
|---|---|---|---|
| A1 | ngspice runs a card starting `*#` as a command, and a title `*ng_script` turns the deck into a script. Both passed the allowlist's "any `*` card" rule. `*# alter R1=2200` in a 47 kΩ request measured **186.40 ns with exit 0** (reproduced by the lead), and `*# shell touch …` wrote a file. Not reachable from a specification: the title is the only card holding spec text, and it starts `* aethelgard-twin`. | high | **Fixed:** the title must start `* `, and no other card may be a comment. Five new hostile cases in `request::tests`. The repro is now `request_refused`, exit 2. |
| A2 | One bus id on two peripherals validated, and `emit_faulted` named the first peripheral. After 2202, the wrong devices would be omitted. Hand-written topology only. | should-fix | **Fixed:** `AnalogBusMismatch` also refuses one bus on two peripherals, with a test. |
| A3 | The methods-agree check's mutant survived: nothing fed a disagreeing `meas`. | test gap | **Fixed:** the check is `agreed()`, tested on agreeing, disagreeing, missing and NaN values. |
| A4 | The external-source count's mutant survived. | test gap | **Fixed:** a request with no external source is a hostile case. |
| A5 | No `SOURCE.md`, and nothing pinned the header's hash. | should-fix (§1.3 as written) | **Fixed:** `SOURCE.md`, and a `gates` test that hashes both files against it. |
| A6 | A bus too slow to finish rising, or a line that cannot be pulled low, is exit 5, not a `fail`. | should-fix (classification) | **Decision-needed:** F7. |
| A7 | The unsafe scanner missed a raw byte string ending in `\`, never scanned `build.rs`, and did not stop a function-level `allow(unsafe_code)` in `main.rs`. | note | **Fixed:** `b`/`c` raw-string prefixes, `build.rs` scanned, exactly one `allow(unsafe_code)` in `main.rs`, and the attack's string is a test case. |
| A8 | Hand-written `ceiling_ns: u32::MAX` validates; no bridge identity; a 1 fF line's rise is the switch ramp, not RC; no bridge from `cargo install`; `svd-hal` test edit. | notes | F14, F15, F11, and "The fence". The 1 fF case passes nothing by accident. |
| A9 | The sandbox held: a hostile stub bridge could not read or write the tool copies, the status file, `/cache`, or another process's `environ`; its `setsid` child was killed. | held | — |

**Held, as the adversary ran them:**
- Lines in either order, alone, three repeats, and 64 lines in one process: byte-identical.
- Hostile specification values are refused by the schema, `solve` or the bridge.
- The 300 ns boundary: 3,540 Ω measures 299.94 ns and passes; 3,541 Ω measures 300.02 ns and fails.
- 1 mV and 5 V rails measure correctly, and a slow SCL with a fine SDA faults only `[SCL]`.
- Refused: `bg_run`, `source`, `pre_alter`, `.options interp`, a second `.tran`, `{1/0}` and `pwl file=`.
- `ControlledExit` gives exit 3.
- The FFI matches the header line by line, every pointer is copied before return, and no lock is held across a call.

One open item: `cast_mut()` passes `char*` that ngspice is not expected to write. The adversary did not check that against ngspice's source, and neither did the lead.

**Mutation, one mutant per guard this build adds** (run by the adversary; A3 and A4 then fixed): 29 mutants. Killed: every `validate_analog_lines` refusal, each `answers()` field check, `exceeds`, `measure::rise`'s release and low-at-release, the card allowlist (four), the platform the plan boots, the fault note, the FFI types, and the scanner's comment skip. Survived:
- the methods-agree check and the external-source count, now killed (A3, A4);
- `answers()`'s `is_finite`, equivalent, since serde_json refuses `1e400`;
- `booted_is_emitted` (F16);
- `SetBkpt`'s return type, which is killed by the compiler, not a test.

## Gates

- **Reduction pass (diff):** −30 net lines · 1 dead path removed (`twin::analog::render`, reached only by a test, which now compares the `Deck`s; `decimal` lost its `fmt::Write`) · 0 duplicates removed · 0 protected kept · 2 findings above (F16, F17).
  - Considered and not cut: `request::Line` holds each time and threshold as an integer and as a float, and `Ngspice::inputs` holds the waveforms `host` also holds. Removing either costs as many lines as it saves.
- **Local, in the tools image** (`--network none`, after the adversarial fixes):
  - bridge: `cargo test` 10 passed, and clippy `-D warnings` is clean;
  - workspace: `cargo fmt --all -- --check` is clean; clippy `--all-targets --features aethelgard-cli/with-tools -D warnings` is clean;
  - `cargo test --workspace --features aethelgard-cli/with-tools`: **48 suites, 618 passed, 0 failed**, tier 1 and tier 2.
- **Local, on the Furnace host** (no libngspice, no Renode, as in `gates`): `cargo test --workspace` and clippy are green. The bridge's fmt and clippy pass without libngspice.
- **Engine image, built locally with podman:**
  - its own layer is exactly `usr/local/bin/aethelgard` and `usr/local/bin/aethelgard-bridge`;
  - `engine-smoke.sh` passes, `bridge.measure` included.
- **CI:** below, once the PR runs.

## Amendments after review (PR #70)

The architect's review ruled every finding. Two changes were asked for in this PR, and both are built.

**F7, option (b): what the design does to a line is a fault, never a stop.**
- **The bridge** reports one of three outcomes per line, each with exit 0:
  - a rise, `rise_30_70_ns`;
  - a rise still unfinished when the window ended, `complete: false` with `rise_30_70_ns_lower_bound`;
  - a line never low, `volts_at_release`.
- **The bound** is the window left after the 30 % crossing, or after the release if the line never got there. A charging line slows as it rises, so its 30–70 % rise takes longer than its 0–30 % rise did, and an unfinished rise is at least 4/7 of the 20 µs after the release: 11.4 µs, past any I2C ceiling.
- **The cross-check** still holds: ngspice's `meas` must agree, the same rise to 0.01 ns or no rise at all.
- **Exit 1** now means only that the bridge failed on a line (`methods_disagree`, `solver_failed`, `early_return`, unreadable vectors).
- **The CLI** records an unfinished rise as `analog.rise_time_measured` with `complete: false` and the bound, fails with `analog_rise_exceeds_ceiling` stating it as a lower bound, and faults the bus. A line never low is an `analog.line_stuck_high` record, fails with `analog_line_stuck_high`, and faults the bus.
- **`faults[].from`** lists each distinct cause on the bus. Exit 5 is only for a bridge that fails.
- **A bridge bug the change exposed, and fixed:** asking ngspice for a vector a failed `meas` never made prints `Error: vector … not found!`. That line was then read as the next command's error, so the line after an unfinished rise failed. `Ngspice::meas` now drains the log after its read.
- **The A6 edges, measured with the real bridge in the image** (`bridge/tests/edges.rs`, tools job; each line alone equals it among the others):

| Line | Outcome |
|---|---|
| 2.2 kΩ, 100 pF | rise 186.40139798114484 ns |
| 47 kΩ, 400 pF | unfinished, `rise_30_70_ns_lower_bound` 13,298.0277 ns |
| 10 Ω, 100 pF | stuck high, 1.65 V at release (the 10 Ω pull-up against the switch's 10 Ω) |
| 47 kΩ, 350 pF (checked by hand) | rise 13,938.05 ns, the adversary's figure |

- **The same two edges at tier 1:** `boot_tier1.rs` `a_rise_unfinished_in_the_window_or_a_line_stuck_high_is_a_fault` runs `simulate --firmware` on specifications at 47 kΩ / 400 pF and at 10 Ω, with a stub bridge answering what the real one does. Each is a `fail` with exit 1, with the records, findings and fault above, and a booted platform that carries the fault.
- **One fact the test met:** `serde_json` without its `float_roundtrip` feature reads a float to within one ulp. So the CLI records 13298.027700387644 where the bridge wrote …645. Both are deterministic, so `INV-04` is unaffected.

**F8:** the refusal is `BridgeUnavailable`, in the CLI, its README and the tier-1 test.

**The other rulings, recorded:**
- F1 and F6 ratified as built.
- F5: this PR merges first, and `AET-WO-2202` wires its device declarations through `emit_faulted`.
- F9 and F11 accepted.
- F12: `.gitignore`, in the architect's next docs pass.
- F13 goes to Scotty.
- F14, F15 and F17 go to a hardening work order.
- F16 accepted.

**Also in this push:** the unsafe scan now covers `bridge/tests/`, since integration tests are crates of their own, outside the root's deny.

## Merge of `main` after `AET-WO-2202` (the architect flipped the merge order)

`AET-WO-2202` merged first, at `b924367`. It declares every modelled I2C device in the emitted platform (U2, a BME280 at 0x76), makes the firmware read each one's identity, and adds the judge's `boot.i2c` records and `aethelgard-ir::parts`. `origin/main` is merged into this branch with a merge commit.

**Conflicts, resolved by hand:**
- `twin/src/lib.rs`: both sides' error variants are kept.
- `boot_tier1.rs`, `twin_tier2.rs`: both sides' tests are kept.
- `twin/src/platform.rs`: below.

`Cargo.lock` and the goldens did not conflict. The solve digests are unchanged by 2202, and the committed `boards/sentinel-01/` is `main`'s, which the regeneration test confirms.

**The device declarations go through `emit_faulted` (ruled F5, as amended):**
- `platform::emit(topology, devices)` is `emit_faulted(topology, devices, ∅)`.
- `platform::present(topology, devices, faulted)` is the devices not on a faulted bus. `emit_faulted` declares only those, so a faulted bus declares no device, modelled or not. Its header note names what it left out: `U2 (BME280), U3 (TMP117)`.
- **The CLI:**
  - writes the booted platform from the specification's devices and the recorded faults;
  - expects Renode's listing to hold only the devices left in (`instances(topology, present(…))`, computed after the pass);
  - keeps every modelled device's identity read in the boot's expectations. The firmware still performs the read, so 2202's judge reports what it saw.
- `booted_is_emitted` re-derives from the devices as well.
- **The acceptance test is now the whole of §3:** U2's own record at both resistances, run at `--for 100ms` so a timed-out read still reaches the boot word, as 2202's absent-device test does.

**§3 on the merged tree**, in the tools image, by `p2_gate_clause_2_analog_half_47k_fails_and_2k2_passes`:

| Pull-up | `analog` | `faults` | `boot.i2c` (U2) | Verdict | Wall time per call |
|---|---|---|---|---|---|
| 2.2 kΩ | 186.40139798114484 ns on both lines, `complete`, under 300 | `[]` | `ok`, observed `0x60` | `pass`, exit 0. U3 is `device_unmodelled`, at info severity | 3.12–3.36 s |
| 47 kΩ | 3,982.300059495949 ns on both lines, past 300 | `SENSORS`, nets `I2C_SCL` and `I2C_SDA`, `firmware_sees` the model's rendering | `timeout_txis`, observed `0x0` | `fail`, exit 1: `analog_rise_exceeds_ceiling` ×2, then `i2c_timeout` ("U2 at I2C address 0x76: the firmware's wait for TXIS expired…"). Renode lists no BME280 | 2.98–3.00 s |

- **Each case ran twice:** `analog`, `faults` and `boot.i2c` were byte-identical as JSON text.
- **The whole suite in the image after the merge:** 49 suites, **650 passed, 0 failed** (tier 1, tier 2, the bridge's tests). Host tier 1, fmt and clippy are green as well.
