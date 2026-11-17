# AET-WO-1604 — FEEDBACK

**Branch:** `wo/aet-1604-cli-backlog` · **Base:** `ecc6be9` (`origin/main`) · **Builder:** Claude Code (Opus 5.5) as `forge` on Furnace, as project manager. · **Cadence:** build, with one measured stop (§1.3). · **Work order:** `docs/work-orders/AET-WO-1604.md` on `origin/claude/argon-repo-docs-synthesis-rjrb0d` (`e5a87ae`, docs PR #198).

**Status: READY.** §1.1–§1.5 are built, §1.3 as ruled on PR #199 (§7). The gates, the adversarial pass (§6, §7) and the reduction pass (§8) are done.

Labels: **measured** (run here), **read** (read at source), **inferred**.

## 0. Read at source before the predictions

These were read, not run, before any prediction below was written. JLCPCB's pages and PR #190's package were also read before this file existed, so §1.3's facts are recorded here as read, never as predictions.

- **R1 · `run.rs` has no standalone sandbox probe.** The Landlock ABI and the seccomp result exist only as the trampoline's status line (`run.rs` `confine_main`, `:1201-1276`): `sandbox abi=N uid_boundary=B`, or `unavailable <layer> <reason>`, which `parse_status` (`:826`) turns into `SandboxReport` or `Stop::sandbox(layer, …)` (exit 4, `SandboxUnavailable`). A real `Invocation::run` is the only way to learn either without a second implementation.
- **R2 · The scratch check is `main.rs`'s `scratch_base()` (`:625`), not `run.rs`'s** as §1.1 says. It creates the default base (`$HOME/<USER_SCRATCH>`, or `ROOT_SCRATCH` as root) when `TMPDIR` is unset, and refuses `ScratchUnsafe` (exit 4) on any ancestor not owned by root or this uid, writable by group or other, or (as root) not searchable by others, or a real path outside `[A-Za-z0-9/._-]`. `Scratch::new()` (`:722`) depends on it, so with an unsafe scratch no tool can be probed at all.
- **R3 · Every version probe already runs inside the sandbox:** `crate::version(Invocation)` (`main.rs:794`) for `kicad-cli version` (`:912`, `:1016`, `fab.rs:174`, pinned by `kicad::pinned` at `:912` only), `cargo --version` (`firmware.rs:643`), `cargo kani --version` (`:1279`, with `KANI_HOME`, `RUSTUP_HOME`, `PATH` from `required_var`), and `renode --config <file> --version` (`:1666`, with the two `DOTNET_*` variables and `/proc`, `/dev/urandom` read-only), the version pinned to `Renode v1.17.0` only when a boot is planned.
- **R4 · ngspice has no binary and no version probe.** It is `libngspice.so.0`, linked by soname into `aethelgard-bridge` (`bridge/build.rs:22`), which is looked for beside the CLI (`analog.rs:120`). The bridge prints its identity line first in every mode (`bridge/src/main.rs:78`) and, with no argument, exits 2 with its usage (`:91-93`). The `44.2` the CLI names (`analog.rs:39`) is a constant, checked against what the bridge's answers say; no request-free run reports it. `bridge/` is outside this work order's fence.
- **R5 · The reaper's record carries no time.** Its status file is the sandbox line, `ended …`, then `peak rss_kib=N` for a bounded run (`run.rs:1000-1048`); `2302` surfaced the peak as the verdict's `memory` (`main.rs:1825`, `verdict.rs:126`). The bounded boot test runs with `--spec`, so its verdict carries `memory`.
- **R6 · `LineBound` has no test-pad field** (`eda/src/extract.rs:597`); the eda counts a test pad's copper as `Load::TestPad` (`:1020-1023`, `:1138`) from the schematic's parts, which come from `spec.declared_test_points()`. `eda/src/extract.rs` is outside the fence.
- **R7 · `extract`'s `conditions` reach the fabrication notes.** `verify` copies the last `extract` report's `conditions` into the package's `FAB-NOTES.md` (`verify.rs:1590`, `:1336`, `fab.rs` `notes`), which `MANIFEST.json` hashes. A board with a test pad on a bounded line (Sentinel-01's bench board, `TP1` on `I2C_SDA`, PR #190) therefore gets a changed `FAB-NOTES.md` and manifest from §1.5, beyond §2's "only the two new files". See Q2.
- **R8 · The MCP tool list is computed in the binary crate** (`mcp.rs:172`), and `mcp_tier1.rs` checks the tool names only, not their schemas.

## 1. Predictions (written before any measured run)

| # | fixture | prediction | label |
|---|---|---|---|
| P1 | `doctor` on Furnace's host, outside the image, `TMPDIR` unset | exit 1; `kicad-cli`, `renode`, `cargo-kani` and the bridge missing, each naming the subcommands it blocks; `cargo` found; scratch safe; the sandbox line from a sandboxed `aethelgard --version` reports Landlock ABI ≥ 6, seccomp installed, `uid_boundary` false (`tool_uid`) | inferred |
| P2 | `doctor` with `TMPDIR=/tmp` | exit 1, `ScratchUnsafe` naming `/tmp` (mode 1777), every tool reported as found or missing on `PATH` but not version-probed, and no sandbox line | read (R2) |
| P3 | `doctor` with `PATH` holding a `kicad-cli` stub that prints `10.0.7` | the version is the stub's, judged by `kicad::pinned` as `check` judges it, and the stub ran confined (its sandbox line is the probe's) | inferred |
| P4 | `doctor` inside the pinned tools image under `capped-run`, as root | exit 0 except the bridge if the image does not carry it beside the binary; `uid_boundary` true unless rootless podman makes the namespace non-initial (`user_namespace`, `AET-WO-1601` R4) | inferred |
| P5 | the snapshot test after a required argument is added to a real subcommand | fails, naming the tool and `inputSchema.required` and `inputSchema.properties.<long>` | inferred |
| P6 | the bounded memory boot on Furnace | ends 5–15 s after start, `memory.peak_rss_kib` > `memory.bound_kib` (1,048,576) | inferred |
| P7 | `extract` on a spec with `TP1` on `I2C_SDA` | today: the verdict names no probe; after §1.5, one condition naming `TP1` and `I2C_SDA` | read (R6) |
| P8 | `cargo test` on the CLI after §1.5 | no existing tier-1 golden moves; a tier-2 package golden carrying a test point on a bounded line would move (`FAB-NOTES.md`) | inferred |

