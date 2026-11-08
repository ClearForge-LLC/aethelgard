# AET-WO-1301 — FEEDBACK

**Branch:** `wo/aet-1301-placement` · **Base:** `65d3532` (PR #52, `origin/main`) · **Builder:** Claude Code (Opus 5.5) as `forge` on Furnace · **Cadence:** build-only. **Result:** §1 built. §3 passes in `gates` and `tools`: zero parity findings, and only `unconnected_items` on the emitted reference board. B8 is closed under the architect's mid-build ruling on the exposed pad, case (a), below. The full adversarial pass found six ways through, all now closed; one limit of KiCad's own check (A5) is recorded for a ruling. No §7 stop.

Entry schema: `finding · where · type · recommendation · decision-needed`.

## §3.2 numbers

| Measure | Value | Where it comes from |
|---|---|---|
| Unconnected items on the emitted `sentinel-01` board | **65** | `check drc --spec --schematic`, KiCad 10.0.7 in the pinned image. It equals the minimum spanning edges of the 13 nets with two or more pads (78 pads), so KiCad reports one per ratsnest edge. The S5-names board also gives 65 |
| Parity findings / other findings | **0 / 0** | the same run, under `"rules":"specification"` |
| Board size | **44.7 × 39.9 mm** | outline `(72.7, 76.65)` to `(117.4, 116.55)`, 2 mm outside every keep-out box. It was 43.9 × 39.2 mm before A3 widened the reference-text estimate |
| Format version | **`20260206`** | the version of the vendored footprints, and the one `kicad-cli pcb upgrade` writes. A 20240108 board upgraded by KiCad 10.0.7 came back as `20260206`: no top-level net table, `(net "<name>")` on each pad, and the layer ids renumbered (F.Cu 0, B.Cu 2, Edge.Cuts 25, …). The emitter writes that dialect and that layer table (B7) |
| Placer rules | below | `crates/aethelgard-eda/src/pcb.rs` `place`; also `crates/aethelgard-eda/README.md` |

Each line is printed to the `tools` job's stderr as `AET-WO-1301 …` by `the_emitted_board_has_no_parity_finding_and_only_unconnected_items` and `no_courtyards_overlap_and_every_one_is_inside_the_outline`.

## Build

**The grounds it consumes from 1300's FEEDBACK:** B2 (zones; none are emitted here, and the note for `1302` is below), B7 (the format version, `20260206`, measured) and B8 (the exposed pad, B1 below).

**What was built (§1).**
- **`aethelgard-eda`'s `pcb` module**, the board emitter:
  - each part's vendored `.kicad_mod` tree, given:
    - the schematic's reference and value;
    - every symbol field but Reference, Value and Footprint, as hidden `F.Fab` properties;
    - `(path "/<symbol uuid>")` from `schematic::symbol_uuid`;
    - on each pad, the net KiCad's netlist gives its pin;
    - UUIDs derived from content.
  - It drops `version`, `generator` and `generator_version`. Nothing else in the footprint changes.
  - **Net names follow KiCad 10.0.7's own source:**
    - `SCH_PIN::GetDefaultNetName`: a root-sheet label is `/<label>`, a power symbol's net is its value (domain id, or `GND`), and an unconnected pin's is `unconnected-(<ref>-<pin name>-Pad<number>)`;
    - `EscapeString(CTX_NETNAME)`: `/` is written `{slash}`, line breaks are dropped.
  - Stacked unconnected pins, whose naming the emitter does not model, are refused (`EmitterError`, exit 2), never guessed. The unit tests show the refusal firing on an MCU with its supply pins left open.
- **`schematic.rs`, only to expose what the board needs.**
  - `Part` and `Conn` are crate-visible. `emit` builds the parts once, then writes the sheet, the BOM and the board from them, so each pad's net is taken from the very connections the sheet is drawn from.
  - A decoupling capacitor records the IC it was drawn for (`decouples`).
  - **The schematic and BOM bytes are unchanged:** the sha256 of the emitted `sentinel-01.kicad_sch` is `349968e3…` before and after, and no existing test changed.
- **The outline:** a rectangle on `Edge.Cuts`, 2 mm (`MARGIN_NM`) outside every keep-out box, snapped outward to 0.05 mm. A compile-time assertion keeps `MARGIN_NM ≥ rules::FLOOR_NM`.
- **The setup block:** none is emitted. The gate writes its own (`board::prepare` inserts it where the board has none), and that is the board KiCad judged in every measurement here.
- **`emit`** writes `<project_name>.kicad_pcb` beside the schematic and the BOM, and its report gains `board: {path, sha256}`.
- **`check drc --schematic <sch>`:**
  - copies and hashes the schematic as `input.kicad_sch` beside the board's copy, where KiCad looks for it;
  - scans the schematic and the prepared board with the new `parity` module, all exit 3, before any tool runs:
    - a schematic that is not one `kicad_sch` (`MalformedSchematic`, read by the same strict reader as a board);
    - one with a `(sheet …)` anywhere (`HierarchicalSchematic`), because KiCad would read the file a sheet names, which the gate never copied or hashed (1300's A8, for this path);
    - and the four adversarial refusals below;
  - adds `--schematic-parity`, and the verdict gains `schematic: {path, sha256}`.

  The `schematic_parity` findings are judged like every other (`kicad::judge` already read that key). Without `--schematic`, nothing changes; a tier-1 test checks that the stand-in tool then gets no `--schematic-parity` and no `input.kicad_sch`. MCP's `check_drc` gains the optional `schematic` argument from the command tree.

**The placer's rules (§1.2, §3.2).** It is integer-only and a function of the parts alone:
1. The MCU's origin goes at `(100 mm, 100 mm)`.
2. Every other IC goes in reference order, beside the placed IC it shares most nets with.
3. Every passive goes in reference order, beside the IC it shares most nets with. A decoupling capacitor goes beside the IC it was drawn for.
4. **Counting:** ground and `unconnected-…` nets are not counted. Ties go to the lower reference.
5. **"Beside":** the first spot, in square rings of 0.25 mm out from a target, where the part's keep-out box clears every placed one by 0.5 mm (`GAP_NM`). Within a ring, candidates go by squared distance, then y, then x. Origins snap to 0.05 mm.
   - **The keep-out box** is the courtyard plus the silkscreen graphics, plus each visible silkscreen text as this part writes it. Every glyph counts as wide as KiCad's widest, `m`, whose advance is 4/3 of the font width (A3). The silkscreen is included so that a long reference cannot reach a neighbour.
6. **The target:** the anchor's lowest-numbered pad on the first net they share, signal nets before rails. So a decoupling capacitor sits by the power pin its rail names. With no shared net, the target is the anchor's courtyard centre.
7. Every part is at rotation 0 on the front.

ICs are placed before passives because a passive's anchor must already be on the board. A reading of §1.2 that took "in order of reference" strictly across all parts would place `C4` before `U2`, which it decouples. It is stated here as the builder's choice.

**Where §1.5 lives.**
- **Tier 1** (`crates/aethelgard-cli/tests/placement_tier1.rs`, plus unit tests in `pcb.rs`):
  - `emit_writes_the_board_and_two_runs_are_byte_identical`: two processes, `env_clear` with `LC_ALL=tr_TR.UTF-8`, `TZ`, `SOURCE_DATE_EPOCH`, other working directories and out paths;
  - `the_board_does_not_depend_on_the_parts_order`: the parts given forward and reversed;
  - `no_courtyards_overlap_and_every_one_is_inside_the_outline`: by `pcb::geometry`, read back from the board as written, inside the outline by `FLOOR_NM`. It also checks that the board holds no segment, via, zone, setup block or net table;
  - `many_parts_with_long_references_never_overlap_or_leave_the_board`: §5.3's shape, 61 parts with 40-character references;
  - `every_schematic_pin_net_is_on_its_pad_and_every_footprint_is_its_symbol`: connectivity read from the sheet's own geometry, wires to labels and power symbols. Each pad matches. Each no-connect pin's pad carries `unconnected-(<ref>-…-Pad<n>)`. The only pad left over is U3's pad 7, which has no net. Each footprint's library id, value, path and every field equal its symbol's;
  - `check drc --schematic` with a stand-in tool, and its refusals (`HierarchicalSchematic`, `MalformedSchematic`, `UnreadableInput`), and MCP's schema.
- **Tier 2** (`with_tools.rs`, the `AET-WO-1301` section):
  - `the_emitted_board_has_no_parity_finding_and_only_unconnected_items`: `sentinel-01`, and the board of 1300's S5 names (`U"3`, `I2C(SDA)`, `VDD"3V3\`, …);
  - `every_pad_carries_the_net_kicads_netlist_gives_its_pin`: `kicad-cli sch export netlist` of the emitted schematic is the oracle. Its 188 (reference, pin) nodes equal the board's 188 netted pads exactly;
  - `parity_fails_a_board_that_does_not_match_its_schematic`: §5.2 as a regression (below).
- **The adversarial pass's tests:** `crates/aethelgard-eda/tests/adversarial_1301.rs`, `crates/aethelgard-cli/tests/adversarial_1301.rs` (tier 1) and `adversarial_1301_tools.rs` (tier 2). Each attack in them asserts the secure outcome.
- **The gate's 1300 tests** pass unchanged, the adversarial ones included. `board.rs`, the gate's general board preparation, is byte for byte its base: every parity-only check lives in the new module. The emitted board passes the gate's own refusals: no local clearance, no net class, no waiver. Its footprints carry `duplicate_pad_numbers_are_jumpers no`, which is allowed.

| # | Finding | Where | Type | Recommendation | Decision-needed |
|---|---|---|---|---|---|
| B1 | **1300's B8, the exposed pad: ruled (a) mid-build.** KiCad's parity refuses a netted pad whose number has no symbol pin (`drc_test_provider_schematic_parity.cpp`, measured: `net_conflict` "No corresponding pin found in schematic" on `Pad 7 [GND] of U3`). Renumbering the pad to `2` would trip library parity, because `padNeedsUpdate` compares pad numbers. So §1.1's "joined to … ground" and §3's zero parity could not both hold without a schematic pin. **TI's TMP117 datasheet, SNOSD82D (June 2018, revised September 2022)**: §9 *Power Supply Recommendations*, p. 35: "The package thermal pad is not connected to the device ground and should be left unsoldered for best measurement accuracy. If the thermal pad is soldered, it must be left floating or grounded." §10.1 *Layout Guidelines*, p. 36, says the same ("connected to the ground or left floating"). The datasheet permits floating, so under ruling (a) pad 7 carries **no net**, and the schematic is unchanged. B8 is resolved by the datasheet, not by a ground connection | `pcb.rs` | ruling applied | `1302` may tie the pad to ground in copper only if parity allows it, and measured, it does not while the symbol has no pin 7 | no: ruled |
| B2 | ICs are placed before passives, against a strict reading of "in order of reference" (above) | `place` | builder's choice | none, unless the architect wants another order | no |
| B3 | A decoupling capacitor's IC is the one the schematic emitter drew it for (`Part::decouples`). By nets alone, every decoupling capacitor on a rail ties across all ICs on it, and would sit by the MCU | `schematic.rs`, `place` | builder's choice | none | no |
| B4 | `check drc --schematic` refuses a hierarchical schematic. This closes 1300's A8 for the DRC path only; `check erc` still does not scan | `parity::schematic` | scope edge | A later work order applies the same scan to `check erc` | yes, low |
| B5 | **Fence readings.** `verdict.rs` gains an optional `schematic` field, and `kicad.rs` gains `prepare_parity`. Both files also serve `check erc`; the changes are `check drc --schematic`'s alone. The first build put the schematic scan in `board.rs`, which is not a new module; it moved to the new `parity` module, and `board.rs` diffs to empty | `verdict.rs`, `kicad.rs` | fence reading | none, unless the architect reads the fence narrower | no |
| B6 | Which IC a capacitor decouples comes from the specification's device order, not from the schematic's text. So reversing `i2c_devices` emits a byte-identical schematic but a different board: C4 and C5 change places (adversarial pass, measured). `INV-01` holds, since the arrays are ordered input | `schematic.rs` `decouple` | measured | none; noted so that a byte-identical schematic is not read as a promise of a byte-identical board | no |
| B7 | Placer time grows with parts and reference length: 80 flashes with 43-character references take 15 s in a debug build, and 100 take 25 s. The solver bounds the parts, refusing more than about 103 SPI devices (`NoFreePin`) | `spot` | measured | Revisit when boards grow, by a coarser first ring or by indexing the taken boxes | no |
| B8 | The footprints' 3D `model` paths (`${KICAD10_3DMODEL_DIR}/…`) are kept as copied; no check reads them | `pcb.rs` | noted | none | no |

**What `1302` should know.**
- **Escape from the LQFP:** the pitch is 0.5 mm and the pads are 0.3 mm wide, so the gap between pads is 0.2 mm. Under the 0.2 mm floor (`FLOOR_NM`) no track passes between two pads, so every MCU net escapes straight out from its pad. Parts sit at least 0.5 mm, keep-out to keep-out, from the LQFP's courtyard. With both courtyards' margins, that leaves at least 1 mm between the LQFP's pad tips and any neighbour's copper. Of the MCU's 144 pads, 109 are single-pad `unconnected-…` nets, which need no route (U3's ALERT is the 110th on the board).
- **The nets to route:** 13 nets, 78 pads, 65 ratsnest edges:
  - `VDD_3V3`: 29 pads;
  - `GND`: 23 pads;
  - `/I2C_SDA` and `/I2C_SCL`: 4 pads each;
  - nine two-pad nets.

  `VDD_3V3` and `GND` are the case for zones.
- **Zones:** the gate refuses `(connect_pads (clearance …))` (1300 B2), so a zone must be emitted without it and left unfilled (`--refill-zones`). U3's pad 7 has no net, so a GND zone over U3 must clear it like any other copper.
- **What the parity gate now refuses (A1, A2, A6, A7) binds `1302` too:** no `board_only` footprint, no `pintype` marking `no_connect` on a netted pad, and no `unconnected-` net on two pads. A track joining an unconnected pad to anything would be the last of these.
- **No setup block or net table is emitted.** Tracks and vias carry `(net "<name>")` in this format, as pads do.
- **Placement is not tuned for routing.** `GAP_NM`, the rings and the target choice are the levers if `1302` measures congestion.

## Flag-and-stop check (§7)

None arose:
- **Parity and the existing schematic tests:** parity passes with the schematic emitter's output unchanged. The emitted `sentinel-01.kicad_sch` hashes `349968e3…` and its BOM `5ae35212…`, as at the base. The exposed pad's conflict was ruled mid-build (B1), not parked.
- **A placement decision that depends on anything but the specification and the vendored data:** none. Two emissions are byte-identical across processes, environments, locales, time zones, working directories, out paths and the parts' order (tier 1, and the adversarial pass's host runs).
- **Outside the fence, `unsafe`, or an image change:** none. The fence readings are B5. `container/`, `.github/workflows/`, `aethelgard-ir`, the steering documents, `README.md`, `LICENSE-*` and `docs/inputs/**` diff to empty against `65d3532`. No dependency was added; `Cargo.toml` and `Cargo.lock` are unchanged.
- **A test that can only go green by being skipped, ignored or weakened:** none. Three of the reviewer's tests were changed to follow where the fixes went; each still asserts the secure outcome:
  - its `board_only` eda test runs the parity gate after `board::prepare`, because the refusal is parity-scoped;
  - its "two unconnected pins on one KiCad name" row now expects the gate's exit-3 refusal, where at `6b55cb3` KiCad gave `net_conflict`;
  - its metadata test (A5) is pinned as measured, a limit of KiCad's check, for a ruling.
