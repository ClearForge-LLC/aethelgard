# AET-WO-3301 — FEEDBACK

**Branch:** `wo/aet-3301-verify` · **Base:** `8d02e3f` (`origin/main`, as the kickoff set it) · **Builder:** Claude Code (Opus 5.5) as `forge` on Furnace, as project manager. A fresh Opus subagent ran the full adversarial pass on a frozen copy in its own directory and target. · **Cadence:** build-only. No §7 condition fired (see "§7, held").

**Where everything ran.**
- **Tier 1:** on Furnace, in a capped user scope (`systemd-run --user --scope -p MemoryMax=32G -p MemorySwapMax=0`), with a target directory of its own.
- **Tier 2:** in the pinned tools image `ghcr.io/clearforge-llc/aethelgard-tools@sha256:2df68259…` (`container/IMAGE`), under rootless podman 5.7, offline, in the CI `tools` job's own layout: the repository at `/repo`, `CARGO_HOME=/repo/target/cargo-home`, `CARGO_TARGET_DIR=/repo/target`. Each container ran with `--memory 16g --memory-swap 16g --pids-limit -1`; the runner then wrote 0 into the container's `memory.swap.max`.
- **Caps, read back from the container's own scope on every run:** `memory.max` = 17179869184, `memory.swap.max` = 0, `pids.max` = `max`.

**§7, held:**
- **Every generator is pure.** `generate` writes a function of the specification, the pin table and the twin's model table alone: the tree is composed in memory by one function that reads no file and starts no process, and two runs are byte-identical (tier 1).
- **Building the firmware needs no network and no dependency.** `build` runs cargo `--offline --locked` with an empty `CARGO_HOME`; the board crate has no dependency and the firmware's one is the board crate at `..`.
- **No test was weakened.** Every test that changed is listed under "Tests changed".

Entry schema for findings: `finding · where · severity · type · recommendation · decision-needed`.

## §3, acceptance

### §3.1: `generate` reproduces the reference
- **Tier 1** (`generate_tier1.rs`, `generate_reproduces_the_committed_board_and_emits_files_byte_for_byte`): `generate` on `examples/sentinel-01-minimal.json` writes 14 files. The 11 under `board/` are `boards/sentinel-01/**` byte for byte, and the three under `kicad/` are `emit`'s byte for byte. The report's hashes are the files' own. `topology_sha256` is the hash of `solve`'s line.
- **The regeneration test** (`aethelgard-svd-hal/tests/board.rs`) now takes `board/` from the same function (§1.5) and passes unchanged in every assertion.
- **Tier 2, `build`:** `build_gives_one_sha256_at_two_paths` builds the committed board crate at two paths of different lengths. Both give `2d9ee2641f6e0a1b6e48f1637cefe4fa01cab3f97849a1dd074474c879fe0d2f`.

### §3.2: `verify` on the reference
- **Measured** in the image (`board_tier2.rs`, `verify_binds_the_reference_and_two_runs_print_the_same_bytes`):
  - `verify --spec examples/sentinel-01-minimal.json --scenarios scenarios/sentinel-01/pullup-2k2.json --against <committed board + emit's files>` exits 0 and passes.
  - The verdict names all 14 files' hashes, the specification's (`acd60081…`) and the topology's (`d684cd6d…`), the ELF's (`2d9ee264…`), and seven gates in order, each with its exit code: `erc`, `drc`, `prove` for `proofs::every_valid_sequence` (cover 9), `prove` for `proofs::i2c1_transfer_is_bounded_and_ordered` (cover 7), `build`, `simulate` and `campaign`. Every gate passes, and both proofs are `generated`.
  - It also names the run (`pullup-2k2`, run id `c77d0936a7aacf1e630909f40597af3b`), the engine version and `"rise_times": "computed from declared values"`.
- **Two runs print the same bytes.** The run id and state hash equal what `campaign` gives directly for that ELF and scenario.
- **The 47 kΩ run fails** (`verify_fails_the_47k_pull_up_with_the_campaigns_findings`): exit 1. The campaign gate fails with `analog_rise_exceeds_ceiling` and `i2c_timeout`, though the campaign's own exit is 0, because the scenario expects that fail (F7). Every other gate passes.
- **Wall time** in the image on Furnace: 140, 143 and 144 s per `verify`, most of it the two proofs.

