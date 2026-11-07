# AET-WO-0300b — FEEDBACK

**Branch:** `wo/aet-0300b-verdict-gaps` · **Base:** `f9f7a04` (PR #17) · **Builder:** Claude Code (Sonnet 5.5) as `forge` on Furnace · **Result:** §1 built; every §3 item passes; no §7 stop.

Entry schema: `finding · where · type · recommendation · decision-needed`.

## Item 1.1: the measurement

Run in `tools`, inside the image pinned by `container/IMAGE`, by a temporary tier-2 probe (since removed; the final diff leaves `tests/with_tools.rs` unchanged). Run: https://github.com/ClearForge-LLC/aethelgard/actions/runs/42406190783 (job `tools`; the probe panicked on purpose so the harness printed its captured `ANSWER 0300b-1.1:` lines).

Loaded the way `simulate` loads (`--disable-gui --console --plain`, `DOTNET_EnableDiagnostics=0`), every line containing `[ERROR]` or `[WARNING]`:
- `@platforms/boards/nucleo_h753zi.repl`: **none** (exit 0, one CPU slot).
- `@platforms/cpus/stm32h753.repl`: **none** (exit 0, one CPU slot).

So §7's first stop did not fire. The same result held on Furnace with the same Renode 1.17.0.

## Item 1.2: the fixture

**No input was found that lists a CPU and also logs `[ERROR]`. No tier-2 fixture was added.** Tried, all in the same CI run and on Furnace:

| Input (appended to an `using` of the Nucleo board) | CPU listed | Output |
|---|---|---|
| unknown peripheral type; unknown property; duplicate registration; overlapping memory; misaligned memory; unregistered irq target; bad `cpuType` | no | `There was an error executing command 'machine LoadPlatformDescription …'` + `Error Exx` text |
| `sysbus: init:` with a bogus command, or `LoadELF @/nonexistent.elf` | yes | `There was an error executing command 'sysbus …'` (no `[ERROR]`); already the existing exit 5 `ToolError` |
| two overlapping `Tag`s in `init:` | yes | nothing |

Renode 1.17.0 reports platform failures through the monitor's "There was an error executing command" line, not through the `[ERROR]` log level. The new `judge` arm is therefore a fail-closed backstop that has been exercised only on a stub transcript (§3.2); it has not been observed on real Renode. The adversarial pass reached the same conclusion independently (it also saw `[WARNING]` from an unmapped-address access in an `init:` block, with the CPU listed and no `[ERROR]`).

## What was built

- `renode.rs` `judge`: a CPU listed with no `There was an error executing command` and any `[ERROR]` line gives fail, exit 1, `platform_load_error`, message = the first `[ERROR]` line, trimmed; no `machine`. The other three outcomes, and `[WARNING]` handling, are unchanged.
- `input.rs`: `MAX_INPUT_BYTES` (moved here from `main.rs`; one definition, used by `validate` too) and `MAX_CRATE_FILES = 4096`. `copy_hashed` takes the remaining budget, reads at most one byte past it, and refuses with `InputTooLarge` (exit 3). The budget is shared across a crate's files; the file cap fires in `walk`, before any copy. `run.rs`: `Stop::too_large`.
- README: exit 3's row and `simulate`'s row of the witness table.
- Tests (tier 1, `tools_tier1.rs` §12): the `[ERROR]` fail with its exact message and no `machine`; a `[WARNING]` still passes; a crate 1 byte over / exactly at the byte cap; three files each under the cap but over together; 4,096 vs 4,097 files; a 16 MiB+1 sparse file under top-level `target/` ignored; a board file 1 byte over / exactly at the cap. Refusals check that the message names the input and the cap. No existing test was edited.

## Gates

- Local (Furnace): `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings` (also with `--features with-tools`), `cargo test --workspace`: pass. Tier 1: 110 tests in `tools_tier1` (101 before).
- CI, first push (`gates` and `tools` both green on the first run): https://github.com/ClearForge-LLC/aethelgard/actions/runs/42405828419. Probe run (red by design): https://github.com/ClearForge-LLC/aethelgard/actions/runs/42406190783. The final push's run is on PR #18.
- Tier 2: every existing case passes unchanged (13 tests in `with_tools`, including `simulate_loads_the_reference_board`, which now also proves a clean Nucleo load has no `[ERROR]`).
- Reduction pass (diff-sized): removed the second `Stop` constructor I had added for the file cap (one `too_large(message)` now), the temporary probe, and a stray `mut`; nothing else in the diff was removable without losing a §3 item.

## Adversarial pass (fresh Sonnet subagent, §5; it ran real Renode and a copy of `input.rs` in a scratch harness)

No high or medium findings.

| # | finding · where | severity | type | recommendation | decision-needed |
|---|---|---|---|---|---|
| A1 | The check is a case-sensitive substring match on `[ERROR]`. An error under another tag (`[ERR]`, `[FATAL]`), or with a colour code inside the brackets, passes. `--plain` rules out colour (zero ANSI bytes, and none when not a tty); it does not rule out other tags. Real Renode uses only `[INFO]`, `[WARNING]`, `[ERROR]`. · `renode.rs` | low | accepted gap | none; the load witness for real failures is "There was an error executing command" | no |
| A2 | A tag split across two writes would be missed. Renode writes one message per line; not reproducible. `(monitor)` echo of our own command wraps mid-line but holds no `[ERROR]`. · `renode.rs` | info | not reproducible | none | no |
| A3 | Stderr-only output is covered: `Output::text()` is stdout then stderr. | info | held | none | no |
| A4 | No input found that reaches the new arm on real Renode (see 1.2). | info | coverage | keep as backstop | no |
| A5 | Cap held against: exactly-at and 1-over for file and crate; many files under the per-file size; hard links (each link counted: conservative); a 1 TiB sparse file (reads 16 MiB+1, refused); a file growing during the copy (3 trials, refused, copy capped at 16 MiB); nested `sub/target/`, a top-level file named `target`, a top-level symlinked `target` (counted or refused; only a top-level `target/` directory is skipped); hash equals `sha256sum` of the copy. No copy path bypasses the budget. | info | held | none | no |
| A6 | Empty directories and tree depth are neither counted nor capped; empty directories are not recreated in the copy (consistent with the hash). 20,000 empty directories passed. Cost is walk time and memory. · `input.rs` `walk` | low | gap | none under the scope test; or state in the README that 4,096 counts files only | no |
| A7 | `copy_hashed` opens with `File::open`: no `O_NOFOLLOW`/`O_NONBLOCK`. A file swapped for a FIFO between walk and open would block `open` before the `is_file` check. Pre-existing; the diff did not introduce it. · `input.rs` | low | backlog | `O_NOFOLLOW\|O_NONBLOCK` as `kicad::read_report` already does | yes, if wanted |
| A8 | A partial copy (up to 16 MiB) stays in scratch until the temporary directory is removed. | info | by design | none | no |
| A9 | Which refusal comes first (`UnreadableInput` vs `InputTooLarge`) for a crate with both a symlink and over 4,096 files depends on `read_dir` order; both exit 3. No test pins it. | info | note | none | no |

## Not built

- A tier-2 fixture for item 1.2 (none could be made; see above).
- Any change to how KiCad or Kani output is read; any time cap; any allowlist of `[ERROR]` lines (none was needed).
- A directory-count cap, `O_NOFOLLOW` on the copy (A6, A7).

## Promote or needs-none

`architecture.md` §2.4: **promote** one sentence: Renode 1.17.0 reports platform failures as "There was an error executing command", never at `[ERROR]` level, in everything tried; the `[ERROR]` check is a backstop. `architecture.md` §6 ("copy first, then hash, then judge"): **needs none**; the cap sits inside the copy step. `RUL-13` already states the 16 MiB cap; the file cap of 4,096 is new: **promote** it there if the architect wants the number in the architecture and not only in the README. Both are the architect's edit; the steering documents are untouched here.

## Process notes

- Item 1.1 needed `panic!` in the probe to surface stdout, because `.github/workflows/` is protected and the job runs without `--nocapture`; that cost one extra (red) CI run.
