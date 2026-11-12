# AET-WO-1601 — FEEDBACK

**Branch:** `wo/aet-1601-uid-boundary` · **Base:** `366e8fd` (`origin/main`, carrying `AET-WO-3301`) · **Builder:** Claude Code (Opus 5.5) as `forge` on Furnace, as project manager. · **Cadence:** spike (§1.1), then build.

**Where everything ran.**
- **The image:** the pinned tools image `ghcr.io/clearforge-llc/aethelgard-tools@sha256:2df68259…` (`container/IMAGE`), under rootless podman 5.7 on Furnace, in the CI `tools` job's layout (`/repo`, `CARGO_HOME=/repo/target/cargo-home`, `CARGO_TARGET_DIR=/repo/target`), `--memory 16g --memory-swap 16g --pids-limit -1`, with 0 written into the container's `memory.swap.max` after start. Caps read back from every long-running container's own scope: `memory.max` 17179869184, `memory.swap.max` 0, `pids.max` `max` (`~/.cache/aet-1601/caps.log`).
- **The host:** Furnace, as `forge` (uid 1001).
- **Spike scripts and raw output:** `~/.cache/aet-1601/spike/` (`spike.sh`, `spike2.sh` to `spike4.sh`, `out/`, the `*.log` files and `PREDICTIONS.md`, written before the runs). `strace` came from the host, run through the host's loader (`/hostlib`), since the image has none.

## §1.1 Spike: measured before building

Predictions were written to `spike/PREDICTIONS.md` before each run.

### What each environment permits

| # | measured | prediction | result |
|---|---|---|---|
| S1 | effective uid | image 0; host 1001 | image 0 (`CapEff` `800405fb`, `CapBnd` the same); host 1001 |
| S2 | uid mapping in the image | — | `uid_map` `0 1001 1` and `1 165536 65536`: ids 0 to 65536 exist; `forge`'s subuid range is 165536+65536 |
| S3 | image: `setresgid`, `setgroups([])`, `setresuid` to 65533, 65534, 1000, 65536 | succeed | all four succeed; `CapEff` 0 afterwards, `CapBnd` unchanged (still full) |
| S4 | image: the same to 65537 and 70000 | `EINVAL` | `EINVAL` (unmapped) |
| S5 | host as `forge`: `setresgid`, `setresuid` to 65534, 65533, 0, and `setgroups([])` | `EPERM` | `EPERM` for every one |
| S6 | CI's `tools` job | real root under Docker, no user namespace | read from `ci.yml`: `docker run` with no `--user` and no userns remap, so the CLI is real root and every uid exists. It cannot run on Furnace; tier 2 in CI is the proof of this row |

### Each tool under a separate uid (image)

Each tool ran under `setpriv --reuid=U --regid=U --clear-groups`, with `HOME` and `TMPDIR` a directory U owns (mode 0700), reading a root-owned copy (files 0444, directories 0555) inside a root-owned directory (0711). First with U = 65533, then with U = 65534.

| tool (gate) | prediction | U = 65533 | U = 65534 |
|---|---|---|---|
| `cargo build` (`build`) | runs; nothing written in the copy; ELF as root's | rc 0; ELF `2d9ee264…`, the root build's; nothing written in the copy | the same |
| `cargo kani` (`prove`), `every_valid_sequence` | passes | `VERIFICATION:- SUCCESSFUL`, 9 of 9 covers | the same |
| `kicad-cli` ERC and DRC | run; may try a lock file beside the input | run; nothing written beside the input. Writes `~/.config/kicad`, `~/.cache/kicad`, fontconfig caches in `HOME`. Tries `O_RDWR` on `/var/cache/fontconfig/*` (`EACCES`, falls back to `HOME`; Landlock already denies `/var`). The ERC report is identical to root's (date and source dropped) | the same |
| Renode (`simulate`, `campaign`) | runs | `--version` and a boot of the reference ELF run; writes `~/.config/renode/{config,history}` and a 1.5 MB `renode-<pid>/*.tmp` in `TMPDIR` | the same |
| the analog bridge (libngspice) | runs | **SIGSEGV (139)** | rc 0; output byte-identical to root's |
| `ngspice -v` | runs | rc 0 | rc 0 |

**Why uid 65533 failed (S7).** (65533 is now the tool *gid*, which no lookup needs: A3.) Under every uid with no passwd entry (65533 and 65520 measured), the bridge dies. `strace` shows NSS looking the uid up in `/etc/passwd`, then `/etc/userdb`, `/run/userdb`, `/run/host/userdb`, `/usr/local/lib/userdb` and `/usr/lib/userdb`, each `ENOENT`, and then `SIGSEGV` at address `0x20`, which is `pw_dir` in a NULL `struct passwd`. libngspice reads `getpwuid(getuid())->pw_dir` without checking. Uids 65534 (`nobody`) and 1000 (`kicad`) pass.

**No tool writes to its own installation** (S8). In every trace, the only refused writes outside `HOME` were KiCad's fontconfig cache opens above. That is not a §7 stop.

### Which tools read configuration from an ancestor of their working directory (S9)

Prediction: cargo, rustup and Kani; nothing else. The result matched.

| tool | what it reads above its working directory |
|---|---|
| cargo, and rustup's proxy | `.cargo/config`, `.cargo/config.toml`, `rust-toolchain` and `rust-toolchain.toml` at every ancestor up to `/`. Also `Cargo.toml` at every ancestor of the manifest (the workspace search) |
| `cargo kani` | the same, plus `cargo.toml` |
| `kicad-cli` | nothing above its working directory but `/kicad.mo` and `/wxstd.mo` (root-owned `/`). It reads `sym-lib-table`, `fp-lib-table` and `design-block-lib-table` beside its input, which is the copy |
| Renode, ngspice, the bridge | nothing |

**A per-tool override does not exist for cargo (S10).** With the working directory `/` and `--manifest-path`, cargo still walks `Cargo.toml` up from the manifest. A planted `[workspace]` two levels up stopped the build with exit 101 ("current package believes it's in a workspace when it's not"). So §1.4 is met by where scratch lives, not by a cargo setting.

