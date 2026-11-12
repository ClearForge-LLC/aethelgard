# AET-WO-1601 — The tool boundary is a uid: separate-uid tools when root, a private scratch root, and the carried sandbox items

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main` after `AET-WO-3301` (`verify`), named in the kickoff.
**Branch:** `wo/aet-1601-uid-boundary`.
**Author:** Claude (architect) · **Builder:** strong tier, with a full adversarial pass (`docs/roadmap.md` §6).
**Phase:** P3 hardening. It **gates `AET-WO-3300`**, whose fabrication clause rests on a `verify` verdict.

**Grounds:**
- **`AET-WO-3301` F1, measured by its adversarial pass.** A same-user process wrote into a gate's private scratch after the gate hashed its copy and before the tool read it, and `verify` passed on artifacts the specification never produced:
  - **B1:** firmware source swapped in `build`'s scratch;
  - **B2:** a `.cargo/config.toml` planted in the tool's `HOME`;
  - **B3:** the harness swapped in `prove`'s scratch;
  - **B4:** the ELF swapped in `simulate`'s scratch.

  Every gate has this window. The harnesses are in `~/.cache/aet-3301-adv/` on Furnace, as `forge`.
- **Why a mount namespace alone does not close it** (the architect's correction on PR #91):
  - a read-only bind mount shares inodes, so a write through the original path is seen inside;
  - a private tmpfs is reachable through `/proc/<pid>/root`, because the same uid owns the user namespace and `execve` resets the dumpable flag;
  - the same uid can replace the toolchain itself (`~/.cargo/bin`, the Kani kit, both `forge`'s on Furnace).

  **On Linux, the boundary is the uid.**
- **`AET-WO-3301` F13:** scratch lives under `std::env::temp_dir()` (`crates/aethelgard-cli/src/main.rs`, `Scratch::new`). Cargo reads every ancestor's `.cargo/config.toml`, so any local user can plant `/tmp/.cargo/config.toml` (or `/tmp/Cargo.toml`) and stop `prove`, `build` and `verify`. That one fails closed (`ToolError`), but it is a denial any user can cause.
- **Carried items** (`docs/roadmap.md` §7.1):
  - **1300 A8:** `check erc` does not hash a hierarchical sheet a schematic names, so a sheet outside the copy would be judged unhashed.
  - **2100 F25:** a scratch path with a space or `;` reaches Renode's monitor `@path` unquoted (exit 5).
  - **Review A12.1:** a tool's scratch writes are uncapped, and scratch is RAM-backed on Furnace.

> **What this is:** the code half of a uid boundary, a scratch root no other user can plant configuration into, and three carried sandbox items. Each is proven by the attack that defeats it today.
>
> It is NOT:
> - creating users or changing anything on Furnace. The deployment half is the operator's (§1.3);
> - a change to any gate's verdict on an existing input, beyond the new disclosure field of §1.2.

**Cadence:** spike first (§1.1), then build. Stop on §7. One PR, left unmerged.

## 1. Scope

### 1.1 Spike: measure before building

In each of three environments, record what is permitted, with commands and outputs in FEEDBACK:
- Furnace's host, as `forge`;
- the pinned tools image under rootless podman on Furnace;
- CI's `tools` container: read `ci.yml`, and run its image the way the job does.

Measure:
- the effective uid;
- whether `setresuid` and `setresgid` to an unprivileged id (for example 65534) succeed;
- whether each tool runs under that uid, with `HOME` and its caches pointed at a directory that uid owns:
  - `cargo` and `rustc` (`build`), and Kani (`prove`);
  - `kicad-cli` (ERC and DRC);
  - Renode (`simulate` and `campaign`);
  - `ngspice` (through the bridge).

  Note any tool that needs to write to its own installation.
- which tools read configuration from an ancestor directory of their working directory, as cargo does.

Then re-run 3301's B1–B4 harnesses in the image three ways:
- the CLI as root, the racer as root;
- the CLI as root and tools under the separate uid, the racer as a third uid;
- the CLI as `forge` on the host, the racer as `forge`.

Predict each result before running it, and record both. **The second configuration must hold on all four.** If it does not, that is a §7 stop.

### 1.2 Separate-uid tools when the CLI runs as root

- **When the CLI's effective uid is 0:**
  - every tool runs under one fixed unprivileged uid and gid, set in the trampoline before Landlock and seccomp (`run.rs`, `confine_main`);
  - the hashed copy is owned by root, mode `0444` (directories `0555`);
  - the tool's writable directory is owned by the tool uid, mode `0700`.

  The uid is a constant in `run.rs`, chosen from §1.1's measurements and justified in FEEDBACK. A tool that cannot run under it is a §7 stop, not a quiet fallback to root.
- **When the effective uid is not 0,** nothing changes in how tools run.
- **Every verdict's `sandbox` object gains `uid_boundary`:**
  - `true` only when the tool ran under the separate uid;
  - `false` otherwise.

  This is `RUL-19`'s rule that a verdict claims only what it shows. `verify` names the boundary of each gate. Existing keys do not change.
- The CLI README says what `uid_boundary: false` leaves open: a same-user process can change what a tool reads (3301 F1).

### 1.3 The deployment half: written, not done

Add a short section, "Running `aethelgard` behind a uid boundary", to `crates/aethelgard-cli/README.md`. It covers:
- a dedicated uid for the CLI and its MCP server;
- the toolchain owned by root, or by that uid, and not writable by the client;
- the MCP client run as a different uid.

Nothing is created on any host. The operator decides Furnace's set-up.

### 1.4 A private scratch root (3301 F13; 2100 F25)

- **No file another uid can create may be read by a tool as configuration.**
  - From §1.1's measurements, choose how to meet this. One option: a scratch root that the CLI creates with mode `0700`, under an ancestry no other uid can write. Another: a per-tool override that turns off ancestor configuration.
  - Record the choice and its reason.
  - If the default location changes, `ci.yml` and the quickstart change in the same PR. A run that cannot meet the rule refuses with a new typed error (exit 4), never a default (`INV-06`).
- **A scratch path outside a fixed safe character set** (`[A-Za-z0-9/._-]`) is refused with that same error, so no tool's command language (Renode's `@path`) sees it. This closes 2100 F25.

### 1.5 Hierarchical sheets in `check erc` (1300 A8)

`check erc` refuses a schematic that names a hierarchical sheet, with a typed error (exit 2 or 3, matching the existing input refusals) and before any tool runs. The emitter never writes one. Hashing sheets is the alternative; take it only if refusing would break an existing input, and say why.

### 1.6 `RLIMIT_FSIZE` for tool scratch (review A12.1)

- The trampoline sets `RLIMIT_FSIZE`.
- The value is measured: the largest file any gate writes on the reference (Kani's and cargo's build outputs included), with headroom, recorded in FEEDBACK.
- A tool that exceeds it ends with `SIGXFSZ`, which the run reports as a tool error (exit 5).

## 2. Invariants and fences

- `INV-03`, `INV-06`, `RUL-10`, `RUL-13`, `RUL-19`. No `unsafe`. Use `rustix`, already a dependency: its `thread` feature has `set_thread_res_uid` and `set_thread_res_gid`, and the trampoline is single-threaded when it calls them. Supplementary groups must be cleared too; if `rustix` cannot do that safely, report it rather than reach for `unsafe`. Justify any new dependency against `RUL-12`.
- **Fence:**
  - the CLI's `run.rs` (the trampoline), `main.rs` (scratch), `kicad.rs` (the sheet refusal) and the README;
  - `ci.yml` and the quickstart, only if §1.4 moves the default;
  - tests;
  - this work order's FEEDBACK.
- **Every existing gate's verdict on an existing input is unchanged,** apart from the new `uid_boundary` key. Every committed generated file stays byte-identical.
- **Tier 2 runs as root in the image,** so after this work order tier 2 exercises the separate uid. The tier-2 canary floor rises by exactly the tier-2 tests added.
- One PR, left unmerged. **Push and revoke by the FUR-WO-0301 F7 route.**

## 3. Acceptance

- **§1.1's table is in FEEDBACK,** each prediction beside its result.
- **In the image:**
  - B1–B4, re-run with the CLI as root, the tools under the separate uid and the racer as a third uid, each fail to change what the tool reads;
  - each run's `verify` verdict says `uid_boundary: true` for every gate.

  Re-run as root against root, the race still succeeds, and the verdict says `false`. That proves the field tells the truth.
- **On Furnace as `forge`:**
  - the verdict says `uid_boundary: false`;
  - a planted ancestor configuration (3301 F13's `/tmp/.cargo/config.toml`, from another uid where measurable, or simulated by the scratch-root rule) is no longer read, or the run refuses with the new typed error.
- **A scratch path with a space or `;`** is refused with exit 4 before any tool runs.
- **A schematic with a hierarchical sheet** is refused before ERC runs.
- **A tool writing past `RLIMIT_FSIZE`** ends as exit 5. The reference's gates all pass under the cap.
- **The gates** (`CLAUDE.md`) pass, and `gates`, `tools`, `image` and the quickstart are green in CI.

## 5. Adversarial pass — full

1. **Reach what a separate-uid tool reads, as any uid except root:** scratch, the copy, the tool's `HOME`, the toolchain, `/proc/<pid>/root`, `/proc/<pid>/fd` and `ptrace`.
2. **Make a tool read configuration you planted** from another uid, through any ancestor, environment variable or cache directory.
3. **Make `uid_boundary` say `true` when the tool ran as root,** or the reverse.
4. **Find a gate whose verdict changed on an existing input,** beyond the new key.

## 7. Flag-and-stop conditions

- The separate-uid configuration does not hold B1–B4 in the image.
- A tool cannot run under an unprivileged uid without writing to its own installation.
- Meeting §1.4 needs a change outside the fence, such as the image's `Dockerfile`.
- `RLIMIT_FSIZE` cannot be set below a value that any legitimate gate needs.
