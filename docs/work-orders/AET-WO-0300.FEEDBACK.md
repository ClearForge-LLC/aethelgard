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

**STOP.** The build half (the four subcommands, exit codes 4 and 5, the `tools` job, the fixtures and smoke ERC check) has not started. It waits for the architect's ruling on S1-F, S3-F and S4-F.
