# AET-WO-1302 — Routing: the emitted board passes DRC with parity and nothing left unconnected

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main` after `AET-WO-1301` (3b1f90f), named in the kickoff.
**Branch:** `wo/aet-1302-routing`.
**Author:** Claude (architect) · **Builder:** a **strong** builder with a full adversarial pass (`docs/roadmap.md` §6). A fresh session on Furnace.
**Phase:** P1 · **Gate clause served:** **clause 1, completed.** From the checked-in reference specification:
- one command emits a schematic and a placed and routed board;
- `kicad-cli sch erc --exit-code-violations` and `kicad-cli pcb drc --exit-code-violations --schematic-parity` both exit 0;
- DRC judges by the gate's rules, derived from the specification (`INV-07`, `RUL-03`).

**Grounds:**
- **`docs/work-orders/AET-WO-1300.FEEDBACK.md` S4 and D8** (ruled on PR #40). A Rust grid router's two-layer board passed DRC with parity in the spike. Zones are emitted unfilled. The gate runs `--refill-zones` and never `--save-board`. There is no DSN, no Freerouting, no Python and no Java.
- **`docs/work-orders/AET-WO-1301.FEEDBACK.md`, "What 1302 should know":**
  - **Escape from the LQFP:** the pitch is 0.5 mm and the pad gap 0.2 mm, so under the 0.2 mm floor no track passes between pads. Every MCU net escapes straight out from its pad. 109 of its 144 pads are single-pad `unconnected-…` nets that need no route.
  - **The board:** 13 multi-pad nets over 78 pads, 65 ratsnest edges. `VDD_3V3` has 29 pads and `GND` 23, which makes them the case for zones.
  - **Zone settings:** zones carry no `(connect_pads (clearance …))` (1300 B2). U3's pad 7 has no net and must be cleared like any other copper.
  - **The parity refusals bind here too:** no track may join an `unconnected-` pad to anything.
  - **The format:** tracks and vias carry `(net "<name>")`; there is no setup block or net table.
  - **Tuning levers** if routing is congested: `GAP_NM`, the rings and the target choice.
- **`aethelgard-eda`'s `rules`:** clearance and track width at `FLOOR_NM`, 0.2 mm. The gate's Default net class has 0.6 mm vias with a 0.3 mm drill.
- `docs/northstar.md` §4: `INV-01`, `INV-03`, `INV-07`.

> **What this is:** `emit`'s board gains copper:
> - tracks on F.Cu and B.Cu and vias, from a deterministic, integer-only grid router;
> - unfilled zones for the ground net, and for the supply net if the router needs it.
>
> The board then passes the gate with nothing left over.
>
> It is NOT:
> - parasitic extraction, which is reserved as `AET-WO-1303` for P2's twin, and reads this router's geometry;
> - fabrication output;
> - a change to the gate's rule files, refusals or parity checks;
> - more than two copper layers.

**Cadence:** build-only, with one measurement stop (§7): if the router cannot complete the reference board within the levers above, stop with the data. Never loosen a rule. One PR, left unmerged.

## 1. Scope

1. **The router, `aethelgard-eda`'s `route` module.**
   - **The search:** a grid maze search (Lee or A*) on an integer nanometre grid, with a pitch that divides the 0.5 mm LQFP pitch.
   - **Layers:** two, with a via cost.
   - **Order:** nets are routed in a fixed order: by pad count, then by name. Ties are broken by name, never by hash or address.
   - **Spacing:** every track and via keeps `FLOOR_NM` from foreign copper, including pads with no net and the board edge.
   - **Escape:** the LQFP's pads escape straight out.
   - **Multi-pin nets:** grown as a tree from the first pad in order.
   - **Rip-up and retry:** bounded, with the bound recorded.
2. **Zones.**
   - GND gets a zone on B.Cu over the board, unfilled, with no `connect_pads` clearance token. The gate's `--refill-zones` fills it.
   - If `VDD_3V3` cannot be tracked, it may take an F.Cu zone too. Record which.
   - Vias stitch zone-connected pads where the fill alone cannot reach.
3. **`emit`** writes the routed board in place of the placed-only board. Two emissions are byte-identical.
4. **Tests:**
   - **tier 1:**
     - byte-identical emission, across processes and environments;
     - the router's own clearance check over every pair of copper items;
     - no track touches an `unconnected-` pad;
     - every net's copper is connected, by the router's own connectivity;
   - **tier 2, in `tools` — P1 gate clause 1 as one test:**
     - `aethelgard emit --file examples/sentinel-01-minimal.json`;
     - then `check erc` on the schematic, which must exit 0;
     - then `check drc --spec examples/sentinel-01-minimal.json --schematic <sch>` on the board, which must exit 0 with **no finding**, unconnected items included;
   - **every existing test** passes unchanged, including 1300's and 1301's adversarial suites.

## 2. Invariants and fences

- `INV-01`, `INV-03`, `INV-07`. No `unsafe`. No floating point in any routing decision. No new dependency without a ruling.
- **Fence:** `crates/aethelgard-eda` (the new `route` module, and `pcb.rs` to emit copper), the CLI's `emit` tests and README, and this work order's FEEDBACK.
- **Protected:**
  - the gate's rule and refusal code (`rules.rs`, `board.rs`, `parity.rs`) diffs to empty;
  - so does every protected surface named in other work orders: the steering documents, `README.md`, `LICENSE-*`, `docs/inputs/**`, `container/` and `.github/workflows/`.
- One PR, left unmerged. **Push and revoke by the FUR-WO-0301 F7 route.**

## 3. Acceptance

1. §1.4 passes in `gates` and `tools`: P1 gate clause 1 holds.
2. FEEDBACK records:
   - routing time;
   - total track length;
   - via count;
   - the rip-up count;
   - which nets took zones;
   - the grid pitch.
3. CI is green on the first push.

## 5. Adversarial pass — full

1. **Make two emissions differ:** net order, grid rounding, environment.
2. **Make the router produce copper that DRC passes but that is wrong:** a short between nets the gate's parity misses, or a zone that joins two nets.
3. **Make it hang or blow up:** a specification with more parts, longer names, or a part placed hard against the edge. It must fail closed, naming the net it could not route, never loop.

## 5a. Reduction pass — diff-sized

Run the `reduction-pass` skill on the finished diff. One line in FEEDBACK's gates.

## 6. Upward-feedback directive

`docs/work-orders/AET-WO-1302.FEEDBACK.md`, same schema. It carries:
- the §3.2 numbers;
- what `1303`'s parasitic extraction will need from the geometry;
- the gates line.

## 7. Flag-and-stop conditions

- The reference board cannot be routed to zero findings without loosening a rule, or changing placement beyond the 1301 levers. Stop with the congestion data.
- A change to the gate's rules, refusals or parity code.
- `unsafe`, floating point in a routing decision, or an image change.
- A test that can only go green by being skipped, ignored, or weakened.
- Any step that needs `sudo`, a system package, or a secret beyond the push route above.

## 8. Kickoff prompt

```
/goal AET-WO-1302: §1 is built and P1 gate clause 1 passes as one tier-2 test in tools (ERC exit 0; DRC with --spec and --schematic exit 0 with no finding), or a §7 stop is recorded in FEEDBACK with the congestion data; the full adversarial and reduction passes and FEEDBACK are written; one open unmerged PR from wo/aet-1302-routing.
```
