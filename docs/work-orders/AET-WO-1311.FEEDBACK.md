# AET-WO-1311 — FEEDBACK (build)

**Branch:** `wo/aet-1311-eda-hygiene` from `origin/main` `4e821ed` · **Work order:** `docs/work-orders/AET-WO-1311.md` on `main`, with roadmap §7.1's rows owned by `1311` (the Grok review's row included) · **Builder:** Claude Code (Opus 5.5) as `forge` on Furnace · **Cadence:** build only; targeted adversarial pass.

Each answer below is labelled **measured**, **read** (at source), or **inferred**. The predictions in §1 were committed before any measured run, and they are not edited afterwards.

## 0. Readings taken before the predictions

- **R1 · Two of the rows are already closed on `main`** [read]. `3300` F21: `cli/src/fab.rs` `LAYERS` carries each layer's description as its fourth column and `describe` reads it (folded by `AET-WO-3303`, `6f754a3`; at `fd9731f` `describe` had its own `match`). Review 2 A16's `circle` claim: `route::part` refuses a `circle` pad whose size is not square (`route.rs:4328`), with the test `a_circle_pad_that_is_not_square_is_refused` (also `6f754a3`).
- **R2 · A16's `via1`/`track_x` claim is closed in `check` and open in `keepouts`** [read]. `check` picks the tracks and vias for the edge rule by position (`route.rs:3731-3735`), with the test `a_part_named_like_copper_is_judged_as_a_pad`. `keepouts` (`route.rs:3678-3683`, added by `4300d2`) still classifies an item by its text: a pad of a part named `via1` reads as a via, `track_x`'s as a track.
- **R3 · The strict reader's paths** [read]. Every `Node` carries its full path as a `String`, built in `Node::lists` for every child it yields (`strict.rs:889-905`); `check` pushes every child of a node before popping one. The only bounds are `sexpr::MAX_DEPTH` 256, `MAX_TOKENS` 1 000 000 and the CLI's 16 MiB input cap. A list head has no length limit.
- **R4 · `route::read` has no item cap** [read]; `connectivity` compares each segment with every item (`route.rs:3929`), and `check` every pair of items (`route.rs:3713`): both O(N²) in the items read. `Grid::new` caps nodes at `MAX_NODES` 4 000 000.
- **R5 · Layer names** [read]. `route::declared` judges the layer table's copper rows; nothing judges an item's `layer` or `layers` against the table. `route::part` keeps a pad's layers that `strict::copper_pad_layer` accepts and skips a pad with none and no drill (`route.rs:4290-4300`), so a pad on a name outside that set is not read as copper. The vendored footprints use `F.Cu`, `F.Mask`, `F.Paste`, `F.SilkS`, `F.CrtYd`, `F.Fab`, `Cmts.User` and, on pad `layers` only, `*.Cu` and `*.Mask`. The emitter's table (`pcb.rs` `LAYERS`) gives seven rows a user name (`F.Silkscreen`, `F.Courtyard`, …).
- **R6 · The drill check** [read]: `eda/src/fab.rs` `holes` counts each tool's hits; `Drill` holds no positions.
- **R7 · The straps** [read]: `schematic.rs:733-775` maps BME280 0x76/0x77 and TMP117 0x48–0x4B to the strap's connection, and keeps an `allowed` text per part; `parts.json` lists the same addresses in the same order. `validate` already refuses an address the part's row does not offer (`I2cAddressNotOffered`, `ir/src/lib.rs:2657`).
- **R8 · The blocking pair** [read]: `Router::connect` skips a soft path whose only blockers are fixed (`route.rs:1335-1339`, `contest` leaves fixed nets out), then ends with `Unroutable(what)`, which names the net and pad but not the pair.
- **R9 · The KiCad pin** [read]: `check drc` and `check erc` record the version `kicad-cli version` prints and compare the report's own field to it (`kicad.rs:198`); nothing compares it to 10.0.7. `fab` refuses another version only through the Gerber header lines it strips (`cli/src/fab.rs:174-176`).

## 1. Predictions (committed before any measured run)

| # | Prediction | Basis |
|---|---|---|
| P1 | **No emitted byte changes on any board**, since every row changes a reader, a judge, a refusal's text or the emitter's source of a fact it already agrees with. | inferred |
| P2 | **Every fixture's board names only layers in its own table**, canonical names only, plus `*.Cu`/`*.Mask` on pad `layers`: refusing every other name refuses no board that passes today. | inferred (R5) |
| P3 | **No list head in any held KiCad file is longer than 32 bytes** (KiCad's longest keywords are about 22); a 64-byte head cap refuses nothing real. | inferred |
| P4 | **The largest fixture board holds under 3 000 copper items**, and `connectivity` plus `check` on it take well under a second; at a cap of 50 000 items they take minutes, quadratically. A cap near 20 000 keeps both under a minute. | inferred (R4) |
| P5 | **KiCad 10.0.7 writes every hit at whole micrometres**, three decimals at most, so each predicted position compares exactly. | inferred (R6's fixtures: `X105.875Y-109.875`) |
| P6 | **The `keepouts` misreading reproduces:** a part named `via1` whose pad sits in a keepout that forbids pads but not vias passes `check`; renamed `R9`, it is refused. | inferred (R2) |
| P7 | **The blocking pair reproduces on a small board** with a pair beside a net that must cross it on B.Cu, and the refusal names that net and its pad, never the pair. | inferred (R8; `1308` A4) |
| P8 | **`kicad-cli version` prints exactly `10.0.7`** in the tools image, so the pin changes no tier-2 outcome. | inferred (`with_tools.rs` `assert_judged`) |
| P9 | **The seeded neck agrees on both sides:** a ground zone whose sure core keeps a one-node neck is joined by KiCad's refill, and one step narrower both `connectivity` and KiCad's DRC refuse. | inferred (Grok response row 3: `zone_core` keeps clearance + min_thickness/2 + ε) |
| P10 | **Neither table and the emitter disagree on an address** (the §7 stop does not fire). | inferred (R7) |
