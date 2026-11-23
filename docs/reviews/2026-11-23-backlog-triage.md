<!-- The backlog triage of 2026-11-23, written by a triage agent for the architect against main f50c29f4. Its L numbers are that file's lines of docs/roadmap.md. The waves it proposes are recorded in roadmap §5 (the backlog plan); this file holds the row-by-row evidence. -->

# Aethelgard backlog → launch plan (roadmap §7.1)

**Base read:** `ClearForge-LLC/aethelgard` `main` at `f50c29f4` (#335, 2026-11-23). The checkout moved from `e144f533` to `f50c29f4` while this was being written. #335 added nine §7.1 rows (`4300h` D4/D6, F2, F3, F4, F6, F8, the bench session, the inference models, Tviburi) and re-owned `1216` F1 to `AET-WO-4300i`. All of them are folded in below. **Roadmap line numbers (L) are for `f50c29f4`.** Evidence line numbers (for example `main.rs L1017`) are source lines in the same checkout.
**Method:** I parsed all 394 rows of the §7.1 table (L743 to L1143). Rows whose Owner cell says done, merged, removed, won't-fix, won't build, held equal, accepted or ruled-and-built were set aside. So were rows owned by a work order that has a `.FEEDBACK.md` and whose row says nothing more. I spot-checked several of those against the code (`879`/`880` serial fault and loaded floor, `929` RSS share, `1030` PWM idle, `1021` `Scale2`, `1082` README): all were closed. That leaves **198 rows** to classify. I checked each A row's crate by grepping for the named function, file or refusal.

## 0. Counts

| Class | Rows | Meaning |
|---|---|---|
| Closed (not listed) | 196 | done, merged, ruled-and-built, or owned by an existing WO with FEEDBACK |
| **A** buildable now | **82** (+1 A?) | code and data are in the repo |
| **B** blocked | **79** | held document, purchase, bench, ruling, trigger, or a later phase (each named) |
| **C** idea-only / low | **13** | why is given per row |
| **U** probably closed already | **11** | a later WO looks to have done it but the row was not updated. Verify before scoping (§5) |
| **X** excluded | **12** | Scotty, the operator, fetches, another repo, or being built by `4300i` |

B, by cause (counted by keyword in the note, so roughly): a trigger not met 15, `4500` not written (held until Sentinel-01's bench) 14, a ruling or decision 14, a held document 12, bench 6, `4300e1` not written 3, upstream Renode 3, lock-step not built 2, a purchase 1, a later phase or other 9.

## 1. Constraints applied

- **`AET-WO-4300i` is busy** (its WO file, §3): `crates/aethelgard-ir`, `aethelgard-svd-hal`'s generator, `aethelgard-hwdef`, `aethelgard-pincheck`, `aethelgard-eda`, `examples/sentinel-03*.json`, and every version literal and golden its specification bump moves. Its own "Not" list is the twin, chaos, `verify`, workflows and `ci/`. It stops once at §1.2 for the schema shape, so those crates stay held longer than the build alone takes.
- **The bench-package queue is serial** (`fab/sentinel-01-bench/package`). Every specification bump moves its specification-hash lines. Any change to a trace header, a verify verdict line, or a firmware ELF on that board moves `MANIFEST.json`'s `verdict_sha256` (`1606` moved it through the bridge digest alone). Wave 1 therefore carries **no specification bump, no trace-version bump, and no firmware-byte change**. Each wave-1 WO also carries a stop condition: "a `fab/` byte moves".
- **Shared files.** `ci/test-weights.tsv` is edited by any WO that adds a tier-2 test (§6's standing rule), and the edits merge cleanly. `crates/aethelgard-cli/tests/fixtures/*.json` are re-versioned by `4300i`, so wave-1 WOs add no specification fixtures.

## 2. Wave 1: launch beside `4300i` (four WOs plus two optional)

The fences are disjoint from `4300i` and from each other at the crate level. The only exception is W1-CLI and the optional W1-CI, which share no file. None of them bumps the specification.

| WO | Title | Rows (From) | Fence | Spec bump | fab/ bytes | Size |
|---|---|---|---|---|---|---|
| **W1-IMG** | Image hygiene: the revision label, `# syntax=` refused, apt from a snapshot date, GtkSharp's source mirrored, one pin regex, a bounded echo | `4403` Q3 (#186) · `4401` Q2, Q3 · `4403` D5 Q1 · `1605`, main run 44299148057 · `4403` D4 · `4402` Q3 · the `4402` Q4 half of `1311` adversarial J, F; `4402` Q4 | `container/` (Dockerfiles, `check-pins.sh` and its fixtures, `check-ardupilot-pin.sh` and its tests, `oci-mirror.sh`, `*-SOURCES.md`, `*-DEBIAN-GPL2.tsv`); `.github/workflows/tools-image.yml`; `ardupilot-image.yml`'s publish job only | no | no by design. The `container/IMAGE` and `ARDUPILOT-IMAGE` re-pins after `main`'s publish are a follow-up, as in `1605`/`4402`. Stop if a tool version moves | M (L if the snapshot pin fights the GPL tables) |
| **W1-CLI** | The CLI's small backlog: `check support` pinned, test scratch off `$HOME`, `verify` on a board that draws none, the lifecycle banner on every verdict | `1311` F4 · `1603` F2 · `1211e` F11 · Grok review 2026-11-16 (the `readme_status.rs` half) · `1210` F2 · `4211` F2, adversarial A-B1 (verdicts only, not KiCad files) | `crates/aethelgard-cli`: `src/main.rs`, `kicad.rs`, `verify.rs` (the DrawsNoBoard path), `pinout.rs` (banner line only); `tests/common/mod.rs`, `fab_tier1.rs`, `readme_status.rs`, new tests. **Not** `renode.rs`, `campaign.rs`, `fixtures/*.json`, the IR | no (no new validate refusal) | should not: the bench package carries `verify`'s verdict, which already has the banner. Stop if any moves | M |
| **W1-EVOLVE** | A BMP581's witness counts as evidence | `3306` F1 (bme-bmp, measured) | `crates/aethelgard-evolve` (`witness_fields`, `tests/attribute.rs`, which inverts `a_collide_onto_a_bmp581_stays_i4…`). **Not** `models/renode` (the C# model's log line is pinned by sha256) | no | measure first: if a committed seeded case re-attributes on the bench board, its verdict moves. Stop | S–M |
| **W1-CHAOS** | Replay and the writer hold their own bounds | `2303` F3, D3, #200 · `2302` Q5, A2-2 (#175) · `3201` adversarial 5.8 | `crates/aethelgard-chaos` (`archive.rs` `TraceWriter`, `replay.rs`, tests). **Not** `campaign.rs`'s scenario format, the CLI | no (no trace-version bump: only malformed archives are newly refused) | no. Stop if any committed archive or trace fixture fails the stricter replay | S–M |
| W1-CI *(optional 5th)* | The shard planner's P3 | `1910` D6 (#233) · `1907` review (A?: not reproduced on this box) | `ci/shard.sh`, `ci/test-shard.sh`, `ci/fixtures/` | no | no | S. **Caveat:** a new 10–60 s refusal can fail parallel WOs' CI. Land it last in the wave, or behind a warn-only first run |
| W1-SPIKE *(optional)* | Where `verify`/`prove`'s 9–13 GB peak comes from | `3304` F3, `4201` F4 | `spike/` only (a measurement) | no | no | S. Needs `--mem 16G` and Kani, so it contends with `4300i` for Furnace's memory. Run it only if the box has headroom |

Notes:
- **W1-IMG then W2-MATRIX:** both edit `ardupilot-image.yml`, so they run in series.
- **W1-CLI then W2-CITEST:** both touch `crates/aethelgard-cli/tests/common/`, so they run in series.
- **W1-CHAOS then W2-TRACE then W2-COVER:** all three touch `chaos`, and the last two also touch `cli/src/renode.rs`, so they run in series.

## 3. Later waves (dependency order)

**Wave 2: after `4300i` merges.** It frees `ir`, `svd-hal`, `hwdef`, `pincheck` and `eda`. At most one IR WO runs at a time, and at most one package-moving WO (⊕) enters the queue at a time.

| WO | Title | Rows (From) | Fence | Bump | fab/ | Size |
|---|---|---|---|---|---|---|
| W2-EDA | eda's reduction and reader hardening | `1208` F3 · `1311` adversarial C · the eda half of `1311` adversarial J, F · `1211f` F3, `1217` Q1 (IPROPI text) · `1321` F1, F2 · `1218` F2 · `1310` (PadEscapeExhausted shape) | `crates/aethelgard-eda` (`schematic*.rs`, `board.rs`, reader); `ir/src/names.rs` (one `pub`) | no | no (stop if a byte moves) | M |
| W2-HWDEF | Generator and pinout nits | `4400b` F2, A9 (#189) · `4402` Q5 · `1218` F4 | `crates/aethelgard-hwdef`, `crates/aethelgard-pincheck` (and its fixtures) | no | no | S |
| W2-GEN ⊕ | The firmware generator: timer break, I2C status codes told apart, unreachable variants | `4240b` D-3 · `3302` B2, 100K · `1204` reduction | `crates/aethelgard-svd-hal` (generator, UI goldens) | no | **yes**: every I2C firmware ELF moves (the bench board's too). Queue | M |
| W2-IR-POWER ⊕ | The power tree's static backlog | `1218` F1, F3 · `1211e` F7 · `2304b` K6 · `4300g` K7 · `4300g` K8 (ESR on ceramics; 2304 FEEDBACK L107) · `4300d` F-B6 | `crates/aethelgard-ir` (`power.rs`, `pin_devices.rs`, `power_parts.rs`, `data/*.json`, versions) | **yes** (new refusals; parts and power-parts schema) | yes. Queue | L (split into a data half and a rules half if needed) |
| W2-TRACE ⊕ | One trace and scenario schema bump | `4300d` D-B1 · `2302` reduction finding 2 (#175) · `4241` §0a · `2302` Q1 (#175) · `1606` D3, F4 · `3201` Q4 (the `renode.rs` literals) | `crates/aethelgard-chaos` (`campaign.rs` scenario format, archive header); `crates/aethelgard-cli/src/renode.rs`, `campaign.rs`; `scenarios/` | trace version (not spec) | **yes**. Queue | M |
| W2-MATRIX | The ArduPilot matrix grows (`INV-07`) | `4404` (draft) four-port board · the Sentinel-01 review F7 (2026-11-19) | `ardupilot-image.yml` (generated matrix), one new four-port fixture at the post-`4300i` version; the `pin_table`-vs-`STM32H753xx.py` test and a hand-entered DS12117 pad table (`ir` data or a test fixture, the WO decides; it must stay out of `svd-hal/src` while W2-GEN runs) | no | no | M |
| W2-CITEST | Split the long `verify` tier-2 tests | `4300b` (c), `4230` §12 · `1909` B | `crates/aethelgard-cli/tests/` (`verify_*_tier2`, `against_fab_tier2`, `four_layer_fab_tier2`, `common/verify.rs`); `ci/test-weights.tsv`, `ci/colocate.tsv` | no | no | M |

**Wave 3:**

| WO | Title | Rows (From) | Fence | Bump | fab/ | Size |
|---|---|---|---|---|---|---|
| W3-IR-RULES ⊕ | Validation rules | `1208` F4 (pull-up floor over the rail range, a measured stop) · `4220b` Q5 (#192) · `4400b` F3 (#189) · `1219` D2 · `1211d` F4 · `4240a` F4 (A10) (foreign HSE; `ir/src/vos.rs` + `cli` `simulate`) | `ir` (validate, `vos.rs`, `parts.json`, `tests/versions.rs`); the CLI's `simulate` | yes | yes | M–L |
| W3-COVER ⊕ | Coverage that tells, and a per-device I2C absence | `3304` F1, #236 · `4201` F5, #237 · `4600` F4 · `3303` F3, ruled (comment 6900789002) (the pull-up and capacitance half) | `chaos` (override), `cli` `verify.rs`/`renode.rs`, `twin` | scenario format | yes. **May refuse Sentinel-01 as ordered** (U2 empty): a stop for Scotty | L |
| W3-SILICON | The silicon oracle reads what was emitted | `4230` adversarial F5, F6 · `4230` §18 reduction · `4240v` F5 (from `4230`) | `ir/src/silicon.rs`, `svd-hal`, `cli/src/ledger.rs` | no | maybe (ledger text). Stop | M |
| W3-SOLVER | Solver: DShot DMA per timer, chip-select backtrack | `4300h` F8 · Grok review 2026-11-16 (free chip-selects, "low") | `ir/src/solve.rs`, RM0433 DMAMUX as data | yes, if it refuses | yes if a pin moves (stop) | M |
| W3-IR-SPEC ⊕ | Loads named by what they are; acknowledgements that survive a re-pin | `4202`, ruled 2026-11-19 · `4300h` F6 (general fix; pair with `2304b`'s person-directed acknowledgement WO) | `ir` (`power.loads`, `acknowledge_unwitnessed`), every spec carrying them | yes (key change) | yes | M |
| W3-ROUTER | Router backlog | `1310` ruling · `1310` A7 · `1311` F2 · `1307` A4 (a bound that refuses by name) · `1910` F1 (#233) | `eda` `route*`, `extract` | no | possibly (stop) | M–L |
| W3-TWIN | Serial options read back in the twin | `4404` (draft), USART_CR2 | `twin`, `cli/src/renode.rs` (after W2-TRACE) | no | possibly | S–M |
| W3-AUTHOR | `aethelgard init --template`, errors that quote the fix | review 2 B7 | `cli` (new subcommand, templates from `examples/` by `include_str!`), IR error text | no | no | M. "Named at P4's gate, before any announcement" |
| W3-CITE | Pages for every citation, with `1219`'s guard as a test | `1219` F1 | `ir/data/*.json`, IR source constants, a test | maybe (data schema) | possibly (FAB-NOTES cites). Stop | L |

**Wave 4:**

| WO | Title | Rows (From) | Fence | Notes | Size |
|---|---|---|---|---|---|
| W4-PLACER (spike first) | Placement independent of references and order; anchored groups; capacitance-aware placement | `1307` K3, I1 · `1313` D2, Q2 (#197) · `1313` Q4 (#197) · `3401` F1 (#190) · `1315` (c), ruled (a) · `1313b` §3, #208 | `eda` `pcb.rs` | moves pinned placements, and Sentinel-01 is ordered: stops for a ruling (also unblocks B rows `1317` F1, F2 / F3 / F6). After W3-ROUTER (same crate) | L |
| W4-EMIT ⊕ | Emitted output: the KiCad lifecycle banner, the sheet's stand-in label, `ORDER.md`, arms and capacitor rows as data | `4211` F2 (the KiCad half) · `2308` F1, F3, F8 (not Scotty's Note-column half) · `4600` D4, F15 · `1211d` R9, A4 | `eda` emit; `fab/sentinel-01-bench/ORDER.md` | moves emitted bytes on every nrnd-carrying board. Queue. First check what `1319`'s rows already closed | L |
| W4-FAB | The fab package uses the rule-sized via | `1314` stop (#227) | `eda`/`cli` `fab.rs` | owned by `4500`'s fabrication step, but buildable now and needed before any Sentinel-03 package. Ask whether to pull it forward | S–M |

## 4. Classification of every open row

### A — buildable now (82 rows)

| L | From | Item (short) | WO | Evidence / note |
|---|---|---|---|---|
| 921 | `3201` adversarial 5.8 | A booted run whose registers and records are all set to unread replays as `N1`: a rule of all read or all … | W1-CHAOS | replay rule: all read or all unread across kinds 10-13, 21, 22 |
| 945 | `2302` Q5, A2-2 (#175) | Replay should require exactly one kind 26 per kind 24 in a booted run | W1-CHAOS | replay.rs has no one-kind-26-per-kind-24 rule (grepped) |
| 984 | `2303` F3, D3, #200 | `TraceWriter` writes past the reader's 256 MiB `MAX_TRACE_BYTES` (it wrote 367 MB, which `replay_dir` then … | W1-CHAOS | chaos/archive.rs TraceWriter (L92) has no MAX_TRACE_BYTES check; only the reader (L329) has |
| 1000 | `1910` D6 (#233) | The planner's P3: time-stamped log lines and a 10 s threshold, refusing unweighted tests in the 10–60 s band … | W1-CI (optional) | ci/shard.sh has P2 (60 s line) only; P3 (time-stamped lines, 10 s) absent |
| 916 | `1603` F2 | Tier-1 tests that take `build`/`prove` past the pre-flight fail on a machine with `~/.cargo/config.toml`, … | W1-CLI | tests/common/mod.rs private_tmp L196-207 still roots scratch under $HOME/.cache/aethelgard |
| 961 | `1311` F4 | `check support` runs `kicad-cli` unpinned (its netlist export records the version without comparing it) | W1-CLI | check_support (cli main.rs L1017) runs Tool::find("kicad-cli") with no kicad::pinned, unlike L966 |
| 978 | Grok review 2026-11-16 | README status line (P3 "under way"; `cargo install` points at P1-era 0.1.0); `readme_status.rs` checks only … | W1-CLI | tests/readme_status.rs checks only phases marked passed; README itself is protected, the test is not |
| 1046 | `1211e` F11 | `fab_tier1::a_rail_or_bus_no_fault_kind_reaches_is_named` fails with `BaseOutsideRoot` when … | W1-CLI | fab_tier1 scratch() uses CARGO_TARGET_TMPDIR (L30-36), so BaseOutsideRoot when the target dir is outside the repo |
| 1096 | `1210` F2 | Plain `verify` on a specification that declares it draws no board is refused up front (`DrawsNoBoard`, the … | W1-CLI | DrawsNoBoard still refuses plain verify up front (main.rs L2743) |
| 929 | `4211` F2, adversarial A-B1 | The lifecycle banner is missing from the KiCad files and from the `emit`, `solve`, `check drc --spec`, `check … | W1-CLI (verdicts) + W3-EMIT (KiCad files) | banner present in extract/speeds/campaign/simulate/validate/generate; absent from emit, solve, check drc --spec, check support, prove, check pinout and every KiCad file. The KiCad half moves emitted bytes (W25Q64JV is nrnd on Sentinel-01): later, queued |
| 1111 | `3306` F1 (bme-bmp, measured) | A collide onto a BMP581 stays I4: evolve's `witness_fields` reads only `Write XX`/`Read XX`, and the BMP581 … | W1-EVOLVE | evolve witness_fields reads only Write XX/Read XX; pinned at I4 by tests/attribute.rs L2236 |
| 914 | `4403` Q3 (#186) | `tools-image.yml`'s `publish` sets no `org.opencontainers.image.revision` label (ArduPilot's does), so the … | W1-IMG | tools-image.yml publish sets only the documentation label (L97); add org.opencontainers.image.revision |
| 924 | `4401` Q2, Q3 | A `# syntax=` directive (BuildKit pulls a frontend by tag) is not refused; dependencies of pinned apt … | W1-IMG | check-pins.sh has no `# syntax=` refusal (grepped); the apt-deps half is the snapshot row |
| 925 | `4403` D5 Q1 | Pin apt to a snapshot.debian.org timestamp in every Dockerfile, so Debian packages stop floating. That ends … | W1-IMG | Dockerfile L17/L147 `apt-get update` against the moving index; snapshot URLs appear only in the dpkg tables |
| 930 | `4403` D4 | GtkSharp `3.24.24.95` in the tools image is named at commit `964cb9c` by inference; mirror its source at that … | W1-IMG | TOOLS-SOURCES.md L18 names GtkSharp 964cb9c by inference; mirror its source by sha256 (oci-mirror.sh/gcc-source pattern) |
| 957 | `4402` Q3 | The `publish` job of `ardupilot-image.yml` checks the pushed digest with its own copy of the pin regex | W1-IMG | ardupilot-image.yml L489 holds its own digest regex; route through container/check-ardupilot-pin.sh |
| 976 | `1605`, main run 44299148057 | The tools image's apt layer reproduces only through the CI build cache: a fresh build resolves Debian's … | W1-IMG | Dockerfile L17/L147 `apt-get update` against the moving index; snapshot URLs appear only in the dpkg tables |
| 962 | `1311` adversarial J, F; `4402` Q4 (the pin check echoes the file, so a `::warning::` line reaches the log) | A refusal echoes the refused text whole (a 14 MB layer name gave 49 MB on stdout; bounded by the 16 MiB input … | W1-IMG + W2-EDA | split: the `4402` Q4 pin-check echo (container/check-ardupilot-pin.sh) in W1-IMG; eda reader echo bound and courtyard count in W2-EDA |
| 1016 | `3304` F3, `4201` F4 | `verify` and `prove` on a board with Kani harnesses need more than 8 GiB: 9.1 GiB peak (`3304`), and an OOM … | W1-SPIKE (optional) | a measurement only (spike/), no engine change; needs 16G and Kani, so it contends with 4300i for Furnace |
| 895 | review 2 B7 | Specification authoring: `aethelgard init --template`, and errors that quote the fix | W2-AUTHOR | no `init` subcommand exists; "errors that quote the fix" reaches IR error text, so after 4300i |
| 808 | `1909` B | B: `verify_binds…` and `verify_fab_out_writes…` sharing one pair of `verify --against --fab-out` runs. Built, … | W2-CITEST | `1909` B, "if CI time matters"; low |
| 912 | `4300b` (c), `4230` §12 | Split the long `verify` tier-2 tests, which hold the slowest shards | W2-CITEST | verify_47k_tier2, against_fab_tier2, four_layer_fab_tier2 still whole |
| 880 | 3303 F3, ruled (comment 6900789002) | A covering tolerated fault must move its quantity past the declared tolerance (a pull-up beyond its resistor … | W2-COVER | 2304 FEEDBACK L102/L562: the droop half is built (verify.rs L2973); the pull-up and capacitance half went "to a static work order" and is open |
| 1015 | `3304` F1, #236 | A per-device I2C absence in the twin, the counterpart of `spi_device_absent`. Its only I2C absence is the … | W2-COVER | no i2c_device_absent override exists (only spi_device_absent, chaos campaign.rs L173); needs chaos + cli renode/verify + twin plan |
| 1019 | `4201` F5, #237 | `3303`'s coverage rule counts SENSORS as covered by `pass` scenarios with no modelled device on the bus. With … | W2-COVER | coverage rule in cli verify.rs; may refuse Sentinel-01 as ordered (U2 empty): a stop condition |
| 1034 | `4600` F4 | A bus whose only devices are unmodelled counts as covered by an electrical fault that nothing in the twin … | W2-COVER | coverage rule in cli verify.rs; may refuse Sentinel-01 as ordered (U2 empty): a stop condition |
| 911 | `1310` | `PadEscapeExhausted`'s JSON shape has no test | W2-EDA | PadEscapeExhausted JSON shape test (schematic.rs L103, pcb.rs L2273) |
| 926 | `1208` F3 | eda guards that `emit` can no longer reach now that it validates first (`UnsupportedAddress`, a crossed bus's … | W2-EDA | eda guards now unreachable (schematic.rs L80 UnsupportedAddress etc.); delete or label |
| 959 | `1311` adversarial C | `board::prepare_read` clones the carrying footprint's or pad's name onto every child it scans: a 10 kB … | W2-EDA | board.rs prepare_read L171: memory blow-up on a long name (24 GB kill) |
| 1058 | `1211f` F3, `1217` Q1 | `declared_bulk` repeats `schematic_power::two_pin`'s row push (its `note` is fixed to `None`); the IPROPI … | W2-EDA | IPROPI guard text, schematic_arms.rs (1321 did not rewrite it; 1217 merged after) |
| 1061 | `1218` F2 | eda's reference claim chain (`schematic.rs`, `design`) is a hand copy of `names::references`' order: a kind … | W2-EDA | eda claims names::references (ir names.rs L100 is pub(crate)); touches ir names.rs, one visibility change |
| 1075 | `1321` F1, F2 | `1319`'s harness mutant M2 now fails to compile under `1320`'s count check, so the harness no longer measures … | W2-EDA | 1319 harness M2 compiles no longer; DECOUPLERS entry stated twice not refused |
| 815 | `1204` reduction | `svd-hal`'s unreachable `CeilingMismatch` and I2C `IncompletePeripheral` | W2-GEN | svd-hal board.rs L93/L101 IncompletePeripheral/CeilingMismatch still present |
| 899 | `3302` B2, 100K | The firmware's NACK and TXIS-timeout status codes, told apart on silicon (`ISR.NACKF` against `ARLO`) | W2-GEN | NACKF vs ARLO status codes; moves every I2C firmware ELF, so the bench package (queue) |
| 1137 | `4240b` D-3 | The generated firmware arms no timer break (BDTR is MOE alone, BKE 0), so no motor output is held by hardware … | W2-GEN | BDTR.BKE for TIM1/8/15-17; svd-hal motor code, which 4300i also edits: right after it |
| 939 | `4400b` F2, A9 (#189) | A chip-select name with an alternate-function prefix is refused by ArduPilot, not by the generator by name | W2-HWDEF | hwdef: refuse a CS name with an AF prefix by name |
| 958 | `4402` Q5 | The `pincheck` `hwdef.dat` fixture headers say they were written from specification `a277c933…`, but since … | W2-HWDEF | pincheck fixture headers name spec a277c933… |
| 1062 | `1218` F4 | `check pinout` judges the topology's part after reading the ELF, so a topology refusal can be hidden behind … | W2-HWDEF | PartMismatch is pincheck lib.rs L172; the order of topology vs ELF read |
| 865 | 4300d F-B6 | Sensor rows' supply current, so device loads enter a domain's load; also a translator's supply current (not … | W2-IR-POWER | sensor rows' and translator supply current into domain loads (parts.json); datasheets held |
| 907 | `4300g` K7 | J2's current rating against the battery sag | W2-IR-POWER | 2304 FEEDBACK L106: static, wording wrong (S-03's J2 is SM06B-SRSS-TB); a connector current rating against the sag |
| 908 | `4300g` K8 | A capacitor's dielectric and ESR in the parts data | W2-IR-POWER | 2304 FEEDBACK L107: dielectric is a field already; ESR on ceramics is the open half (power-parts data) |
| 1043 | `1211e` F7 | A power-tree control `wire` can join an MCU net (PG to nFAULT validates; a buck's EN to a bridge output … | W2-IR-POWER | control wire net judged against its other drivers: a new refusal, so a bump |
| 1060 | `1218` F1, F3 | "A declared reservoir is a capacitor row" lives in two places (`pin_devices::analog` and `power::sense`), … | W2-IR-POWER | one reservoir helper for pin_devices::analog and power::sense, rating included: a new refusal, so a bump |
| 1097 | `2304b` K6 | IR's static headroom counts no input-path drop, so a healthy board already sits below its battery's declared … | W2-IR-POWER | static headroom with the input-path I·R; may refuse committed boards (measure first) |
| 927 | `1208` F4 | The I2C pull-up floor judged at its maximum over the declared rail range [min, max], not at the maximum or … | W2-IR-RULES | rule change with a measured stop; a bump |
| 932 | `4220b` Q5 (#192) | No rule holds a device's input-edge maximum: DS-000347 Table 6 gives the ICM-42688-P's SCLK rise and fall at … | W2-IR-RULES | a device input-edge maximum (DS-000347 Table 6 held) as a parts field and rule; a bump |
| 938 | `4400b` F3 (#189) | An RC port's tolerance (`4300b`'s `peer_deviation_ppm`) is judged at the declared baud (CRSF 420000), while … | W2-IR-RULES | 4400c FEEDBACK L179: unreachable there (no RC port), so still open; judge at ArduPilot's 416666 too |
| 1063 | `1219` D2 | `versions::current_specifications()` does not read `crates/aethelgard-cli/tests/fixtures/solve/*.json`, so 4 … | W2-IR-RULES | ir/tests/versions.rs L144 current_specifications(): one line; 4300i touches this file, so after it |
| 1113 | `4240a` F4 (A10) | `simulate --foreign` refuses an HSE specification (`HseNotGenerated`), and `vos::judge_foreign` names no … | W2-IR-RULES | vos::judge_foreign is in aethelgard-ir (vos.rs L574); HseNotGenerated in cli generate.rs; 4240b merged, so unblocked |
| 1136 | `1211d` F4 | `ir/src/drawn.rs` L69's comment says no arm draws a pin device until `1211d`, which is stale | W2-IR-RULES | stale comment, drawn.rs L69 |
| 1027 | `4404` (draft) | A four-port generated board in CI. Every CI board has a console only, so nothing generated exercises SERIAL1–9 | W2-MATRIX | ardupilot-image.yml generated matrix L258-262 has sentinel-01-minimal, not sentinel-01.json, and no four-port board; the pin_table test lives in svd-hal (busy) and the DS12117 pad table in ir data |
| 1029 | the Sentinel-01 review F7 (2026-11-19) | `verify`'s gates take each AF number and pin position from one source, ST's pin table. `check pinout`, the … | W2-MATRIX | ardupilot-image.yml generated matrix L258-262 has sentinel-01-minimal, not sentinel-01.json, and no four-port board; the pin_table test lives in svd-hal (busy) and the DS12117 pad table in ir data |
| 863 | 4300d D-B1 | The droop's own `bor_level` dropped from the scenario format (today refused when it contradicts … | W2-TRACE | chaos campaign.rs L221/L484 still parses a droop `bor_level`; a scenario-format change |
| 943 | `2302` Q1 (#175) | A core that sleeps and then writes its boot word exits 5 (`2100` A10), so a firmware under test can abort a … | W2-TRACE | cli renode.rs/campaign.rs: a run finding, not exit 5 |
| 944 | `2302` reduction finding 2 (#175) | The verdict's and campaign lines' `window` field always says `MAX_VIRTUAL_NS` now; removing it is a schema … | W2-TRACE | a trace/verdict schema change; batch with the other trace-header rows |
| 1054 | `1606` D3, F4 | Tool identities are self-reported names (ngspice's banner, rustc's `-vV`, Renode's `--version`): nothing … | W2-TRACE | content hashes of libngspice/toolchain in the header: moves every trace header, so the bench package (queue) |
| 1116 | `4241` §0a (the architect's, 12:06Z) | An event planted at a fixed count of spins from reset is brittle to any change in boot length: `4241`'s boot … | W2-TRACE | plant events relative to the boot word; scenario format |
| 1064 | `1219` F1 | The held tables and the IR's source constants cite documents without pages (26 source items, 63 row … | W3-CITE | pages into held tables + IR source constants with 1219's guard as a test; data files in ir, so after 4300i |
| 1057 | `2308` F1, F3, F8 | The JLCPCB BOM's fifth `Note` column departs from JLCPCB's four-column sample (#199); that the upload … | W3-EMIT | sheet stand-in label (eda) and ORDER.md L81 (fab/sentinel-01-bench: package bytes, queue); Scotty's Note-column half excluded |
| 1088 | `4600` D4, F15 | A new device part costs code in four files, so "capability enters through the specification" fails its own … | W3-EMIT | arms as data; 1319 built six arms as rows, so verify what is left before scoping; capacitor ratings/footprints as rows |
| 1134 | `1211d` R9, A4 | Capacitor ratings and footprints are text in the value, and every arm capacitor is 0402 (a 10 µF 0402's DC … | W3-EMIT | arms as data; 1319 built six arms as rows, so verify what is left before scoping; capacitor ratings/footprints as rows |
| 993 | `1314` stop (#227) | A fabrication package for a board with `fab_tolerances` uses the router's rule-sized via (`fab.rs` names the … | W3-FAB | buildable now (fab.rs names the engine 0.6/0.3 via), but owned by 4500's fabrication step; matters before any Sentinel-03 package |
| 1070 | `4300h` F6 | A re-pin of any pin an `acknowledge_unwitnessed` entry names fails `validate` until the entry is rewritten … | W3-IR-SPEC | 4300i meets it for S-03; the general fix (an acknowledgement that survives a re-pin) is a design choice, best with 1078's acknowledgement WO |
| 1090 | `4202`, ruled 2026-11-19 (renamed in `4202`, with a test pinning each name to its nets) | A `power.loads` row names an emitted, sequence-numbered designator (Sentinel-03's R7 and R8, the flash's WP … | W3-IR-SPEC | power.loads by part pin and rail: a key change, a bump |
| 915 | `1307` K3, I1 (promised to the backlog, never added; found 2026-11-16) | Placement orders by reference (`pcb::place`), so renaming specification references still fails some boards: … | W3-PLACER | placement by reference/order; moves pinned placements (Sentinel-01 is ordered): spike first |
| 946 | `1313` D2, Q2 (#197) | The placer targets an anchor's lowest pad on the first shared net by name; targeting the pads of the most … | W3-PLACER | placement by reference/order; moves pinned placements (Sentinel-01 is ordered): spike first |
| 948 | `1313` Q4 (#197) | A name-independence guard should rename references keeping their lengths, so it measures order and not … | W3-PLACER | placement by reference/order; moves pinned placements (Sentinel-01 is ordered): spike first |
| 949 | `3401` F1 (#190) | `extract`'s margin on a line can depend on `test_points`' document order (Sentinel-01's SPI_MOSI 20.8–30.7 pF … | W3-PLACER | placement by reference/order; moves pinned placements (Sentinel-01 is ordered): spike first |
| 1011 | `1313b` §3, #208 | A capacitance-aware placement step, so `extract`'s margin holds by construction rather than by geometry: … | W3-PLACER | capacitance-aware placement; L |
| 1098 | `1315` (c), ruled (a) | The placer handles a large anchored passive by moving its whole group: Sentinel-03's x headroom fell from … | W3-PLACER | large anchored passive moves its group |
| 874 | 1307 A4 | Router wall-time bound (an adversarial 4 × LQFP-144 board took 616 s, bounded only by `RIPUPS`) | W3-ROUTER | route.rs bounds by RIPUPS=500 only (L87); add a wall-time/work bound that refuses by name |
| 909 | `1310` ruling | `U1`'s soft hop in the escape (dropped from `1310`'s build) | W3-ROUTER | router: U1 soft hop, keepout anywhere, dangling breakout stub; bytes may move |
| 910 | `1310` A7 | A keepout anywhere on the board, not only at the edge | W3-ROUTER | router: U1 soft hop, keepout anywhere, dangling breakout stub; bytes may move |
| 960 | `1311` F2 | The router can lay a dangling breakout stub of a fixed pair (`paired(12 mm)` with a part's pads either side); … | W3-ROUTER | router: U1 soft hop, keepout anywhere, dangling breakout stub; bytes may move |
| 999 | `1910` F1 (#233) | The supply gate solves `s024l_board()` twice (60 and 165 mV), about 250–390 CI s each, in both jobs; only a … | W3-ROUTER | supply gate solves s024l twice; extract-internal; "never at the cost of what the gate proves" |
| 954 | `4230` adversarial F5, F6 | The ledger's values are re-derived from the generator's plan, not read from its emitted text; the oracle's … | W3-SILICON | ir silicon.rs (I2C_LIMITS L394), svd-hal board.rs, cli ledger.rs |
| 956 | `4230` §18 reduction | Two more HSI copies, unlabelled (`motor.rs:278`, shared by `serial.rs`; `svd-hal` `board.rs:173`); … | W3-SILICON | ir silicon.rs (I2C_LIMITS L394), svd-hal board.rs, cli ledger.rs |
| 1024 | `4240v` F5 (from `4230`) | `HSI_FAST_PPM` 14,984 is not reproduced by summing Table 36/133's terms (+14,987.5 ppm summed, +15,038.8 … | W3-SILICON | ir silicon.rs (I2C_LIMITS L394), svd-hal board.rs, cli ledger.rs |
| 964 | Grok review 2026-11-16 | Free chip-selects take the first leftover pin in key order with no backtrack against the ADC sense pass (the … | W3-SOLVER | "low"; solver backtrack for free chip-selects |
| 1071 | `4300h` F8 | DShot DMA per timer is unmodelled: the solver offers TIM12–14 for DShot without knowing whether they raise a … | W3-SOLVER | solver offers TIM12-14 for DShot blind to DMA; RM0433 DMAMUX table is held; may move DShot pins (stop) |
| 1028 | `4404` (draft) | The serial options' run-time effect in `USART_CR2`, read back in the twin. `4404` reads it at source | W3-TWIN | USART_CR2 options read back in the twin, after 4400g (merged) |
| 922 | `3201` Q4 | Bare literals: the five unread-boot findings in `renode.rs`, `"info"`/`"error"` across crates, … | fold | bare literals: fold into W2-TRACE (renode.rs) and W1-CLI (verify.rs) only where touched |

### A? — buildable, live status unsure (1 rows)

| L | From | Item (short) | WO | Evidence / note |
|---|---|---|---|---|
| 843 | `1907` review | Three `ci/test-shard.sh` refusal fixtures print the runner awk's `\.` warning instead of their reason; they … | W1-CI (optional) | not reproduced here: ci/test-shard.sh runs with no awk warning on this box (awk build-dependent); fixtures still check exit code only. Unsure it is still live |

### B — blocked (79 rows)

| L | From | Item (short) | Why |
|---|---|---|---|
| 769 | review | Bench comparison early | bench: `3302` is Scotty's bench WO (no FEEDBACK, a .bench file) |
| 792 | `4100` review | A citable copper-thickness class (IPC-6012) for declared copper maxima; today a fixture without a fabricator … | 4500 (not written; waits for Sentinel-01 bench) and IPC-6012 (a purchase) |
| 803 | `2205` F4 | The engine's version in the archive header is the package version (`0.1.0`), not a commit | ruling: a commit in the archive header moves every trace (and the bench package) on every commit; the 0.2.0 release did not close it |
| 809 | `1909` F2 | Two committed-proposal tests are vacuous while `proposals/` is absent; do committed proposals belong in the … | ruling: the architect, when P4's evolve loop first proposes |
| 845 | 4400 S4, D9 | ArduPilot's 10 hand-added CAN AF lines (`STM32H753xx.py`, PA9 … PE1, AF9) that ST's data lacks: refuse CAN on … | later: no CAN work order is planned; upstream report is Scotty's |
| 846 | 4400 D3, D4 | ArduPilot's bootloader (`hwdef-bl.dat`) and an `APJ_BOARD_ID` registration | later phase: after P4 flight |
| 847 | 1303 D1 | A Z bound for high-speed interfaces: `spike/1303/s3/bracket.py`, the certified 2D bracket | trigger: the first impedance-controlled interface (4300e1 USB is the likely one) |
| 848 | 4300c D3 | The device's own drive into MISO's load needs the MCU's input capacitance as a maximum (DS12117 gives typical … | held document or ruling: DS12117 gives typical only; "a source or a declared condition" |
| 853 | 1308 F2, F6 | A per-class clearance field, and the pair skew and breakout bounds (`PAIR_SKEW_NM` 2 mm, `PAIR_BREAKOUT_NM` 4 … | 4300e1 (USB static) is not written; pair skew/clearance fields wait "once an interface cites a number" |
| 861 | 4300a Q3, Q5 | A motor pin's off-board load (ESC input plus cable, cited) and its Fmax check; whole-schedule Kani harness … | Q3 to 4500 (not written); Q5 done |
| 868 | 4300d2 D4 | Sentinel-02 at four layers: TI's SNVSAN3F 10.1 item 5 asks for an inner ground plane, named as not met on the … | 4500 (not written; "waits for Sentinel-01's bench results") |
| 869 | 4300f D9 | Off-board devices (a GPS module's compass on a DS-009 GPS port): modelled in the twin, not placed on the board | 4500 (not written; "waits for Sentinel-01's bench results") |
| 872 | 4220 D7 | ArduPilot-in-the-twin: IMU FIFO in our own models (ArduPilot reads only the FIFO) | later: the ArduPilot-in-twin (flight-loop) work order, not written |
| 878 | 4220 D-B4, F-B5 | `SPI_MAX_FRAMES` 8 caps a read at 7 bytes; ArduPilot reads up to 8 FIFO packets (129 bytes) per transfer | 4400c FEEDBACK P11: not reached (IMU never probed); needs ArduPilot probing an IMU in the twin |
| 883 | 4300d2 K11, A7 | A cited current-to-width source for power classes (U11's SW and `VDD_3V3` at 0.2 mm for 679 mA are named, not … | 4500 (not written; "waits for Sentinel-01's bench results") |
| 889 | review 2 A16; 3303 §9 | The pair judge has no minimum coupled length (`DiffPair` has no length field) | 4300e1 (USB static) is not written; pair skew/clearance fields wait "once an interface cites a number" |
| 891 | 3303 §1.2 | A board with two timers: the harness split's shared reset-clock read is argued, not measured | 4500 (not written; "waits for Sentinel-01's bench results") |
| 893 | 4300d2 D6 (comment 6897291654) | A translator pin's capacitance in `extract`: TI prints Cio as one centred value; whether an IBIS model's max … | ruling: the architect, when a board needs a crossing on a bounded line |
| 896 | review 2, fuzz | A stackup bound below the fabricator's published minimum copper (`copper_max_um` accepted down to 1) | purchase: IPC-6012 (copper_max_um floor) |
| 900 | `4230` Q-F, Q-J | `D2CKRDY` after reset and `BOR_LEV` programmed by `FAB-NOTES`' procedure, both on the Nucleo; the 120 … | bench: the next `3302`/`3304` bench session (Scotty) |
| 902 | `4230` Q-B, Q-C | HSI-only clocking: CRSF 416666 is refused; ESC receivers' tolerance is reported, not judged | 4500 (not written; "waits for Sentinel-01's bench results") |
| 903 | `4300d3` D2 | The buck's input capacitance after DC bias at 26.1 V (4.09 µF effective per input; re-pick, for example three … | 4500 (not written; "waits for Sentinel-01's bench results") |
| 905 | `4300d3` | A TVS on the battery input: none fits 6S; the indoor 3S–4S pack does have one | 4500 (not written; "waits for Sentinel-01's bench results") |
| 906 | `4300g` K3 | BOOT0's resistor value, cited from AN4938 rev 7 (held, `217b5dcb`) | 4500 (not written; "waits for Sentinel-01's bench results") |
| 952 | `2302` D2 | The judge stops each DShot timer at the end of its window; a firmware that reads its timer later sees it … | 4400c: unreachable (no motor output); revisit when ArduPilot drives motors in the twin |
| 965 | Grok review 2026-11-16 | Two `Lifecycle` enums (`parts.rs` `{Active, Nrnd, Eol}`, `power_parts.rs` `{Active, Eol, Unread}`) | trigger: power parts gain accept_lifecycle |
| 968 | Grok review 2026-11-16 | Every pin is a flat 10 pF in extraction, MCU included, stated as a condition; no part's own I/O capacitance … | 4300e1 (USB static) is not written; pair skew/clearance fields wait "once an interface cites a number" |
| 969 | `1303b` S3 (#222) | A 2-layer board needing the supply gate must widen its supply tracks or carry a plane: the 2-layer power … | trigger not met (named in the row) |
| 970 | `1303b` S4 (#222) | A finer flow pitch with one factorisation per net, or a larger via pad: the levers when a supply bound nears … | trigger not met (named in the row) |
| 971 | `1303b` ruling (iii) (#222) | A 2-layer ground return read from the filled zone instead of refused (`GroundReturnUnread`); `extract` would … | trigger not met (named in the row) |
| 972 | `1303b` (#222) | Other domains' return currents in the shared ground copper (the buck's input loop, the 5 V loads): today's … | trigger not met (named in the row) |
| 974 | `1303b`, ruling 6974760231 | A cited pad etch tolerance: pads and via pads are taken at drawn size, a named condition | trigger not met (named in the row) |
| 979 | `4400b2` D1 (b), #196 | `check pinout`'s `hwdef.h` facts (SPI devices, PWM groups, I2C clock, barometer probes, bauds) are bound to … | trigger not met (named in the row) |
| 981 | `4400b2` D3 (A8), #196 | An SPI floor in the identity judge: ArduPilot halves at most 7 times, so 250 kHz runs at 312.5 kHz. Reachable … | trigger not met (named in the row) |
| 982 | `1604` O1 (c), #199 | Per-footprint rotation offsets into JLCPCB's library frame. The fab-format placement writes KiCad's rotation, … | trigger not met (named in the row) |
| 985 | `2303` F9, D4, #200 | An ESC's supply current, idle and per throttle, as a power-parts row named by each motor output. Absent means … | 2304 ruling: the `esc` row is built with 4500 |
| 986 | `2303` F5, D5, #200 | Under lock-step, the judge's DShot stop moves to the run's end; the static judge keeps its +2 ms stop | lock-step: 2304 ruled (a), a declared chain; lock-step is not built |
| 987 | `2303` F6, F7, F10, #200 | Lock-step's costs, measured at 10 µs: Renode's log at about 10 MB per simulated second (two INFO lines per … | lock-step: 2304 ruled (a), a declared chain; lock-step is not built |
| 989 | `2303` D1, D2, #200 | The battery in `4500`'s campaign: a declared current profile (no reboot loop is visible). When lock-step is … | 4500 (not written; "waits for Sentinel-01's bench results") |
| 994 | `1314` D4 (#227); `1314b` F3 | `d3-24-i2c-12-spi`, `serial-four`, `sentinel-02-shaped`, `solve/stress` and … | trigger not met (named in the row) |
| 995 | `1314` adversarial L6 (#227) | A through-hole pad's ring at the declared tolerances: the ring rule sizes vias only | trigger not met (named in the row) |
| 1003 | `4400f` §3.3, F5 (#229) | RCC-following timers (`4400f` (b)): Renode needs upstream `b14469bba` and `f2ac99657` (unreleased) and a … | upstream Renode |
| 1004 | `4400f` F2 (#229) | Renode's RCC drops the D2 kernel-clock selections: `D2CCIP2R` reads 0 after ArduPilot writes `0xE01009`, so a … | upstream Renode |
| 1009 | Scotty, 2026-11-17 | Component height (Z). No part carries a body height and the specification refuses a `depth` key, so nothing … | trigger: a drone body chosen |
| 1010 | `1313b` §2.4, #208 | A test-pad rule that cooperates with compaction (pads placed after it, by their nets). `1313b`'s rule … | trigger not met (named in the row) |
| 1022 | `4240v` F1 | The `VOSRDY` wait can be met by the 1 it reads before the `VOS` write: RM0433 states no latency for it to … | held document: an ST source for ACTVOS's encoding |
| 1026 | `4404` (draft) | ArduPilot pulls up every SERIAL1–9 line at run time: on H7, `set_options` sets TX and RX pull-ups whenever a … | ruling needed (named in the row) |
| 1033 | `4600` F3 | Coverage has no motor-output target and no fault kind reaches one. The motor driver's package says … | after 4500, before P4's gate |
| 1037 | `4600` F7, F8 | The power input is an XT30 and a series pack, or nothing: no 1S regulator, no buck-boost, no USB or wall … | after the P4 baseline (D7) |
| 1042 | `1211e` F6 | `bulk` judges the rating alone: an 18 pF C0G validates as a motor-rail bulk. Wanted: a bulk capacitance … | held document (named in the row) |
| 1050 | `1216` D5, F3 | DRV8876 IPROPI: no page states its maximum, its level in sleep, or the VREF clamp's figure; the abs max … | held document (named in the row) |
| 1052 | `1216` F2 | Option (b)'s remedy (U11.PG → U10.EN) needs a pull-up on a control-wire net, which the schema cannot declare | only if (b) is ever ruled |
| 1053 | `1216` §4 | Every dynamic power case is unjudged: power-down, brown-out (D1's nSLEEP undefined between VIL and VIH), … | bench: only if the bench shows a need |
| 1065 | `1219` F2 | Outside the IR the coupling is total: `aethelgard-svd-hal::pin_table` knows `STM32H753ZITx` alone, … | with the second vendor (RUL-08) |
| 1066 | `4300h` D4, D6 | A connector pin's off-board drive maximum (`connectors[].pins[].offboard.drive_max_millivolts`, a held … | held page: the work order that declares a peer (S-03 ESCs, a GPS or radio); D4 (b) deferred by the ruling |
| 1067 | `4300h` F2 | The drawn battery input connector is not a `connectors[]` pin (J4 on `sentinel-03`, J1 on … | with (b), deferred by the 4300h ruling |
| 1068 | `4300h` F3 | The bench's Nucleo header wiring (`sentinel-02-bench`, `-hse8`: IMU_SCK on PA5 at CN7.10, five more on … | with (b), deferred by the 4300h ruling |
| 1069 | `4300h` F4 | NRST and BOOT0 face J2 and a test point on four boards; NRST is held to VDD + 0.3 V powered (note 3) | with (b), deferred by the 4300h ruling |
| 1076 | the Grok review 2026-11-21, §2 and suggestion 3 | A machine-readable silicon-delta ledger: each bench sheet's prediction, source, label and measured value as … | after the first Sentinel-02 bench |
| 1082 | `4240d` F7 | SPI SCK stalls between bytes on silicon (RM0433 p. 2176), so `3304` B6's SCK rate must also be read within a … | bench: the next `3302`/`3304` bench session (Scotty) |
| 1083 | `1319` F1 | `aethelgard_ir::names::device_nets` restates the arm rows' own-net names, held to them by a test | ruling: the P4 baseline's schema-epoch decision |
| 1089 | `2304b`, ruled 2026-11-19 | Sentinel-03's ledger: 280 rows across 35 registers are unacknowledged, so its `--fab-out` is refused before … | person-directed: acknowledgements are a person's judgment per register and class |
| 1099 | `1317` F1, F2 | The placer's tie key re-rolls when any part leaves, so about 73 % of one-part removals move a fitting board's … | ruling: a placer change that moves pinned placements must be ruled first |
| 1100 | `1317` F3 | Removing C4 swaps U10 and U11 in the rank, and no seed recovers the fit (112.25 mm on the fixture). No … | ruling: a placer change that moves pinned placements must be ruled first |
| 1101 | `1317` F6 | `layout_of`'s least-over tie rule (the earlier seed) is untested: mutant M4 survives | ruling: a placer change that moves pinned placements must be ruled first |
| 1108 | `4405` D2 | At compiled defaults only the first GPS port is bound (`GPS2_TYPE` defaults to 0, `AP_GPS_Params.cpp:31`), so … | decision needed: finding, or acknowledged as needing GPS2_TYPE |
| 1114 | `4406` D4, D5, D6 | Parameters can also be applied at boot by ChibiOS persistent params (`Util.cpp:581-640`, compiled only with … | trigger: bootloader flash or an IOMCU board |
| 1118 | `4240c` Q4 | An external HSE clock is refused at `emit` (`HseSourceNotDrawn`): its source has no declared other end. … | bench: the bypass-bench work order (4240 D7 (c)), not written |
| 1119 | `4240c` Q5, A4 | No refusal bounds the crystal's distance from PH0/PH1 or its track length: no held source states one … | held document (named in the row) |
| 1123 | `1211b` Q-B, ruled 19:06Z | The ADC's calibration, enable and conversion polls are bounded at a ruled 2× the figure DS12117 states, since … | held document (named in the row) |
| 1124 | `4240c` F1 | The declared 18 pF load capacitors fit the crystal's CL 12 pF only near 6 pF of stray per net; the load lies … | held document (named in the row) |
| 1125 | `4240c` F2 | On the crystal fixture, a B.Cu `VDD_3V3` track crosses under both oscillator nets. No held source states a … | held document (named in the row) |
| 1126 | `4240c` F8 | The ±1 % capacitor family `4240c` §9 names has no held source; the row holds only the cited J (±5 %) part | held document (named in the row) |
| 1129 | `1214` F1 | A `gps` port's declared baud is only AP_GPS's first probe (`AP_GPS.cpp:707-723`), so step 21 judges a rate … | decision needed: which rates a gps port must reach |
| 1131 | `1211d` R6, A5 | No gate judges thermal copper: an exposed pad's thermal connection (vias, plane), or a power net's width (VM, … | ruling: no gate criterion is stated; TI's Ø0.2 vias vs 300 µm drill is a design question |
| 1132 | `1211d` R7, A6 | The IPROPI reservoir is not judged against current-regulation response: R2·C1 = 150 µs in D1, and SLVSDS7B p. … | held document: SLVSDS7B warns qualitatively, no bound |
| 1133 | `1211d` R8, F-D2 | The committed D2 is undrawable (`PartLimitUndeclared`): SLVSAR1E states no input PWM maximum. No value is … | held document (named in the row) |
| 1141 | `4400d` D5, #207 | Flying Sentinel-02's forward direction. `4400d`'s fixture declares forward as the board's top edge, a fixture … | 4500 (not written; "waits for Sentinel-01's bench results") |
| 1142 | `4400c2` D2, D3, #204 | USART receive DMA lines in the twin's platform: held. ArduPilot needs USART1's line to receive at all, but … | 4400e P9: not needed; foreign-judge's next order, cause of the HardFault first |

### C — idea-only / low value (13 rows)

| L | From | Item (short) | Why |
|---|---|---|---|
| 789 | `4100` D1 | 6-layer stackups: the two-cap inner bound (the exact stripline case, already checked) and a fixture | 6 layers measured no better than 4, and routed worse; no consumer |
| 804 | `2205` F6 | Replay cannot bind kind 10's archived segments to the ELF, which is not archived | a named residual under RUL-19 (3) |
| 810 | `1909` E (parallel Kani declined, 11.3 GB peak) | A Kani proof cache keyed on the harness and the board crate's hash | "an idea only" |
| 842 | `1907` review | A new tier-2 test takes the shard plan's 0.2 s default weight until its time is added to … | a standing rule, not a build |
| 936 | the Grok review 2026-11-17, round 2 | The generated project carries no licence note; the obligation is none (kicad-lib README, `RUL-11`), but it is … | low by its own owner; no obligation (RUL-11); moves every emitted board |
| 953 | `4230` FEEDBACK §18 | `AET-WO-4230`'s residuals: the twin presents Renode's `OPTSR` image, not silicon's; Fm+'s data-valid maximum … | a tracking row; each residual rides with whatever touches its code |
| 1001 | `1910` F3, D10 (#233) | 42 MULTI tests pin several properties each; split them into named tests over shared runs, keeping every … | "its own work order if wanted"; splits 42 tests for naming only |
| 1002 | `1910` F5, F6 (#233) | `twin_tier2`'s per-test CI seconds include `RENODE_TURN` queueing (upper bounds, not costs); `1909`'s … | recorded; nothing to build |
| 1078 | `1215` D4 | A `validate` refusal, or a key, added without a bump is a bump, or the work order says why not (C1: eight … | a review rule for every IR WO, not a build |
| 1085 | `2307` F2 | `validate` accepts one reason copied into every acknowledgement entry, or a copy of the ledger's `judge`: … | ruled not built in 2307 (a rule about people's text) |
| 1094 | `1103` F2, F3 | `board_names` does not see a name built by `concat!` or at run time, nor `bridge/build.rs`, … | a manual check at the next P-gate review |
| 1115 | `4406` D8 | With no `AP_GPS::var_info` and no declared GPS port, a compiled GPS-protocol serial row is not named; … | unreachable at the pinned tag |
| 1122 | `1211d` Q4, ruled 19:06Z | The DRV8833's system-level bulk (SLVSAR1E §9.1) is not judged. D2's one 10 µF is the datasheet's stated VM … | ruled; the revisit came with 1211e's bulk |

### Unsure: likely closed already (verify before scoping) (11 rows)

| L | From | Item (short) | Why |
|---|---|---|---|
| 761 | 2100 | 2100 F24 (cold-write every `RCC_*ENR`) | likely closed: 4300a FEEDBACK §P6 "2100 F24: every RCC_*ENR cold"; the row was not updated |
| 791 | `4100` F9 | The 4-layer tier-2 tests' CI weights (460 s each) are estimates, not measured alone on CI | likely closed by the refits 1910/1912/1913; the Kani harness split is 3303/3303b |
| 793 | `4300` D7 | The battery-sag chain: battery, then buck/LDO dropout, then VDD, fed into `--rail` from declared parts | likely closed: 2304b solved the battery-sag chain |
| 844 | 4400 D1 | A crystal part (HSE) with its load capacitors, so `mcu.hse_hz` may be non-zero | likely closed: the 4240 series built HSE (4240a-c, a 16 MHz crystal fixture) |
| 892 | 4300d2 D5 (comment 6896976241) | The Sentinel-02 power fixture's full fabrication package: `INV-03` needs its campaign, which refuses a power … | likely closed: 2304b deleted PowerNotSimulated; what remains is 1078's acknowledgements |
| 920 | `1206` F10 | A part with a table row but no vendored model (`ICM42688P`, `ICP10111`, `BMP581`) passes `validate`, and … | likely closed: 1209/1210 refuse PartNotDrawable at validate for drawn boards |
| 955 | `4230` §18 | CI cost: Kani's `every_valid_sequence` takes 158–245 s per seed; the motor fixtures may prove the I2C1 and … | likely closed by the refits 1910/1912/1913; the Kani harness split is 3303/3303b |
| 980 | `4400b2` D2 (A6), #196 | Judge each port's `DEFAULT_SERIALn_PROTOCOL` (from its role) and `_OPTIONS` (from `inverted`). Measure the … | likely closed: 4405 binds protocol, options and baud to var_info |
| 1018 | `4201`, #237 | A BMP581 put through `droop-bor3`'s brown-out reset: a scheduled line is stamped 1 ns before its count … | likely closed: 4202 "the twin's BMP581 brown-out stamp fault is fixed" |
| 1025 | `4240v` F7 | `4240v`'s reduction proposals not taken (P1, P6, P7, P8, R1, R2, R4: about −50 lines), each safe in a later … | unsure: 4240b rewrote the bring-up; whether 4240v F7's seven reductions were taken is not stated |
| 1130 | `1211d` R5, F1 | A current output cannot be tied off: SLVSDS7B p. 13 allows IPROPI to GND with VREF above GND (regulation … | likely closed: 1217 built the IPROPI-to-GND tie (same as 1211e F2) |

### Excluded (Scotty, operator, fetch, other repo, or being built) (12 rows)

| L | From | Item (short) | Why |
|---|---|---|---|
| 825 | review A10.2; Scotty 2026-11-12 | `aethelgard mcp --root DIR`: every path flag confined to one directory | deferred by Scotty to the operator's dedicated uid |
| 840 | `1601` §1.3, R4 | A dedicated uid for `aethelgard` on Furnace, so `uid_boundary` holds there | the operator's |
| 850 | 1303 D6 | IPC-6012 Tables 3-4 and 3-14 (copper and plating minima) | Scotty: purchase/fetch |
| 864 | 4300d D-B3, D-B4 | An inductor with a guaranteed Isat minimum (the held IHLP states typical only); a declared service life for … | Scotty: purchase/fetch |
| 942 | `3401` review R4 (#190) | ST AN2606 (the system bootloader's USART protocol) is unread; `BENCH.md` B8's protocol line is [inferred] … | Scotty's fetch list |
| 1020 | operations, 2026-11-19 | Every builder pushes through `gh-write`, which mints its token on the S25. When Termux died on 2026-11-19, … | Scotty (phone) |
| 1051 | `1216` F1 | **Sentinel-03's MOTOR1 is on PE9, a TT_ha pin, at J5**: a 5 V-logic peer on J5 exceeds TT even with the MCU … | being built: AET-WO-4300i |
| 1072 | `4240d`, `3304` | **The bench session** at the Nucleo: `4240d`'s sheet (B-0 ST-LINK version, B-1 `MCO` at 8.000 MHz as the … | Scotty, at the bench |
| 1073 | the 2026-11-22 cleanup | Furnace's `~/inference/models` (277 GB) is the furnace777 inference plane's (an enabled … | kept; furnace777's |
| 1074 | Tviburi #3, #4 | Tviburi: the blind `TVI-WO-0100` build (37/40) is Tviburi PR #4, kept as the better output (Scotty, … | another repository (Tviburi) |
| 1079 | `4240d` F1 | DS11853's current revision is unconfirmed: only Rev 8 (3-Feb-2022) is held, from a mirror, for the ST-LINK … | the architect, a fetch when st.com is readable |
| 1081 | `4240d` F6 | CN12's fitting on the NUCLEO-H753ZI is contradictory at source: UM2407 Rev 6 §9.2 says soldered by default, … | Scotty, at the bench |

## 5. Caveats

- **"U" rows** are rows I believe are already closed, but whose owner cell does not say so. Each names the WO that seems to have closed it. The architect should close them in the next docs PR or say otherwise. They are `L761` (2100 F24, in `4300a` FEEDBACK §P6), `L791`/`L955` (weights and refits), `L793` (the battery-sag chain, `2304b`), `L844` (the HSE crystal, `4240`), `L892` (`PowerNotSimulated`, `2304b`), `L920` (part without a model, `1209`/`1210`), `L980` (serial options, `4405`), `L1018` (BMP581 stamp, `4202`), `L1025` (`4240v` F7 reductions: genuinely unsure), and `L1130` (IPROPI tie, `1217`).
- **A?** `1907` review (the awk `\.` warning): `ci/test-shard.sh` printed no warning on this box. It may depend on which awk is installed, so it is live only on some runners.
- **B rows owned by `4500`** are blocked only because `4500` is held ("waits for Sentinel-01's bench results"). `1314` stop (#227) is the exception: it is buildable now and is listed as W4-FAB.
- **The 2304 FEEDBACK split.** It sends "L695's pull-up and capacitance half" (`3303` F3) and "L723" (ESR on ceramics, `4300g` K8) "to a static work order" that no §7.1 owner names. They are W3-COVER and W2-IR-POWER here.
- **Rows that span crates were split:** `4211` F2 (CLI verdicts in W1-CLI, KiCad files in W4-EMIT), `1311` adversarial J, F (pin-check echo in W1-IMG, eda reader in W2-EDA), and `2308` F1/F3/F8 (Scotty's Note column excluded, the rest in W4-EMIT).
- **Sizes** are judgements from the row text and a grep, not measured.
