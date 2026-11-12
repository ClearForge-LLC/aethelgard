# AET-WO-1900 — FEEDBACK

**Branch:** `wo/aet-1900-evidence` · **Base:** `6a765c6` · **Builder:** Claude Code (Sonnet 5.5) as `forge` on Furnace · **Cadence:** build-only. No §7 stop. The I2C witness measurement (§1.10) found a forgery that passes and no observable fits this work order, so it goes to `AET-WO-2301`, as §7 allows; **say so in the PR.**

Entry schema: `finding · where · type · recommendation · decision-needed`.

## Where everything ran

Tier 2 ran in the pinned tools image (`container/IMAGE`, `sha256:2df68259…`) under rootless podman: `--memory 24g --memory-swap 24g --pids-limit -1`, then `0` written to the container's `memory.swap.max`. Read back from `/sys/fs/cgroup/…/libpod-<id>.scope`: `memory.max` = 25769803776 (24 GiB), `memory.swap.max` = 0, `pids.max` = max. Host tier 1: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace --no-fail-fast` all clean, in a `systemd-run --user --scope -p MemoryMax=32G -p MemorySwapMax=0` cgroup. In the image, `cargo test --workspace --offline --features aethelgard-cli/with-tools --no-fail-fast`: no failure; the first run found one the host cannot (`with_tools.rs:594`, below).

## Test pairs (fails before, passes after)