- **`sudo`, a system package, or a secret:** none, beyond the push route: an S25-minted token through `gh-write`, revoked after use. The TMP117 datasheet was fetched once for B1, and `pypdf` went into a scratch virtual environment to read it; neither is in the repository.

## Adversarial pass (§5, full)

**How it was run.**
- **The reviewer:** a fresh Opus subagent in its own worktree, at `6b55cb3`, given the code and the work order but not the builder's conclusions.
- **Measured in the image:** the reviewer ran KiCad 10.0.7 in the pinned image under `--network none`, with the host-built CLI.
- **Kept as regression tests:** its tests are in the `adversarial_1301` files.
- **The builder's own cases:** F6 and F7 below came from reading KiCad's parity source for the reviewer's F2, and were measured the same way.

| # | Finding (all measured at `6b55cb3`) | Severity | Fixed by | Decision-needed |
|---|---|---|---|---|
| A1 | **A `board_only` footprint passes parity.** A copy of R5 as R99 on real nets, and a second `R5` valued `0R`, each gave nothing beyond `unconnected_items` (65 → 67). KiCad's parity check skips `board_only` as neither extra nor duplicate | high | `parity::board` refuses it under `--schematic` (`BoardOnlyFootprint`). Without parity the flag waives nothing, so `board.rs` is unchanged | no |
| A2 | **Two unconnected pins share one net.** `RUL-14` admits `)`, so devices `T` and `T-ALERT-Pad3)` name their ALERT nets with one a prefix of the other. T's pad 3 on the longer name passed parity (72 → 73). KiCad accepts a pad net that starts with `unconnected-` and with the schematic's net | medium | `parity::board` refuses an `unconnected-` net on two pads (`SharedUnconnectedNet`). KiCad's own update writes one per pin; stacked unconnected pins, which would share one, are refused by the emitter anyway | no |
| A3 | **Wide references beat the keep-out estimate.** Measured by Gerber plot: `m` advances 1.301 mm per 1 mm of font, and `@` 1.263 mm; the estimate allowed 1.2 mm. Twenty-four devices with 64-byte references of `m` gave 9 `silk_edge_clearance`, 2 `silk_overlap` and 20 `silk_over_copper` | medium (fails closed) | every glyph counts 4/3 of the font width, plus the stroke. That board is now clean | no |
| A4 | **References KiCad takes for one.** `u2` beside `U2`, and `r1` beside the emitter's `R1`, gave `duplicate_footprints` and `extra_footprint`; `U01` beside `U1` gave `extra_footprint`. A lone `U07` is clean (measured by the builder) | medium (fails closed) | `emit` refuses two references that are equal with ASCII case folded and leading zeros of the trailing number dropped (`DuplicateReference`, id `R1 and r1`). This is done in `pcb`, so the IR and the sheet are unchanged | no |
| A5 | **Metadata KiCad's parity check does not see:** a field the symbol lacks, an empty `Datasheet` removed, a `path` naming no symbol, `exclude_from_pos_files`, a single-pad unconnected net renamed under its own prefix. Each gave nothing beyond `unconnected_items`. None changes connectivity | low | pinned as measured (`kicad_parity_does_not_see_metadata_only_differences`), so it turns red if KiCad starts to see one | **yes, low:** whether the gate should compare fields and paths itself. The scope test suggests backlog |
| A6 | **(builder) A `no_connect` pad moves onto another real net.** KiCad accepts a `no_connect` pad whose net starts with the schematic's net and `_`. With `SPI_MOSI` renamed `I2C_SDA_X`, the MCU's SDA pad moved onto `/I2C_SDA_X` with `(pintype "bidirectional+no_connect")` passed parity (65, unchanged) | high | `parity::board` refuses a `no_connect` pad on any net but an `unconnected-` one (`NoConnectPadOnNet`) | no |
| A7 | **(builder) A rail named `unconnected-V` leaves a pin.** U4's supply pad moved to `unconnected-V-cut`, off the rail, passed parity (65 → 64): the rail's net is a prefix KiCad matches | high | `emit` refuses a net or rail starting `unconnected-` (`ReservedName`), and `parity::schematic` refuses a label or value starting with it (`ReservedNetName`) | no |

