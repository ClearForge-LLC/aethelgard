# AET-WO-0300 — The `aethelgard` CLI calls the tools: `check drc`, `check erc`, `prove` and `simulate`, each returning one trustworthy JSON verdict

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main` (named in the kickoff; verify live HEAD before branching).
**Branch:** `wo/aet-0300-cli-tools`.
**Author:** Claude (architect) · **Builder:** Claude Code session, tmux `aet` on Furnace, user `forge`, directory `~/aethelgard`.
**Phase:** P0 (Foundation) · **Phase exit gate:** `docs/roadmap.md` §5 P0. This work order owns the Renode half of gate clause 6: with `renode` removed from `PATH`, the run halts with an error naming it and produces no verdict. The remote-mirror half belongs to `AET-WO-0400`. It also makes the CLI contract that clauses 4 and 6 are judged through.
**Grounds:** `docs/northstar.md` §4 `INV-03`, `INV-06`, `INV-07`.
`docs/architecture.md`:
- §2.4, with the measured tool behaviour this work order must not rediscover: `kicad-cli` exits 0 on violations without the flag, a failed Renode load exits 0, and Kani is pre-installed;
- §4 `RUL-03`, `RUL-10`, `RUL-12`, `RUL-13`, `RUL-15`;
- §5, the operational facts;
- §6, the subprocess posture: argument arrays, no shell, a sandbox per invocation.

The code seams:
- `crates/aethelgard-cli/src/main.rs`: the `Refusal` and `emit` pattern, the exit-code constants, and the 16 MiB read;
- `crates/aethelgard-cli/README.md`: the exit-code contract;
- `container/IMAGE`: the published tools image, pinned by digest;
- `container/fixtures/`: the DRC boards, the RC netlist and the Kani crate.

> **What this is:** the CLI subcommands that run the external tools on artifacts handed to them:
> - `check drc` (KiCad `pcb drc`) and `check erc` (KiCad `sch erc`);
> - `prove` (Kani on one harness);
> - `simulate`: Renode loads a platform and reports what it built. That is all it does in P0.
>
> Each returns one JSON verdict whose pass or fail no crash, missing tool or lying exit code can counterfeit. The verdict names the tool version that judged and the sha256 of the input judged.
>
> It is NOT a `build` subcommand: nothing generates until `AET-WO-1300`, and a stub would put a lie in the command tree that `AET-WO-1500` turns into MCP tools. It is NOT an ngspice subcommand, because the analog side arrives in P2 through `libngspice` (`OQ-02`'s prior). It is NOT firmware running in Renode (`AET-WO-2100`), NOT the trace archive (`AET-WO-0400`) and NOT the `mcp` subcommand (`AET-WO-1500`).
>
> **Why now:** every later gate (P1's ERC and DRC, P1's Kani proof, P2's campaigns) is judged through these subcommands. P0 is where the judge gets proven honest, before there is anything worth judging.

**Cadence:** spike first, then STOP for the architect's ruling, then build. One PR for both halves, left unmerged.

**The constraint that shapes everything:** Furnace still has no container runtime, and none of the four tools is installed there. Write and unit-test the Rust on Furnace. **Every run of a real tool happens in GitHub Actions inside the published image**, `docker pull "$(cat container/IMAGE)"`; the package is public since 2026-10-20. Every push is a tap on Scotty's phone, so design the spike to answer all of §0 in one or two pushes.

## 0. Spike first — the questions the spike must answer, measured in CI

Push one spike commit carrying a temporary workflow that pulls `container/IMAGE` and runs probes inside it. Record every answer in `docs/work-orders/AET-WO-0300.FEEDBACK.md` under "Spike", with its CI run URL. Then STOP.

Probes are shell (CI glue, per `RUL-12`) plus, for S3 and S4, the smallest Rust program that answers the question. Put them all under `spike/`; the build half deletes them.

- **S1 What KiCad's JSON reports contain.**
  - Run `kicad-cli pcb drc` and `kicad-cli sch erc`, both with `--format json`, on clean and violating inputs. `container/fixtures/` has the boards. Add a clean schematic and one with a single deliberate ERC violation, for example an unconnected input pin.
  - Record which report fields identify a finding stably (type, severity, description, the items it names).
  - Record ERC's exit code with and without `--exit-code-violations`, as AET-WO-0200 did for DRC.
  - Record whether the report's violation count and the flag's exit code ever disagree.
- **S2 The smallest environment each tool runs in.**
  - Run each tool with its environment cleared except for an allowlist, `HOME` and the working directory set to a fresh temporary directory, and stdin from `/dev/null`.
  - Record the exact allowlist each tool needed, and everything each one writes outside that temporary directory. KiCad's config directory and cargo's `Cargo.lock` are the likely ones.
- **S3 Can the tool's process be confined?**
  - Inside the image on `ubuntu-latest`, under Docker's default seccomp profile, report which Landlock ABI version is available, if any.
  - Run each tool confined by Landlock to read-only access on the input, the tool's install directories and the system directories, plus read-write access on its temporary directory. Does it work?
  - Use the `landlock` crate's safe API. Record per tool: confined and working, or the exact failure.
- **S4 Do child processes outlive the CLI?**
  - Start Renode (the longest-lived) and `cargo kani` from a probe binary, then kill the probe with `SIGTERM` and with `SIGKILL`. Do the children survive?
  - Measure one mechanism that kills them in both cases without `unsafe` in a workspace crate. A dependency's safe wrapper, such as `rustix`'s `prctl` for the parent-death signal, plus a process group, is the prior.
- **S5 What `cargo kani` does to the crate it is given.**
  - With `CARGO_TARGET_DIR` pointing into the temporary directory, list every file `cargo kani --harness` creates or changes inside the input crate's directory. Run it on a read-only copy.
  - Also record the exact output lines that distinguish proved, failed, and did-not-compile.
- **S6 Cost.**
  - Wall time to `docker pull` the image by digest on a fresh `ubuntu-latest` runner.
  - Per-call latency of each tool's `--version` and of one real run on each fixture. Each verdict asks the tool its version, so that latency is paid every call.

**STOP** when §0 is answered: FEEDBACK's Spike section written, the PR open, every number cited with its run. A question that cannot be measured counts as answered when its reason is recorded. The architect rules by a comment on the PR and a new goal; then the build half starts.

## 1. Scope — the build half, after the ruling

The contract below is ratified unless the spike disproves a premise, in which case say so and stop.

1. **Four subcommands**, each a plain `clap` subcommand with help text. `AET-WO-1500` derives MCP tool schemas from the command tree, so there is no hand-written registry and no subcommand that does nothing.

   ```
   aethelgard check drc --board <file.kicad_pcb>
   aethelgard check erc --schematic <file.kicad_sch>
   aethelgard prove --crate <dir> --harness <name>
   aethelgard simulate --platform <file.repl | @renode-builtin-path>
   ```

   Inputs are paths given as flags, not JSON on stdin. Board files are not JSON, and a path keeps them out of `RUL-13`'s 16 MiB stdin cap. stdout stays exactly one JSON line. The architect amends `RUL-10`'s wording at close.

   `simulate` in P0 only loads the platform, runs `peripherals`, and reports the machine Renode built.

2. **Exit codes extend the contract and change nothing existing.** 0, 2, 3, 64 and 70 keep their meanings in `crates/aethelgard-cli/README.md`.

   | Code | Meaning |
   |---|---|
   | `0` | The tool ran, and its verdict is pass |
   | `1` | The tool ran, and its verdict is fail: violations found, the proof failed, the platform did not load, or the crate did not compile. The input is at fault and the tool worked |
   | `3` | Also: an input path that does not exist, cannot be read, or is not a regular file (a directory for `--crate`) |
   | `4` | `ToolMissing` or `ToolUnusable`: the tool is not on `PATH`, is not executable, or will not report its version. The `error.id` names the tool (`INV-06`) |
   | `5` | `ToolError`: the tool ran, but its output cannot be trusted as a verdict. That covers a signal, an exit code outside the ones measured for it, an unparseable or absent report, and a report that disagrees with the exit code |

   A `5` is never mapped to pass or fail. "Crashed" and "found violations" must stay distinguishable, because a fault campaign in P2 will run these thousands of times, and a crash that reads as either answer poisons it.

3. **One JSON shape for every verdict.**
   - `ok` is `true` exactly when the exit code is 0.
   - Refusals keep today's `{"ok":false,"error":{"kind","id","message"}}`.
   - Verdicts carry:
     - `verdict` (`"pass"` or `"fail"`);
     - `check` (`drc`, `erc`, `prove` or `simulate`);
     - `input`, with the canonical path and the **sha256 of the bytes judged**: for `--crate`, a hash over the crate's file tree, defined in the README;
     - `tool`, with the tool name and its version string exactly as the tool reported it;
     - `findings`, a list normalized to `{kind, severity, message, where}`, from the S1 fields.

   The hash and the version are what let `INV-03` later say "these gates ran on that exact pair". A verdict that cannot name its input's hash or its judge's version is not emitted; refuse instead.

4. **Two witnesses for every verdict.**
   - **DRC and ERC:** the pass or fail comes from the JSON report's violation count, and it is cross-checked against the exit code that `--exit-code-violations` produces (always passed, per `RUL-03`). If they disagree, exit 5.
   - **Renode:** pass requires a CPU in the `peripherals` listing and no `There was an error executing command`. The exit code is recorded and never trusted alone (§2.4).
   - **Kani:** use the S5 output lines, cross-checked with `cargo kani`'s exit code.

5. **The subprocess posture (`architecture.md` §6), as the spike rules it:**
   - argument arrays only, never a shell;
   - every input passed as a canonical absolute path, so a file named `-o/etc/x` cannot be read as a flag;
   - the environment cleared to the S2 allowlist;
   - stdin from `/dev/null`;
   - a fresh temporary directory as working directory and `HOME`, removed afterwards, with tool reports written there and never beside the input;
   - children that die with the CLI (S4);
   - Landlock confinement where S3 says it works.

   Captured output is bounded, and an overflow is a `ToolError`, not truncation. The CLI owns no wall-clock timeout (`RUL-13`); the caller's timeout ends the CLI, and S4's mechanism takes the children with it.

6. **Tests in two tiers, neither skipped silently.**
   - **Tier 1 runs everywhere, including the existing `gates` job:**
     - argument parsing;
     - path refusals;
     - each tool absent, by running the binary with `PATH` pointed at an empty directory, which gives exit 4 naming the tool;
     - stub tools on a temporary `PATH` that lie: exit 0 with no report, a report that disagrees with the exit code, death by signal. Each gives exit 5.
   - **Tier 2 needs the real tools:** a `with-tools` cargo feature on `aethelgard-cli`. Each subcommand gets a pass case and a fail case on the fixtures, and the Kani crate also gets a compile-error case.
   - **A new `tools` job in `.github/workflows/ci.yml`** runs tier 2 on every pull request inside the image pulled by the digest in `container/IMAGE`. It is the first consumer of `RUL-15`'s pin. The tool runs themselves happen under `--network none`; how the build fetches its crates first is the builder's call, recorded in FEEDBACK.
7. **Fixtures.**
   - The two schematics from S1 go in `container/fixtures/`, beside the boards.
   - `container/smoke.sh` gains the matching ERC check, clean exits 0 and violation exits 5, in the same style as the DRC check.
   - Extend `container/fixtures/README.md`: these are tool-calibration inputs, not specification artifacts (`INV-01`).
8. **Documentation.**
   - `crates/aethelgard-cli/README.md` documents each subcommand, the new codes, the verdict shape and the crate-hash definition.
   - Add one runnable example per subcommand, written as the `docker run` line against `container/IMAGE`, so a reader without the tools can run it.

## 2. Invariants (`docs/northstar.md` §4), the slice this work order touches

- **`INV-06` Absence halts.** A missing tool gives exit 4 naming it, and never a verdict. A tool that misbehaves gives exit 5, and never a verdict. Nothing defaults: no fallback tool, no "assume pass when the report is missing", no skipped test pretending to pass.
- **`INV-03` "Verified" means the gate ran on those hashes.** Every verdict carries the sha256 of exactly the bytes the tool judged and the version of the tool that judged them. Hash the bytes handed to the tool, not a re-read made afterwards.
- **`INV-07` The gates are not computed from the artifact they judge.** *Note for P1, not a build item here:* in P0, DRC uses the design rules stored inside the board file, which is acceptable for calibration fixtures. Generated boards will carry rules the emitter wrote, so `AET-WO-1300` must write them from the specification. Record this in FEEDBACK as a finding addressed to 1300.
- **No `unsafe` in any workspace crate** (`CLAUDE.md`). A dependency's safe API over a syscall is allowed and is the expected route for S3 and S4. Name each such dependency in FEEDBACK.
- **Protected surfaces diff to empty:** the three steering documents, `README.md`, `LICENSE-*`, `docs/inputs/**`, `crates/aethelgard-ir`, `container/Dockerfile`, `container/check-pins.sh`, `container/IMAGE`, and `.github/workflows/tools-image.yml`. Adding the fixtures and the smoke ERC check under `container/` is in scope.
- **The existing exit codes and refusal kinds are a contract.** Adding is allowed; changing the meaning of 0, 2, 3, 64 or 70 is a flag-and-stop.
- **Secrets by name only, and no new secret.** One PR, left unmerged. Push with `gh-write`, and batch the pushes.

## 3. Tests and acceptance, proven not asserted

1. Every §0 answer is recorded in FEEDBACK with the CI run that measured it.
2. In the `tools` job, on the PR's head, inside the image by digest:
   - each subcommand's pass case exits 0 and its fail case exits 1, with the verdict, hash, version and findings checked by exact key;
   - `check drc` on `container/fixtures/drc-violation.kicad_pcb` reports exactly one `clearance` finding.
3. **The P0 gate clause, Renode half:** with `renode` removed from `PATH`, `aethelgard simulate --platform @platforms/boards/nucleo_h753zi.repl` exits 4 with `error.id` = `renode` and prints no `verdict` key. The same holds for each other tool and its subcommand.
4. Tier 1 passes in the existing `gates` job, which has none of the tools, and on Furnace.
5. The workspace gates (fmt, clippy `-D warnings`, test) pass in `gates` and inside the image.
6. CI is green on the first push after the ruling. A red first run is a finding.

## 4. Scope fence — what is NOT in this work order

- A `build` subcommand, or anything that generates a board or a crate (`AET-WO-1300`, `AET-WO-1400`).
- An ngspice subcommand or `libngspice` bindings (P2, `OQ-02`).
- Running firmware in Renode, generated platform descriptions, the lock-step bridge (`AET-WO-2100`, `AET-WO-2200`).
- The trace archive, Cap'n Proto, R2 (`AET-WO-0400`).
- `rmcp` and the `mcp` subcommand (`AET-WO-1500`).
- Changing the tools image itself, `RUL-15`'s publish flow, or `container/IMAGE`.
- Making `tools` a required check in the repository ruleset. That is a settings change and the operator's; recommend it in FEEDBACK.
- Batch modes, parallel runs, caching of verdicts. One call, one tool run, one verdict.

## 5. Adversarial pass — break it before calling it done

Hand this to a fresh subagent if you can. Every finding, fixed or not, goes in FEEDBACK with a severity.

1. **Shadow each tool with a stub that lies.** Exit 0 with no report; a clean report with exit 5; a violations report with exit 0; garbage on stdout; death by `SIGKILL`; a report of 100 MB. Every one must give exit 5 and never a verdict.
2. **Remove each tool in turn.** Exit 4, naming exactly that tool.
3. **Hostile paths:**
   - a file named `-o` or `--output=/tmp/x`;
   - a symlink to `/dev/zero`, a FIFO, a directory where a file is expected;
   - a path with a newline or invalid UTF-8;
   - a board that is valid JSON but not a board;
   - a crate whose `build.rs` tries to write outside the temporary directory or reach the network.

   Each must refuse or fail cleanly, with no hang, no write beside the input, and JSON that stays valid.
4. **Kill the CLI** with `SIGTERM`, then `SIGKILL`, during a Renode run and a Kani run. No tool process survives; check with `ps` inside the container.
5. **Check the input hash honestly.** Change the input file while the tool is running. The verdict's hash must be of what the tool judged, or the call must refuse; it must never pair a stale hash with a fresh judgement.
6. **Look for assumptions that hold by luck:**
   - a tool's output format that one version happens to have;
   - the runner's kernel happening to allow Landlock;
   - the `tools` job's pull being cached;
   - fixture paths that only resolve from the repository root.

## 5a. Reduction pass — diff-sized

Run the `reduction-pass` skill on the finished diff. One line in FEEDBACK's gates.

## 6. Upward-feedback directive

`docs/work-orders/AET-WO-0300.FEEDBACK.md`, same schema as AET-WO-0200's: `finding · where · type · recommendation · decision-needed`. It must carry:
- what did not work, and why;
- the gates line with CI run URLs;
- what was deliberately not built;
- the measured facts for `architecture.md` §2 (S1–S6);
- the `INV-07` note for `AET-WO-1300`;
- a recommendation on making `tools` a required check;
- a promote-or-needs-none pass over AET-WO-0200's FEEDBACK. Its findings were promoted at its close into §2.4, `RUL-15` and §5 (PR #8), and B7's guard was corrected in PR #9. Say so.

## 7. Flag-and-stop conditions

Park the branch and say so, in FEEDBACK and in the session, if any of these appears:

- A verdict that can only be produced by trusting a single witness the spike showed can lie, or by mapping an unexplained exit code to pass or fail.
- Invoking a tool through a shell, or needing `unsafe` in a workspace crate.
- Changing the meaning of an existing exit code or refusal kind.
- Anything that needs a new secret, a personal access token, or a change to repository or organization settings.
- A tool that cannot run without network access at run time.
- A test that can only go green by being skipped, ignored, or weakened.
- Any step that needs `sudo` or a system package on Furnace.

## 8. Kickoff prompt

**Step 1, in its own turn:** send `/goal` alone, read what is in force (AET-WO-0200's achieved goal), and dismiss it.

**Step 2, in its own turn, set the goal for the spike half.** It ends at the STOP, the builder's own authority boundary. The words "or a recorded reason it cannot be" keep it reachable when a question is legitimately unmeasurable; AET-WO-0200's goal lacked them and looped:

```
/goal AET-WO-0300 spike: every §0 question has a measured answer or a recorded reason it cannot be, cited to its CI run in FEEDBACK, on an open unmerged PR from wo/aet-0300-cli-tools, and the build half has not started.
```

**Step 3, the kickoff message:** carried by the architect at kickoff, with the live base and anything that changed since this file was written.

The architect sets a second goal for the build half after ruling on the spike.