### The callers a scratch rule must hold for (S11)

| caller | uid | `HOME` | `TMPDIR` | runs a tool subcommand |
|---|---|---|---|---|
| Furnace host | `forge` 1001 | `/home/forge` (0700) | unset (`/tmp` is tmpfs, 1777) | yes |
| tools image under podman | 0 | `/root` (0700) | unset | yes |
| CI `tools` job (`ci.yml`) | real root | `/root` | unset; `/repo` belongs to the runner's uid | yes |
| CI `gates` job | runner | the runner's home | unset | yes (tier 1, stub tools) |
| `engine-image.yml` smoke (`container/engine-smoke.sh`) | 0 | `/root` | unset | **yes**: `check drc` twice |
| `tools-image.yml` | 0 | `/root` | unset | yes (`cargo test --workspace`, tier 1) |
| the README quickstart (`quickstart.yml`) | the runner's uid via `--user`, no passwd entry | `/` | unset; only `/work` (the run directory) is the caller's | yes: `check erc`, `check drc` |
| `aethelgard mcp` | the caller's | inherited | inherited | each call is a child CLI with the parent's environment |

`engine-smoke.sh` and `engine-image.yml` are outside this work order's fence, and they run a tool as root with `TMPDIR` unset. A rule that reads scratch's place from `TMPDIR` alone would refuse `/tmp` there and force an edit outside the fence, a §7 stop. The design below needs no edit there.

### B1–B4 re-run in three configurations

The racer is 3301's (`~/.cache/aet-3301-adv/repo/target/adv/racer.sh`), copied to `~/.cache/aet-1601/adv/racer.sh` and changed in three ways:
- it finds scratch under `$SCR`, or from `/proc/*/cmdline` when it cannot list `$SCR`;
- it logs every attempt and its result, so a racer that cannot reach scratch is seen failing rather than doing nothing;
- when it cannot traverse a scratch root, and so cannot see 3301's trigger (`status-version`), it attempts anyway, up to 200 times.

The B2 plant, B1 and B3 swaps, and B4's ELF are 3301's files. `adv/b14.sh` runs three `verify` calls: R1 races `build` and `prove` (B1, B3) under `--scenarios pullup-2k2`; R2 plants the cargo config (B2); R3 swaps `simulate`'s ELF (B4). The reference verdict is: ELF `2d9ee264…`, covers 9 and 7, every gate passing.

Covers are the two proofs' (`every_valid_sequence`, `i2c1_transfer_is_bounded_and_ordered`). The results are in `~/.cache/aet-1601/adv/out/<config>/R{1,2,3}.json`, with each racer's own log beside them.

| config | prediction | B1 (R1: ELF) | B3 (R1: covers) | B2 (R2: ELF) | B4 (R3: simulate) | `uid_boundary` |
|---|---|---|---|---|---|---|
| C1: CLI root, racer root, unmodified `366e8fd` | all four broke; no key | **broke** `897184e4` | **broke** 1, 1 | **broke** `e678e904` | **broke**: fails on `bad.elf` while bound to `2d9ee264` | no key |
| C3: CLI and racer uid 1001 (`--userns=keep-id --user 1001:1001`), unmodified | all four broke | **broke** `897184e4` | **broke** 1, 1 | **broke** `e678e904` | **broke** | no key |
| **C2: CLI root, tools 65534, racer uid 4242, this branch** | **all four held**; every attempt `EACCES`; `true` everywhere | **held** `2d9ee264` | **held** 9, 7 | **held** `2d9ee264` | **held**: passes | `true`, every gate |
| C2b: as C2, racer uid 65534 (the tool uid) | B1, B3, B4 held; B2 broke (stated residual) | **held** `2d9ee264` | **held** 9, 7 | **broke** `e678e904`: a plant in the tool's `HOME` | **held** | `true` |
| C1b: CLI root, this branch, racer root | all four broke; `true` | **broke** `897184e4` | **broke** 1, 1 | **broke** `e678e904` | **broke** | `true` |
| C3b: CLI and racer uid 1001, this branch | all four broke; `false` | **broke** `897184e4` | **broke** 1, 1 | **broke** `e678e904` | **broke** | `false`, every gate |

