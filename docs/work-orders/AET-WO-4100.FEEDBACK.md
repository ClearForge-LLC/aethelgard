# AET-WO-4100 — FEEDBACK (spike)

**Branch:** `wo/aet-4100-stackups` · **Base:** `9035203` (`origin/main`) · **Work order:** `origin/claude/argon-repo-docs-synthesis-rjrb0d` at `3110ac5`, `docs/work-orders/AET-WO-4100.md` (lands through docs PR #117; not committed here) · **Builder:** Claude Code (Opus 5.5) as `forge` on Furnace · **Cadence:** spike (§1), then **STOP** for the architect's ruling. No build code before it.

**Where everything runs.** Every KiCad step runs `kicad-cli` 10.0.7 in the pinned tools image (`container/IMAGE`, `…@sha256:c69ff9bc…`) under rootless podman with `--memory 8g --memory-swap 8g --pids-limit -1`, `TMPDIR` unset, one container at a time. After start, `0` is written into the container's `memory.swap.max`, and `memory.max`, `memory.swap.max` and `pids.max` are read back into `spike/4100/caps.log`. Router and arithmetic probes run on the host.

Entry schema for findings: `finding · where · severity · type · recommendation · decision-needed`.

## Predictions — written before any run

This section is committed before the first probe runs. Nothing in it is edited after a run; the results table at the end sets each result beside its prediction.

**Provenance labels.** **[read]**: read at source on `9035203`. **[inferred]**: reasoned, not read or measured.

### What the predictions rest on

- `pcb.rs` `LAYERS` (L61): KiCad 10's two-layer table, `F.Cu` id `0`, `B.Cu` id `2`; `(general (thickness 1.6))` literal (L785); the zone is written with `(layers "F.Cu" "B.Cu")` at L860 **[read]**.
- `route.rs`: `enum Layer {F, B}` (L100), `Layer::other`, the search state `id = (l.index() * n + node) * 5 + d` and `layer_of` (L589–590), `[bool; 2]` in claims (L763, L788), `Item.layers` (L1444, L1470) and `connectivity`'s `via_layers` (L1551); ground is joined pad → F.Cu → via → the unfilled B.Cu zone (L973–1004), and `connectivity` refuses any zone but ground on B.Cu (L1586–1588); `read` refuses a via whose layers are not exactly `F.Cu B.Cu` (L1848) **[read]**.
- `extract.rs`: the trial `ψ = max(0, 1 − d/S)`, `S` = 0.2 mm, made valid across the board by `BOARD_THICKNESS_NM − 2·COPPER_NM > CLEARANCE_NM` (L120); per-length 280 fF/mm, per track 124 fF, via 1,519 fF; the reference's bounds SDA 46,800 fF and SCL 58,235 fF against a declared 100,000 fF (1305 FEEDBACK) **[read]**.
- `cli/src/fab.rs` `LAYERS` (L50–58): the Gerber list with the creator line's index per layer (9 for copper, 8 for `Edge.Cuts`); `strip_clock` deletes line 2 and that line **[read]**. `eda/src/fab.rs` L86: "Two copper layers: F.Cu (top) and B.Cu (bottom)." **[read]**.
- **1302 B10/A6** **[read]**: `D1` is "one of the reviewer's 24-device short-reference variants", failing on `/S0_SDO` after 500 rip-ups. **Its specification was not committed** (only the in-tree 24-device short control, `adversarial_1301_tools.rs` L250–320, which routes). 2 × 5 has routed since B4.

### S1 — KiCad 10.0.7 at 4 and 6 layers

- **P1.1** `pcb upgrade` keeps a 4-layer table as given and writes inner copper with KiCad 9's even-number ids: `In1.Cu` `4`, `In2.Cu` `6` (and at 6 layers `In3.Cu` `8`, `In4.Cu` `10`), named `In1.Cu`… with type `signal` unless the board says `power` **[inferred]**.
- **P1.2** KiCad requires no `(stackup …)` block: a board without one loads, DRCs and plots. `pcb upgrade` does not add one to a board that has none **[inferred]**.
- **P1.3** DRC does not read the stackup's dielectric thicknesses or permittivities (no impedance check is configured); the Gerber layer files do not either. Only the Gerber job file (`.gbrjob`) carries the stackup **[inferred]**.
- **P1.4** With `--refill-zones`: a plane on `In1.Cu` fills; a seeded clearance violation between two `In1.Cu`/`In2.Cu` items of different nets is reported as `clearance`; a through via of another net crossing a plane gets an antipad from the refill and is **not** a violation. So §7's "DRC does not report a seeded inner-layer violation" does not trigger **[inferred]**.
- **P1.5** Inner-layer Gerbers are named `In1_Cu.g1`, `In2_Cu.g2` (6 layers: up to `In4_Cu.g4`), with `%TF.FileFunction,Copper,L2,Inr*%` … and the outer layers become `Copper,L1,Top` and `Copper,L<n>,Bot`. The two clock lines sit at line 2 and line 9 exactly as on `F.Cu`, so `RUL-03`'s amended rule covers them with creator index 9 and the §7 stop does not trigger **[inferred]**.

### S2 — the specification's `board` section

- **P2.1** The proposal is `schema_version` 2 with a required `board` section; a version-1 file is refused by name (`UnsupportedSchemaVersion` or the existing equivalent). Every committed specification migrates: the two `examples/` files and every JSON fixture under `crates/*/tests/fixtures` that carries a specification **[inferred]**.
- **P2.2** The stackup is better declared in the specification as bounds with sources than named from a fabricator table, because the bound needs minima and maxima, which fabricators publish as nominal values with a tolerance. A table of named fabricator stackups (the `4200` parts-table precedent) is the convenience layer over it, not the home **[inferred]**.
- **P2.3** Furnace does not reach a fabricator's stackup page as text that names the dielectric thicknesses (JavaScript-rendered or blocked); Scotty may need to fetch it **[inferred]**.

### S3 — the router at N layers

- **P3.1** The smallest change: `Layer` becomes an index `0..n` over the routing layers (2 at 2 and 4 layers, 4 at 6 layers), every `[bool; 2]` becomes a per-layer set sized by the stackup, the search's state count scales by `n`, and a via joins every routing layer. Planes are not routing layers: a plane net (ground, a supply) is joined pad → outer track → via, and the plane's connectivity is judged like today's zone, with only foreign vias' antipads as obstacles **[inferred]**.
- **P3.2** At 4 layers (S, P, P, S) the congested 24-device variants that fail closed at 2 layers route, because B.Cu stops carrying the ground zone and the supply leaves the tracks; escape planning is not needed for them **[inferred]**.
- **P3.3** 6 layers are not needed for anything 4 layers fail on in the 1302 set **[inferred]**.
- **P3.4** `route::check` must learn per-layer clearance for N layers and that a via's disk sits on every copper layer, planes included (a plane's antipad); `connectivity` must learn that a plane joins every via of its net that it reaches, and that a via is joined on every layer it crosses **[inferred]**.