## 2. Measured

- **M1 · `doctor` on Furnace's host** (`target/debug/aethelgard doctor`, `TMPDIR` unset, outside the image): exit `1`; `scratch` `/home/forge/.cache/aethelgard`; `sandbox` `{"landlock_abi":8,"tcp_confined":true,"scoped":true,"sockets_denied":true,"uid_boundary":false,"uid_boundary_unmet":"tool_uid"}`; missing `kicad-cli`, `renode`, the bridge, `cargo-kani`, and `RUSTUP_HOME` (unset in this shell, so cargo is found but not probed). P1 held, except that cargo was not probed: P1 did not foresee the unset variable.
- **M2 · `doctor`'s tier 1** (`tests/doctor_tier1.rs`, 14 tests, stub tools on a `PATH` of their own and a stub bridge beside a hard-linked copy of the binary): everything present → exit `0`; one tool missing; cargo missing (named once for `build`, `prove`, `verify`); a missing variable; tools off their pins; a tool that will not say its version; each probe in a scratch of its own (a `kicad-cli` that plants a file in its `HOME` is not seen by the `renode` probe); an unsafe scratch (`TMPDIR=/tmp`: `ScratchUnsafe` naming `/tmp`, every tool found but none run, a marker stub never ran); an inherited descriptor (the sandbox `null`, `SandboxUnavailable` `descriptors`, no probe ran); a stub that tries to read a file outside its grant prints the version, not the file (the probe ran confined); the bridge absent, from another tree, refused by the loader (exit 127, the loader's line named), and exiting 0 with no request. P2 and P3 held.
- **M3 · The snapshot on the real tree** (`~/.cache/aet-1604/mut/M1-extract-rev.txt`): a required `--rev` added to `extract` fails `the_served_tool_list_is_the_committed_snapshot` with `tool `extract`: field `/inputSchema/properties/rev/description` added ("MUTANT")`, `…/properties/rev/type` added and `…/required/2` added (`"rev"`). P5 held.
- **M4 · The memory-cap boot** on Furnace: 5.05 s, `memory.peak_rss_kib` above `memory.bound_kib`. P6 held.
- **M5 · `extract` on the bench fixture** (`tests/fixtures/sentinel-01-bench.json`, emitted): four conditions, `TP1` on `I2C_SDA`, `TP2` on `I2C_SCL`, `TP3` on `SPI_SCK`, `TP4` on `SPI_MOSI`; none for `TP5`–`TP12`. Without `TP1` in the specification the board has no `TP1`, `I2C_SDA`'s `pad_count` drops from 5 to 4, and its condition goes. P7 held.
- **M6 · The gates:** `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (Furnace, capped scope). Clippy's first run caught `RUL-16`'s compiled fence: the first build of §1.5 read `Specification::declared_test_points`, which `crates/aethelgard-cli/clippy.toml` disallows ("a test point is drawn on the board only; nothing solves it, and no runner reads it"). §1.5 now reads the test pads from the board it judges (D8).

- **M7 · `doctor` inside the pinned tools image** (`container/IMAGE`, `aethelgard-tools@sha256:598501ab…`), run with `~/bin/capped-run --mem 8G -- --rm --network none`. The cap was read back in the scope: `memory.max=8589934592`, `memory.swap.max=0`, `pids.max=8192`. The run used this branch's debug binary and a bridge built from the same bridge sources (digest `91e72fcc…`, from `~/.cache/aet-4300c`), both mounted read-only. Result: exit `0`, `missing` empty, `kicad-cli` `10.0.7`, `Renode v1.17.0`, the bridge `0.1.0` with `ngspice_loaded`, `cargo 1.99.1`, `Kani Rust Verifier 0.68.0 (cargo plugin)`, scratch `/var/lib/aethelgard`, Landlock ABI 8. `uid_boundary` is false with `user_namespace` on every probe: rootless podman's root is not the initial namespace's (`AET-WO-1601` R4). CI's runner is rootful, so the boundary is expected there. P4 held. Rerun at `bd9da55`, after F1, with the same result. Kept in `~/.cache/aet-1604/img/doctor-in-image.json`.
- **M8 · The whole workspace** (`cargo test --workspace --no-fail-fast`, Furnace, capped scope): 2,076 passed, 0 failed. `mcp_tier1`'s README count (19 → 20 tools) and `strict_fence`'s module list (`doctor.rs`, role `Other`: it reads no KiCad file) moved with the new subcommand. No tier-2 test was added, so `TIER2_MIN` does not move.

- **M9 · CI on PR #199 at `b91bede`** (§1.1, §1.2, §1.4, §1.5, before the §1.3 ruling): every job passed, the tier-2 shards and their aggregate included (`ANSWER tier2_tests_passed=176 floor=176`; `TIER2_MIN` exact). The shards run the workspace as root in the pinned image, so `doctor_tier1` ran as root there, and the refactored `prove`, `build` and `simulate` (D3) met their real tools there.
- **M10 · Sentinel-01 as ordered** (`examples/sentinel-01.json`, merged with #190): `extract` on its emitted board adds six conditions the committed `FAB-NOTES.md` does not hold, `TP9`–`TP14` on `SPI_MOSI`, `SPI_SCK`, `EXT_SCL`, `EXT_SDA`, `I2C_SCL` and `I2C_SDA` (`~/.cache/aet-1604/sentinel-01-notes-delta.txt`). The package moves when regenerated, as Q2 ruled.

## 3. Decisions taken

- **D1 · The sandbox probe is this executable, asked `--version` through `run::Invocation`.** `run.rs` has no standalone probe (R1), and §7's second stop forbids writing one. The trampoline's own status line, from a real confinement, is the probe, so `doctor` reports exactly what the trampoline achieved and never calls `landlock` or `seccompiler` itself. It needs no external tool, so the sandbox is reported even when no tool is installed. §7 did not fire, because no new probe was needed.
- **D2 · Exit `1` when something is missing, not `4`.** Exit `4` is "never a verdict", and its stdout is a refusal (`{"ok":false,"error":…}`). `doctor` reports; it does not refuse. Through `mcp`, `1` is a successful call, so a client reads the report either way. The README's exit-code table names `doctor` in rows `0` and `1`.
- **D3 · One definition of each tool's setup.** (Evidence for "no emitted byte moves": tier 1 on Furnace and the in-image `doctor` run; the refactored `prove`, `build` and `simulate` meet their real tools first in CI's tier-2 shards on this PR.) `firmware::toolchain` (cargo for `build`), `kani_toolchain` (`prove`), and `renode_tool`, `renode_env`, `renode_config` and `renode_pinned` (`simulate`) were factored out of their subcommands, which now call them, and `doctor` calls the same functions. No emitted byte moves (M6).
- **D4 · ngspice.** No `ngspice` binary is looked up, only the bridge, which has no version mode (R4). `doctor` runs it with no request, requires the identity this CLI was built against (`analog::identified`), then the usage exit `2`, and reports `ngspice_loaded: true`, since the bridge links `libngspice.so.0` by soname. It does not report ngspice's version. See Q1.
- **D5 · The scratch is judged by `main.rs`'s `scratch_base`** (R2). §1.1 names `run.rs`; this is the check every tool call applies, so it is the one §1.1 means.
- **D6 · The snapshot** is `tests/mcp-tools.json`, the wire `tools/list` as a client receives it, pretty-printed. It is compared value by value, and each difference is named by tool and JSON pointer. It is rewritten with the repository's existing convention, `AETHELGARD_BLESS=1` (as in `svd-hal`'s goldens): `AETHELGARD_BLESS=1 cargo test -p aethelgard-cli --test mcp_snapshot_tier1`. CI never sets it.
- **D7 · `boot_tier1`'s bound is 50 s,** with the reason in a comment: 10 s of margin below the stub's 60 s sleep, as the sibling wall-bound test has. The reaper's record carries the peak it killed at, not a time (R5), so the test now also asserts `memory.peak_rss_kib > memory.bound_kib` from that record. Adding a time to the record would move emitted bytes, so it was not done.
- **D8 · Test pads come from the board, never the specification** (`RUL-16`). They are every footprint the board draws as `TestPoint:TestPoint_Pad_D1.0mm` (`1312`'s, `eda/schematic.rs` `TEST_POINT_FOOTPRINT`, which is private to `eda`) with a pad on the line's board net, by reference, in the lines' order. The condition is one line per pad: "`` `TP1` is a test pad on `I2C_SDA`: a probe clipped to it is an off-board load outside this bound, so a measured edge is compared with the bound plus the probe's own capacitance ``". The constant is duplicated from `eda`, which is out of the fence; the bench tests fail if the footprint changes.

## 4. §1.3 — the fabricator formats' column map (STOP)

**Sources.** All dated 2026-11-17 and kept in `~/.cache/aet-1604/jlcpcb/`. Every string quoted in this section was found in the saved pages' text, tags stripped and entities unescaped. Nothing on any page mentions mirroring or a bottom-side rotation convention.
- JLCPCB, "Bill of Materials(BOM) File for PCB Assembly", https://jlcpcb.com/help/article/bill-of-materials-for-pcb-assembly ("Last updated on Sep 09, 2026") [read].
- JLCPCB, "Pick & Place File for PCB Assembly", https://jlcpcb.com/help/article/pick-place-file-for-pcb-assembly ("Last updated on Sep 09, 2026") [read].
- JLCPCB, "How To Export BOM and Pick & Place Files From KiCad 10", https://jlcpcb.com/help/article/how-to-generate-the-bom-and-centroid-file-from-kicad ("Last updated on Nov 10, 2026") [read].
- The two sample files those pages link, `Sample-BOM_JLCSMT.xlsx` (sha256 `f6fa6b2c…`) and `Sample-CPL_JLCSMT.xlsx` (sha256 `6cc4f67e…`), read cell by cell [read].
- **Our side:** PR #190 at `9bb5bb7`, `fab/sentinel-01-bench/package/` (`sentinel-01-bench-pos.csv`, `sentinel-01-bench.bom.csv`); `main`'s `cli/src/fab.rs` (`kicad-cli pcb export pos --format csv --units mm --side both`) and `eda/src/bom.rs` (`Reference,Value,Footprint,Symbol,Note`) [read].

**What JLCPCB asks for** [read].
- BOM: "Comment (e.g. 10k, 470uF), Designator (e.g. R1, C2), Footprint (e.g. 0805, 1206) and the JLCPCB / LCSC Part #". The sample's header is `Comment,Designator,Footprint,JLCPCB Part #（optional）`, with full-width parentheses, and it groups designators of one part in one cell (`R5,R6`).
- CPL: "Designator, Mid X, Mid Y, Rotation and Layer". The sample's header is `Designator,Mid X,Mid Y,Layer,Rotation`, its values carry a unit (`95.0518mm`), and Layer is `T` or `B`. The pick-and-place page says "Top / Bottom", "Required units: Metric(mm)" and "Positive values are counter clockwise". The KiCad page says "KiCad writes headers Ref, PosX, PosY, Rot, Side, which you then rename to Designator, Mid X, Mid Y, Rotation, Layer for JLCPCB".