### §3.3: the review's A4 inputs cannot pass `verify`
- **A board from another specification** (the reference with the TMP117 removed, from `generate`): `verify --against` exits 1, naming among others `kicad/sentinel-01.kicad_sch` and `kicad/sentinel-01.bom.csv`. No gate runs (tier 1).
- **A hand-written vacuous harness** (`kani::assume(x != x); assert!(1 == 2)`):
  - `prove --spec` still passes it, as Kani does, but now reports `harness_provenance: "supplied"` with `cover_count` 0 (tier 2, `prove_with_spec_tells_a_hand_written_harness_from_the_generated_one`).
  - `verify --against` a tree holding it exits 1, naming `board/src/proofs.rs` (tier 1).
  - Tier 1 also covers provenance itself (`provenance_is_generated_only_for_the_generated_crate`): a covers-cutting edit to the harness, a code change under an unchanged harness, and an added file are each `supplied`.
- **A firmware ELF not built from the generated crate:**
  - `verify` builds its own ELF and requires `simulate`'s and `campaign`'s firmware to be it. The adversarial pass's C3 swapped the delivered ELF in `verify`'s scratch between `build` and `simulate`, and `verify` refused with `BindingMismatch`, exit 5. A unit test also checks every bound field of every gate (`every_bound_field_of_every_gate_is_checked`).
  - **Not held against a same-user process inside a gate's own scratch:** F1.

### §3.4: `--against boards/sentinel-01`
- With the KiCad files from `emit`, it passes (the §3.2 run).
- One changed byte in each of the 14 files fails, exit 1, naming exactly that file, with no gate run (tier 1, `verify_against_fails_on_one_changed_byte_in_any_file_naming_it`). A missing file and an extra file are each named too.

### §1.4 in `gates` and `tools`
- The provenance classification, `prove --spec`'s refusal before any tool, and every `--against` and `build` refusal are tier 1.
- They run in `gates`. The tier-1 tests run with `PATH` empty, so a refusal that waited for a tool would show as exit 4.

## What was built

