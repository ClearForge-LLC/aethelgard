# AET-WO-1302 — FEEDBACK

**Branch:** `wo/aet-1302-routing` · **Base:** `6e34432` (PR #54), with `origin/main` merged by merge commits (`28d846e` for `ec26c5f`, AET-WO-1600; `b0b778a` for `89a3514`, which also bumps `container/IMAGE`) · **Builder:** Claude Code (Opus 5.5) as `forge` on Furnace · **Cadence:** build-only, one measurement stop. **Result:** §1 built. **P1 gate clause 1 holds** in the `tools` test, locally in the pinned image and on CI (run 42460567224, green on the first push): from `examples/sentinel-01-minimal.json`, `emit`, then `check erc` exit 0, then `check drc --spec --schematic` exit 0 with **no finding of any kind**. So does the board of 1300's S5 names. No §7 stop: one 1301 lever was used, `GAP_NM`, between ICs only (B1).

Entry schema: `finding · where · type · recommendation · decision-needed`.

## §3.2 numbers

From the `tools` test `p1_gate_clause_1_the_reference_board_passes_erc_and_drc_with_no_finding`, which prints each line to the job's stderr as `AET-WO-1302 …`. The rip-ups come from routing the board's own pads again, which lays the same copper.

| Measure | `sentinel-01` | S5 names | Where it comes from |
|---|---|---|---|
| ERC / DRC exit | 0 / 0 | 0 / 0 | `check erc`; `check drc --spec --schematic`, `"rules":"specification"`, verdict `pass` |
| DRC findings | **0** | **0** | every kind, `unconnected_items` and `schematic_parity` included |
| Routing time | **1.0 s** release; 6.8 s debug (13.2 s on CI), emit included | 0.21 s release; 1.4 s debug (1.8 s on CI) | Furnace, `aethelgard emit` (release); the debug figures are the `tools` test's own, in the image and on CI |
| Total track length | **348.5 mm** in 360 segments | 332.9 mm in 332 segments | `route::read` of the board as written |
| Via count | **70** | 68 | 0.6 mm, drilled 0.3 mm (the gate's Default net class) |
| Rip-ups | **37** | 8 | nets taken up and routed again; the bound is 500 (`route::RIPUPS`) |
| Nets that took zones | **GND only, on B.Cu** | GND only | `VDD_3V3` is tracked; no F.Cu zone was needed |
| Grid pitch | **0.125 mm** | 0.125 mm | `route::PITCH_NM`; it divides the LQFP's 0.5 mm pitch, so every LQFP pad centre is a node |
| Board size | 45.2 × 40.4 mm | 45.2 × 40.2 mm | the outline; 44.7 × 39.9 mm before B1 |

The 24-device boards that 1301's adversarial suites emit route too, and pass the same gate with no finding (measured on Furnace in the pinned image):

| Board | Release time | Debug time | Segments | Vias | Gate (pinned image) |
|---|---|---|---|---|---|
| 24 I2C devices, short references | 4.0 s | — | 1172 | 221 | ERC 0, DRC 0, no finding |
| 24 I2C devices, 64-byte references (200 × 122 mm) | 38 s | — | 1127 | 258 | ERC 0, DRC 0, no finding |
| 2 buses × 2 devices (adversarial D12c) | — | — | 514 | 97 | ERC 0, DRC 0, no finding |
| 2 buses × 5 devices (adversarial D14) | — | — | 757 | 132 | ERC 0, DRC 0, no finding |

## Build

**What was built (§1).**
- **`aethelgard-eda`'s `route` module, the router.** Integer-only, a function of the placed pads alone:
  - **Grid:** 0.125 mm (`PITCH_NM`), which divides 0.5 mm, so every LQFP pad centre is a node. Moves are axis-aligned, so every track is a horizontal or vertical segment, and every clearance is an exact squared distance between boxes.
  - **Search:** A* over (layer, node, entering direction) on F.Cu and B.Cu. Each step costs 2 on F.Cu and 6 on B.Cu; a bend costs 1 and a via 24. Ties go to the lower state index; no hash, address, clock or thread enters a decision. `HashMap` is not used.
  - **Spacing:** a node is open to a net only if a 0.2 mm track centred there keeps 0.2 mm (`FLOOR_NM`) from all foreign copper, pads with no net included (U3's pad 7), and 0.5 mm from the edge. A via site keeps the same on both layers, and 0.25 mm hole to hole. Each reach carries a 10 µm margin; a compile-time assertion shows it covers the chord between two open nodes (`8·R·EPS ≥ PITCH²`).
  - **Escape:** on F.Cu no track crosses a footprint's pad field (the box round its pads) except inside its own pad. So every pad is left straight out across the field's edge, and at the LQFP that is the only way out.
  - **Order:** ground's stitching first (B2), then every net with two or more pads, `unconnected-…` aside, by pad count ascending, then name (B3). A net grows as a tree from its first pad (by reference, then pad number), joining the nearest remaining pad next.
  - **Rip-up and retry:** when a connection finds no free path, a soft search that may cross other nets' copper, at a price, names the nets in the way. They and the stuck net are taken up, the stuck net is routed again first, and every contested node's shadow costs every later search more. **The bound is 500 nets taken up in all** (`RIPUPS`). Past it, or when even the soft search finds nothing, `emit` refuses with `Unroutable`, naming the net.
  - **Its own judges:** before it returns, `check` judges every pair of copper items exactly (no grid, no margin), and `connectivity` judges every net. `read` reads a written board back for the same judges.
- **Zones (§1.2).** GND takes one zone on B.Cu over the whole outline: unfilled, with no `connect_pads` token and no fill (`--refill-zones` fills it). Every pad is on F.Cu, so the zone touches no pad; it joins the ground vias. Each ground pad is taken by F.Cu to a via in the largest piece of the zone as the B.Cu copper then leaves it, or to ground copper that already has one. A zone left in pieces holding ground vias takes up the nets whose B.Cu copper fences it, and ground is stitched again after them. **`VDD_3V3` is tracked; no F.Cu zone was needed.**
- **`emit`** writes the routed board in place of the placed one (`pcb::write`). The copper's UUIDs come from content (net, layer, coordinates). Two emissions are byte-identical (tier 1, and the adversarial pass).
- **Placement:** one 1301 lever, `GAP_NM`, between ICs only (B1).

**Where §1.4 lives.**
- **Tier 1:**
  - `crates/aethelgard-cli/tests/routing_tier1.rs`, all on the board read back from the file `emit` wrote:
    - byte-identical emission across two processes in different environments, and the router run again on the board's own pads lays exactly its copper;
    - the router's clearance check over every pair of copper items;
    - no copper within the clearance of any `unconnected-` pad;
    - every net's copper connected by the router's own connectivity, ground through the zone, and a track taken away is caught;
    - every pad left straight out of its field;
    - the adversarial pass's orphan-copper and relabelled-copper rows (A1).
  - `route.rs` unit tests: crossing nets routed clear and joined; the clearance check exact to 1 nm; the edge, hole and axis rules; the zone and both layers of a via; what `read` and `part_of` refuse; a board past the grid cap.
- **Tier 2,** `with_tools.rs`: `p1_gate_clause_1_the_reference_board_passes_erc_and_drc_with_no_finding`. It runs `emit`, then `check erc` (exit 0, `pass`, no finding), then `check drc --spec --schematic` (exit 0, `pass`, no finding of any kind), on `sentinel-01` and the S5-names board, and records the §3.2 numbers.
- **Every existing test passes,** 1300's and 1301's adversarial suites included. Four 1301 tests that encoded an unrouted board were changed to follow it (B7).

| # | Finding | Where | Type | Recommendation | Decision-needed |
|---|---|---|---|---|---|
| B1 | **The placement lever: `GAP_NM` between ICs, 1 mm (`IC_GAP_NM`); 0.5 mm elsewhere, as before.** Measured: at 0.5 mm, U1's pad 34 (`/FLASH_CS`, y 107.75) and U4's pad 7 (`/U4_HOLD`, y 107.765) face each other across a 1.15 mm channel that holds one track column. Whichever escapes first walls in the other, and the two took each other up 200 times. With 1 mm between every pair of parts, the board routes, but 1301's `a_decoupling_capacitor_goes_beside_its_ics_power_pin` fails its 4 mm "beside" bound. With 1 mm between ICs only, every board routes and every 1301 placement test passes unchanged. 1.25 and 1.5 mm between all parts also routed the four boards measured, and only widen the gap the decoupling test bounds. The board grows from 44.7 × 39.9 to 45.2 × 40.4 mm | `pcb::place`, `spot` | lever used (§7 allows) | none | no |
| B2 | **Ground is stitched before the ordered nets.** §1.1 orders "nets" by pad count, then name. Ground is not tracked between pads: its stubs and vias are the zone's stitching (§1.2), and they are laid first so that tracks go round them. Routed in pad-count order (23 of 29), the stubs found their via sites walled in by `VDD_3V3` and the I2C nets, and the two took each other up until the bound | `route::route` | builder's choice | none, unless the architect reads §1.1 as binding ground too | no |
| B3 | "By pad count" is read ascending: two-pad nets first, `VDD_3V3` last. Ascending leaves the largest tree the most freedom to hop to B.Cu round what is already laid | `route::route` | builder's choice | none | no |
| B4 | **Escape needs more than "straight out" when pads sit side by side.** On the 24-device boards, a ground pad between two signal pads (U1's 136–138) put its via in the shallowest legal spot, and that via walled in its neighbour. **Soft corridors fix it:** a node straight out from a pad not yet routed costs more the nearer it is, for a track, or for a via whose copper would reach it. So a net that must cross a cluster crosses it far out. <br>The build also had a hard keep-out: within 1 mm of a field, no other net's via in a pad's corridor. Mutation testing showed it survived on the 24-device suite. The later fixes, zone-aware stitching and the IC gap, had made it obsolete. Measured over 20 specifications, without it: <br>• every board that routed still routes; <br>• three more route (the adversarial pass's D12, D12c and 2 × 5); <br>• the short-reference board takes 4 s instead of 13 s, and the wide one 38 s instead of 17 s (release). <br>The reduction pass removed it | `route::corridor` | measured | none | no |
| B5 | **The edge clearance is 0.5 mm, not `FLOOR_NM`.** Measured: KiCad 10.0.7 under the gate's project reports `copper_edge_clearance` "board setup constraints edge clearance 0.5000 mm" for a via 0.45 mm from the edge. The router keeps 0.5 mm. That is stricter than the WO's floor and loosens nothing | `route::EDGE_NM` | measured | the gate writes no edge rule of its own; a later rules WO could derive one | yes, low |
| B6 | **The KiCad facts this build rests on,** measured in the image: <br>• the 20260206 dialect writes `(net "<name>")` on segments, vias and zones; <br>• a B.Cu zone with no `connect_pads` passes `board::prepare` and joins two stitched vias (65 → 63 unconnected, nothing else); <br>• vias are tented by default, so a via under silkscreen raises nothing; <br>• a via joined only to the zone is `via_dangling`, so every via has a track on F.Cu | `pcb::copper` | measured | none | no |
| B7 | **Four 1301 tests encoded an unrouted board; each now follows the routed one and still asserts the secure outcome.** <br>• `placement_tier1::no_courtyards_overlap…` asserted no segment, via or zone. It now requires all three, and still forbids a setup block and a net table. <br>• `with_tools::the_emitted_board_has_no_parity_finding_and_only_unconnected_items` asserted `unconnected > 0`. It is replaced by the clause-1 test, which asserts zero findings of every kind on the same two boards. <br>• `with_tools::parity_fails_a_board_that_does_not_match_its_schematic` and `adversarial_1301_tools::parity_fails_every_other_mismatch_with_kicads_finding` assert the exact finding kinds KiCad gave on the placed board. On the routed board a mismatched pad also shorts the tracks that reach it (`shorting_items`, `solder_mask_bridge`). Both now judge their rows on the placed board, the routed board's copper stripped, and every row reproduces 1301's kinds exactly. Each then also judges the routed board and asserts it fails with the parity finding among its findings | CLI tests | test change | none, unless the architect reads these as a weakening | yes, low |
| B8 | **Fence readings.** <br>• `lib.rs` gains `pub mod route`. <br>• `schematic.rs` gains two `EmitError` variants, `Unroutable` and `BoardUnroutable`. <br>• `pcb.rs` gains `IC_GAP_NM` (B1). <br>• The eda and CLI READMEs gain the routing rules and the refusal. <br>• New test files: `routing_tier1.rs` and `adversarial_1302_tools.rs`. <br>The gate's `rules.rs`, `board.rs` and `parity.rs` diff to empty, and so does every other protected surface | — | fence reading | none, unless the architect reads the fence narrower | no |
| B9 | **Test time.** Routing in a debug build is about 6–7 times slower than release. 1301's adversarial suites route the 24-device boards: `adversarial_1301` and `adversarial_1301_tools` now take about 210 s each. The local run of the `tools` job's test step took 17.5 min, of which `board_tier2` (Kani) was 8.5 min. CI's 1301 run took 15 min for the whole job | `route` | measured | `[profile.dev.package.aethelgard-eda] opt-level = 2` in the workspace `Cargo.toml` (outside this fence) would cut it | yes, low |
| B10 | **Routability is not monotonic in parts** (adversarial A6). At the reviewer's head, 2 buses × 2 and × 5 failed closed while × 3, × 4 and × 6 routed. Since the keep-out's removal (B4) all five route. One 24-device short-reference variant of the reviewer's (`D1`) still fails closed after 500 rip-ups. Every congested case fails closed, naming the net | `route` | measured | a later router WO: pad-pair escape planning, or placement aware of routing | no |

**What `1303` should know** (parasitic extraction reads this geometry):
- **The reader exists:** `route::read(&sexpr::parse(board)?)` gives every copper pad (box and corner radius, net, part), every segment (net, layer, start, end; width `WIDTH_NM`), every via (net, centre; 0.6/0.3 mm) and the zone (net, layer; its outline is the board's), all in integer nanometres. It refuses copper of any other size. `check` and `connectivity` judge it.
- **Use the reader's labels only after its judges.** KiCad does not check a track's `(net …)` label (A2), so a board whose labels lie passes the gate. `route::check` and `connectivity` refuse it.
- **Per-net copper on the reference board** (mm, F.Cu / B.Cu / vias):
  - `/I2C_SDA`: 15.5 / 8.8 / 4;
  - `/I2C_SCL`: 34.8 / 8.8 / 7;
  - `/SPI_MISO`: 20.0 / 3.2 / 2;
  - `/SPI_MOSI`: 3.4 / 7.2 / 2;
  - `/SPI_SCK`: 11.5 / 3.0 / 2;
  - `/FLASH_CS`: 8.1 / 4.0 / 2;
  - `/U4_HOLD`: 3.2 / 5.1 / 2;
  - `/U4_WP`: 2.8 / 1.2 / 2;
  - `/U2_SDO`: 8.9 / 0 / 0;
  - `/U1_VCAP1`: 3.1 / 0 / 0;
  - `/U1_VCAP2`: 2.9 / 0 / 0;
  - `GND`: 32.9 / 0 / 22;
  - `VDD_3V3`: 93.2 / 66.9 / 25.
- **The ground plane is not in the file.** KiCad's fill is not deterministic and is never saved (S4.3). `route`'s zone model, a B.Cu node at least 0.2 mm plus half the 0.25 mm minimum width from foreign copper and 0.5 mm plus that from the edge, is a conservative stand-in, private today. The plane is perforated by every signal's B.Cu run, 66.9 mm of it `VDD_3V3`'s, so return paths are not uniform.
- **The stackup is not in the board.** The board says 1.6 mm thick, two copper layers, and nothing about dielectric or copper weight. Extraction needs a ruled source for both.
- **Tracks are axis-aligned and 0.2 mm wide everywhere.** Lengths are Manhattan sums; there are no arcs and no 45° segments.

## Flag-and-stop check (§7)

None arose:
- **The reference board routes to zero findings,** with one 1301 lever used as the WO allows (B1). No rule was loosened: the router keeps the gate's 0.2 mm clearance and width, and the stricter edge clearance KiCad applies (B5). The congestion data for the levers tried is in B1 and B4.
- **The gate's rules, refusals and parity code** diff to empty: `rules.rs`, `board.rs`, `parity.rs`, `kicad.rs`, `verdict.rs`. So do `container/`, `.github/workflows/`, the steering documents, `README.md`, `LICENSE-*` and `docs/inputs/**`.
- **`unsafe`, floating point in a routing decision, or an image change:** none. The eda crate forbids `unsafe`; no `f32` or `f64` appears in `route.rs` or in `pcb.rs`'s diff. No dependency was added; `Cargo.toml` and `Cargo.lock` are unchanged.
- **A test that can only go green by being skipped, ignored or weakened:** none skipped or ignored. The four 1301 tests in B7 were changed to follow the routed board. Each keeps its secure assertion and gains one, and B7 asks for the architect's reading.
- **`sudo`, a system package or a secret:** none, beyond the push route (S25-minted token, `gh-write`, revoked).

## Adversarial pass (§5, full)

**How it was run.**
- **The reviewer:** a fresh Opus subagent in its own worktree, at `d6c3cbd`, given the code and the work order but not the builder's conclusions.
- **Measured in the image:** KiCad 10.0.7 under `--network none`, with the host-built CLI.
- **The builder's spot-checks:** A1 by reading `connectivity`, then by the regression test, which failed before the fix; A2 and A3 by rerunning the reviewer's boards in the image (a `VDD_3V3` via relabelled `GND`, and a floating `/NOPE` loop: exit 0, `pass`, no finding each); A4 end to end.
- **Kept as regression tests:** A1's rows are in `routing_tier1.rs`; A2 and A3 are pinned as measured in `adversarial_1302_tools.rs` (tier 2), so they turn red if KiCad or the gate starts to see them.
- **Headline:** of the 12 boards the router emitted under attack, none passed the gate wrongly. Every one gave zero findings and was right.

| # | Finding (measured at `d6c3cbd`) | Severity | Fixed by | Decision-needed |
|---|---|---|---|---|
| A1 | **The router's own judges passed orphan copper that KiCad rejects.** `connectivity` proved only that each net's pads share a component. Three boards passed both judges and failed the gate: <br>• a closed F.Cu loop of `/I2C_SDA` touching nothing (`unconnected_items`); <br>• a stub on `unconnected-(U3-ALERT-Pad3)` (`track_dangling`); <br>• a track on `/NOPE`, a net no pad carries (`track_dangling`). <br>The router never emitted such copper; the fail-closed check behind `emit` would not have caught it if it had | medium | `connectivity` now requires every segment and via to belong to a net with two or more pads, `unconnected-` aside, and to be joined to that net's pads | no |
| A2 | **The gate does not judge the net a track is labelled with.** Copper with false labels passed: exit 0, no finding. The rows: <br>• one SDA via, or one VDD via, relabelled `GND`; <br>• all SDA copper relabelled `/NOPE`, then `/I2C_SCL`; <br>• every `VDD_3V3` segment and via relabelled `GND`; <br>• every SDA track relabelled `""`; <br>• one SDA segment relabelled SCL. <br>Inferred, not measured: KiCad takes a track's net from the pads it reaches before it refills zones. The geometry is right and only the labels lie, but `1303` reads the labels. `route::check` and `connectivity` refuse every row | medium | pinned as measured (`kicad_does_not_judge_the_net_a_track_is_labelled_with`) | **yes:** whether the gate runs `route::read` + `check` + `connectivity` before KiCad. That would also refuse every board this router did not write (other widths, `gr_*` copper). The gate code is protected here, so it is a ruling |
| A3 | **A floating closed loop on a net no pad carries passes the gate.** Four F.Cu segments on `/NOPE` (or with no net), touching nothing: no dangling end and no ratsnest, so exit 0 with no finding | low–medium | pinned as measured (`kicad_does_not_see_a_floating_loop_on_a_net_no_pad_carries`). The router's judges refuse it since A1 | **yes,** with A2 |
| A4 | **A refusal of the whole board named no net:** `id ""`, and the message "cannot complete net ``". Measured on 24 I2C devices plus 6 SPI buses of 10 flashes, with 64-byte names: a 285 × 242 mm board, refused in 1.3 s. The post-route judges took the same path | low | `EmitError::BoardUnroutable`: kind `Unroutable`, `id` null, "the router cannot complete the board: …" (measured) | no |
| A5 | Reversing `i2c_devices` emits the same schematic and a different board: U3, C5 and R3 move, and the copper follows. Both boards pass the gate. This is 1301's B6 (placement by device order), which the routed board inherits | low | none | no: `INV-01` holds, the arrays are ordered input |
| A6 | **Routability is data, not a promise** (B10): <br>• 2 buses × 2 devices fails closed, as do 2 × 5 and one of the reviewer's 24-device short-reference variants (`/S0_SDO`, after 500 rip-ups, 24.6 s); <br>• 2 × 3, 2 × 4 and 2 × 6 route and pass the gate. <br>Every congested case fails closed, naming the net | — | none | no |

**What it tried and the code withstood** (each measured):
- **Determinism (§5.1).** These gave byte-identical boards and schematics: minified JSON; shuffled keys; reversed `net_constraints`; `env -i` with cwd `/`; `tr_TR` with TZ Kiritimati; the C locale with a 64 KiB minimum stack; a 128 KB stack limit; an out path with spaces and quotes over 100 characters; debug against release; a repeat run. The 24-device board was identical across release, `env -i` and debug.
- **Wrong copper the gate caught (§5.2),** with KiCad's kinds:
  - netless or `""` track bridging SDA and SCL: `clearance`, `solder_mask_bridge`;
  - GND track to U3's netless exposed pad: `shorting_items`;
  - track from an `unconnected-` pad to GND: `shorting_items`, `tracks_crossing`;
  - segment dropped: `track_dangling`, `unconnected_items`;
  - dangling via: `via_dangling`;
  - netless F.Cu zone: `isolated_copper`;
  - VDD or GND zone on F.Cu: `starved_thermal`;
  - an SDA zone of higher priority on B.Cu: 22 `unconnected_items`;
  - `gr_poly`, `gr_line` or copper text bridging pads: `shorting_items`, `clearance`;
  - a prefilled zone: `zones_intersect`;
  - the ground zone shrunk, at a 3 mm minimum width, on VDD, or removed: `via_dangling`, `unconnected_items`.
- **Names.** These reached KiCad, and ERC and DRC passed on each: `Q"1`, `Q\2`, `(SCK)`, `~MOSI`, `M^ISO`, `C'S#%&*`, and a domain `V"D(D)`.
- **Hang and memory (§5.3).** Every case ended:
  - the worst, near the grid cap, failed closed in 112 s at 531 MB after 500 rip-ups;
  - a 249 mm square with 40 crossing nets routed in 67.6 s at 507 MB;
  - a pad past the edge was refused by name in 276 µs.

**Inferred, not measured** (the reviewer's list): there is no wall-clock bound, only the rip-up bound. The worst case is about `RIPUPS` × 2 whole-board searches, around 30 minutes, never infinite. A node-expansion budget per emit would bound it (backlog, by the scope test).

## Gates

On Furnace at the code head `b0b778a` (every later commit changes only this file):
- **Workspace gates:** pass.
  - `cargo fmt --all -- --check`;
  - `cargo clippy --workspace --all-targets -- -D warnings`, with and without `--features aethelgard-cli/with-tools`;
  - `cargo test --workspace --locked`: 42 suites, all `ok`, 378 s.
- **The `tools` job's steps** in the pinned image (`container/IMAGE`, `2df68259…`, as bumped by #59; KiCad 10.0.7), run locally under rootless podman (`--network none`, `--pids-limit=-1`):
  - `cargo fmt`, `cargo clippy --features aethelgard-cli/with-tools` and `cargo test --workspace --features aethelgard-cli/with-tools` all pass: 42 suites, exit 0, 1163 s for the test step;
  - its `AET-WO-1302` lines are the §3.2 numbers above: on `sentinel-01` and S5 names, ERC exit 0 and DRC exit 0 with 0 findings;
  - A2 and A3 are pinned at exit 0, with no finding;
  - 1301's short and wide references give exit 0 with nothing beyond `unconnected_items`, which is in fact nothing.

  This is a pre-check; CI is the record.
- **CI:** [run 42460567224](https://github.com/ClearForge-LLC/aethelgard/actions/runs/42460567224), commit `e715cd9`, the first push (PR #60):
  - `gates` success, in 12.8 min;
  - `tools` success, in 28.6 min, all 42 suites `ok`;
  - `image` success, in run 42460567229.

  The `tools` log carries the clause-1 lines: `sentinel-01` and S5 names each give ERC exit 0 and DRC exit 0 with 0 findings. Its emit times, in a debug build with routing included, are 13.2 s and 1.8 s. A2 and A3 are pinned at exit 0. §1.4 passes in `gates` and `tools` (§3.1), and CI is green on the first push (§3.3). Every commit after `e715cd9` changes only this file.
- **Mutation (diff-sized):** one mutant per guard this work order adds, each against its suite, in a `systemd-run --user --scope -p MemoryMax=32G` cgroup.
  - **The first sweep, 27 mutants,** against the eda unit tests and `routing_tier1` and `placement_tier1`. 20 fail their suite:
    - the track and via reaches;
    - the track and via edge;
    - the pad field;
    - vias near a field;
    - `check`'s clearance, edge, axis and hole gap;
    - `connectivity`'s both-layers and pads-joined rules;
    - `read`'s width and via size;
    - `part_of`'s rotation and front-SMD refusals;
    - the grid cap;
    - zone-split detection;
    - the IC gap.

    One did not apply: its text had been reformatted.
  - **The follow-up, after A1's fix and the new tests:**
    - **Fail their suite:** `connectivity`'s zone net and layer, orphan copper, and copper on an unrouted net. The zone-reach rule fails its suite once the test asserts its message.
    - **Fail only the 24-device suite** (`adversarial_1301`), where these guards are used: the grid's hole-gap marking, and ground vias kept to the zone's main piece.
  - **Survivors,** each accounted for:
    - **Two labelled `belt-and-suspenders`:** the final `check` and `connectivity` before `route` returns, and the `unconnected-` filter in the net order. Each has a primary guard ahead of it.
    - **Hop flattening:** the primary handling of a path with two of its own vias closer than the hole gap. It was measured once, at 2000 rip-ups, and no tested board reaches it now. The final `check` stands behind it.
    - **The via escape keep-out:** it survived because it was no longer needed, so the reduction pass removed it (B4).
- **Reduction pass (diff):** −113 lines net · 7 duplicates removed · 1 obsolete mechanism removed · 2 protected kept · 3 findings below.
  - **The duplicates:**
    - the router's own copies of `Rect`'s point, union, grow and centre, now `pcb::Rect`'s, `pub(crate)`;
    - the two blocker tails, now one `contest`;
    - `release` and `unrelease`, now one;
    - the largest-piece rule, now `main_label`;
    - the edge distance, now `to_edge`;
    - the nodes' span, now `Grid::span`;
    - `connectivity`'s routed-net rule, now one map. Its zone-layer closure and second `first_seg` went too.
  - **The obsolete mechanism:** the via escape keep-out (B4).
  - **The proof:** for the duplicates, the four boards came out byte-identical before and after. For the keep-out, 20 specifications were routed again and six boards gated.
  - **The findings, not cut:**
    - `with_tools.rs` now holds three one-line stderr recorders (`record`, `record_1301`, `record_1302`). Merging them is taste.
    - `unrouted` and the relabelling scan appear in two test binaries each, which compile alone, as in 1300 and 1301.
    - `Routed::ripups` and `length_nm` have no production reader. They exist for §3.2's numbers.
