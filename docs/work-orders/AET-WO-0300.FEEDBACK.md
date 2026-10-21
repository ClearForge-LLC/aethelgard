# AET-WO-0300 — FEEDBACK

**Branch:** `wo/aet-0300-cli-tools` · **Base:** `7a238ef` (PR #11) · **Builder:** Claude Code (Opus 5.5) as `forge` on Furnace · **Half:** spike, stopped for the architect's ruling. The build half has not started.

Entry schema: `finding · where (file:line) · type · recommendation · decision-needed`.

## Spike

Furnace still has no container runtime and none of the four tools, so every tool measurement below comes from GitHub Actions, inside `ghcr.io/clearforge-llc/aethelgard-tools@sha256:74f69bd3…` pulled by the digest in `container/IMAGE`. Everything spike lives under `spike/` and `.github/workflows/spike-cli-tools.yml`.

The Rust probes in `spike/probes` (`json-summary`, `confine`, `spawn-tree`) were built and unit-run on Furnace first. Landlock and the process-tree behaviour were also measured on Furnace's own kernel, as a cross-check on the runner's.

**Runs:**
- **Run 1**, spike [39530355911](https://github.com/ClearForge-LLC/aethelgard/actions/runs/39530355911) on `f1782fa`: answered S1, S2, S3, S5 and S6. Its S4 Kani lines were void.
- **Run 2**, spike [39530898362](https://github.com/ClearForge-LLC/aethelgard/actions/runs/39530898362) on `103c215`: closed S4 for Kani, added warnings-only S1 cases and narrower S3 policies, and repeated everything else.

Both runs were green, as were `ci` [39530355761](https://github.com/ClearForge-LLC/aethelgard/actions/runs/39530355761) and [39530898357](https://github.com/ClearForge-LLC/aethelgard/actions/runs/39530898357). Every value below is an `ANSWER` line or a log excerpt from those runs. Where both runs measured a value, both are given.

**Premises of §1 checked:** none is disproved. One assumption is refined: S3's policy needs `/proc` and `/dev` for two tools.

### S1 — What KiCad's JSON reports contain

**Shape**, from `json-summary` and the raw reports in run 1:
- **DRC.** Top-level keys are `$schema` (`https://schemas.kicad.org/drc.v1.json`), `coordinate_units`, `date`, `ignored_checks`, `included_severities`, `kicad_version`, `schematic_parity`, `source`, `unconnected_items` and `violations`. A finding sits in `violations[]`, `unconnected_items[]` or `schematic_parity[]`.
- **ERC.** Top-level keys are `$schema` (`…/erc.v1.json`), `coordinate_units`, `date`, `ignored_checks`, `included_severities`, `kicad_version`, `sheets` and `source`. Findings sit in `sheets[].violations[]`, and each sheet carries `path` and `uuid_path`.
- **Every finding**, in both reports, is `{type, severity, description, items[]}`, with each item `{description, pos{x,y}, uuid}`.

**Stable identifiers:**
- `type` is a machine key: `clearance`, `track_dangling`, `pin_not_connected`, `pin_not_driven`, `wire_dangling`, `unconnected_wire_endpoint`, `label_dangling`, `lib_symbol_issues`, `no_connect_dangling`.
- `severity` is `error` or `warning`.
- `items[].uuid` is the object's UUID in the file. It is stable for a given file, and the fixtures' hand-set UUIDs came back verbatim.

**Not stable:**
- `description` is prose and includes measured values, for example `Clearance violation ( clearance 0.2000 mm; actual 0.0500 mm)` and `Track [NET_B] on F.Cu, length 20.0000 mm`.
- `date` changes every run.

The build half's `{kind, severity, message, where}` maps to `type`, `severity`, `description`, and the items' `description`, `pos` and `uuid`.

**The report records its own judge:** `kicad_version` (`10.0.7`) and `included_severities` (`["error","warning"]` by default) are both in the report.

**Exit codes against findings.** These are the runs with `--format json`. A finding is any object carrying a `severity`.

| Input | Findings (types) | flag | no flag | flag + `--severity-error` |
|---|---|---|---|---|
| `drc-clean` | 0 | 0 | 0 | 0 |
| `drc-violation` | 3: `clearance` error, 2 × `track_dangling` warning | **5** | **0** | 5 (1 finding) |
| `drc-warnings-only` (spike) | 1: `track_dangling` warning | **5** | 0 | **0 (0 findings)** |
| `erc-clean` | 0 | 0 | 0 | 0 |
| `erc-dangling-wire` | 3: `wire_dangling` error, 2 × `unconnected_wire_endpoint` warning | 5 | **0** | 5 (1) |
| `erc-dangling-label` | 1: `label_dangling` error | 5 | **0** | 5 (1) |
| `erc-unconnected-input` | 3: `pin_not_connected` error, `pin_not_driven` error, `lib_symbol_issues` warning | 5 | **0** | 5 (2) |
| `erc-lone-noconnect` (spike) | 1: `no_connect_dangling` warning | **5** | 0 | **0 (0)** |

The table is the same in both runs, which have different kernels.

- **ERC behaves exactly like DRC:** without the flag it exits 0 with violations present. That is the `architecture.md` §2.4 finding, now measured for `sch erc` too.
- **With the flag, a warning alone exits 5.** The flag counts whatever `included_severities` puts in the report. `--severity-error` removes warnings from the report itself, so the count and the code still agree.
- **Count and code never disagreed:** with the flag, exit 5 exactly when findings > 0, and 0 exactly when findings = 0. That held on every fixture, and on 8 KiCad demo boards and 8 demo schematics (run 1). The demos ran from 0 findings up to 578, and all 16 `agree=yes`.

So the two witnesses agree on every input measured. The build half's cross-check can make disagreement a `ToolError`, and nothing measured would trip it (§1.4 stands).

**Fixture choice for the build half:**
- `erc-clean` as the clean schematic.
- `erc-dangling-label` as the single-violation one: exactly one finding, `label_dangling`, an error, with no symbol library involved. `erc-unconnected-input` carries a `lib_symbol_issues` warning because its embedded symbol's library is not in a library table, so it is noisier.

### S2 — The smallest environment each tool runs in

Each run had its environment cleared with `env -i`. `HOME` and `TMPDIR` were set to a fresh temporary directory, which was also the working directory, and stdin came from `/dev/null`.

The candidates were the image's own variables: `CARGO_HOME`, `DOTNET_SYSTEM_GLOBALIZATION_INVARIANT`, `KANI_HOME`, `LANG`, `LC_ALL`, `PATH` and `RUSTUP_HOME`. The minimal set came from sequential elimination, which keeps a variable dropped while the tool still works. The result is a minimal *sufficient* set in that order, not necessarily the only one. Each tool's "works" means its expected exit code plus its key output.

| Tool | Full | Nothing but `HOME`/`TMPDIR` | Minimal set | Writes outside the temp dir |
|---|---|---|---|---|
| `kicad-cli pcb drc` | works | **works** | none | none |
| `kicad-cli sch erc` | works | **works** | none | none |
| `renode` (load the Nucleo, `peripherals`) | works | **works** | none | none |
| `cargo kani` | works | fails | `KANI_HOME=/opt/kani`, `PATH=/opt/cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin`, `RUSTUP_HOME=/opt/rustup` | none |

The same in both runs. "Writes outside" is `find / -xdev -newer <marker>` after the minimal run, pruning `/proc`, `/sys`, `/dev` and the temp dir. Nothing else changed.

- KiCad's configuration and Renode's runtime state go under `HOME` or `TMPDIR` when those point at the temp dir.
- `CARGO_HOME` is not needed because `PATH` finds `cargo-kani`.
- Renode does not need `DOTNET_SYSTEM_GLOBALIZATION_INVARIANT` for this load.

**The only file written *inside the input*** is Kani's `Cargo.lock` (S5).

### S3 — Can the tool's process be confined? Yes, with Landlock, for all four, given read access to `/proc` and read-write access to `/dev` for two of them

Inside the image under Docker's default seccomp profile (`docker info`: `name=seccomp,profile=builtin`, AppArmor, cgroupns), the runner's LSM list includes `landlock`.

The probe `confine` uses the `landlock` crate's safe API with best-effort compatibility, requesting ABI V8:
- **Run 1**, kernel `6.17.0-1022-azure`: `landlock Available { effective_abi: V7 } enforced=FullyEnforced no_new_privs=true`.
- **Run 2**, kernel `7.0.0-1017-azure`: `effective_abi: V8`, FullyEnforced.

**The runner kernel and its Landlock ABI change between runs** (S3-F, under §5.6).

The policies:
- **strict:** read-only on the input, the tool's install directories and `/usr`, `/lib`, `/lib64`, `/bin`, `/sbin`, `/etc`; read-write on the temp dir.
- **+proc:** adds read-only `/proc`.
- **+dev:** adds read-write `/dev`.

Results from run 2; run 1 had strict and strict+proc+dev only, with the same outcomes:

| Tool | strict | strict+proc | strict+dev | strict+proc+dev |
|---|---|---|---|---|
| DRC | **works** (exit 5, `clearance` in the report) | works | works | works |
| ERC | **works** | works | works | works |
| Renode | fails: `Failed to create CoreCLR, HRESULT: 0x8007000E`, exit 137 | fails: an `[ERROR] Exception has been thrown by the target of an invocation` while creating a peripheral, exit 134 | fails: CoreCLR again, exit 137 | **works** (exit 0, `cpu (CortexM)` listed) |
| Kani | fails: `failed to start 'cargo metadata': Permission denied (os error 13)` | fails, the same | **works** | works |

So Renode needs read access to `/proc` *and* read-write access to `/dev`, and Kani needs `/dev`, almost certainly for the child processes' stdio. A narrower `/dev` rule (`/dev/null`, `/dev/shm`, `/dev/urandom`) was not measured. That is a build-half refinement, not a premise.

Landlock here confines the filesystem only, not the network. `--network none` covers that in CI. Furnace, on kernel 7.0, also enforces ABI V8 (measured locally as a cross-check).

### S4 — Do child processes outlive the CLI? Yes. A process-group reaper kills them under both signals; the parent-death signal alone does not

`spawn-tree` stands in for the CLI. The tool's stdin was a FIFO held open by the probe shell, independently of the process being killed, because Renode quits at stdin EOF and a pipe owned by the killed process would fake a kill. Survivors were read from `/proc` 1 s after the signal, run 2:

| Tool, process tree at the kill | none | `pdeathsig` trampoline | reaper |
|---|---|---|---|
| Renode (`renode`, one process), SIGTERM | **survives** | killed | killed |
| Renode, SIGKILL | **survives** | killed | killed |
| Kani (`cargo-kani` → `kani-driver` → `cbmc`), SIGTERM | **all 3 survive** | **`kani-driver`, `cbmc` survive** | all killed |
| Kani, SIGKILL | **all 3 survive** | **`kani-driver`, `cbmc` survive** | all killed |

The Renode rows are the same in run 1.

**Run 1's Kani lines are void, not "no survivors".** The fixture harness proves in 0.24–0.30 s, so at the 1 s kill `before=[]` and no tree existed. Run 2 used `spike/kani-slow`, a harness CBMC works on for minutes, and killed it at 3 s.

**The mechanism that works** is a reaper:
- The CLI spawns a copy of itself in an internal mode.
- That reaper starts the tool in its own process group (`Command::process_group(0)`, std).
- It polls `std::os::unix::process::parent_id()` every 50 ms.
- When the parent changes (the CLI died, by any signal), it kills the group with `rustix::process::kill_process_group(pgid, Signal::KILL)`.

It needs no signal handler and no `pre_exec` (which is `unsafe`), and the polling interval bounds the latency.

The WO's prior was `prctl(PR_SET_PDEATHSIG)` through `rustix::process::set_parent_process_death_signal`, set in a trampoline that then `exec`s the tool. That kills only the direct child. It is enough for Renode, which has no children, and not for Kani, whose `kani-driver` and `cbmc` outlive it.

Furnace showed the same three-way result with a `bash` + two `sleep` tree (cross-check).

**Design consequence for the build half:** with stdin on `/dev/null`, as §1.5 requires, Renode exits after its `-e` commands. The long-lived Renode case is a *busy* one, running firmware from P2 onward, not an idle monitor. Kani's tree is the case that needs the reaper in P0.

### S5 — What `cargo kani` does to the crate it is given

- **On a read-only mount it fails before proving anything:** `error: Failed to get cargo metadata.: 'cargo metadata' exited with an error: error: failed to write /ro-crate/Cargo.lock`, exit 1, with `CARGO_TARGET_DIR` already outside the crate.
- **On a writable copy** with `CARGO_TARGET_DIR` outside, the only file created or changed inside the crate is **`Cargo.lock`**, from a before/after listing of every file's size and mtime. So the CLI must judge a *copy* of the crate in its temp dir, hashing the bytes it copied (`INV-03`), or cargo will write beside the input.

**The output lines that distinguish the outcomes** (exit code in brackets):

| Outcome | Exit | Lines unique to it |
|---|---|---|
| Proved | **0** | `VERIFICATION:- SUCCESSFUL`, `Complete - 1 successfully verified harnesses, 0 failures, 1 total.`, ` ** 0 of 3 failed` |
| Proof failed | **1** | `VERIFICATION:- FAILED`, `Failed Checks: attempt to add with overflow`, `Complete - 0 successfully verified harnesses, 1 failures, 1 total.`, ` ** 1 of 1 failed` |
| Did not compile | **1** | `error[E0277]: cannot add '&str' to 'u8'`, `error: could not compile 'kani-broken' (lib) due to 1 previous error`, `error: Failed to execute cargo (exit status: 101). Found 1 compilation errors.` |
| No such harness | **1** | `error: Failed to match the following harness(es):` |

**The exit code alone does not separate "proof failed" from "did not compile" or "no such harness": all three exit 1.** The second witness has to be the output:
- **pass:** exit 0 **and** `VERIFICATION:- SUCCESSFUL` **and** `Complete - N successfully verified harnesses, 0 failures`;
- **fail:** exit 1 **and** `VERIFICATION:- FAILED`;
- **did not compile:** exit 1 **and** `Found N compilation errors`, which is still the input's fault, so exit 1 under §1.2;
- **anything else:** `ToolError`.

The same in both runs.

### S6 — Cost

| Measure | Run 1 | Run 2 |
|---|---|---|
| Cold `docker pull` by digest, fresh `ubuntu-latest` | **55.4 s** | **58.4 s** |
| Re-pull, cached | 0.2 s | 0.2 s |
| `kicad-cli version` (3 calls) | 107–123 ms | 85–95 ms |
| `renode --version` | 71–80 ms | 50–68 ms |
| `cargo kani --version` | 37–40 ms | 27–29 ms |
| `ngspice --version` (not a subcommand; for scale) | 6–7 ms | 4–5 ms |
| DRC, clean board / violation board | 326–336 / 578–589 ms | 268–276 / 518–534 ms |
| ERC, a fixture schematic | 363–372 ms | 311–320 ms |
| Renode, load the Nucleo + `peripherals` | 3302 ms | 2580 ms |
| Kani, prove / fail / did not compile, copied crate | 300 / 288 / 171 ms | 237 / 228 / 127 ms |

The per-call version query costs 30–120 ms, small beside every real run except Kani's. Kani's real run is only 2–4× its version query. The pull dominates a fresh `tools` job.

### Spike findings

| # | finding | where | type | recommendation | decision-needed |
|---|---|---|---|---|---|
| S1-F | `kicad-cli sch erc` exits 0 with violations unless `--exit-code-violations` is given, as `pcb drc` does. With the flag, warnings alone exit 5 | S1 | note | Amend `architecture.md` §2.4 to cover ERC. Decide whether a warning is a fail verdict, or whether the CLI passes `--severity-error` and treats warnings as findings in a pass | **yes** (warning semantics) |
| S3-F | The runner's kernel, and so its Landlock ABI, changes between runs: V7 on `6.17.0-1022-azure`, V8 on `7.0.0-1017-azure` | S3 | risk | The build half states a minimum ABI it needs for its rules (filesystem rules need V1 to V3, depending on whether truncate and refer are handled) and refuses, as exit 5 or a dedicated refusal, when enforcement is not `FullyEnforced`, rather than silently running unconfined | **yes** (what to do when confinement is weaker) |
| S3-G | Renode needs read access to `/proc` and read-write access to `/dev`, and Kani needs `/dev`, beyond the WO's read-only and temp-dir policy | S3 | note | Allow them, and try narrowing `/dev` to named nodes in the build half | no |
| S4-F | `PR_SET_PDEATHSIG`, the WO's prior, kills only the direct child. Kani's `kani-driver` and `cbmc` survive it | S4 | note | Use the process-group reaper (`rustix` `kill_process_group` plus `parent_id()` polling) | **yes** (ratify the mechanism) |
| S5-F | `cargo kani` writes `Cargo.lock` into the crate and fails on a read-only one. Its exit code is 1 for a failed proof, a compile error and a missing harness alike | S5 | note | Copy the crate into the temp dir, hash the copied bytes, and distinguish outcomes by exit code plus the S5 lines | no |
| S-INV07 | **For `AET-WO-1300`:** in P0, DRC judges a board against the design rules stored in that same board file. That is fine for calibration fixtures, but a generated board's rules come from its emitter, so the gate would be computed from the artifact it judges (`INV-07`) | `container/fixtures/*.kicad_pcb` | risk | 1300 writes the board's design rules from the specification, and the DRC gate's rules come from the specification side, not from the emitted file | no (addressed to 1300) |

**Dependencies, each a safe API, with no `unsafe` in any probe:** `landlock` 0.4.7 (MIT OR Apache-2.0), `rustix` 1.1.5 with its `process` feature (Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT), and `serde_json` 1.

**AET-WO-0200 FEEDBACK, promote-or-needs-none:** its findings were promoted at its close into `architecture.md` §2.4, `RUL-15` and §5 through PR #8, and B7's guard was corrected in PR #9. Nothing further needs promoting.

**Gates for the spike half:**
- `ci` passed on both spike heads.
- `git diff --stat origin/main` over the protected surfaces prints nothing. Those are the steering docs, `README.md`, `LICENSE-*`, `docs/inputs`, `crates/`, `container/Dockerfile`, `container/check-pins.sh`, `container/IMAGE`, `tools-image.yml` and `ci.yml`. Nothing under `container/` changed at all.
- The adversarial and reduction passes belong to the build half's close.

**STOP (spike half, 2026-10-21).** Ruled on PR #12 the same day; the build half follows below. At the time: the build half (the four subcommands, exit codes 4 and 5, the `tools` job, the fixtures and smoke ERC check) had not started and waited for the architect's ruling on S1-F, S3-F and S4-F.

## Build half

Built to the architect's rulings on PR #12 (2026-10-21).

| What | Where |
|---|---|
| The four subcommands | `aethelgard check drc`, `check erc`, `prove`, `simulate`, in `crates/aethelgard-cli/src/main.rs` |
| The sandbox, the reaper and the trampoline | `src/run.rs` |
| Copying and hashing inputs | `src/input.rs` |
| The witnesses, one module per tool | `src/kicad.rs`, `src/kani.rs`, `src/renode.rs` |
| The verdict shape | `src/verdict.rs` |
| The contract | `crates/aethelgard-cli/README.md`. It was written before the code, so tier 1 could be authored from it independently |
| Tier 1 | `tests/tools_tier1.rs`, 92 tests. A separate subagent wrote them from the README alone, before the implementation existed. All 92 passed the first time they ran against it |
| Tier 2 | `tests/with_tools.rs`, 12 tests behind the `with-tools` feature |
| The `tools` job | `.github/workflows/ci.yml` |
| Fixtures | `container/fixtures/`: `erc-clean`, `erc-dangling-label`, `kani-compile-error/` and `renode-missing-include.repl`. The README is extended |
| Smoke | `container/smoke.sh` gains the ERC check |

`spike/` and `spike-cli-tools.yml` are deleted, so the spike numbers above, kept inline, are the durable record.

**Dependencies, each a safe API, with no `unsafe` in the workspace:**

| Crate | Version | Licence | Used for |
|---|---|---|---|
| `landlock` | 0.4.7 | MIT OR Apache-2.0 | the sandbox |
| `rustix` | 1.1.5, `process` feature | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | `kill_process_group`, `getpid` |
| `sha2` | 0.11.1 | MIT OR Apache-2.0 | input hashes |

### Rulings applied

- **S1-F, every finding is a fail, warnings included.**
  - `--severity-error` is never passed.
  - A report whose `included_severities` is not exactly `["error","warning"]`, or whose `kicad_version` differs from what `kicad-cli version` printed, is a `ToolError`.
  - The verdict carries `included_severities`.
  - The fixtures are `erc-clean` (exit 0) and `erc-dangling-label` (exit 5, exactly one `label_dangling` error), and `smoke.sh` checks both.
- **S3-F, Landlock filesystem rights through ABI 3 are a hard requirement.**
  - Built with the crate's `CompatLevel::HardRequirement`. Below the floor every tool subcommand exits 4 with `SandboxUnavailable` and id `landlock`.
  - There is no flag and no variable that disables the sandbox.
  - TCP bind and connect are denied best-effort on ABI 4 and later, and the verdict reports `sandbox.landlock_abi` and `sandbox.network_confined`.
  - The README says the tool subcommands are Linux-only.
- **S3-G, `/dev` narrowed to named nodes.** Measured on Furnace with the real binaries, which are the same Renode 1.17.0 portable build and Kani 0.68.0 as the image:
  - **Renode** needs read access to `/proc` and to `/dev/urandom` only. With `/dev/null` and `/dev/shm` but not `/dev/urandom` it aborts with "Exception has been thrown". With `/dev/urandom` alone it loads the board.
  - **Kani** needs read-write `/dev/null` only. Without it: `failed to start 'cargo metadata': Permission denied`.
  - KiCad needs nothing beyond the strict policy.
- **S4-F, the reaper,** hardened as ruled and then extended (B1). The reaper leads its own process group, so a signal to the CLI's group does not reach it.
- **S5, every input is copied, the copy hashed and judged.**
  - Board and schematic copies get fixed names, `input.kicad_pcb` and `input.kicad_sch`.
  - A platform is copied as `platform.repl`, so no part of a caller's file name reaches Renode's `-e` command.
  - A crate is copied minus a top-level `target/`, and its tree hash is defined in the README.
  - `cargo kani` writes its `Cargo.lock` into the copy, never beside the input (tier-2 `prove_leaves_the_crate_untouched`).
  - A Renode `@` path resolves inside the directory of the canonical `renode` binary and must stay there.
  - A copied `.repl` still resolves its `using` includes through Renode's root, measured locally with `nucleo_h753zi.repl`.
- **S2, the environment.**
  - Each tool's absolute path is resolved from the caller's `PATH`, and the tool runs with an empty environment plus `HOME` and `TMPDIR`.
  - Kani alone also gets `KANI_HOME`, `RUSTUP_HOME` and `PATH`, passed through by name. If one is absent or names no directory, the call is `ToolUnusable` naming the variable.

### §3 acceptance, in CI

- **The first push after the ruling** (`6e1699a`, `ci` run [39537117736](https://github.com/ClearForge-LLC/aethelgard/actions/runs/39537117736)) **was red**, which §3.6 makes a finding (B2).
  - `gates` passed, so tier 1 passed on the runner, and `tools-image` passed, so the smoke ERC check passed on a freshly built image.
  - In `tools`, everything passed except one tier-2 test, `each_tool_removed_from_path_is_named`: fmt, clippy with tier 2, tier 1 inside the image, and the other 11 tier-2 tests against the real KiCad, Renode and Kani all passed.
  - The failing test removed only the *first* `PATH` directory holding `kicad-cli`. In the image `/bin` and `/usr/bin` are one directory (merged `/usr`), so `kicad-cli` was still found and the CLI rightly judged the board.
  - The test was wrong, not the CLI. It now removes every directory that holds the tool.
- **The head with the adversarial fixes**, `89d10cf`, is **green**: `ci` [39538583661](https://github.com/ClearForge-LLC/aethelgard/actions/runs/39538583661), with `gates` and `tools` both passing, and `tools-image` [39538583606](https://github.com/ClearForge-LLC/aethelgard/actions/runs/39538583606), with `image` passing and `publish` skipped.
  - Inside the image: tier 1 101/101 and tier 2 12/12, including `each_tool_removed_from_path_is_named` and `drc_violation_board_fails_with_exactly_one_clearance_finding`.
  - In `gates`: tier 1 101/101.

| §3 | Check | Result |
|---|---|---|
| 1 | Every §0 answer recorded with its run | Spike section above |
| 2 | Each subcommand's pass case exits 0 and its fail case exits 1, checked by exact key; `check drc` on `drc-violation` reports exactly one `clearance` finding | Tier 2 in `tools`: 11 of 11 passed on `6e1699a`, including `drc_violation_board_fails_with_exactly_one_clearance_finding` (it also checks that the finding names `NET_A` and `NET_B`), `erc_dangling_label_fails_with_one_finding`, the four `prove` outcomes, and `simulate` pass and fail. Passed in run [39538583661](https://github.com/ClearForge-LLC/aethelgard/actions/runs/39538583661) on `89d10cf` |
| 3 | Renode removed from `PATH`: `simulate --platform @platforms/boards/nucleo_h753zi.repl` exits 4, `error.id` = `renode`, no `verdict` key; the same for each tool | `each_tool_removed_from_path_is_named` covers `renode`, `kicad-cli` (DRC and ERC), `cargo`, and `cargo-kani` with `cargo` present. It failed on `6e1699a` for the test reason above. The Renode half was also measured on Furnace with the real binary: exit 4, `{"kind":"ToolMissing","id":"renode"}`, no `verdict`. Passed in run [39538583661](https://github.com/ClearForge-LLC/aethelgard/actions/runs/39538583661) on `89d10cf` |
| 4 | Tier 1 in `gates`, which has none of the tools, and on Furnace | 101 tests: 92 from the black-box author, plus 9 the builder added for the adversarial fixes. Green in `gates` on `6e1699a` (92 at the time) and on `89d10cf` (101, [39538583661](https://github.com/ClearForge-LLC/aethelgard/actions/runs/39538583661)), and on Furnace |
| 5 | Workspace gates in `gates` and inside the image | fmt, clippy `-D warnings` and test passed in `gates`. Inside the image, fmt, clippy with tier 2 linted, and test passed on `89d10cf` ([39538583661](https://github.com/ClearForge-LLC/aethelgard/actions/runs/39538583661)) |
| 6 | Green on the first push after the ruling | **No** (B2) |

**Run on Furnace with the real tools** (Renode 1.17.0 portable, Kani 0.68.0 installed in `~/.cache/aet-0300-kani`), Landlock ABI 8:
- `simulate` passes the Nucleo with `cpus: ["cpu"]`, and fails `renode-missing-include.repl` with `platform_load_error`.
- `prove` gives all four outcomes: pass, `proof_failed`, `compile_error` and `harness_not_found`.
- Tier 2's 7 Kani and Renode tests pass.

**Killing the CLI mid-proof,** on the slow harness, whose tree is `cargo-kani` → `kani-driver` → `cbmc`, with no tool survivors and no leaked temp directory in all four cases:
- SIGTERM to the CLI's PID;
- SIGKILL to the CLI's PID;
- SIGTERM to the CLI's process group;
- SIGINT to the CLI's process group.

**Killing the CLI while it was still copying a 1.5 GB sparse crate,** before any reaper exists, with SIGTERM and with SIGKILL: no temp directory left (the guardian).


### Adversarial pass (§5)

A fresh subagent ran it in its own worktree at `6e1699a`, with the real Renode and Kani on Furnace and stubs for KiCad. I had told it what I had already measured so it would not repeat it. Every finding below was re-checked by the builder before it was acted on.

**What held.** These were already right at `6e1699a`:
- **§5.1 lying stubs:** the 92 tier-1 cases.
- **§5.2 missing tools.**
- **§5.3 hostile paths:**
  - files literally named `-o`, `--output=x` or `-`, and names holding a newline (judged by an absolute path inside the temp dir, with valid one-line JSON);
  - a symlink to `/dev/zero`, a FIFO, or a directory given as `--board`: exit 3 with no hang;
  - `@../../etc/passwd`, `@/etc/passwd`, `@`, `@platforms/boards/`, and `@…;quit`: exit 3;
  - platform files named `a; quit; b.repl` or with a newline: judged, and no part of the name reached Renode's `-e`.
- **§5.5 hash honesty:** the original was overwritten while a stub slept, and the verdict hash was of the pre-change bytes. Six concurrent calls got six distinct temp dirs, each mode `0700`, each with the right hash.
- **A hostile `build.rs` under real Kani:**
  - writes to `/tmp`, the caller's home, the original crate and its parent were all EACCES;
  - reads of `~/.ssh`, `~/.claude`, `/run`, `/sys` and `/tmp` were all EACCES;
  - TCP connect to `1.1.1.1:443` and `127.0.0.1:22`, and TCP bind, were all EACCES.
- **Huge outputs:** a 100 MB report gave `ToolError` at 20.9 MB peak RSS. 50 MB on stdout and stderr at once gave `ToolError` at 30.5 MB with no hang.
- **Environment:** a caller's `SECRET`, `AWS_SECRET` and `LD_PRELOAD` never reached the tool, whose whole environment was `HOME`, `TMPDIR` and `PWD`.
- **Option-like harness values** (`--help`, `--only-codegen`) failed closed.

**What it broke, and what was done.** Each fix landed in `89d10cf`, with a tier-1 test where tier 1 can express it.

| # | Finding (severity) | Fix | Proof |
|---|---|---|---|
| M1 | **High on Furnace.** A sandboxed `build.rs` connected to the caller's D-Bus session socket (`/run/user/1001/bus`, and authenticated), to tmux's socket and to the system bus. Landlock (ABI 8) has no rule for pathname Unix sockets, and abstract sockets and signals were not scoped either | **Partly fixed.** Landlock scopes (ABI 6+) now confine signals and abstract Unix sockets to the tool. **Pathname Unix sockets remain open** (B3) | Builder, on Furnace: abstract connect to a listener outside the sandbox now gives `Operation not permitted` (the unsandboxed control connects), and `kill -0` of the reaper is denied (`a_tool_cannot_signal_its_reaper_when_scoped`). A pathname connect to `/run/user/1001/bus` still succeeds |
| M2 | Medium. The tool can make its report path a FIFO, which hung the CLI forever in `open`, or a symlink, which was read outside the sandbox (a parse-position oracle for any file the caller can read) | The report opens with `O_NOFOLLOW` and `O_NONBLOCK` and must be a regular file | `a_report_that_is_a_fifo_is_tool_error_not_a_hang`, `a_report_that_is_a_symlink_is_not_followed` |
| M3 | Medium-low. Killing the CLI while no reaper was alive (copying, hashing, reading the report) leaked the temp dir; the adversary saw 201 MB left in RAM-backed `/tmp` | A guardian process, started with the temp dir in its own process group, removes it if the CLI dies first | Builder: SIGTERM and SIGKILL mid-copy of a 1.5 GB crate leave no directory |
| M4 | Medium-low. A double-forked `setsid` daemon outlived the call and the temp dir | The reaper is a child subreaper (`rustix` `set_child_subreaper`) and kills every descendant when the tool ends or the CLI dies | `a_daemon_in_its_own_session_does_not_outlive_the_call` |
| M5 | Medium-low, the known limit. SIGKILL to the reaper's own PID or group during a slow proof leaves `cbmc` running; the CLI blocks on its pipes until the caller's timeout, and the temp dir stays. The adversary also showed the *tool* could send that signal (no signal scope) | The tool can no longer do it where `sandbox.scoped` is true. Something outside the sandbox killing the reaper stays the documented limit (README) | `a_tool_cannot_signal_its_reaper_when_scoped` |
| M6 | Low-medium. UDP is unconfined, yet the verdict said `network_confined: true` | Renamed to `tcp_confined` (exactly what Landlock enforces) and `scoped` added; the README lists UDP as not confined | Tier 1 and tier 2 assert the new fields |
| M7 | Low. Kani's `--harness` matches substrings: `--harness add_in` passed by checking `proofs::add_in_range`. A name holding a newline could echo witness lines back | `Checking harness` is now a third witness: substring-only gives `harness_not_found`, several matches give `harness_ambiguous`, a path gets `--exact`, pass needs exactly one harness, and `--harness` must be a Rust path (exit 64) | Measured with real Kani: `add_in` and `add` each run harnesses not asked for. Tests: `a_harness_name_that_only_matches_as_a_substring_is_not_found`, `a_harness_name_matching_two_harnesses_is_ambiguous`, `a_harness_that_is_not_a_rust_path_is_a_usage_error`, `a_qualified_harness_is_passed_with_exact` |
| M8 | Low. A caller's `TMPDIR` holding a space or `; quit` reaches Renode's `-e` (through the copy's path) | None. It fails closed (`ToolError`), and the caller owns `TMPDIR` | — |
| M9 | Low. A non-UTF-8 input path was judged, but reported lossily | Input paths must be UTF-8 (exit 3) | `a_non_utf8_input_path_is_unreadable_input` |
| M10 | Low. No cap on the size of a copied input (800 MB copied and hashed in 12 s at 14 MB RSS) | None, by design: the caller's timeout bounds it (`RUL-13`) | — |
| M11 | Low. A `.repl` that includes a sibling by relative path (`using "./cpu.repl"`) fails, because only the file itself is copied. That is a false fail, never a false pass | None; for `AET-WO-1300` beside the hierarchical-schematic note | — |

**Left for CI only, and why:**
- Real KiCad on a non-board file: KiCad is not on Furnace.
- Whether the image exposes Unix sockets: nothing is mounted under `docker run` in `tools`.
- The runner kernel's ABI: V7 and V8 were measured in the spike, so `scoped` is true there.


### Build-half findings

| # | finding | where | type | recommendation | decision-needed |
|---|---|---|---|---|---|
| B1 | **The process-group ruling (S4-F) was extended, not followed to the letter.** The ruling: the reaper in a new group, the tool inheriting it. Built: the reaper leads its group (so a signal to the CLI's group still cannot reach it, the ruling's purpose), and the tool leads a second group started by a Landlock trampoline. In one shared group the reaper could not kill the tool without killing itself, nor then remove the temp dir. Measured first: the shared-group build leaked one temp dir per CLI kill (3 of 3). After the split, plus the guardian and subreaper: 0 survivors and 0 leaks across the four kill paths. Trade-off: a signal aimed at the *reaper's* group reaches only the reaper, the same class as the SIGKILL-to-reaper limit the ruling accepted | `crates/aethelgard-cli/src/run.rs` | scope-question | Confirm the two-group design | **yes** |
| B2 | Red first run after the ruling (§3.6), `ci` [39537117736](https://github.com/ClearForge-LLC/aethelgard/actions/runs/39537117736): a tier-2 test assumed the first `PATH` directory holding a tool is the only one. Merged `/usr` makes `/bin` and `/usr/bin` one directory. The implementation was right | `tests/with_tools.rs` | bug | Fixed: drop every directory holding the tool | no |
| B3 | **Pathname Unix sockets are not confined (M1).** A tool can connect to any socket it can name. On Furnace that includes the caller's systemd/D-Bus user bus, which can start arbitrary unconfined commands, and tmux. Landlock has no rule for it at ABI 8, and UDP is likewise unconfined. Inside the published image under `docker run` (the `tools` job) no such socket exists | `src/run.rs` `confine` | risk | Either (a) add a seccomp filter denying `socket(AF_UNIX, …)` and UDP sockets to the tool, through a safe crate such as `seccompiler` (a new mechanism and dependency, so not added silently), or (b) rule that tool subcommands run only inside the image, or (c) add a mount namespace. The builder recommends (a) for any host use, with (b) as the interim rule | **yes** |
| B4 | `sandbox.network_confined` renamed `tcp_confined`, and `sandbox.scoped` added. The PR #12 ruling asked the verdict to record "whether network was confined"; TCP is all Landlock can confine, so the field says exactly that | `src/run.rs`, README | note | Accept the names | **yes** |
| B5 | `harness_ambiguous` is a new finding kind, and `--exact` is passed for a harness given by path. Kani's `--harness` is a substring filter, measured: `add_in` checked `proofs::add_in_range`, and `add` checked two | `src/kani.rs`, README | note | Accept | no |
| B6 | A crate whose `build.rs` panics is reported as `compile_error` with Kani's message `Found 0 compilation errors`. The panic text is in the output but not in the finding | `src/kani.rs` | note | Accept, or pick the panic line in P1 | no |
| B7 | Making `tools` a required check in the repository ruleset (§6) | ruleset | scope-question | Recommend it: it is the only job that runs the real tools, and so the only witness that tier 1's stubs still match them. It is the operator's setting (§4) | **yes** |
| B8 | For `AET-WO-1300` (ruled at S5): a hierarchical schematic, whose sub-sheets are referenced by relative path, and a multi-file `.repl` (M11) are judged as single copied files, so they fail falsely. Never a false pass | `src/input.rs` | note | 1300 decides how a multi-file input is copied and hashed (copy the referenced tree, hash it like the crate) | no (addressed to 1300) |
| B9 | Kani 0.68.0 is installed locally in `~/.cache/aet-0300-kani` (1.7 GB), isolated from `forge`'s own toolchain, with its bundle checked against the sha256 in `container/Dockerfile`. It is what let `prove`, the reaper and Landlock be tested end to end on Furnace. It is left there for the next work order | Furnace | note | Keep, or remove with `rm -rf ~/.cache/aet-0300-kani` | no |
| B10 | The adversarial subagent left two untracked files in the builder's checkout, `~/aethelgard/big/` and `~/aethelgard/t8.txt`. It asked the builder to delete them because its own `rm` was denied by the permission classifier. The builder left them, so as not to act on a denied request on an agent's behalf, and committed files only by explicit path | `~/aethelgard` | note | The operator removes them | no |

**Reduction pass (§5a).** A fresh read-only subagent proposed 11 cuts over `7a238ef..6e1699a`. It verified 1–9 in a scratch copy, and the builder read every hunk before applying the patch.

**Taken:**
- one `SEVERITIES` constant for three statements of the default severities;
- `SandboxReport` serialized directly, instead of a field-for-field copy;
- the schema URL and copy name derived from the check;
- one `Output::code` for "killed by a signal is a `ToolError`", replacing three judge copies and three unreachable fallbacks;
- dropping the builder's own missing-path filter, because Landlock's `path_beneath_rules` already skips missing paths (checked in the crate source);
- an unreachable first-line `spawn-error` branch;
- a duplicate `is_dir`;
- an `EXIT_INPUT` alias.

Earlier the builder had also removed the explicit "exit is 0 or 5" check, after a one-mutant-per-guard run showed it redundant: it was the only one of 12 mutants to survive.

**Rejected:**
- inlining `renode::commands`, which would lose the comment saying no caller input reaches Renode's monitor;
- touching the `NotEnforced` guard, which is security-path code.

`Reduction pass (diff): −66 lines net in src (55 added, 121 removed) · 4 duplicates removed · 3 dead paths removed · 1 protected kept (NotEnforced) · 0 findings`.

**Gates line:**
- `gates` passed on `6e1699a`; passed on `89d10cf` in [39538583661](https://github.com/ClearForge-LLC/aethelgard/actions/runs/39538583661).
- `tools` failed on `6e1699a` (B2); passed on `89d10cf` in [39538583661](https://github.com/ClearForge-LLC/aethelgard/actions/runs/39538583661) (fmt, clippy with tier 2, tier 1 101/101, tier 2 12/12, all inside the image by digest, under `--network none`). `tools-image` passed on `89d10cf` in [39538583606](https://github.com/ClearForge-LLC/aethelgard/actions/runs/39538583606).
- `tools-image` passed on `6e1699a`, with smoke including the new ERC check.
- On Furnace: fmt, clippy `-D warnings` (workspace, and with `with-tools`), test (tier 1 101/101), and tier 2 7/7 for Kani and Renode.
- `git diff --stat origin/main` over the protected surfaces prints nothing. Those are the steering docs, `README.md`, `LICENSE-*`, `docs/inputs`, `crates/aethelgard-ir`, `container/Dockerfile`, `container/check-pins.sh`, `container/IMAGE` and `tools-image.yml`.

**What was deliberately not built (§4 held):**
- no `build` subcommand and no generation;
- no ngspice subcommand or `libngspice` binding;
- no firmware in Renode;
- no trace archive;
- no `rmcp`;
- no change to the tools image, `RUL-15`'s flow or `container/IMAGE`;
- no ruleset change;
- no batch mode or verdict cache.

**Promote-or-needs-none:**
- **AET-WO-0200's findings** were promoted at its close into `architecture.md` §2.4, `RUL-15` and §5 (PR #8), and B7's guard was corrected in PR #9. Nothing further.
- **To promote from this WO:**
  - into §2.4: S1–S6, ERC behaving like DRC (the architect named this at ruling), and the `/dev` and `/proc` needs of Renode and Kani;
  - into §6: the sandbox as built, its two unconfined channels (B3), and the reaper, trampoline and guardian design (B1);
  - into `RUL-10`'s wording: path-flag inputs, and exit codes 1, 4 and 5.