**What it tried and the code withstood** (kept as green tests):
- **Every other board mismatch fails with KiCad's finding:**
  - values swapped, or a field altered;
  - a library id outside the library, or of another vendored footprint;
  - pads swapped, resized, renumbered or made a bare hole;
  - an unconnected pin's pad deleted;
  - a reference changed or lower-cased;
  - a footprint missing, extra or duplicated;
  - `dnp` or `exclude_from_bom` set on the board alone.

  The gate's footprint tables make KiCad's library check catch every change to a footprint's geometry.
- **On the schematic side:**
  - `on_board no` and an empty schematic both fail;
  - a `Netclass` field on a label changes nothing, so no rule enters by the schematic;
  - schematics KiCad cannot load (bare, a future version, no version, `(Sheet …)`, `("sheet" …)`) are no verdict (exit 5, `ToolError`), never a pass.
- **The schematic copy:** the board's own file, a directory and a FIFO are refused before any tool runs, and nothing blocks.
- **Determinism:** the board is identical across `env -i`, `LC_ALL=tr_TR.UTF-8`, `TZ`, working directory, out path, a 1 MiB stack, and reversed JSON keys with other whitespace.
- **Placement:** 80 flashes with short references gave no overlap and nothing beyond unconnected items, and nothing panicked.