### S4 — extraction's bound at a stackup

- **P4.1** A valid trial exists: below the piece, an elliptical cap `ρ = √((d_lat/S)² + (z/h)²)`, Euclidean distance (scale `S`) beside and above. At `h = S` it is exactly today's trial, so taking `h_eff = min(h_min, S)` leaves the 2-layer reference byte-identical. The bound rises roughly as `A/h` for a piece's area over the plane **[inferred]**.
- **P4.2** Per-length at `h` = 0.1 mm, 70 µm copper, 0.2 mm track: about 345 fF/mm, against 280 at 2 layers (+20 to +25 %) **[inferred]**.
- **P4.3** With a 7628 prepreg (about 0.2 mm, typical of the cheapest 4-layer 1.6 mm stackups) the minimum dielectric may sit just under `S` once its tolerance is taken, so the bound rises only slightly **[inferred]**.
- **P4.4** The reference's I2C lines on a 4-layer board at `h` = 0.1 mm stay under their declared 100,000 fF: SDA near 50 to 55 pF, SCL near 62 to 68 pF, because the pins' `Ci` (10 pF each) dominate **[inferred]**.
- **P4.5** Refusing inner-layer tracks is enough for 6 layers only if the router keeps I2C nets on the outer layers; otherwise extraction would refuse a board the router itself produced. The same trial bounds a track between two planes (caps on both sides), which would cover inner tracks between planes at little cost **[inferred]**.

### S5 — the fabrication package at N layers

- **P5.1** `verify --fab-out` cannot take a hand-built 4-layer board today: `verify` generates from the specification and `read` refuses non-two-layer copper and any thickness but 1.6 mm. So S5 is measured by running the package's own `kicad-cli` export arguments on a hand-built 4-layer board **[inferred]**.
- **P5.2** At 4 layers a fabricator needs, beyond today's package: the inner Gerbers, the layer order (which file is which layer), the stackup table (dielectric thicknesses, copper weights, material) and the finished thickness. `FAB-NOTES.md` today says "Two copper layers", which would lie **[inferred]**.
- **P5.3** KiCad's Gerber job file carries a creation date and generation software at places `RUL-03` does not name; including it would need a new deletion rule. I will recommend keeping it excluded and stating the stackup in `FAB-NOTES.md` from the specification **[inferred]**.

### S6 — scope check

- **P6.1** Outside `aethelgard-eda` and the CLI's `generate`, `check`, `extract`, `verify` paths, nothing reads a board's layer count or thickness: the twin, the analog deck, the campaign and `evolve` read the solved topology and declared nets, not the board **[inferred]**.
