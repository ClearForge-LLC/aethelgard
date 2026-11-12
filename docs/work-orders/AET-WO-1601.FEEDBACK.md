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

**Why 65533 failed (S7).** Under every uid with no passwd entry (65533 and 65520 measured), the bridge dies. `strace` shows NSS looking the uid up in `/etc/passwd`, then `/etc/userdb`, `/run/userdb`, `/run/host/userdb`, `/usr/local/lib/userdb` and `/usr/lib/userdb`, each `ENOENT`, and then `SIGSEGV` at address `0x20`, which is `pw_dir` in a NULL `struct passwd`. libngspice reads `getpwuid(getuid())->pw_dir` without checking. Uids 65534 (`nobody`) and 1000 (`kicad`) pass.

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

| config | prediction | result |
|---|---|---|
| C1: CLI as root, racer as root, unmodified tree `366e8fd` | all four broke; no `uid_boundary` key | B1 **broke** (ELF `897184e4…`, `verify` exit 0). B2 **broke** (ELF `e678e904…`). B3 **broke** (cover 1). B4: *pending* |
| C3: CLI and racer the same non-root uid, unmodified tree (image under `--userns=keep-id`, uid 1001; the host has no `kicad-cli` or `ngspice`) | all four broke | *pending* |
| C2: CLI as root, tools as 65534, racer a third uid (4242), after §1.2 | all four held, every attempt `EACCES`; `uid_boundary` true for every gate | after the build (below) |