1. **`aethelgard_cli::generate::tree`** (`crates/aethelgard-cli/src/generate.rs`, the CLI's new library target). The one composition of the generators (§1.5): the KiCad emission, `generate_with` with the twin's model table as `answers`, and `twin::platform::emit`, keyed by path under `kicad/` and `board/`. `crates/aethelgard-svd-hal/tests/board.rs` calls it and takes `board/`, so the regeneration test and `generate` cannot drift.
2. **`aethelgard generate --spec --out`** (§1.1). Every directory and file is made through a descriptor of its parent (`mkdirat`, `openat` with `O_CREAT | O_EXCL | O_NOFOLLOW`), and the emptiness check is made on the descriptor written through (`src/tree.rs`).
3. **`aethelgard build --crate --out`** (§1.2, `src/firmware.rs`). Cargo in the sandbox, as `prove` runs it, with symbols stripped so the ELF is path-independent (F4).
4. **`aethelgard verify --spec [--scenarios] [--against]`** (§1.3, `src/verify.rs`). The gates run as the binary's own subcommands on a tree written into a guarded scratch directory, and every hash each gate reports is bound to the generated bytes (F7).
5. **`prove --spec`**: `harness_provenance` and `cover_count` (§1.4, F5, F6).
6. **MCP:** the three subcommands and `prove`'s `spec` appear in the tool list with no change to `mcp.rs` beyond its pinned lists.
7. **README** (the CLI crate's): `generate`, `build`, "Harness provenance", `verify`, the exit-code table and the MCP tool list.

## Findings and the forks taken

**Decision-needed:**

- **F1 · `verify` binds what each gate reports judging, not what the gate's tool reads: a same-user process racing a gate's private scratch made `verify` pass on artifacts the specification never produced · every gate's copy-then-run window · high · security residual · decision-needed.**
  - **Measured by the adversarial pass** (frozen `5002724`, in the image; `~/.cache/aet-3301-adv/`):
    - **B1:** after `build`'s copy was hashed and before cargo read it, `firmware/src/main.rs` in the build gate's scratch was overwritten. `verify` exited 0, with `ok` true, naming ELF `897184e4…` (it holds the planted marker). `build`'s `input_sha256` was still the generated `fecf82ff…`. Two of two runs (`runA.json`).
    - **B2:** `tool/.cargo/config.toml` was planted in the build gate's scratch. That is the tool's `HOME` and an ancestor of its working directory. `verify` passed with ELF `e678e904…` and no hashed byte touched (`runB.json`).
    - **B3:** `src/proofs.rs` was swapped in each prove gate's scratch after provenance was judged. Both proofs passed `generated`, with `cover_count` 1 against 9 and 7 (four of four).
    - **B4:** `simulate`'s ELF copy was swapped. It judged another ELF while bound to the generated hash; shown in the fail direction (`runS.json`).
  - **Swaps in `verify`'s own scratch are refused** (C1 to C3: `BindingMismatch`, exit 5).
  - **Every gate has this window.** It hashes its copy, then hands the copy's path to its tool. `run.rs` and `input.rs` are unchanged by this work order (`git diff 8d02e3f -- crates/aethelgard-cli/src/{run,input}.rs` is empty).
  - **The boundary is the uid.** On Furnace, `~/.rustup`, `~/.cargo/bin/cargo` and the Kani kit belong to `forge`. In the image, every tool runs as root. A same-user process can replace the tools themselves, not only race scratch. `RUL-19` (4) gives an MCP client exactly that identity.
  - **Not patched here:**
    - A re-hash after the tool exits loses to a racer that restores the file once the tool has read it.
    - B2 touches no hashed byte.
    - A tool that writes into its own copy (cargo writing a lockfile) would turn existing passes into refusals, which §2 forbids.
  - **Options:**
    - (a) run tools as a separate uid that cannot write the copy;
    - (b) a private mount namespace with the copy bound read-only;
    - (c) accept it as `RUL-19` (4)'s residual and say so wherever `verify` is cited.

    (a) and (b) are sandbox work for a work order of their own, before `AET-WO-3300` rests its fabrication clause on a `verify` verdict. The README's `verify` section and `verify.rs` now say what is not bound.
  - **Recommendation:** (a) or (b), before `3300`.
- **F8 · The firmware's sha256, and with it every run id, changes · `attribute_seeded.rs`, PR #88 · medium · decision-needed.**
  - The kickoff's path independence strips symbols, so the reference ELF is now `2d9ee264…` at any path. 3200's literal `/tmp/aethelgard-reference-firmware` is gone, and the seeded set builds through `aethelgard build`.
  - Every run id built from the reference firmware is new.
  - **PR #88's committed `proposal.json` will not re-run to its bytes once this merges.** Its run id came from the unstripped ELF built at 3200's fixed path.
  - **Recommendation:** regenerate the proposal branch on top of this work order's merge (its tier-2 test re-runs it).

**Forks taken:**

- **F2 · The composition function's home is a new library target of the CLI · `src/lib.rs`, `src/generate.rs` · medium · fork.**
  - `aethelgard-svd-hal` cannot name `aethelgard-twin` or `aethelgard-eda`. A home there would inject `answers` and the platform emitter at both call sites, and those are the parts that must not drift.
  - `aethelgard_cli::generate::tree` composes all three generators. `aethelgard-svd-hal`'s test takes it as a path-only dev-dependency, as it already takes `aethelgard-twin` (`workspace_shape` updated).
  - The package step passes: the CLI packages with its library, and `svd-hal`'s path-only dev-dependency is stripped.
- **F3 · `build --crate <board dir> --out <file>` · `src/firmware.rs` · medium · fork.**
  - §1.2 names only `--crate`. The ELF must land somewhere that outlives the call, and `build` never writes beside its input. So `--out` names a file that must not exist.
  - `--crate` is the board crate, not `firmware/`: the firmware's one dependency is the board crate at `..`, so the copy must hold both.
- **F4 · Path independence by stripping symbols (kickoff item 2) · `src/firmware.rs` · high · fork, measured.**
  - **Measured in the image:** the same crate at two paths gave `ff0c79ea…` and `65faee64…` unstripped, and the same two with `--remap-path-prefix`. With `strip = "symbols"`, both paths gave `2d9ee264…`. Remap does not reach the crate disambiguators cargo hashes from the path.
  - `image_sha256` (the loaded segments `simulate` judges) is `84c823e3…` in all six builds, so the judge's checks are intact.
  - The strip is `--config profile.release.strip="symbols"` on cargo's command line, so no committed generated file changes.
  - **This holds for the generated crate.** A supplied crate can still embed its path, through rustflags in its own `.cargo/config.toml` or `env!("CARGO_MANIFEST_DIR")` (adversarial B6). The doc says so.
- **F5 · `prove`'s `harness` key already holds the harness name · `verdict.rs` · medium · fork.**
  - §1.4's `harness: "generated" | "supplied"` would change an existing key, which §2 forbids.
  - The new keys are `harness_provenance` and `cover_count`. `cover_count` is the total from Kani's cover summary, or 0 when Kani printed none (as on a compile error).
- **F6 · "generated" means the whole copied crate is byte-identical to `generate`'s `board/`, not only the harness file · `main.rs` `provenance` · medium · fork, stricter than §1.4's wording.**
  - An unchanged `src/proofs.rs` over rewired code is not the generated proof. That is §5.1's "matches in name only".
  - The comparison is of the copy, before Kani runs.
- **F7 · The campaign gate passes only when every run's own verdict is `pass` and it matched · `verify.rs` · medium · fork.**
  - `pullup-47k.json` expects `fail`, so `campaign` exits 0 on it. §3.2 says `verify` fails with it. A scenario that expects a fail can therefore never let `verify` pass.
- **F9 · `verify` runs each gate as the binary's own subcommand, with `AETHELGARD_INTERNAL_PARENT_PID` set, so a killed `verify` takes its gates and their tools with it · low · note.**
  - It has no bound of its own (`RUL-13`): the caller's timeout ends it.
  - Through MCP it is one long call, about 2.5 minutes for the reference.

**Residuals and notes:**

- **F10 · `build`'s environment is `RUSTUP_HOME`, `PATH`, `CARGO_TARGET_DIR` and the caller's `RUSTUP_TOOLCHAIN` · `architecture.md` §6 · low · protected document.**
  - §6 lists each tool's variables. `build` is not in it, and the architect amends it.
  - **Lead L1:** the verdict names cargo's version, not rustc's. A caller's `RUSTUP_TOOLCHAIN` could give two ELFs under one version string. Recommendation: name `rustc -vV`'s release (backlog).
- **F11 · The roadmap's P0 "deliberate gap" (the `build` subcommand) is closed by this work order · `docs/roadmap.md` · low · protected document.**
  - The architect updates the row.
  - The root `README.md`'s claim that `aethelgard` generates the board crate and firmware is now true of the CLI (review A3).
- **F12 · `build`'s dependency refusal · `src/firmware.rs` · medium · fixed in part, residual noted.**
  - **The adversarial pass broke the first version twice:**
    - **B5:** a path dependency into `RUSTUP_HOME` passed the lockfile check, which looked only for `source`.
    - **Re-test 2:** `#[path]`, `include_bytes!` and `[lib] path` reached outside through `..`, and a build script read an outside file.
  - **Now refused before any ELF is written** (`CrateHasDependency`, exit 3):
    - any unit built from a manifest outside the copy;
    - any source cargo's dep-info lists outside it, each path resolved;
    - any build script cargo ran.

    Re-test 3 held on all of them.
  - **Residual:**
    - A linker input named in a supplied crate's own rustflags (`-T`, `-C link-arg`) is not in dep-info. It can come only from what the sandbox lets cargo read: the tool installation and the system directories. Planting it there takes F1's boundary.
    - A build script is refused only after cargo has run it, in the sandbox.
  - `verify` builds only the generated crate, which has none of these.
- **F13 · An environment fault was a design fail · `src/firmware.rs` · medium · fixed (adversarial B7).**
  - A `/tmp/.cargo/config.toml` planted by any local user made cargo exit 101, which `build` reported as `build_failed`, exit 1.
  - Now cargo's `build-finished` message is the second witness. Exit 101 without it is `ToolError`, exit 5.
  - **Residual (lead):** a compiler killed mid-build (out of memory) still ends `success: false`.
  - **Backlog:** any local user can stop `prove`, `build` and `verify` on a shared host with a planted `/tmp/.cargo` or `/tmp/Cargo.toml`, because scratch lives under `/tmp`. `prove` had this before this work order.
- **F14 · Two small leads from the adversarial pass:**
  - L3: the campaign gate's entry has no tool version; Renode's is in `simulate`'s entry.
  - L4: `--against` compares files, so an extra empty directory is not reported.
- **F15 · The canary floor is 71 · `ci.yml` · low · per the kickoff.** The four new tier-2 tests are in `board_tier2.rs`, a counted file, and `TIER2_MIN` is raised by exactly four, with the reason beside it.
- **F16 · Build artifacts were committed by mistake, and removed before any push · low · process.**
  - A host run with a relative `CARGO_TARGET_DIR` made trybuild write under `crates/aethelgard-svd-hal/target/`, and 1,478 files went into a local commit. The adversarial pass caught it.
  - The unpushed commits were rewritten without them; the branch tracks no `target/` file.
  - **Recommendation:** `.gitignore` anchors only `/target/`; add `/crates/*/target/` (backlog).
- **F17 · `AET-WO-3200` F12 is closed.** The re-gate's `every_gate()` now runs `generate`, `prove --spec` (asserting `generated`) and `build`, in place of `generate_with` and a hand-run cargo.

## Tests changed

- **New:**
  - `generate_tier1.rs` (8, tier 1);
  - four tier-2 tests at the end of `board_tier2.rs`;
  - unit tests in `tree.rs` (3), `verify.rs` (2), `firmware.rs` (3) and `main.rs` (provenance, 1).
- **`aethelgard-svd-hal/tests/board.rs`:**
  - `generated()` now calls `aethelgard_cli::generate::tree` and takes `board/`; the local `answers` and `CRATE` are gone;
  - every assertion is unchanged.
- **`attribute_seeded.rs`:**
  - `firmware()` builds through `aethelgard build` in the per-process scratch, and `reference_firmware_dir` is gone (F8);
  - `every_gate()` runs `generate`, `prove --spec` and `build` (F17);
  - every assertion is unchanged, plus one new one: the re-gate's harness is `generated`.
- **`mcp.rs` unit tests and `mcp_tier1.rs`:** the tool list gains `generate`, `build` and `verify` after `emit`; the count goes from 12 to 15; `prove`'s keys gain `spec`.
- **`workspace_shape.rs`:** `svd-hal`'s dev-dependencies gain `aethelgard-cli`, path-only, as `aethelgard-twin` is.

## The fence

**Changed:**
- the CLI:
  - `generate`, `build` and `verify` (`src/generate.rs`, `src/lib.rs`, `src/tree.rs`, `src/firmware.rs`, `src/verify.rs`, `src/main.rs`);
  - `prove`'s `--spec` and two fields (`src/main.rs`, `src/kani.rs`, `src/verdict.rs`);
  - MCP's pinned lists;
  - its README;
  - `campaign::sha` made `pub(crate)` (reduction R3, one word);
- `aethelgard-svd-hal`: its regeneration test, one path-only dev-dependency and one README line;
- tests;
- this FEEDBACK;
- one line of `ci.yml` (`TIER2_MIN`, per the kickoff).

**Not touched:**
- every generator, and every committed generated file (`boards/` is byte-identical);
- `run.rs`, `input.rs`, `kicad.rs`, `renode.rs`, `analog.rs` and `elf.rs`, so every existing gate's verdict on an existing input is unchanged apart from `prove`'s two fields;
- `propose.rs`: reduction R2, below, would have touched it;
- the steering documents, `README.md` and `AET-WO-3301.md`.

## Adversarial pass (full, §5)

**Who and how.**
- A fresh Opus subagent, chosen because this is judgement work, on a frozen copy at `5002724` in its own directory and image target.
- It was then resumed three times on the fixes (`e19a7d1`, `5739d27`, `067a408`).
- **Its racer** is a same-user bash loop. It finds each gate's scratch by the pid in its name and acts when `status-version` appears: after the copy is hashed, before the tool reads it.
- **Caps on every one of its containers:** `memory.max` 17179869184, `memory.swap.max` 0, `pids.max` `max`.
- **Report:** `~/.cache/aet-3301-adv/FINDINGS.md`. Harnesses: `repo*/target/adv/`.

| id | attack (§5) | result | fix | test |
|---|---|---|---|---|
| B1 | 1, 2: swap the firmware source in `build`'s scratch | **broke** (2 of 2) | none here: F1 | — |
| B2 | 1: plant a cargo config in `build`'s `HOME` | **broke** | none here: F1 | — |
| B3 | 1: swap the harness in `prove`'s scratch after provenance | **broke** (4 of 4) | none here: F1 | — |
| B4 | 2: swap the ELF in `simulate`'s scratch | **broke** (fail direction) | none here: F1 | — |
| B5 | `build`: a dependency from outside the crate | **broke**, then broke seven more ways on re-test | manifests and dep-info sources resolved and kept inside the copy; build scripts refused (F12); linker inputs a residual | `a_unit_or_source_from_outside_the_copy_and_a_build_script_are_refused`, `dep_info_lists_every_source`; re-test 3 held |
| B6 | `build`: a crate's config defeats the strip | **broke** the doc's claim | the claim is scoped to the generated crate (F4) | — |
| B7 | `build`: a planted `/tmp/.cargo` read as a design fail | **broke** | `build-finished` is the second witness (F13) | `build_finished_is_read_and_its_absence_is_none`; re-test held (`ToolError`, exit 5) |
| C1–C3 | 1, 2: swap the schematic, the specification or the ELF in `verify`'s own scratch | held: `BindingMismatch`, exit 5 | — | `every_bound_field_of_every_gate_is_checked` |
| G1–G7 | 3: `generate` outside `--out`, or over a file | held. It reproduces the reference, and refuses a non-empty directory, a symlink to an empty one, a dangling symlink, a file, and a directory holding only a dotfile, writing nothing. A symlinked ancestor of `--out` is the path the user named | — | `two_generations_are_identical_and_a_used_directory_is_refused_untouched`, `write_refuses_a_used_or_linked_directory_and_a_path_out_of_it` |
| D1 | 4: two `verify` runs disagree | held: byte-identical verdicts, and one ELF at every scratch path | — | `verify_binds_the_reference_and_two_runs_print_the_same_bytes` |
| S1 | scenario binding | held: a base other than `--spec`, an absolute base and one outside the working directory are refused; run ids are recomputed and bound | — | `verify_refuses_a_scenario_whose_base_is_not_the_specification` |
| P1 | provenance spoofing without a race | held: whole-crate comparison | — | `provenance_is_generated_only_for_the_generated_crate` |
| R1 | a refusal that becomes a pass | none found: each gate refusal carries its own kind and code; no output, a signal or bad JSON gives `ToolError` | — | — |
| X1 | build artifacts in a commit | **found** | history rewritten before push (F16) | — |

## Mutation testing (sized to the diff)

<!-- filled -->

## Reduction pass

**Who and how.** A fresh subagent ran the four questions over the work order's own commits, on a frozen copy at `e19a7d1`. Each patch was proved with fmt, clippy (`with-tools`) and the tier-1 suites it touches. Report: `~/.cache/aet-3301-red/`.

**Applied** (3 of 4, −32 net):
- **R1:** one helper for the `generate::tree` error mapping and topology hash, and one for printing a `Stop` (each had been written twice).
- **R3:** `verify` reuses `campaign::sha` instead of its own hex decoder.
- **R4:** `verify`'s argument trait and its two impls become `&dyn AsRef<OsStr>`.

They were re-proved on the branch head with the same suites, all green.

**Not applied:**
- **R2** (−40): `propose::write_out` is a third copy of `tree::write`'s write discipline, but `propose` is outside this fence. **Finding:** one line for the next work order that owns `propose`.

**Findings, not changed:**
- **Kept separate on purpose:**
  - `verify`'s refusal type, whose kind is a runtime string from a gate's JSON;
  - `generated_board`'s copy of `solve_spec`'s error mapping in `Stop` form (merging means reworking `solve_spec`).
- **Small duplicates:**
  - `build`'s invocation set-up repeats `prove`'s;
  - test helpers (`copy_tree`, `sha`) live in two test files;
  - `parse_spec` and `elf.rs` hash beside `tree::sha256`, as they did before this work order.
- **Not a copy:** `emit`'s writer, which allows an existing directory and overwrites its own three names.

## Gates

<!-- filled -->
