# AET-WO-4100 — Stackups: a specification selects 2, 4 or 6 copper layers, and every gate follows it

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-4100-stackups`.
**Author:** Claude (architect) · **Builder:** strong tier, with a full adversarial pass (`docs/roadmap.md` §6).
**Phase:** P4, its second work order (Scotty's order, 2026-11-12: `2301`, then `4200`, then `4100`). **Cadence: spike (§1), then STOP for the architect's ruling, then build (§2).** The spike runs while `4200` builds. The build merges whatever `4200` has landed.

**Grounds, each read on `main` at `9035203`:**
- **The specification declares no board.** `Specification` (`crates/aethelgard-ir/src/lib.rs`) has no layer count, no stackup and no rules field. `rules::Rules::from_specification` ignores its argument and returns the engine floor, 0.2 mm (`FLOOR_NM`). The specification rules field is this work order's (roadmap §7.1, 1300 B1 and review A5).
- **Two copper layers are code in the eda crate:**
  - `pcb.rs`: `LAYERS` is KiCad 10's two-layer table, and `(general (thickness 1.6))` is written as a literal.
  - `route.rs`: `Layer` is `{F, B}`, layer sets are `[bool; 2]`, and a via must name exactly `F.Cu` and `B.Cu`.
  - `fab.rs`: `FAB-NOTES.md` states "Two copper layers".
  - `cli/src/fab.rs`: the Gerber list names `F_Cu.gtl` and `B_Cu.gbl`.
- **Extraction assumes two layers** (`extract.rs`, `AET-WO-1305`). The trial `ψ = max(0, 1 − d/S)` uses one distance, `S` = the 0.2 mm clearance, and is valid only because every foreign conductor lies at least `S` away. Across the board, that is asserted as `BOARD_THICKNESS_NM − 2·COPPER_NM > CLEARANCE_NM`. An inner plane on a 4-layer board typically lies about 0.1 to 0.2 mm under an outer track, so the assertion fails: the bound as written is not a bound there. `read` also refuses any board that does not declare 1.6 mm.
- **Routability.** 1302 B10: congested cases fail closed. The 24-device short-reference variant (`D1`) still fails after 500 rip-ups. Escape planning is this work order's (roadmap §7.1).

> **What this is:**
> - a specification that declares its layer count (2, 4 or 6), its stackup and its design rules;
> - the router, the gate-written DRC files (`RUL-03`), extraction's bound and the fabrication package all follow that declaration;
> - Sentinel-01 regenerates byte-identical at 2 layers.
>
> It is NOT:
> - blind, buried or micro vias, via-in-pad, or more than 6 layers;
> - controlled impedance or differential pairs (USB is `4300`'s; this work order only makes the stackup available to it);
> - full parasitic extraction (`AET-WO-1303`, before `4500`);
> - Sentinel-02 (`4500`). Sentinel-01 stays 2 layers.

## 1. Spike

Probes live under `spike/4100/`, in any language Furnace or the pinned image runs (roadmap §6; the build deletes `spike/`). **Predictions go into FEEDBACK before any run.** Every KiCad step runs `kicad-cli` 10.0.7 in the pinned engine image (`architecture.md` §2.12).

**S1. What KiCad 10.0.7 writes and reads at 4 and 6 layers.** Measure:
- the layer table `kicad-cli pcb upgrade` writes for a 4- and a 6-layer board, including the inner layer ids and names;
- the `(setup (stackup …))` block: what KiCad requires, what it ignores, and whether DRC or the Gerber export reads it;
- DRC with `--refill-zones` on a board with inner-layer zones: a filled plane on `In1.Cu`, a clearance violation seeded on an inner layer, and a through via crossing a plane of another net. Does KiCad report each one?
- the Gerber export of inner layers: file names, the `%TF.FileFunction` attribute, and whether the two clock lines `RUL-03` (amended 2026-11-13) deletes appear at the same places. A new place breaks that rule, which is a stop condition.

**S2. The specification's shape.** Propose a `board` section without building it. It covers:
- copper layers (2, 4 or 6);
- each copper layer's role: `signal`, or a plane carrying one named net. A plane's net must be a declared domain or ground;
- the stackup, given as bounds the gates can use: finished thickness range, each dielectric's minimum thickness, the copper weight's maximum, and the relative permittivity's maximum, each with its source;
- the design rules (clearance, track width, via drill and pad), with a refusal below the engine floor.

Constraints:
- **`INV-06`:** every key is required, and nothing defaults.
- **`RUL-14`:** say whether this is `schema_version` 2 and what happens to a version-1 file. Refusing it by name is the default; list every fixture and example that migrates.
- **The home of the stackup.** Say whether the stackup is declared in the specification, or named from a table of fabricator stackups (`4200`'s parts table is the precedent). If a fabricator's published stackup is cited, record whether Furnace reaches the page. The ST PDFs needed Scotty, and so may this.

**S3. The router at N layers.** Read `route.rs` and propose the smallest change that routes:
- **4 layers** as signal, plane, plane, signal: two routing layers, through vias, and the planes as zones;
- **6 layers** with two inner signal layers.

Measure on the `D1` variant and the 2 × 5 case (1302 B10):
- whether 4 layers (planes freeing the outer layers of ground and supply) or 6 (inner signal layers) route what 2 cannot;
- whether escape planning is still needed.

Name every `[bool; 2]`, `Layer` match and two-layer literal, with what it becomes. Say what `route::check` and `connectivity` must learn: a plane connects through vias, and a via joined on every layer it crosses.

**S4. Extraction's bound at a stackup.** Derive, on paper and then checked numerically, a valid trial for an outer-layer piece whose nearest foreign conductor below is a plane at a distance `h` smaller than `S`. One candidate is an anisotropic trial: lateral scale `S`, vertical scale `h`.

State the bound's formula and its conditions. Compute it for:
- the reference at 2 layers. It must be unchanged, byte for byte, in `extract`'s output;
- one I2C line on a 4-layer fixture at the S2 stackup.

Say whether the reference's declared capacitance bounds would still hold on 4 layers, and by how much. A bound that exceeds a declared value is a real finding about 4-layer I2C, not a test to loosen. Extraction refuses an inner-layer track until `1303`: say whether that is enough for 6 layers.

**S5. The fabrication package at N layers.** Measure, by running `verify --fab-out` against a hand-built or probe-generated 4-layer board where the build is not yet there:
- the Gerber set per layer count;
- `FAB-NOTES.md` stating the declared stackup;
- `MANIFEST.json`;
- `--against-fab`.

Say what a fabricator needs that the package lacks at 4 layers, for example the layer order and the stackup table. Say whether KiCad's Gerber job file (excluded today) should now be included, through `RUL-03`'s amended rule.

**S6. Scope check.** List every place outside `aethelgard-eda` and the CLI's `generate`, `check`, `extract` and `verify` paths that reads a board's layer count or thickness: the twin, the analog deck, the campaign, `evolve`. Say whether anything there changes. Nothing should; say so with the grep.

**Stop** with the table, a recommendation per question, and the decisions needed.

## 2. Build, after the ruling

Expected, to be confirmed by the ruling:
1. **The specification's `board` section,** validated and refused by name when malformed (`INV-06`). Every fixture migrates. `Rules::from_specification` reads the declared rules.
2. **The router, the board writer and the gate-written DRC files** at 2, 4 and 6 layers. Planes are emitted unfilled and refilled by the gate, as zones are today (`RUL-03`).
3. **Extraction's bound at the declared stackup,** with the S4 derivation in the module's documentation and a test that checks it against an exact case.
4. **The fabrication package at N layers.** `FAB-NOTES.md` states the declared stackup, and `--against-fab` holds.
5. **Sentinel-01 regenerates byte-identical** at 2 layers. Every committed generated file is unchanged, or each change is named in FEEDBACK with its cause.
6. **A 4-layer test fixture** that routes, passes DRC, extracts and emits a package in CI. If the ruling takes 6 layers, a 6-layer fixture too.
7. **1302 B10:** the `D1` variant routes at the layer count the ruling names, or escape planning is built if the ruling asks for it.

## 3. Invariants and fences

- **Invariants and rulings:** `INV-01`, `INV-03`, `INV-06`, `INV-07`; `RUL-03`, `RUL-12`, `RUL-14`, `RUL-16`.
- **Fence (at the build, refined by the ruling):**
  - `aethelgard-ir` (the `board` section and its validation);
  - `aethelgard-eda` (`pcb`, `route`, `rules`, `extract`, `fab`, `strict` where it must read inner layers);
  - the CLI's `generate`, `check`, `extract` and `verify`;
  - fixtures, examples and tests;
  - `ci.yml` (`TIER2_MIN`) and `ci/test-weights.tsv`;
  - this work order's FEEDBACK.
- **`4200` is building beside this work order.** It changes `aethelgard-ir` (the parts table, SPI) and the reference's generated firmware. At the build, merge `origin/main` with a merge commit whenever it lands. A `schema_version` bump is shared with any fixture `4200` adds.
- One PR, left unmerged; retitle it when the spike becomes the build. **Push and revoke by the FUR-WO-0301 F7 route.**

## 4. Acceptance

- **The spike table, ruled.**
- **Three specifications pass `verify --fab-out`:** Sentinel-01 at 2 layers, byte-identical to `main`; a 4-layer fixture; and a 6-layer fixture if ruled.
- **Extraction's bound** at each declared stackup is derived in the module's documentation, checked against an exact case, and unchanged at 2 layers.
- **A malformed `board` section is refused by name,** for each field.
- **The CLAUDE.md gates pass.** CI is green with every shard under 20 minutes (`AET-WO-1908` §8).

## 5. Adversarial pass, full

1. **Make a board lie about its stackup:**
   - a plane on the wrong net;
   - a via that skips a layer;
   - an inner-layer track where the specification declares a plane;
   - a stackup the board's `(setup …)` disagrees with.

   The gate must judge the board by the specification and by copper, never by the board's own claims (`RUL-03`).
2. **Make extraction under-bound:** a plane closer than declared, a thinner dielectric, a higher permittivity. Each must either raise the bound or be refused.
3. **Make the specification lie:** a layer count with the wrong number of roles; rules under the floor; a version-1 file; a plane on an undeclared net. Each must be refused by name.
4. **Make the package lie:** a missing inner Gerber; a `FAB-NOTES.md` stackup that disagrees with the specification. `--against-fab` must name it.

## 7. Flag-and-stop conditions

- KiCad 10.0.7 writes a nondeterministic line in an inner-layer Gerber at a place `RUL-03`'s amended rule does not name.
- No trial the builder can derive bounds an outer piece over a near plane. Extraction would then refuse 4-layer boards, and `verify --fab-out` could not pass one. That is a ruling.
- Sentinel-01's generated files change at 2 layers for a reason this work order does not name.
- KiCad's DRC does not report a seeded inner-layer violation (S1). The gate would then need its own inner-layer check, which is a ruling.