- **C2 is not vacuous.** uid 4242 cannot list `/var/lib/aethelgard` (0711), so it found each scratch path in `/proc/*/cmdline`, then failed on every attempt, 200 times per gate and mode. For example: `…/tool/crate/firmware/src/main.rs: Permission denied` and `mv: cannot stat '…/tool/crate': Permission denied` (`C2/R1.racer.log`); `mkdir: cannot create directory '/var/lib/aethelgard/aethelgard-…': Permission denied` (`C2/R2.racer.log`).
- **C2b shows the sticky directory at work.** (C2 and C2b ran with the tool gid still 65534; the adversary's A3 then moved it to 65533 and re-tested B2.) As 65534 the racer passes the call directory (group 65534). Each write to the copy was `Permission denied` (root-owned 0444), and each `mv` of the crate aside was refused (sticky 1770). B2's plant succeeded, because the tool's `HOME` is the directory the tool uid must write. That is the residual stated in the README.
- **C2b R1** was re-run after its first run was stopped by my own concurrent rebuild of the shared binary (`cannot run gate build: No such file or directory`, kept as `R1-stopped-by-rebuild.json`). The re-run held: ELF `2d9ee264`, covers 9 and 7, `true`, 0 successful attempts. Writes were `Permission denied`; `mv` of the crate aside was `Operation not permitted` (the sticky bit).
- **C3 first ran as root by mistake.** `--userns=keep-id` alone left the image's `root` user. That run is kept as `C3-was-root.log` and is not counted.

**The second configuration holds on all four** (§1.1's must), so there is no §7 stop.

## What was built

- **§1.2, separate-uid tools when root** (`run.rs`). When the trampoline's effective uid is 0, it does the following before Landlock and the filter, while it is single-threaded:
  - empties the bounding set while `CAP_SETPCAP` is still effective;
  - clears the supplementary groups (`set_thread_groups(&[])`);
  - becomes uid 65534, gid 65533, with `set_thread_res_gid` and `set_thread_res_uid`;
  - reads all four uids, all four gids and `Groups:` back from `/proc/thread-self/status`.

  Any failure is `SandboxUnavailable`, `id` `uid`, exit 4: there is no run as root. `empty_bounding_set` reads the set back after emptying it, on both the root path and `drop_capabilities`' own. The status line is `sandbox abi=N uid_boundary=0|1`, and `uid_boundary` is read from that line alone, never from a uid comparison. A CLI started as 65534 says `false`.
- **Root-owned copy, tool-writable directory** (`run.rs`, `hand_over` and `seal`). The first tool run in a root CLI's work directory seals everything the CLI put there (files 0444, directories 0555). It works by descriptor: `openat` with `O_NOFOLLOW`, then `fchmod`. Anything but a root-owned file or directory refuses the hand-over (the CLI copies no link). It then makes the directory root's, group 65533, mode 1770. A later run finds it handed over and touches nothing. The call directory is root:65533 0710 (`main.rs`). The bridge's own directory goes through the same hand-over. For `prove`, the crate copy's root directory is made the same before the hand-over (`run::add_only`), because cargo writes a missing `Cargo.lock` there (adversarial A2). A failed chown, chmod or hand-over is `SandboxUnavailable`, `id` `uid`, exit 4 (adversarial A6).
- **`uid_boundary`** is in every verdict's `sandbox` object (`SandboxReport`), with `uid_boundary_unmet` when it is false (the architect's ruling, below). It is on every gate entry of `verify` (`null` for a gate that did not run; the campaign's is `true` only when every run line says so) and on every `campaign` run line.
- **§1.4, the scratch base** (`main.rs`, `scratch_base`). The base is `TMPDIR` if set (it must exist); otherwise `/var/lib/aethelgard` as root and `$HOME/.cache/aethelgard` as a user, made if missing. Every ancestor of its real path must be a directory owned by root or the effective uid, writable by no group or other, and (as root) searchable by others. The path must be in `[A-Za-z0-9/._-]`. Anything else is `ScratchUnsafe`, exit 4, naming the directory, before any tool runs.
- **§1.5** (`kicad.rs`, `refuse_sheets`). `check erc` reads its copy with `aethelgard-eda`'s strict reader. Any `(sheet …)` is `HierarchicalSchematic`, exit 3, the same kind parity uses; a schematic that reader refuses is `MalformedSchematic`, exit 3. Both happen before the tool is looked for.
- **§1.6, `RLIMIT_FSIZE` 64 MiB**, soft and hard (`run.rs`). After every run, a regular file at the cap anywhere in the work directory makes the call `ToolError`, exit 5. Cut off itself, a tool dies by `SIGXFSZ` (exit 5 as before).
- **§1.3**, the deployment half: the CLI README's "Running `aethelgard` behind a uid boundary". Nothing was created on any host.
- **Root reads tool output only beneath its directory** (`firmware.rs`, finding F7). `build` reads the ELF and its `.d` with `openat2(RESOLVE_BENEATH | RESOLVE_NO_SYMLINKS | RESOLVE_NO_MAGICLINKS)` from the target directory. KiCad's report was already read with `O_NOFOLLOW` from a root-owned directory.

### §1.6 measured

- **The largest file any gate writes on the reference** is 2,212,600 bytes: Kani's goto binary for `i2c1_transfer_is_bounded_and_ordered`. It was sampled every 20 ms under the scratch base during `verify --scenarios 2k2 47k` (`adv/sample.sh`, `out/sample/sizes.txt`). Next were Renode's 1.5 MB `.tmp` and the 131,776-byte ELF.
- **Bracketed with throwaway builds** of the same tree:
  - a 2 MiB cap made `verify` exit 5: `gate prove: …i2c1_transfer….out reached the 2097152-byte file-size cap`. That is a grandchild of cargo cut off while cargo-kani exited on its own terms, caught by the witness;
  - a 4 MiB cap passed.

  The cap is 64 MiB, 16× the bracket and 4× `RUL-13`'s 16 MiB input cap.
- **What else the cap met (F3).** Renode's .NET runtime truncates a `memfd` named `doublemapper` to 2 TiB (`ftruncate(8, 2199023255552)`) for its W^X double mapping. Under any cap below 2 TiB that is `EFBIG` and `SIGXFSZ`, so the version probe died by signal 25. It is a sparse file that writes nothing. With `DOTNET_EnableWriteXorExecute=0` Renode starts under the cap (`spike/spike5.sh`).

## Findings and the forks taken

Entry schema: `finding · where · severity · type · recommendation`.

**Decision-needed:**

- **A1 · The pinned tools image's Kani kit belongs to uid 1001 · `container/Dockerfile` (outside the fence) · high · ruled on PR #96: not a stop. `uid_boundary` now says `false`, `toolchain`, for `prove` in this image, and the `Dockerfile` fix is the architect's `AET-WO-1602`.**
  - Measured by the adversarial pass and re-checked by the builder. Everything under `/opt/kani/kani-0.68.0/` but its top two directories is `1001:1001`: 144 entries, `bin/cbmc` and `bin/kani-driver` among them. The Dockerfile untars as root and runs `chmod -R a+rX`, never `chown`. Nothing else under `/opt` or `/usr` belongs to anyone but root.
  - As uid 1001 in the image, the adversary swapped `kani-driver` for a stub that hands `--version` to the real driver. `prove` of a harness that begins `kani::assert(false, …)` went from exit 1 `proof_failed` to **exit 0, pass, `uid_boundary: true`**, with the same input sha.
  - So §5.1's "reach the toolchain" breaks for one uid in the published image. No process runs as 1001 there unless one is added, but the image's first `useradd` gets 1001, which is the README deployment section's own suggestion of a separate client uid.
  - **Not a §7 stop, by my reading.** §7 names four conditions, none of them this one, and B1–B4 held as posed in the second configuration. The fix is outside the fence (3301 F1 is the precedent for a measured break with no in-fence fix). Both in-fence mitigations (refusing `prove`, or reporting `false` for it) would change a §3 result in the pinned image, which is a ruling, not a build step. **READY rests on this reading; the architect may rule it a stop.**
  - **Since fixed on `main`** by `AET-WO-1602`: the image `c69ff9bc`, merged here with `dbf2cba`, has a root-owned toolchain, and `find /opt /usr -xdev` with the shared rule prints nothing there.
  - **Recommendation (as made):** `tar --no-same-owner` (or `chown -R root:root /opt/kani`) in `container/Dockerfile`, a new pinned image, and an image check that `find /opt /usr -xdev ! -uid 0` prints nothing. The README now says `uid_boundary` excludes no one who can write the toolchain, and names the check.

- **F1 · §3's "re-run as root against root … the verdict says `false`" cannot happen · §3 · medium · accepted on PR #96: root against root wins and says `true`.**
  - §1.2 makes a root CLI always run its tools as 65534, with no fallback. So no verdict from a root CLI says `false`.
  - Re-run root against root (C1b), every race still wins, as predicted, and the verdict says `true`. That is the truth §1.2 defines (the tool did run as 65534): the boundary never claimed to hold against root.
  - The `false` witness is C3b: the same CLI as a non-root uid in the image, raced by its own uid. All four broke, and every gate says `false`. The host row agrees (tier 1 as `forge`: `uid_boundary_is_true_exactly_when_the_tool_ran_under_the_tool_uid`).
  - **Recommendation:** amend §3 to "root against root wins, and says `true`; the same non-root uid wins, and says `false`".
- **F2 · The tool uid is 65534, `nobody`, by measurement; the tool gid is 65533 · `run.rs` `TOOL_UID`, `TOOL_GID` · medium · fork, decision-needed for the image.**
  - **The gid was 65534 until the adversarial pass (A3).** In the image, `sync` (uid 4) and `_apt` (uid 42) also have primary gid 65534. Each of them, and a uid 4242 holding supplementary group 65534, planted B2 into `build` (ELF `e678e904`, pass, `true`). 65533 is no passwd entry's group and has no group entry, so only root can grant it. Every tool runs under it: the bridge byte-identical, Kani fixtures as base, and tier 2 (*see Gates*).
  - libngspice dereferences `getpwuid()` unchecked, so the bridge segfaults under any uid with no passwd entry (S7). In the image that leaves 1000 (`kicad`, a login user) and 65534.
  - **Residual:** on a host where other services run as `nobody`, a root CLI's tools share that uid. The README's deployment section therefore recommends a dedicated non-root uid on hosts.
  - **Recommendation:** a passwd entry for a dedicated uid in the tools image (a `container/Dockerfile` change, outside this fence). Then `TOOL_UID` would be that uid.
- **F3 · Renode runs with `DOTNET_EnableWriteXorExecute=0` · `main.rs` · medium · fork, decision-needed.**
  - It is needed so the cap holds for every tool (§1.6 measured). It turns off .NET's W^X for the JIT inside Renode, which is defense in depth inside a process already confined by Landlock, the filter and the separate uid.
  - The alternative was to exempt Renode from `RLIMIT_FSIZE`, which would leave its scratch writes uncapped.
  - Every Renode verdict on the reference is unchanged: *see Gates*.
- **F4 · The quickstart block changed · `README.md` · low · fork, protected document.**
  - The engine image's quickstart runs the CLI with `--user uid:gid`, which Docker gives `HOME=/`. The new rule then has nowhere private and refuses (`ScratchUnsafe`, `id` `/.cache`; reproduced with `-e HOME=/`).
  - The block's `ae()` now passes `-e HOME=/work`, the run directory: `mktemp -d`, 0700 the runner's.
  - Measured on both binaries: the README's pinned image `4c440a2d…` and this branch's each pass `check erc` and `check drc --spec --schematic` with it.
  - `README.md` is protected by `CLAUDE.md`, but the WO's fence names the quickstart when §1.4 moves the default, so the change is in this PR. The architect may prefer to make it on `main`.

**Forks, ratification asked:**

- **F5 · Where scratch lives · `main.rs` · high · fork.**
  - A `TMPDIR`-only rule would refuse `/tmp` for `container/engine-smoke.sh`, which runs `check drc` as root with `TMPDIR` unset (S11). Editing it is outside the fence (a §7 stop). The per-identity default needs no edit there, nor in `ci.yml`.
  - A group-writable directory is refused even when the group is the user's own private one: the CLI cannot see who else is in a group. On Furnace, `forge`'s umask makes 0775 directories, so a `TMPDIR` must be made with `mkdir -m 700`. The default `~/.cache/aethelgard` is made 0700.
  - Scratch moves off RAM-backed `/tmp` on Furnace, which helps review A12.1.
- **F6 · The tool's directory is root's, group 65533, mode 1770, not "owned by the tool uid, mode 0700" · `run.rs` · high · fork.**
  - In a tool-owned directory the tool uid could rename the root-owned copy aside and put its own in its place. §5.1 counts 65534 among the attackers.
  - With the sticky bit and root as owner, 65534 can create (its `HOME`, `target/`, reports) but cannot rename or remove what root put there. C2b measured the refusal.
  - Every tool ran unchanged: *see Gates*. §1.2's intent, a root-owned read-only copy no other uid can change, is met more strictly than its wording.
- **F7 · Root no longer reads through a path the tool uid chose · `firmware.rs` · high · fixed, outside the literal fence.**
  - Before this WO, `build` read the ELF with `is_file()` and `File::open`, and checked `starts_with(target)` lexically, which accepts `target/../..`. With CLI and tool the same uid that granted nothing. As root it would let a link the tool uid planted, or a crafted `executable` path in cargo's JSON, make root read and deliver any file.
  - Now it uses `openat2` beneath `target`, with no symlink, no `..` and no magic link, and requires a regular file. Test: `a_file_is_read_beneath_target_only`.
  - `verify.rs` and `campaign.rs` are also outside the literal fence. Their changes are only the `uid_boundary` fields §1.2 requires.
- **F8 · `check erc` now refuses what the strict reader refuses · `kicad.rs` · low · fork.**
  - A sheet can hide wherever the reader and KiCad disagree (`AET-WO-1300`'s lesson), so a schematic the reader cannot read is `MalformedSchematic`, exit 3, where KiCad used to run on it.
  - No existing input changes: every ERC input in the suites, the fixtures and the reference reads clean (tier 1; tier 2 *see Gates*). The adversarial pass read the image's 134 KiCad demo and template schematics: 20 are now `HierarchicalSchematic`, by design, and none is `MalformedSchematic`.
- **F9 · `uid_boundary` for a verdict with several tools · `run.rs` · low · note.**
  - `simulate --firmware` runs the bridge and Renode, and every gate runs a version probe. The verdict's `uid_boundary` is the judging run's.
  - It speaks for all of them because within one CLI process every trampoline either switches (euid 0, or the call refuses) or never does (any other euid). A mixed verdict cannot be printed.

**Residuals, stated in the README:**

- **R1 · A process holding gid 65533 outside the sandbox** (only root can grant it, as primary or supplementary group) cannot change the input copy. It can rewrite what the tool writes and plant files in the tool's `HOME` and in `prove`'s add-only crate root, and so decide the verdict. Measured at b09ab98 (adversarial re-test): a `.cargo/config.toml` aliasing `kani` to a planted crate passed a proof of `false` (exit 0, `true`); a planted `build.rs` ran as the tool; a planted `rust-toolchain.toml` or garbage `Cargo.lock` was read (exit 5). The same alias already worked from the tool's `HOME`. Every such plant as uid 4242 was refused. The README says the holder can decide the verdict.
- **R5 · A tool that ignores `SIGXFSZ` and deletes the file it was cut off on** is judged on what it reports (a stub measured `build_failed`, exit 1). It can turn a cut-off into the crate's `fail`, never a pass. rustc does not do this: killed by `SIGXFSZ`, it leaves its object at exactly 67108864 bytes, and `build` ends exit 5 (an 80 MiB static, measured).
- **R4 · Under rootless podman, the user who owns the user namespace** (now an honest `false`, `user_namespace`: the ruling below) (`forge` on Furnace) can reach every container process (adversarial A5). During a live `prove`, a host `forge` process went through `/proc/<cbmc>/cwd`, created a file in `tool/` and appended to the sealed `src/proofs.rs`, and the verdict said `true`. On Furnace, then, `true` excludes no `forge` process: not an agent session, not an MCP client running as `forge`. CI's rootful Docker has no such owner.
- **R2 · Root can always race** (C1b).
- **R3 · A non-root CLI gives `uid_boundary: false`** and a same-uid process wins (C3b). The deployment section says how to get a boundary without root. The verdict then still says `false`, an under-claim, which `RUL-19` prefers.

**For the protected documents (the architect's):**

- **D1 · `docs/architecture.md` §6** says a tool's scratch is under the system temporary directory and `RLIMIT_FSIZE` is backlog. Both changed here (§1.4, §1.6), and the uid boundary is new to §6's list of what every tool runs with.
- **D2 · `RUL-19` (4) and the `verify` residual** in §6 can now name `uid_boundary`. A root CLI closes the 3301 F1 window to every uid but root and 65534.

## The architect's ruling on PR #96: `uid_boundary` true only when all three hold

As built for review, `true` claimed more than the run showed in two cases the CLI can see for itself: A1 and R4. Now `uid_boundary` is `true` only when all of these hold. Otherwise it is `false`, and `uid_boundary_unmet` names the first that failed:
1. `tool_uid`: the trampoline switched to 65534:65533 and read it back.
2. `user_namespace`: `/proc/self/uid_map` is exactly `0 0 4294967295`.
3. `toolchain`: every read-grant path beyond the system set (`/usr`, `/lib`, `/lib64`, `/bin`, `/sbin`, `/etc`, `/proc`, `/dev`) passes, all the way down, the rule the image check shares (the architect's addendum). An entry is unsafe if it is not owned by uid 0, is writable by others, or is writable by a group other than gid 0; links are skipped. Each ancestor of the real path must pass the same rule or be sticky. On the pinned image, `find /opt /usr -xdev` with that rule prints exactly the 144 `/opt/kani` entries (re-measured by the builder).

**Where it is built:**
- conditions 2 and 3 are checked by the CLI in `Invocation::start`, after each run, once per path per process (`run.rs`, `initial_user_namespace` and `toolchain_clean`);
- a verdict takes the first failure among its tools: `simulate` ANDs in the bridge's report (`analog::Pass::sandbox`);
- `campaign` run lines and `verify`'s gate entries carry `uid_boundary_unmet`.

**The cost, measured in the image** by walking the same paths with `find`: `/opt/cargo/bin` 5 entries in 3 ms; `/opt/rustup` 383 in 4 ms; `/opt/kani` 146 in 3 ms; `/opt/renode` 2,024 in 10 ms; the bridge's `target/debug` 22,352 in 49 ms. Each gate process walks each of its paths once. The ancestor check stops a walk before it starts when an ancestor fails, as `/repo` does below.

**What it says.** The Furnace row is measured (tier 1 and tier 2 in the image). The CI rows are the oracle's prediction, which the PR's `tools` run then checks; the oracle reads the image itself, so no expectation is written in for `prove`. On `c69ff9bc`, `find /opt /usr -xdev` with the shared rule prints nothing (re-measured by the builder), so `prove` says `true`. The engine-image row is a prediction that no run here checks: rootless podman reaches `user_namespace` first.

| where | erc, drc | prove | build | simulate, campaign |
|---|---|---|---|---|
| Furnace, rootless podman, as root | `false` `user_namespace` | `false` `user_namespace` | `false` `user_namespace` | `false` `user_namespace` |
| CI's `tools` job (rootful Docker, root), image `2df68259` (before `AET-WO-1602`), predicted and seen | `true` | `false` `toolchain` (Kani's kit, A1) | `true` | **`false` `toolchain`** |
| CI's `tools` job, image `c69ff9bc` (`AET-WO-1602`'s root-owned toolchain), predicted | `true` | `true` | `true` | **`false` `toolchain`** |
| the engine image as root on `c69ff9bc` (the bridge in `/usr/local/bin`), predicted | `true` | `true` | `true` | `true` |
| any user | `false` `tool_uid` | `false` `tool_uid` | `false` `tool_uid` | `false` `tool_uid` |

**One difference from the ruling's expectation.** In CI's `tools` job the bridge runs from `/repo/target/debug`, and `/repo` is the runner's checkout, owned by the runner's uid. That uid could replace the bridge, so `simulate` and `campaign` say `toolchain` there, not `true`. The same holds in the local podman layout (`/repo` 0775). It is the truth the rule was asked to tell. In the engine image the bridge sits in `/usr/local/bin`, a system directory.

**The tests follow the truth through an oracle written apart from the CLI** (`tests/common`, `expected_boundary`): root, the uid map, and `find … ! -type l ( ! -uid 0 -o -perm -002 -o ( -perm -020 ! -gid 0 ) )` beneath each path, with each ancestor's owner, group and mode.
- `verify_binds_the_reference_…` checks every gate against it.
- `as_root_a_third_uid_cannot_change_what_build_reads` checks `build`'s verdict.
- In tier 1: `uid_boundary_is_true_exactly_…` checks the stub; a new `a_toolbox_another_user_could_write_makes_uid_boundary_false` makes the toolbox 0777 (`toolchain` as initial root; 0775 root:root is clean under the gid-0 rule, which CI caught at `cf3506a`); and `campaign_tier1` checks that a line is `true` exactly when it names no unmet condition.
- A unit test checks the uid-map parser, the entry rule (sticky only for ancestors) and the precedence. Where `KANI_HOME` and `RUSTUP_HOME` are set, it also checks that the walk agrees with `find`: in the pinned image, `/opt/kani` is not clean and `/opt/rustup` is.
- Rootless podman reaches only `user_namespace`, so the `toolchain` rows rest on that unit cross-check locally and on the PR's CI run.

**C2's B1–B4 still hold.** They were re-run at `788d4f2`, with the reductions, gid 65533, the add-only crate root and both merges of `main` (`~/.cache/aet-1601/adv/out/HEAD`, `C2h`). The no-racer control gave ELF `2d9ee2641f6e0a1b6e48f1637cefe4fa01cab3f97849a1dd074474c879fe0d2f` and covers 9, 7: `AET-WO-1205` left the reference ELF unchanged. As uid 4242, B1 to B4 all held, each with the reference ELF, covers 9, 7 and `verify` exit 0, and every attempt was `Permission denied`. The ruling changes only what is reported, not what is enforced.

## Adversarial pass (full, §5)

**Who and how.**
- A fresh Opus subagent, chosen because this is judgement work, on a frozen copy at `858a2df` (`~/.cache/aet-1601-adv/repo`). It was resumed once on the fixes, at `b09ab98` (`repo2`).
- Every run was in the pinned image as root, through the capped runner. Some one-command containers exited before their caps were read (`CAPS NOT READ`); every long container read back `memory.max` 17179869184 and `memory.swap.max` 0.
- **Report:** `~/.cache/aet-1601-adv/FINDINGS.md`, with harnesses in `h/` and evidence in `out/`. It changed no code.

| id | §5 | attack | result at `858a2df` | fix | re-test at `b09ab98` |
|---|---|---|---|---|---|
| A1 | 1 | as uid 1001, replace Kani's `kani-driver` (the kit is 1001's in the image) | **broke**: a proof of `false` passed, `true` | none here: `Dockerfile` (A1 above) | documented; README names the check |
| A2 | 4 | `prove` on a crate without `Cargo.lock` (the `kani-*` fixtures) | **broke**: exit 5 where the base gave pass, `proof_failed`, `compile_error`; 6 tier-2 tests red | `prove`'s crate root add-only (`run::add_only`) | **held**: JSON identical to the base but for `uid_boundary` |
| A3 | 1, 2 | plant B2 as `_apt` (42:65534), `sync` (4:65534), 4242 with group 65534 | **broke**: ELF `e678e904`, pass, `true` | tool gid 65533 | **held**: all `EACCES`, reference ELF; a holder of 65533 lands (R1) |
| A4 | 1 | as the tool uid, leave a link where cargo's ELF is; a `target/../..` path | **broke**: root delivered `/root/secret` 0644 | `read_beneath` (F7), which landed after the freeze | **held**: five variants (link, `..`, hard link, FIFO, linked dir), exit 5, nothing delivered |
| A5 | 1 | host `forge` against a rootless podman container | residual: through `/proc/<cbmc>/cwd` it wrote `tool/` and the sealed copy, `true` | none: rootless podman | documented (R4) |
| A6 | 3 | drop `CAP_CHOWN`, or map only root | exit 5 where the README says 4 | `Stop::sandbox("uid", …)` | **held**: exit 4 `uid` for SETUID, SETGID, SETPCAP, CHOWN and root-only mapping; FOWNER dropped still switches, so `true` |
| A7 | 1 | uid 4242 during a live `prove`: scratch, call dir, `tool/`, copy, `/proc/<cbmc>/{root,cwd,fd,environ,maps,mem}`, `ptrace`, `kill`, toolchain dirs | held: `EACCES`/`EPERM` everywhere | — | — |
| A8 | 2 | `TMPDIR`/`HOME`: space, `;`, newline, link to `/tmp`, `/tmp`, group-writable, foreign-owned, no o+x, missing, relative, unset | held: exit 4 naming the right directory, before any tool | — | — |
| A9 | 3 | make `uid_boundary` lie: forge the status line, an empty campaign, setuid re-entry | held: the tool cannot write the status file; `all()` cannot be vacuous here; `no_new_privs` | — | — |
| A10 | 4 | every other verdict, base against new: ERC/DRC fixtures, generated ERC/DRC with `--spec --schematic`, both `simulate` loads, two `verify --scenarios`, runs as uid 1000 | held: identical but for `uid_boundary` (DRC uuids KiCad mints each run predate this WO). Of the image's 134 KiCad demo and template schematics, 20 are now `HierarchicalSchematic` (by design) and none `MalformedSchematic` | — | — |
| A11 | 1 | the file-size cap on a real tool: an 80 MiB static in firmware | — | — | **held**: exit 5 naming the `.rcgu.o`; a deleting tool is R5 |

**Not covered by the adversary:** the full tier 2 (the builder's own run, under Gates); B1–B4 through `verify` after the fixes (B2 against `build` only; the builder's C2 and C2b cover `verify` at `858a2df`'s design with gid 65534, and A3 re-tests the gid change); Kani, cbmc, KiCad or Renode driven to the cap; any host run as `forge` (tier 1 on the host is the builder's).

## Mutation testing (sized to the diff)

A default-model subagent ran one mutant per guard this WO adds, on frozen copies (`~/.cache/aet-1601-mut/RESULTS.md`), restored every file and `diff -r`'d `src` clean against a pristine export.
- **Round 1** (`bd389fe`): 24 mutants, 18 killed. It found a test bug: as root, the `below` base (0700) was refused for its own mode, so the root-only cases never ran, masking M15 and M17. Fixed (0711 as root), and both were killed in its probe.
- **Kills added for the survivors:**
  - M10 (the seal's not-root's refusal): the hand-over unit test;
  - M24 (campaign line `uid_boundary`): `campaign_tier1` asserts it.
- **Round 2** (`b09ab98`): N1–N8 for the fixes, plus M10 and M24 again. N1–N3, N5, N6, N8, M10 and M24 were killed; N5 by all six real-Kani `prove` tests.
- **Survivors:**
  - **M6** (the bounding-set re-check after the switch): relocated by the reduction pass, not removed. The read-back now ends `empty_bounding_set`, and a mutant deleting it would still survive: a successful `PR_CAPBSET_DROP` cannot leave the bit set, so it is a verify step with nothing to catch. **M11** (the seal's dev/inode re-check): removed, since only root can swap an entry in a 0700 root directory. After the M6 move, M5 (deleting `empty_bounding_set` from `separate_uid`) would have survived. It is now killed by a direct test (`the_trampoline_leaves_the_tool_its_ids_and_no_capability_at_all`, `CapBnd` zero as the tool reads it), re-run in the image by the builder.
  - **N4** (a failed hand-over reported as `ToolError` rather than `SandboxUnavailable`): no tier-1 path can make the hand-over fail. The adversary measured it (A6).
  - **N7** (`read_beneath`'s regular-file check): near-equivalent, since a directory still fails at `read_to_end` (`EISDIR`). A FIFO is the case it exists for; the adversary's A4 FIFO variant was refused with no hang.
- **Containers:** every mutation container read back `memory.max` 17179869184, `memory.swap.max` 0, `pids.max` `max`.

## Reduction pass

A fresh default-model subagent read the diff (`cbc0a7b..b09ab98`), proposal-only. The builder applied these:
- the bounding-set read-back now lives once, in `empty_bounding_set`. `drop_capabilities` lost its `separated` parameter (M6);
- the seal's dev/inode re-check is gone (M11). The directory is root's 0700 until the hand-over, so only root could swap an entry;
- the seal's link arm is gone: a link refuses the hand-over (tested);
- `parse_status` builds its refusal with `Stop::sandbox`, and its stale comment now names every layer;
- the scratch base is made by `DirBuilder::recursive`, not a hand-rolled loop;
- five dead `HOME` lines go from tests that set `TMPDIR`, and two dead probe lines from the cap test;
- the README states the failure rule and the toolchain check once each.

Reported, not removed:
- the copies of the tool ids and of the 0711/0700 choice in three test files, since tests cannot import from a bin crate (one copy in `common` is the floor);
- `scratch_base`'s doc restating the README rule;
- the seal's `st_uid != 0` refusal and `depth > 64`, both unreachable under 0700 but tested or cheap;
- the merged `ParityRefusal → Stop` closures in `kicad.rs` (a helper would be a new layer).

`Reduction pass (diff): −34 lines net · 2 duplicates removed · 3 dead paths removed · 0 protected kept · 5 findings above`

## Tests changed

- **New, tier 1** (`tests/uid_boundary_tier1.rs`, host and image). A `kicad-cli` stub reports what it sees as findings:
  - a scratch path with a space, `;` or `'` is exit 4 with nothing made;
  - `/tmp`, a group-writable base or one below it, a base another uid owns (root), a root base the tool uid cannot search, and a missing base are each exit 4;
  - the default base is `/var/lib/aethelgard` or `~/.cache/aethelgard`, and a user with no `HOME` is refused;
  - as root, the tool's ids are 65534 with no group, the copy is root 0444, `HOME` is root:65534 1770, the call directory is 0710, and the tool can neither write nor rename its copy; as a user, its own uid and `false`;
  - sheets, nested sheets and a schematic the reader refuses are exit 3; the emitted schematic passes;
  - past the cap, a child cut off and the tool cut off are both exit 5; the cap is soft and hard and cannot be raised.
- **New, unit:**
  - `run.rs`: the status line's `uid_boundary` is exactly `0` or `1`; the read-back needs all four ids and no group; the hand-over is once, keeps an add-only directory, and is refused for an entry that is not root's or is a link (root only).
  - tier 1, `the_trampoline_leaves_the_tool_its_ids_and_no_capability_at_all`: the trampoline started as the reaper starts it, on a tool reading its own `/proc/self/status`. As root: uid 65534, gid 65533, no group, every capability set zero, the bounding set included. As a user: its own ids and no capability.
  - `verify.rs`: each gate's `uid_boundary` is its own verdict's; the campaign's is an AND.
  - `firmware.rs`: reading beneath `target`.
- **New, tier 2** (`board_tier2.rs`, counted; `TIER2_MIN` 71 → 73 with the reason beside it):
  - `as_root_a_third_uid_cannot_change_what_build_reads`: B1 and B2 automated, against `build`, uid 4242, every attempt refused, reference ELF;
  - `a_cargo_configuration_another_uid_plants_in_tmp_is_not_read`: 3301 F13 closed. A `/tmp/.cargo/config.toml` owned by 4242 that would break the link, if read, changes nothing.
- **Changed:**
  - `verify_binds_the_reference_and_two_runs_print_the_same_bytes` also asserts every gate's `uid_boundary` (root in tier 2);
  - a `campaign_tier1` test asserts the run line's `uid_boundary`;
  - `lifecycle_tier1`'s direct-reaper helper hands its directory to 65534 when root, since it bypasses the CLI's hand-over.
- **Changed for §1.4, not weakened:**
  - 14 tier-1 call sites that clear the environment and set no `TMPDIR` now pass the test's own `HOME` through, without which a user CLI has nowhere private;
  - `lifecycle_tier1` and `mcp_tier1` take their observed `TMPDIR` from `common::private_tmp`, under the same base, instead of `/tmp`.

## The fence

`git diff --stat cbc0a7b` (`main`, merged in): `run.rs`, `main.rs`, `kicad.rs`, the CLI README; `README.md` (the quickstart only), `ci.yml` (`TIER2_MIN` only); tests; this FEEDBACK. Outside the literal fence, forced by §1.2: `verify.rs`, `campaign.rs` (the `uid_boundary` fields) and `firmware.rs` (F7). No `Dockerfile`, image, workflow besides `ci.yml`, generated file or steering document changed; the one protected file touched is `README.md`'s quickstart block (F4). No `unsafe`. No new dependency: `rustix`'s `fs`, `process` and `thread` features were already on.

## §3, acceptance

| item | where it is shown |
|---|---|
| §1.1's table, each prediction beside its result | §1.1 above (`spike/PREDICTIONS.md` was written before the runs) |
| In the image, B1–B4 with the CLI as root, the tools as 65534 and the racer a third uid, each fail to change what the tool reads; `verify` says `uid_boundary: true` for every gate | C2 (all four held, `true`). Automated in tier 2: `as_root_a_third_uid_cannot_change_what_build_reads` (B1, B2) and `verify_binds_the_reference_…` (every gate `true`) |
| Root against root still wins, and the verdict says `false` | C1b wins, but says `true`: see F1 (decision-needed). The `false` witness is C3b and the host |
| On Furnace as `forge`, the verdict says `false` | tier 1 on the host: `uid_boundary_is_true_exactly_when_the_tool_ran_under_the_tool_uid` |
| A planted ancestor configuration is not read, or is refused with the new error | `/tmp` is no ancestor: tier 2 `a_cargo_configuration_another_uid_plants_in_tmp_is_not_read` (uid 4242's plant, which would break the link, is not read); `TMPDIR=/tmp` is exit 4 `ScratchUnsafe` (tier 1) |
| A scratch path with a space or `;` is exit 4 before any tool runs | tier 1 `a_scratch_path_with_a_space_or_a_semicolon_…` |
| A hierarchical sheet is refused before ERC | tier 1 `a_schematic_naming_a_hierarchical_sheet_…` |
| A tool past `RLIMIT_FSIZE` is exit 5; the reference passes under the cap | tier 1 `a_tool_writing_past_the_file_size_cap_is_a_tool_error`; adversarial A11 (rustc); tier 2 passes in full under the cap |
| The gates pass; `gates`, `tools`, `image` and the quickstart green in CI | local below; CI on the PR |

## Gates

At `d45691f`: the code, with `main` at `dbf2cba` merged in, in the new pinned tools image `c69ff9bc` (`AET-WO-1602`). Every later commit changes this FEEDBACK only.
- **Host, as `forge`:** `cargo fmt --all -- --check` ok; `cargo clippy --workspace --all-targets -- -D warnings` ok; `cargo test --workspace --no-fail-fast` gave 910 passed, 0 failed.
- **Image `c69ff9bc`, as root, the `tools` job's steps** (a networked build, then offline):
  - bridge tests ok, `fmt` ok;
  - clippy with `with-tools` ok;
  - `cargo test --workspace --features aethelgard-cli/with-tools --no-fail-fast`: **978 passed, 0 failed**;
  - **canary `tier2_tests_passed=73`, floor 73**;
  - caps `memory.max` 17179869184, `memory.swap.max` 0, `pids.max` `max` (`aet1601-tools7`).

  Under rootless podman every gate says `false`, `user_namespace`, as the oracle requires.
- **`container/engine-smoke.sh`**, run as root in that image with this branch's binaries first on `PATH`: all checks passed.
- **The quickstart's tool calls**, in its `--user` shape with `-e HOME=/work`, pass on both the README's pinned engine image and this branch's binary (F4).
- **CI:**
  - on `7adb923`: all four jobs green;
  - on `cf3506a`: `tools` red on the one stale test (0775 root:root is clean under the gid-0 addendum), fixed;
  - on `c730163`: `gates`, `image` and `quickstart` green.

  The run on this head is the record.
- **`origin/main` merged** with merge commits: `cbc0a7b`, `0249824` (`AET-WO-1205`), `b227c12`, `05edb43` (`AET-WO-1602`) and `dbf2cba` (the new pin), each clean.