**Inferred, not measured** (the reviewer's own list):
- A footprint read twice: the files are `include_bytes!` and hash-checked.
- That net classes come only from the gate's `.kicad_pro` (`rules.rs`).
- That a hidden sheet could reach little: the sandbox lets KiCad read only the scratch copy.
- Release-build determinism.

**Mutation (diff-sized):** one mutant per guard this work order adds, each against its suite, run in a `systemd-run --user --scope -p MemoryMax=32G` cgroup:
- **The build's 22:** 21 fail their suite, covering the stacked-pin, padless-pin, graphic, courtyard and text-angle refusals, the gap, the overlap check, the margin, the decoupling anchor, ICs first, escaping, the label prefix, silkscreen text in the keep-out box, re-keyed UUIDs, the exposed pad's absent net, and the CLI's sheet, schematic, parity-flag and verdict paths.
- **The survivor:** the duplicate-pin-number guard, because no vendored symbol repeats a number. It is labelled `belt-and-suspenders` at the site.
- **The adversarial fixes' 11:** all fail their suite: `board_only`, the `no_connect` pad, the shared unconnected net, the reserved net name, the sheet, not-a-schematic, the glyph width, the conflated references, the reserved emitter name, and the CLI's two parity scans.
- **After the reduction:** the guards it touched were re-run and still fail (decoupling anchor, ICs first, conflated references).

## Gates

On Furnace at the PR's code head (the commit after `42a8402`, which records the board size past libtest's capture):
- **Workspace gates:** pass. `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --locked -- -D warnings`, with and without `--features aethelgard-cli/with-tools`; `cargo test --workspace --locked`.
- **The `tools` job's steps in the pinned image**, run locally under rootless podman (`--network none`, `--pids-limit=-1`), at `1bc7b6b`: `cargo fmt`, `cargo clippy --features aethelgard-cli/with-tools` and `cargo test --workspace --features aethelgard-cli/with-tools` all pass, 39 suites, exit 0. Its `AET-WO-1301` lines:
  - `sentinel-01` and the S5-names board: 65 `unconnected_items` each, beyond them `[]`;
  - the netlist oracle: 188 pins, 188 netted pads;
  - every mismatch row failing with its finding, and every adversarial refusal at exit 3.

  This is a pre-check; CI is the record.
- **CI:** the first push's run is cited in the PR and below.
- **Reduction pass (diff):** −13 lines net · 1 duplicate removed (`write` re-finding each placement by reference, where `place` already returns them by index) · 1 dead path removed (`passive_for`, a second constructor that existed only to set `decouples`) · 1 protected kept (the duplicate-pin-number guard, labelled `belt-and-suspenders`) · 2 findings:
  - the test helpers `Dir`, `json_line`, `record` and the `drc` runners are copied across test binaries, kept because each binary compiles alone (as in 1300);
  - `parity`'s tree walk and `board.rs`'s are two walkers of one shape, but `board.rs` is outside this work order's fence, so they are reported, not merged.
