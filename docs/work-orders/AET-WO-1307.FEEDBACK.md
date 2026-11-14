# AET-WO-1307 FEEDBACK — escape planning and name-independent order: spike

**Builder:** Claude (strong tier), worktree `~/aethelgard-1307`, branch `wo/aet-1307-escape-planning` from `main` `d00c965`.
**Cadence:** spike (§1), then STOP for the architect's ruling, then build (§2).

The predictions in §1 were committed before any probe was built or run. They are not edited afterwards; corrections go in §3.

Labels:
- **[read]**: read at source in this repository at `d00c965`.
- **[measured]**: run. Host cargo runs under a capped user scope. KiCad runs in the pinned image under rootless podman, with its caps read back.
- **[inferred]**: neither; reasoning from the two.

## 0. Source reads that shape the predictions

**[read]** at `d00c965`: `route.rs` (`route`, `Router`, `check`, `connectivity`), `pcb.rs` (`place`, `copper`), `rules.rs`; except where an item is marked **[inferred]**.

- **Order.** `route::route` sorts by `(!stitched, pad count, net name)`. The stitched nets (at 4 layers, each plane's net: `GND` and `VDD_3V3` on `D3`) are laid first, each pad joined by F.Cu to a through via. The net index is the net's rank by name (`BTreeMap` keys), so names also decide:
  - the order of `Stuck::By`'s `BTreeSet<usize>`, which is the order blockers are pushed back on the queue;
  - which of two equal-count nets goes first.
- **The tree root is a name.** `net_pads` orders a net's pads by **reference**, then pad number, and `route_net` grows the tree from `pads[0]`. `route_ground` stitches pads in that order too, so the first pad by reference claims the nearest via site.
- **Escape.** On F.Cu no track crosses a footprint's pad field (the box around its pads) except inside its own pad. No via stands within `VIA/2 + s` of the field. So at the LQFP every pad leaves straight outward. The whole interior of the pad ring is closed to tracks and vias on F.Cu. It is empty: `pcb::place` keeps every part clear of the MCU's keep-out box (courtyard and silkscreen) by 1 mm for ICs and 0.5 mm for passives. A pad meeting exactly one side of its field gets a soft *corridor* (cost 2 per pitch, 16 pitches deep). The corridor is a price, never a reservation.
- **Rip-up.** A soft search names the blockers. The stuck net and its blockers are ripped and re-queued at the front, with a `HISTORY_COST` of 40 on the contested nodes. The budget is `RIPUPS` = 500 nets in all.
- **Geometry at 0.5 mm pitch** (rules 0.2/0.2, via 0.6/0.3, hole gap 0.25 mm): two vias need 0.8 mm centre to centre. So adjacent LQFP plane pads (0.5 mm apart) cannot drop vias side by side. The second via must stagger outward, and its stub then runs beside the first via, in a neighbour's escape lane.
- **Placement depends on reference names.** `pcb::place` places the MCU, then the other ICs **in reference order**, then the passives in reference order. Each goes beside the IC it shares most nets with, with ties to the lower reference. Each decoupling capacitor goes in the first free spot in rings out from the pad its rail names. So renaming references at the specification level moves parts before the router runs. The decoupling capacitors sit just outside the LQFP's pad ring, by its supply pads, and their pad fields block via sites there too.
- **The gate's rules** (`rules.rs`): a clearance and a track-width minimum, as net class and as `.kicad_dru` custom rules. `route::check` judges clearance, edge, axis and hole gap. Neither has a courtyard rule for vias.
- **Zones.** `pcb::copper` writes each zone's polygon as the board outline's four corners exactly (x0,y0 → x1,y0 → x1,y1 → x0,y1). The emitter does **not** inset it by the edge clearance. KiCad's fill applies that clearance **[inferred]**: `route.rs` says so in `EDGE_NM`'s comment, measured as a board setting, not read in KiCad's fill source. `route::read` keeps only `(net, layer)` per zone and reads no polygon.

## 1. Predictions (committed before any run)

Probe plan: patches to `route.rs` under `spike/1307/`, applied to detached worktrees of this branch's base, with a release build, a runner script and a renaming script. The inputs are D3 and D1 (fixtures), the reference (`examples/sentinel-01-minimal.json`), 2 × 5 (`4100`'s `many(n_i2c=2, per_bus=5)` on the reference's 2-layer board) and the 4-layer fixture.

### S1. Why D3 fails
- **P1.1** The failing pads are U1 (LQFP-144) pads where a `VDD_3V3` pad and a `GND` pad are adjacent or one apart, on the side(s) where the SPI and CS signals leave. `4100` named U1 31 and 39. Every contested escape is within 3 pads of a plane pad.
- **P1.2** The second plane via of an adjacent pair has a free site, but only by taking the escape lane of the next signal pad (a CS or HOLD line). The site is not missing; it sits on another pad's only way out. At least one plane pad has **no** free via site within 16 pitches when the signals are laid.
- **P1.3** At give-up the rip-up loop is **cycling**: fewer than 8 distinct nets make up at least 80 % of the 500 rip-ups. `VDD_3V3` (or `GND`) and the same two or three CS/HOLD nets take turns, and the history cost does not break the cycle.
- **P1.4** The cause, in one sentence (to be confirmed): *at 0.5 mm pitch two adjacent plane pads need staggered vias, and the outer one sits in a neighbouring signal pad's only escape lane, because every LQFP escape is forced outward into the same strip.*

### S2. Candidate escape strategies (each measured on D3, D1, reference, 2 × 5, four-layer)
- **(a) Escape first** (one planned stub and via site per LQFP pad, outward, alternating rows, plane vias claimed first):
  - D3 routes, 55 %;
  - D1, 2 × 5 and four-layer route;
  - the reference's board changes (not byte-identical);
  - via count rises by 0 to 10 % on boards that already route;
  - time within 2× of today's.
- **(b) Plane-pad priority** (each plane pad claims its geometrically nearest free site before any signal escape; that is today's order, made geometric): **D3 does not route**, 70 %, because today's order already lays the planes first. The rest route as today. The reference is byte-identical or nearly so.
- **(c) Inward plane escape** (my third, from P1.4): a plane-net pad of the dense part escapes **inward** on F.Cu, under its own body, to a via inside the pad ring. Signals keep the whole outward strip. The interior of an LQFP-144 is about 17 mm square and empty.
  - **D3 routes, 75 %**: the fastest candidate, with the fewest rip-ups.
  - D1, 2 × 5 and four-layer route.
  - The reference changes at its LQFP plane pads, unless it is limited to 4 layers.
  - KiCad DRC passes: a via under a part's own body breaks no rule the gate writes. The gate writes only clearance and width, and `check` has no courtyard rule. This is **[inferred]** for KiCad's built-in checks (no `rule_severities` entry is written for a via in a courtyard).
  - **Decision flag:** WO §5.2 lists "a via site under another part's courtyard" as an escape lie. A via under the dense part's **own** body is not that case, but it is new, and it needs a ruling.
- **P2.5** **The reference's bytes and bound.** (b) leaves the reference byte-identical, 60 %; (a) and (c) do not. Where the reference's routed board changes, its two extraction bounds (46,800 and 58,235 fF today) move by under 10 % and stay under their declared limits (100,000 fF per line). The second §7 stop is not hit.
- **P2.4** No candidate needs a change to the rules or the stackup (the §7 stop is not hit).

### S3. Name independence
- **Two levels of renaming.** *Specification level* (end to end, what a user sees): device references and bus nets only. `U1`, `C*`, `R*`, `GND`, domains and the emitter's derived nets cannot be renamed there. It moves placement (§0), so it measures the placer and the router together. *Board level* (the router alone, placement fixed): every net and every reference in a `route::Board`, through a probe. Every one of the 20 renamed specifications must reach the router; a refusal other than `Unroutable` is reported as not a routing datum.
- **P3.1** Under today's order the 20 fixed renamings (seeded hash prefixes) give, at both levels:
  - reference 15 to 19 of 20;
  - D1 (4 layers) 10 to 18 of 20;
  - D3 0 of 20.
- **P3.2** A proposed order:
  - stitched first;
  - then pad count;
  - then the bounding box's half-perimeter;
  - then the pads' coordinates in lexicographic order;
  - the name only as the last tie-break.

  With the tree root and stitching order taken from geometry (lowest y, then x) and blockers re-queued by rank, not index, the renamed boards give **the same copper modulo names**: 20/20 identical geometry for the reference and D1.
- **P3.3** The guard (board level): a unit test that routes a `Board` and the same `Board` with every net and reference renamed by a fixed permutation, and asserts equal geometry (segments and vias with names erased). It is stronger and cheaper than counting 20 CLI runs. A tier-1 CLI test over 20 renamings of the reference sits beside it.
- **P3.4** Placement **does** depend on reference names (§0, read). So the proposed order makes the *board-level* renamings 20/20 identical. The *specification-level* ones still differ in copper, because parts move, and some may still fail. Whether `place` is in this work order's fence ("`pcb.rs` where the escape is written") is a decision. Probe check: compare the placed parts (positions with references erased) before routing.

### S4. Zone outlines
- **P4.1** The reader can require each zone's polygon to be exactly one `polygon` with four `pts` equal to the board outline's corners, in the emitter's order. Nothing `emit` writes is refused. All 5 fixtures plus sentinel-01's committed board pass.
- **P4.2** The work order says "minus the edge clearance, as the emitter writes it". The emitter writes the outline itself, not an inset. So the judge is equality with the outline, and the inset is KiCad's at fill time. Saying this is a correction to the work order's wording, not a change.
- **P4.3** A board re-saved by KiCad 10.0.7 keeps the four points in order, so a KiCad round trip of an emitted board is not refused. The only thing refused that someone might call legitimate is a hand-redrawn zone, which `check drc --spec` already refuses as "not the declared board" in spirit.
