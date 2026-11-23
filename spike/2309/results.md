# AET-WO-2309 §1.1 — results, measured after the predictions (`4a84ce75`)

Main is `28fba951`. The rules were built in the worktree as `patches/rules-full-reading.diff`, measured, then taken out of `crates/`. **This branch changes no byte of `crates/` or `fab/`.** The work order stops at §7 (FEEDBACK, "§7 stop").

## Sweep A: the committed archives

`sh/sweep-a.sh`, over `git ls-files '*/trace.cbor'`: four archives under `spike/3305/out/archives`.
- **Before** (main's CLI) and **after** (the branch's CLI, built with the full patch): byte-identical (`out/sweep-a-before.txt`, `out/sweep-a-after.txt`).
- Each is refused as `UnknownVersion`, "unknown format version 14 in header", exit 3. Today's reader is trace 18, so no record rule ever runs on them.

## Sweep B: current-version archives, made once by main's CLI

`sh/measure.sh` is 3306's groups unchanged. It ran in the tools image under `capped-run` (8 GiB, swap 0, read back), with main's CLI from a `git archive` of `28fba951`. It made 64 archives. `bme-bmp` and `bme-bmp-0x47` no longer generate on main (`generate bmp failed`): `spike/3306/spec/bme-bmp.json` is a 3306 spike specification, not a committed group.

`sh/sweep-b.sh` replays every archive with main's CLI and the branch's (full patch): **64 of 64 exit 0, byte-identical output** (`out/sweep-b.txt`).

`census/` (`census2309`, through `replay_dir` alone) describes each archive (`out/census-b.txt`):

| | Measured |
|---|---|
| Runs | 63 read and booted, 1 read and not booted (`seeded-no-boot-word`), **0 unread** |
| Kind 11 | 1,890 read, 0 unread |
| Kinds 12 and 21, value and status | 96 and 128 read, 0 unread |
| Kinds 13 and 22 | 96 and 128 read, 0 unread; none unread holding `absent` or `unrouted` |
| Kinds 24 and 26 | 15 runs hold 4 of each; 49 hold none. The unbooted run holds no motor output |
| Largest trace | 9,056 B |

**The flooded log, made for P8** (`sh/flood.sh`, in the image, main's CLI). One campaign run of the tier-2 flooding firmware (`tests/fixtures/flood`) on `sentinel-01-minimal`, expecting a pass, so it archives whatever the judge finds (`out/flood/`):
- read and booted;
- kinds 11, 12 and 21 all read;
- **kinds 13 and 22 unread** (the log passed the filter's budget).

It replays byte-identically under main's CLI and under the branch's full patch, exit 0 (`out/flood/replay.txt`). The archive is committed as `out/flood/7a624e30…`: the first trace-18 archive in the repository.

## Sweep C: every `check_run` input the tier-1 suite builds

`sh/sweep-instrument.diff`, applied to a copy of main only, wraps `replay_dir` and `check_run`. For each call it logs:
- the sha256 of its input;
- the test thread;
- main's result;
- what each candidate rule would say of the same records.

Nothing panics, so every fixture of every test is judged. `cargo test --workspace --no-fail-fast`, with the target inside the copy (a target outside it makes `fab_tier1`'s `BaseOutsideRoot` fail on main: an artefact of the sweep, not of main). `sh/sweep-c.py` reads the log. Its output is `out/sweep-c.txt`.

The candidates:
- **motor**: rule (2) as the work order says it;
- **full**: rule (3) with kinds 11, 12 and 21 (value and status) read exactly when kind 10 is, and kinds 13 and 22 unread with it;
- **status**: as full, but kinds 12 and 21 bound both ways by their status and their value one way. In `patches/rules-full-reading.diff` that is one line, `observed.is_some() == read && status.is_some() == read` replaced by `status.is_some() == read && (read || observed.is_none())`.

SWEEP-C-TABLE

## Predictions against results

| # | Prediction | Result | |
|---|---|---|---|
| P1 | Sweep A vacuous, identical | four `UnknownVersion`, identical | held |
| P2 | Sweep B 60–80 archives, all identical | 64, all identical | held |
| P3 | Read boots read every 11, 12, 21; unread boots none | read half: held on 64 runs. **Unread half: no unread archive in sweep B**; it rests on `main.rs` `boot_report` (every value is `observed.map(..)`) and on the tier-1 fixtures | half measured |
| P4 | Unread witnesses only with an unread boot, or past the budget | held: none in sweep B; the flooded run is the one | held |
| P5 | One kind 26 per kind 24 in every read run | held on the 15 booted motor runs; **no read-unbooted run holds a motor output**, so the unbooted half is unmeasured | half measured |
| P6 | Largest trace under 64 KiB | 9,056 B | held |
| P7 | Sweep C: every main outcome unchanged | **missed** (the §7 stop, below) | missed |
| P8 | The flooded read boot replays after | measured, replays identically | held |
| P9 | The writer's 256 MiB test under 10 s | not measured: no test was written before the stop | open |
| P10 | One mutant each, killed by name | not run: the stop came first | open |