**The map, from what `verify --fab-out` writes today** (Sentinel-01's bench board, 39 parts).

| JLCPCB column | Filled from | Fillable without a guess? | Label |
|---|---|---|---|
| BOM `Comment` | `.bom.csv` `Value` (`100n`, `10k`, `STM32H753ZITx`) | **Yes** as a copy of the value. But a passive's value does not determine a part (voltage, dielectric and tolerance are in no input), and `U3`'s value `TMP117xxDRV` is a family with a wildcard, not an orderable part | measured |
| BOM `Designator` | `.bom.csv` `Reference` | **Yes.** The BOM and placement sets are equal (39 each). Grouping is open (O5): 13 distinct (value, footprint) pairs | measured |
| BOM `Footprint` | `.bom.csv` `Footprint` (`Capacitor_SMD:C_0402_1005Metric`) | **Yes,** in KiCad's naming; JLCPCB's examples use its own (`0805`, `R0603`) | measured |
| BOM `JLCPCB Part #` | **nothing**: `parts.json` has no fabricator part number for any part, and passives have no parts row at all | **No** for all 39 parts. Empty, and named | read |
| CPL `Designator` | `-pos.csv` `Ref` | **Yes** | measured |
| CPL `Mid X`, `Mid Y` | `-pos.csv` `PosX`, `PosY`: the footprint anchor in mm | **Yes.** Each of the nine footprints' anchor is its pad box's centre (`~/.cache/aet-1604/anchors.txt`). `PosY` is negative, in the Gerbers' own frame: Edge_Cuts runs Y −75.85 to −130.70 and every part lies inside it. A horizontal connector's body centroid is not its pad-box centre | measured; body centroid inferred |
| CPL `Layer` | `-pos.csv` `Side` (`top`; all 39 parts are on top) | **Yes;** the spelling is open (O3) | measured |
| CPL `Rotation` | `-pos.csv` `Rot`: KiCad's rotation, −90 to 180 | **Only in KiCad's frame.** Each footprint's offset between KiCad's zero orientation and JLCPCB's library orientation is in no input (nine footprints). JLCPCB's own Fabrication Toolkit "fixes part rotations/positions for JLCPCB", so the offsets exist | read (offsets exist); offsets unknown |

**Unfillable without a guess** (`INV-06`):
- (U1) **the fabricator part number**, for every part;
- (U2) **each footprint's rotation offset** into JLCPCB's frame;
- (U3) **a bottom-side part's rotation and X**. `--side both` already exports bottom parts, but no board has one yet (all 39 are on top, measured), and none of the pages says whether JLCPCB expects bottom coordinates mirrored or its rotation viewed from the bottom.

**Test pads** (`TP1`–`TP12`) are in neither file (measured: 0 `TP` rows in `-pos.csv` and in `.bom.csv`; `1312` marks them out of the BOM and the placement). Nothing is uploaded for them, and the fabricator places nothing there.

**Open for the ruling** (each read at source, none chosen):
- **O1 · Rotation.**
  - (a) KiCad's rotation as is, with the frame named in `FAB-NOTES.md` and the placement preview checked by eye. This is JLCPCB's own documented KiCad route ("rename"), and what `3401`'s ORDER.md §3 already says.
  - (b) Empty and named: the upload then likely refuses or asks.
  - (c) A per-footprint offset table measured against JLCPCB's library: new work, outside this order.
  - The adversarial case "a column filled with a plausible guess" bears on (a): it writes a measured value in KiCad's frame under JLCPCB's heading.
- **O2 · The part-number header:** the sample's `JLCPCB Part #（optional）` (full-width parentheses), or the KiCad page's "LCSC Part #".
- **O3 · Layer:** the sample's `T`/`B`, the page's `Top`/`Bottom`, or KiCad's `top`/`bottom`.
- **O4 · Units and range:** append `mm` as the sample does, or bare numbers; rotation normalised to 0–359 (the sample has `270`) or KiCad's signed −90.
- **O5 · Designators:** one row per reference as now, or grouped by (value, footprint) as the sample does, quoted, since a grouped cell holds commas.
- **O6 · File names:** for example `<project>-jlcpcb-bom.csv` and `<project>-jlcpcb-cpl.csv`, each described in `FAB-NOTES.md` and `MANIFEST.json`, beside the existing two.

## 5. Questions for the architect

- **Q1 · ngspice's version** (§1.1 lists it). It is reachable only from a measurement's answers, or from a version mode in the bridge (`bridge/` is outside this fence, and its sources are digest-pinned, `BRIDGE_SOURCE_SHA256`). `doctor` reports `ngspice_loaded` and no version (D4). A bridge `--version` that asks libngspice itself would close it in a later order.
- **Q2 · §1.5 against §2.** `verify` copies `extract`'s conditions into `FAB-NOTES.md` (R7), so a package whose board has a test pad on a bounded line changes in `FAB-NOTES.md` and in `MANIFEST.json`'s hash of it, beyond §2's "only the two new files". No golden on `main` moves (the bench tier-2 test asserts exit code and verdict only). PR #190's committed `fab/sentinel-01-bench/package/` will move when it is regenerated after this merges. Is that within §1.5's "extract's new condition line", as read here?
- **Q3 · `boot_tier1`'s "the reaper's own record where 2302 made that available":** `2302` made the peak available, not a time (R5). The test asserts the peak; the wall bound stays the test's own clock (D7).

## 6. Adversarial pass (targeted, §5), for what is built

Each new guard was mutated once against the suite it lives in (`~/.cache/aet-1604/mut/run.sh`, `summary.txt`). Every mutant was killed.

| # | §5 case | mutant | killed by |
|---|---|---|---|
| M1 | a snapshot that passes after a required argument is added | a required `--rev` on `extract` (the real tree) | `the_served_tool_list_is_the_committed_snapshot`, naming `extract` and `/inputSchema/required/2` |
| M2 | `doctor` reporting what it cannot know | an unsafe scratch blocks nothing | `an_unsafe_scratch_blocks_every_tool_subcommand_and_nothing_is_probed` |
| M3 | `doctor` reporting a sandbox it cannot install | `SandboxUnavailable` blocks nothing | `a_sandbox_layer_that_cannot_be_installed_is_reported_never_assumed` |
| M4 | the probe outside the sandbox it reports | the `kicad-cli` probe granted `/` to read | `the_probe_runs_confined` (the stub printed the file) |
| M5 | a version off its pin passed | `kicad::pinned` skipped | `a_tool_off_its_pin_reports_its_version_and_blocks_what_pins_it` |
| M6 | a bridge that is not the bridge | any exit accepted with no request | `a_bridge_that_runs_without_a_request_is_not_the_bridge` |
| M7 | a test-pad line on a line with no pad | the net check dropped | `each_test_pad_on_a_bounded_line_is_named_and_no_other`, `a_line_whose_test_pad_is_removed_states_nothing_for_it` |
| M8 | a part that is not a test pad named as one | the footprint check dropped | three `extract_tier1` tests |

**Finding F1 (fixed, `bd9da55`).** `doctor` first ran every probe in one scratch. That had two effects. A tool's probe could plant a file in its `HOME` that the next probe's tool read: measured, the `renode` stub saw `kicad-cli`'s file. And as root, `renode_config` wrote into a `tool/` directory the earlier probes had already handed to the tool uid; `simulate` always writes it before any tool runs. Each probe now gets a scratch of its own, as each subcommand gives its tool, and `each_probe_has_a_scratch_of_its_own` failed before the fix (measured) and passes after.

The §5 case "a fab-format column filled with a plausible guess" is in §7's table.

(Stopped at §1.3 for the column map's ruling, which came on PR #199 before the stop was pushed; §7.)

## 7. §1.3 as ruled (PR #199, comment 6943750741, 2026-11-17 09:30 UTC)

**The ruling.** Build it. JLCPCB's sample files are the format; where the prose differs, the sample wins.
- U1: the part-number column is present and empty for every part, and is named in `FAB-NOTES.md`.
- O1 (a): KiCad's rotation, normalised to 0–359, with the frame named and the preview checked by eye.
- U3: a bottom-side part refuses by name.
- O2: header verbatim; O3: `T`/`B`; O4: `mm`; O5: grouped by (value, footprint), designators quoted; O6: `<project>-jlcpcb-bom.csv` and `-jlcpcb-cpl.csv`, in `MANIFEST.json` and `FAB-NOTES.md`.
- Q1: backlog. Q2: within §1.5; whichever of `1604` and `3402` merges second regenerates `fab/sentinel-01-bench`. Q3: accepted.

**Built.**
- `aethelgard-eda/src/fab.rs` gains only new items, so `3402`'s change to `Notes` and `notes()` merges beside them:
  - `placement()` reads KiCad's placement file. It was `placement_rows`'s check in the CLI, which now delegates to it, so there is one reader.
  - `jlcpcb()` makes both files from the emitter's BOM and KiCad's placement file.
  - `jlcpcb_notes()` is the notes section, appended by the CLI's `notes()` after the brown-out step, as that step is.
- The CLI's `fab.rs` writes the two files in `derive`, so `--fab-out` writes them and `--against-fab` compares them. `describe()` lists them, so `FAB-NOTES.md`'s `## Files` and `MANIFEST.json` name them.

**Decisions within the ruling.**
- **D9 · `Footprint` is the KiCad footprint's name without its library** (`C_0402_1005Metric`, KiCad's own `Package` column). It is an exact substring of the emitter's column, not a JLCPCB package name, which no input carries.
- **D10 · A bottom-side part is exit `3`, `BottomSidePart`,** with `id` the reference. Nothing is written. A refusal to write is not a tool error, and `3` is `verify`'s other package refusal (`UnwritableOutput`).
- **D11 · Each line ends CRLF,** as the emitter's BOM (RFC 4180) does.
- **D12 · Order.** BOM kinds are in the natural order of each kind's first reference (`C2` before `C10`), and references within a kind likewise. A reference that is not letters then digits sorts by its bytes after every one that is. CPL rows keep KiCad's order.
- **D13 · Every designator is quoted in both files.** Other fields are quoted only when they hold a comma, a quote or a line break.
- **D14 · The two files must name exactly the same parts,** or the export is exit `5`, `ToolError`. JLCPCB refuses an upload whose BOM and CPL disagree ("REF, DES designators don't exist in the CPL file").
- **D15 · Coordinates are KiCad's digits as printed, plus `mm`.** No rounding to the sample's four decimals.

**Tests.**
- `eda` unit tests: rotation normalisation (−90 → 270, −0.5 → 359.5, 360 → 0; malformed refused); grouping, natural order and quoting (a value holding a comma); a bottom-side part refused by name; each malformed input refused (designator sets differ, an LF-only BOM, a short row, text after a quote, another header, side `both`, a coordinate that is not a decimal); the notes section citing the three pages.
- CLI unit tests on KiCad 10.0.7's real placement file and the emitter's real BOM for Sentinel-01 as ordered (copied from the committed package at `ac5f8c8` into `tests/fixtures/fab/`):
  - 13 kinds over 39 references; the first row is `100n,"C1,…,C22",C_0402_1005Metric,`;
  - every row ends in the empty part number;
  - every CPL row is KiCad's row with `mm`, `T` and its rotation normalised;
  - the bottom-side refusal's exit, kind and id.
- Tier 2: `verify_2k2_tier2` lists seventeen files and binds the two to `jlcpcb()` of the package's own BOM and placement file; `against_fab_tier2` expects 18 differences. No tier-2 test was added, so `TIER2_MIN` stays exact at main's 177.

**Adversarial pass for §1.3** (`~/.cache/aet-1604/mut/run-fab.sh`, `summary-fab.txt`). Each guard was mutated once; every mutant was killed.

| # | §5 case | mutant | killed by |
|---|---|---|---|
| M9 | a column filled with a plausible guess | the part number filled with the value | `the_jlcpcb_files_from_real_kicad_output_carry_nothing_invented` |
| M10 | a guessed rotation offset | 90° added to every rotation | the same |
| M11 | KiCad's sign kept | rotation not normalised (−90 stays) | the same |
| M12 | a bottom-side row written | `bottom` written as `T` | `a_bottom_side_part_refuses_the_package_by_name` (CLI and `eda`) |
| M13 | BOM and CPL naming different parts | the comparison skipped | `the_two_files_must_name_the_same_parts_and_be_what_their_writers_write` |
| M14 | coordinates rounded | KiCad's digits cut to 7 characters | `the_jlcpcb_files_from_real_kicad_output_carry_nothing_invented` |
| M15 | after the reduction (§8): a quote inside a plain placement field | the check dropped from `eda::fab::placement` | `a_placement_row_with_a_quote_inside_a_field_is_refused` (moved to `eda`) |

## 8. Gates and the reduction pass

- **Gates:** on Furnace, at the READY head, in a capped scope: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --features aethelgard-cli/with-tools -- -D warnings`; `cargo test --workspace --no-fail-fast`, with no failure; and the tier-2 targets compiled (`--features aethelgard-cli/with-tools --no-run`). Tier 2 itself runs in CI. No tier-2 test was added, so `TIER2_MIN` stays main's 177.
- **Measured at `914ded8`, the head this READY commit adds only this line to:** fmt clean; clippy with `with-tools`, 0 diagnostics; the tier-2 targets compile; `cargo test --workspace --no-fail-fast`, 2,087 passed, 0 failed (`~/.cache/aet-1604/test-workspace-7.log`). `TIER2_MIN` is `177` on main and here. Main then moved to `a323aea` (#200, `spike/2303/` and its FEEDBACK only, which no test reads), merged in with a merge commit; `workspace_shape`, `readme_status`, `package_licences` and `strict_fence` pass after it.
- **Reduction pass (diff):** −12 net · 2 duplicates removed · 0 dead paths removed · 0 protected kept · 1 finding below.
  - Duplicate 1: `placement_rows` became a one-line wrapper, and `jlcpcb()` parses the placement file with the same check, so `derive` checked it twice. The wrapper and its call are gone, and its adversarial test now targets `eda::fab::placement` (M15 still killed).
  - Duplicate 2: `doctor`'s `unprobed()` rebuilt `absent()`'s line; it now calls `absent`.
  - **Finding R1 (outside the fence):** the test-pad footprint `TestPoint:TestPoint_Pad_D1.0mm` is stated twice. It is in `eda/schematic.rs` (`TEST_POINT_FOOTPRINT`, private) and in `cli/src/extract.rs` (`TEST_PAD`, D8). A `pub` on the `eda` constant would let the CLI use it; `schematic.rs` is outside this order's fence. The bench tests fail if the two drift.
- **Disk:** no subagent worktree was made. `~/.cache/aet-1604/img/bin` (the 228 MB binary copy) is removed. Evidence is kept in `~/.cache/aet-1604/` (JLCPCB pages and samples, anchors, mutants, logs, the in-image `doctor` line).