| §1 | Test | Before |
|---|---|---|
| 1 | The `tools` job's canary step, floor `TIER2_MIN=64` | Measured 64 passed in the seven tier-2 binaries; the same binaries built **without** the flag pass 4 (awk run on both logs) → the step fails |
| 2 | `validate::missing_key_in_each_struct_is_refused` now removes every key | The review's mutant (`#[serde(default)]` on `Specification.spi_devices`): old test green, new test red; reverted |
| 3 | Workflow steps only (see below) | Shell of the step simulated with a fake `docker`: other-commit label, tag, foreign repository all exit 1 |
| 4 | `readme_status::*` | `a_stale_status_line_or_an_unmarked_phase_is_caught` mutates the README (two phases behind, the review's finding) and the roadmap; both differ from the real pair. The README itself was already fixed on `main`, so the real-file test passes before and after |
| 5 | `emit_tier1` / `with_tools` (both places) / `adversarial_1300`, plus a unit assert in `rules.rs` | `"specification"` → `"engine-floor"`. `with_tools.rs:594` was missed by the first commit and found by the image run |
| 6 | `validate::refuses_names_that_differ_only_in_case` (`gnd`, `i2c_scl`, `vdd_3v3`) | red on `main`'s `lib.rs` |
| 7 | `archive::a_two_mebibyte_manifest_is_refused_by_name` (`ManifestTooLarge`) | no such variant on `main` |
| 8 | `bounds::a_second_once_only_record_is_refused_at_fold_time` | no `DuplicateRecord` on `main` |
| 9 | `tools_tier1::a_relative_path_entry_is_refused` (`.`, an empty entry, a bare name) | red on `main` |
| 10 | `twin_tier2::a_firmware_that_writes_the_witness_registers_by_hand_passes_today_a_named_residual` | pins the measured pass; see below |
| 11 | A7.4 `mcp_tier1` storm asserts ≥100 descendants seen (mutant: sampler records nothing → red); A7.7 `routing_tier1` asserts each edit's own reason (mutant: one reason swapped → red); `campaign_tier1::replaying_a_missing_directory…` asserts `Io` | |
| 12 | D10 `workspace_shape::the_tools_image_carries_kicads_library_licence`; B9 `…the_dev_profile_optimises_the_router_crate`; F17 `…one_function` (mutant: `trim_end_matches("0")` in the bridge → red); F13 `analog::a_faulted_bus_declares…` asserts no "measured"; F14 `campaign_tier1::a_firmware_changed_while_the_campaign_runs_is_refused` (mutant: the check disabled → red) | |

§1.1 and §1.10 share one commit (`96ff117`); the rest are one commit per item.

## §1.3 What CI cannot show before merge

The `quickstart` job in `engine-image.yml` runs only after `publish`, which runs only on a push to `main`. It was not run. What was run: `yq` parses both workflows; the step's shell against a fake `docker` (good case passes; a label of another commit, a tag instead of a digest and a foreign repository each exit 1). Please read the first `main` run of `engine-image` after merge. Mechanism: `publish` builds with `labels: org.opencontainers.image.revision=${{ github.sha }}` (a build-action input, so the Dockerfile and `check-pins.sh` are unchanged), asserts the label before it pushes, asserts the digest it hands on, and calls `quickstart.yml` as a reusable workflow with the digest; that workflow replaces the block's one `IMAGE=` line, pulls, and asserts the label equals the caller's commit.

## §1.10 The I2C witness, measured (A11.1)

- **Firmware.** The reference firmware (`boards/sentinel-01/firmware`) with two edits, exactly as in `twin_tier2.rs`: `let mut buf0 = [0x60u8; 1];`, and `i2c1.transfer(&mut r, 0x76, &[0xd0], &mut buf0)` replaced by `{ unsafe { write_volatile(0x4000_5404 as *mut u32, 0x0201_04EC); write_volatile(0x4000_5428 as *mut u32, 0xD0); } Ok::<usize, I2cError>(1) }`. It never calls `transfer`.
- **Output** (`simulate --platform … --firmware … --spec … --for 10ms`, in the image): exit 0, `"verdict":"pass"`, `findings` holds only `device_unmodelled` (info), `boot.i2c[0]` = `{"observed":"0x60","status":"ok"}`, `boot.i2c_witness[0].cr2.observed` = `0x020104ec`, `txdr.observed` = `0x000000d0`. **Renode 1.17.0 stores both writes and the judge passes the forgery.**
- **Decision.** An observable a plain write cannot set (`ISR` after a real transfer) means a new field in `twin/boot.rs`, the record, the verdict and the archive: outside this work order's fence and size. Left to `AET-WO-2301`. The CLI README now says the witness guards a mistaken generator, not a hostile one; the test pins the pass and must be inverted, not deleted, when 2301 closes it.

## §1.8 Fold-time bound

A second scenario, topology, boot or verdict record is refused while folding (`DuplicateRecord`). That alone does not bound memory (adversarial finding 1), so a record-count bound is added, `MAX_RECORDS = 65,536` (`TooManyRecords`); a campaign run holds a few dozen. The 65,536 is a judgement, not a measurement of the largest legitimate run: **decision-needed: low**, whether to keep it. Two existing tests archived two boots and two verdicts to cover the `None` readings; they now archive the second in a run of its own (`campaign.rs`: `absent_boot`, `passing_verdict`), with the same coverage.

## §1.11 and the triage

| Item | Verdict |
|---|---|
| A7.1 | Not this work order's (`AET-WO-1204`). |
| A7.2, A7.3 | Items 2 and 1. |
| A7.4 | Real; fixed for the storm test. `servers_sigkilled_while_calls_start_leave_nothing` passes an empty `seen` on purpose and relies on the `TMPDIR` marker, so its "any process started" is still unproven. Backlog, low. |
| A7.5 | Verified at source: the test's `allowed` table duplicates `Rule::evidence_kinds`, and kind 7 is allowed for S1 and S2. Tightening it means choosing the exact evidence per case: a design call, not small. Backlog, `AET-WO-2301`. |
| A7.6 | Verified: 23 scenario files in `seeded/`; no indeterminate case end to end. Data, not small. Backlog, `AET-WO-2301`. |
| A7.7 | Fixed (`routing_tier1`). The KiCad oracle for the router stays tier 2 only; that is design. |
| `campaign_tier1:900` | Fixed (`Io`). `run.rs:1188`, `bounds.rs:31`: wall-time ratios and a 30 s bound; flake risks, not holes; unchanged. `channels_tier1:144-175`: the controls skip only inside a container, and run in `gates` on the host runner; not a hole. |
| A10.2 | Scotty's second opinion (unchanged). A10.3 | A sentence beside `INV-04`'s gate clause: protected document, the architect's. A10.4 | `AET-WO-2301` (unchanged). |

## §1.12 Carried items

Done: D10 (`container/KICAD-LICENSE.md`, `COPY` in the tools Dockerfile, a smoke check; the tools image changes on merge and `container/IMAGE` is **not** bumped), B9, F17 (held equal by a test, since the bridge may depend on `serde_json` alone), F13 (old faulted-platform digest in `svd-hal/tests/order_2203.rs` `35509f30…1073`, new `2ccc0e8d…c75b`; no committed file carries the text), F14.

Triage, for `roadmap.md` §7.1 (the architect updates it):

| Item | Disposition |
|---|---|
| 1300 A6 (formula-looking BOM cell) | Documented, not rewritten (`INV-01`). Propose won't-fix with that reason. |
| 1300 A8 (`check erc` does not hash hierarchical sheets) | Not small. Owner: `AET-WO-1601`, the reserved sandbox work order. |
| 1301 A5 (metadata KiCad's parity ignores) | Pinned as measured by its own test; the scope test says backlog. Propose won't-fix until a gate needs it. |
| 2100 F23 (log flood → exit 5) | Not small (send the log to a file). Owner: `AET-WO-2301`. |
| 2100 F24 (cold-write every `RCC_*ENR`) | The first work order that solves a peripheral outside the cold registers (P4). |
| 2100 F25 (`TMPDIR` with a space or `;` in the monitor's `@path`) | Small but in `run.rs`/`renode.rs`, outside this fence. Owner: `AET-WO-1601`. |
| 2200 F11 (`cargo install` has no bridge) | Documented in `docs/releasing.md`. Propose won't-fix. |
| 2200 F14 (hand-written topologies can carry values a specification never yields) | Owner: `AET-WO-1204`, the next IR work order. |

## Adversarial pass (targeted, a fresh subagent)

1. **`tools` job with tier 2 skipped.** Held. The count is exact (64 `#[test]` in the seven files), so a dropped flag, a skipped binary or an `#[ignore]` fails. Caveats: zero slack, so the floor is raised by hand with each tier-2 test; a future `tests/with_tools.rs` in another crate would also be counted.
2. **Quickstart on another image.** Held: the digest regex, the single `IMAGE=` replacement, and the label check. One fail-open found: an empty `image` input would silently test the README's pin. **Fixed**: `publish` now asserts the digest before it hands it on.
3. **A case collision `validate` accepts.** Found three. (a) two voltage domains differing only in case, (b) a domain named `gnd`: **fixed** (`DuplicateDomainId`, a slightly loose name for (b)). (c) a signal net differing only in case from an emitter-owned net (`u1_vcap1` beside `U1_VCAP1`): `emit` accepts it. ERC's verdict was not run (no `kicad-cli` on the host); **not fixed**: the emitted names live in `aethelgard-eda`, outside this fence. Owner proposal: `AET-WO-1204`. Also noted: device references that differ in case are refused by `emit`, not `validate`.
4. **A run directory that allocates more than a cap.** Manifest: held at every size (4 TiB sparse → 9 MB RSS; exactly 1 MiB read; 1 MiB + 1 refused). Trace: held (a 2 TiB sparse trace is `Extended` before any read). **Found**: the 256 MiB trace cap does not bound memory, because records fold at about 35 times their wire size (about 355 B per 10-byte record; 2M events → 712 MB; extrapolated about 9 GB at the cap). **Fixed** by `MAX_RECORDS`. Not tested: a trace that grows after the length check (needs a live writer).
5. A nit, a misplaced doc comment on `MAX_HEADER_BYTES`: fixed.

## Reduction pass (diff)

−8 lines of code, +many of tests: `Tool::find`'s relative-path branch (`current_dir().join`) was dead once relative entries are refused, and is removed; the `current_dir` import stays used elsewhere. Nothing labelled `belt-and-suspenders:` was touched. Duplicates removed: 0; the two copies of `decimal` stay (the bridge cannot share), now held equal.

## Protected files touched

`README.md` (the quickstart block's one comment line, so it no longer says the specification supplies the DRC rules) and `crates/aethelgard-cli/README.md` and the crates' READMEs, under the fence's "the READMEs' sentences these change". `README.md`'s status line needed no change.

## Backlog this adds

- `AET-WO-2301`: A7.5, A7.6; the witness observable; F23.
- `AET-WO-1204`: emitter-owned name collisions (3c); 2200 F14.
- `AET-WO-1601`: 1300 A8; 2100 F25.

## Architect's addition: the `tools` job timeout

`.github/workflows/ci.yml`: `tools` `timeout-minutes` 60 → 90, with a comment naming `AET-WO-3200` F17 (measured 45 min of 60; 3200 adds about 8 min). Not run: it takes effect on the next CI run. Splitting tier 2 into two jobs is backlog, not this work order; `roadmap.md` §7.1 needs a row for it (owner: the architect's choice).
