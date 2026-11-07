# AET-WO-0400 — FEEDBACK

**Branch:** `wo/aet-0400-trace-archive` · **Base:** `f9f7a04` (PR #17; `origin/main` has since moved to `1a22e40`, docs only) · **PR:** #21 · **Builder:** Claude Code (Sonnet 5.5) as `forge` on Furnace · **Half:** build-only; no §7 stop arose.

Entry schema: `finding · where (file:line) · type · recommendation · decision-needed`.

## What was built

`aethelgard-chaos` graduates from a stub to the trace archive, with `sha2` and `thiserror` only: `cbor.rs` (encoder, strict decoder), `event.rs` (header, events, manifest), `replay.rs` (fold and hash chain), `archive.rs` (writer, commit protocol, reader), `mirror.rs` (credential boundary), `synthetic.rs` (SplitMix64 stream). `workspace_shape.rs` drops `chaos` from the two stub checks and gains `chaos_has_only_its_known_files_keys_and_dependencies` (§1.6). `LIBRARY_DEPENDENCIES` is unchanged: `chaos` had added nothing to it, and `sha2` was never in it.

## What did not work, and why

- **First kill test hung.** libtest prints `test name ... ` without a newline before the child's marker, so a `starts_with` match never fired and three sleeping children outlived the run. Fixed with `contains`; the strays were killed by pid. No product code was involved.
- **Two test arithmetic slips** (nesting depth off by one; the regression index of a swap), found by the first run of `tests/archive.rs`. No code changed.
- **`std::env::var_os` is not a valid `impl Fn(&str)`** (not general over lifetimes). The test pins `|name| std::env::var_os(name)`, and the README says so; the WO's wording "Production passes `std::env::var_os`" is loose by that much.

## Findings

| finding | where | type | recommendation | decision-needed |
|---|---|---|---|---|
| Artifact names in the header are bounded to 1–1,024 bytes of printable ASCII with space. §1.2 specifies only sorted and unique, and "no text in a trace has two Unicode spellings" needs a set | `event.rs` `Header::check` | scope-question | Keep. Rule the set (maybe the `detail` set, as built) in `RUL-14` or the WO | yes |
| An empty-string environment variable counts as unset in the mirror step. §1.5 says "unset" | `mirror.rs` | scope-question | Keep: an empty bucket or key is no credential. Rule it | yes |
| `MissingTrace` and `MalformedHeader` are variants §1.3's table does not list (manifest present but no `trace.cbor`; header of the wrong shape) | `lib.rs` `ArchiveError` | note | Add both rows to the table | no |
| A literal mid-event cut cannot come from this writer: every event is one `write_all` through a `BufWriter`, which never splits it. The "mid-event" kill therefore stops a writer running flat out (a 64-byte buffer, an endless stream, SIGKILL on first progress); the cut-inside-an-event case is covered by the §3.3 truncation test | `tests/kill.rs`, `tests/archive.rs` | note | None | no |
| `kill_child_entry` shows as a passing test in every normal run. It is the writer role the kill test re-executes; it returns at once without its environment variables | `tests/kill.rs` | note | None | no |
| The reader loads the whole trace into memory after the length check against the manifest. Bounded by what the manifest declares, fine at 192 KB per 10,000 events; a long campaign wants a streaming reader | `archive.rs` `replay_dir` | risk | Ruling at `OQ-03` / P2 when volume is measured | no |
| `State::encode` is the profile, but the encoder does not enforce the decoder's 65,536-element bound, so a state with more than 65,536 registers still hashes. Hashing the whole state per event is O(state) | `replay.rs` | note | If P2 measures this as too costly, changing it is a ruling (§1.4) | no |
| Opening a file after checking its type is a check-then-use gap (no `O_NOFOLLOW`, which needs `rustix`/`unsafe`-free equivalents the workspace lacks for this) | `archive.rs` `check_entries` | risk | Out of scope for an archive on a private disk; note for the sandbox work | no |
| Hard-linked or FIFO members: a FIFO fails the regular-file check; a hard link is a regular file and is accepted | `archive.rs` | note | None | no |

## Adversarial pass (§5, targeted)

A fresh subagent (Sonnet 5.5) ran it at `e2d6991` in its own worktree, with probe tests it did not commit. Every finding below was read in its report; those marked *run* it reproduced by running. I did not re-run them except #4, whose fix I mutated.

**Forgery (§5.1).** Rewriting the events and recomputing all four manifest fields (`trace_bytes`, `trace_sha256`, `event_count`, `state_hash`) **succeeds**, as expected: the manifest proves integrity, not authenticity (`a_forged_trace_with_a_recomputed_manifest_passes` records it). The subagent found no other way through: the hash chain is fixed-width over an injective self-delimiting encoding, the decoder admits one encoding per value, and each single-field change is refused. Two writes to different registers swapped is caught by the chain. Mutation fuzz: 3M iterations, 537,843 accepted, no round-trip failure.

| # | Finding | Severity | Done | Decision-needed |
|---|---|---|---|---|
| 1 | **Replay is O(events × state size).** Hashing the whole state per event is quadratic when keys are unbounded (*run*: 5,000 / 10,000 / 20,000 distinct register writes took 0.4 / 1.6 / 6.5 s; a hostile 1M-address trace, self-consistent because the attacker recomputes the manifest, extrapolates to hours). Past 65,536 entries the hashed state encoding itself leaves the decoder's array bound. §1.4 chose this and says changing it is a ruling | medium-high | Not changed (a ruling) | **yes**: cap distinct keys, or hash incrementally |
| 2 | **Decode memory is about 32 bytes per input byte, and the trace size is uncapped** (*run*: a 16.8 MB crafted event took peak RSS from 36 to 578 MB before `Event::from_value` refused it). The bounds are per array and per depth, not per item | medium | Not changed | **yes**: a trace-size cap in the reader, and a shape check before nested values are built |
| 3 | **A crash between `mirror.cbor.tmp` and its rename leaves a committed run that replay refuses (`UnexpectedEntry`) and the mirror cannot retry** (*run*, with a hand-planted tmp). That sits badly with §1.5's "the commit's result is unchanged". The kill test covers only the manifest's stages. §1.3 says a leftover tmp is refused, so the code follows the text | medium | Not changed | **yes**: rule that a stale `mirror.cbor.tmp` is ignorable, or that the mirror step removes it |
| 4 | **A symlinked run directory passed the check when the path ended in `/`** (*run*) | low-medium | **Fixed**: `replay_dir` normalises the path's components first. `symlinks_are_refused_wherever_they_sit` now covers the slash; with the fix disabled that test fails | no |
| 5 | Check-then-use gap between `check_entries` and the opens (no `O_NOFOLLOW`, no fstat on the handle); a concurrent attacker could swap in a FIFO or `/dev/zero` (reading only) | low-medium | Not changed | no: a private disk; revisit with the sandbox work (see the table above) |
| 6 | `mirror.cbor` is never read, so its contents and size are unchecked (*run*: 1,000 bytes of `0xFF` replay fine). That follows "replay never reads it" and sits against §1.1's "each file is consumed exactly" | low | Not changed | **yes**: read it in replay, or accept a covert-data slot |
| 7 | `state_hash` does not see timestamps, fault scenario or detail, or no-op events (*run*: two traces differing only in `detail` share a state hash). Only `trace_sha256` protects them | low | Documented here | no |
| 8 | The parent of the run directory is never `sync_all`ed, so after a power cut a committed run could vanish; `sync_all` is a no-op on tmpfs, and the SIGKILL tests exercise none of it. §1.3 asks only for the run directory's sync | low | Not changed | no |
| 9 | "A timestamp of `u64::MAX` followed by one more event" refuses only if that event is lower (equal timestamps are allowed, §1.2) | low | Test uses a lower one | no |
| 10 | Variants outside §1.3's table: `MissingTrace`, `MalformedHeader`, `MirrorBeforeCommit`, `MirrorExists`; `MalformedManifest` carries text, not an offset; the log says "is not set" for an empty value; a hard-linked `trace.cbor` is accepted (sha-bound, harmless) | note | Listed above | no |
| 11 | A failed `put` after `replayer.apply` leaves the writer ahead of the file; the commit is then refused on read, so it is safe | note | Not changed | no |

Not tested by the subagent: `sync_all` across filesystems, a real TOCTOU race, a multi-GB file, a kill during the mirror.

**Assumptions that hold by luck, checked:** `UnexpectedEntry` names the first bad entry in sorted order, so directory listing order does not matter; the generator does produce equal timestamps (the golden test asserts it); the chain catches a swap of two writes to different registers (test); the writer refuses everything the reader would refuse except the over-large state of #1.

## Gates

- Reduction pass (diff): `Reduction pass (diff): 6 cuts · unused read_item, bytes_written and a debug_assert removed · the RefCell pair of publish closures merged into one hook · a mem::take and a stray test variable removed · 0 protected kept · 0 findings`.
- Local: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` all green on Furnace.
- `ci` on `e2d6991`+reduction (`35c83fc`): [42407103904](https://github.com/ClearForge-LLC/aethelgard/actions/runs/42407103904), `gates` and `tools` both green **on the first push**, with `golden_trace_and_state_hash_are_pinned` and the four kill tests passing in `gates`.
- **Golden constants** (sha256 of `trace.cbor`: `51c6193f…896d`, `state_hash`: `535db225…adac4`, 191,873 bytes) were computed on Furnace and reproduced unchanged in `gates`. No §7 stop.
- Protected surfaces: `git diff f9f7a04 --name-only` lists only `Cargo.lock`, `crates/aethelgard-chaos/**` and `crates/aethelgard-cli/tests/workspace_shape.rs` (plus this file).
- One mutation, on the guard this WO adds late (the trailing-slash normalisation): disabling it makes the symlink test fail.
- The adversarial subagent's isolated worktree was created under `~/aethelgard/.claude/worktrees/` by the harness. I removed it and its branch (`git worktree remove --force`, `git branch -D`) once its report was in; nothing else in `~/aethelgard` was touched.

## Plumbing numbers (not `OQ-03`)

Encoded bytes per event, over the golden stream: register write 17.0, pin level 13.1, interrupt 9.7, analog sample 22.3, fault injected 33.8 (mean 19.2; 191,873 bytes for the header and 10,000 events). Write throughput on Furnace for the 10,000-event stream, `write + commit` including `sync_all`s: **0.213 s, 46,888 events/s in release; 1.334 s, 7,496 events/s in debug.** Replay: 0.195 s release, 1.308 s debug. These measure the plumbing, not any volume `OQ-03` asks about; the replay figure is dominated by the per-event state hash of finding #1.

## What was deliberately not built (§4 held)

R2 upload, S3 client, any network, credential or bucket; real simulator events; a CLI subcommand; compression, retention, sampling or garbage collection; signing; a second writer. No `unsafe`, no dependency beyond `sha2` and `thiserror`.

## Promote-or-needs-none

- **`AET-WO-0300a` FEEDBACK:** its findings were promoted at its close through PRs #16 and #17 (the `architecture.md` and roadmap amendments). Its remaining open item, B5 (non-socket IPC outside the filter), belongs to a later sandbox work order; finding #5 above is a second input to it. Nothing else needs promoting.
- **`AET-WO-0300b`:** not merged at `origin/main` `1a22e40`, so there is no FEEDBACK to read.

## Promotions this work order asks for

Findings 1, 2, 3 and 6, plus the three `decision-needed` rows in the first table, are the architect's to rule; the first two bear on `OQ-03` and P2.
