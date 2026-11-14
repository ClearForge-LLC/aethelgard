# AET-WO-4100 — FEEDBACK (spike, then build)

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

---

## Results — measured after the predictions commit (`9b20f83`)

Evidence: `~/.cache/aet-4100/` on Furnace (boards, verdicts, Gerbers); probes in `spike/4100/`; caps read back in `spike/4100/caps.log` (every container: `memory.max=8589934592 memory.swap.max=0 pids.max=max`).

### S1 — KiCad 10.0.7 at 4 and 6 layers (image `c69ff9bc`, `kicad-cli version` = `10.0.7`)

**Where the Gerber and antipad numbers come from.** The gate verdicts below are the real `check drc --spec`. The five-run determinism test, the stackup comparison and the 0.5005 mm antipad gap came from **unprepared copies** (raw `kicad-cli` on the probe board, KiCad's default project settings), all in one container on Furnace. The package exports the gate-prepared copy instead, which differs in its `(setup …)` and its rule files. At the build, the determinism test should be repeated on the prepared copy.

Probe boards: `spike/4100/s1/variants.py` turns the emitted reference (`aethelgard emit`, board sha256 `7b9b0e3d…`) into `b4` (F, In1, In2, B; GND plane on In1, `VDD_3V3` plane on In2, the B.Cu GND zone kept), `b6` (GND In1, `VDD_3V3` In4, In2/In3 signal), and seeded variants. Runs: `s1/drc.sh`, `s1/gerb.sh`.

| Question | Measured |
|---|---|
| Layer table KiCad writes (`pcb upgrade --force`) | `(0 "F.Cu" signal) (4 "In1.Cu" signal) (6 "In2.Cu" signal) [(8 "In3.Cu" signal) (10 "In4.Cu" signal)] (2 "B.Cu" signal)`: kept as given, inner ids 4, 6, 8, 10. Plain `pcb upgrade` says "Board file was not updated" (format current) |
| Stackup required? | No. A board with none loads, DRCs and plots; a forced save adds none (`(general (thickness 1.6))` is the only thickness) |
| Stackup read by DRC? | No: raw DRC of `b4` and of `b4_stackup` (dielectrics 0.01 mm, `epsilon_r 50`) gives the same 15 violations. **And the gate never sees it:** `board::prepare_read` replaces the whole `(setup …)` with `rules::SETUP` (`(setup (pad_to_mask_clearance 0))`), so a board's stackup claim is discarded before DRC and before export |
| Stackup read by the Gerbers? | No: every copper and outline Gerber of `b4_stackup` equals `b4`'s, line for line, once the two clock lines are removed. The job file is identical too, apart from its date: its `MaterialStackup` lists layer order and `FR4` but **no thickness and no permittivity**, with or without my block. That block listed only copper and dielectric layers; a complete KiCad block (with silk, paste and mask) was not tried, so this claim is limited to that block |
| Gate on `b4`, `b6` (planes, refilled) | `pass`, no finding (`check drc --spec`, exit 0) |
| Seeded inner clearance (`b6_seed`: In2.Cu `/I2C_SDA` and `/I2C_SCL` 0.1 mm apart) | **Reported:** `clearance`, "rule 'specification_clearance' clearance 0.2000 mm; actual 0.1000 mm", both tracks named on In2.Cu |
| Seeded inner short (`b4_short`: In1.Cu track from an `/I2C_SDA` via to a `/FLASH_CS` via, across the GND plane's layer) | **Reported:** `shorting_items` "nets /FLASH_CS and /I2C_SDA", `Via [/FLASH_CS] on F.Cu - B.Cu` and `Track [/I2C_SDA] on In1.Cu` |
| Through vias of other nets crossing a plane | No violation: the refill cuts an antipad. Measured from the Gerber regions (`s1/antipad.py`): smallest gap from plane copper to a foreign via's 0.6 mm disk is **0.5005 mm** on In1 (48 foreign vias) and on In2 (45), above `S` = 0.2 mm. The emitted zone declares no clearance, so where KiCad takes 0.5 mm from is not established here; S4 needs only that the gap is at least `S` and judged by DRC. 1305 measured 0.5005 mm on the 2-layer zone. Via pads are flashed on every inner layer (`%TA.AperFunction,ViaPad*%`, 70 on In1) |
| Copper on an undeclared layer (`b2_undecl`: 2-layer table + an In1.Cu track) | **Reported:** `item_on_disabled_layer` "Item on a disabled copper layer (layer In1.Cu)", error |
| A one-ended track of net A touching only a via of net B (`b4_seedvia`) | KiCad re-nets it by copper: reported as `Track [/FLASH_CS] on In2.Cu`, `track_dangling` only. Not a short, and correctly so (`RUL-03`: copper, not labels). My first short seed was this shape by mistake; `b4_short` is the real one |
| Inner Gerber names and attributes | `In1_Cu.g1` … `In4_Cu.g4`; `%TF.FileFunction,Copper,L2,Inr*%` … `L5,Inr`; outer `Copper,L1,Top` and `Copper,L4,Bot` / `L6,Bot` |
| Clock lines | In every copper file at 4 and 6 layers: `%TF.CreationDate` at **line 2**, `G04 Created by … date` at **line 9** (outline: 2 and 8), the places `RUL-03` names. Five exports per board across wall-clock seconds: raw files all differ, **stripped files byte-identical** (1 distinct sha256 per file), and no other line holds a date or time shape. **The §7 stop does not trigger** |
| Job file (`input-job.gbrjob`, written by every `export gerbers`) | Two exports differ only in `"CreationDate"` (line 8, JSON). It also carries a `GUID` derived from the file stem, `"Revision": "rev?"`, `BoardThickness` from `(general …)`, and `DesignRules` with an `Inner` `PadToPad 0.0` |

**S1 answers.** KiCad 10.0.7 handles 4 and 6 layers as predicted: P1.1, P1.2, P1.3, P1.4 and P1.5 hold. DRC reports seeded inner-layer clearance and shorts and copper on an undeclared layer, so the gate needs no inner-layer check of its own. The inner Gerbers carry the two clock lines at the measured places and nothing else nondeterministic. One thing the predictions missed: **a stackup in the board is inert everywhere that matters** (the gate drops it; DRC and the Gerbers ignore it; the job file prints no thickness). The stackup therefore lives only in the specification and `FAB-NOTES.md`, and adversarial §5.1 item 4 ("a stackup the board's `(setup …)` disagrees with") is closed by construction: the gate discards the board's `(setup …)`.

### S2 — the specification's `board` section (proposal; nothing built)

**Shape.** Units follow the specification's own integer style (lengths in micrometres, permittivity in thousandths). Every key is required (`INV-06`).

```json
"board": {
  "copper_layers": 4,
  "layers": [
    {"name": "F.Cu",   "role": "signal"},
    {"name": "In1.Cu", "role": "plane", "net": "GND"},
    {"name": "In2.Cu", "role": "plane", "net": "VDD_3V3"},
    {"name": "B.Cu",   "role": "signal"}
  ],
  "stackup": {
    "finished_thickness_um": {"nominal": 1600, "min": 1440, "max": 1760, "source": "…"},
    "dielectrics": [
      {"between": ["F.Cu", "In1.Cu"],   "min_thickness_um": 179,  "source": "…"},
      {"between": ["In1.Cu", "In2.Cu"], "min_thickness_um": 891,  "source": "…"},
      {"between": ["In2.Cu", "B.Cu"],   "min_thickness_um": 179,  "source": "…"}
    ],
    "copper_max_um": {"value": 70, "source": "…"},
    "relative_permittivity_max_permille": {"value": 5400, "source": "IPC-4101 FR-4 maximum at 1 MHz (extract.rs EPS_R)"}
  },
  "rules": {"clearance_um": 200, "track_width_um": 200, "via_drill_um": 300, "via_diameter_um": 600}
}
```

**Validation, each refused by name:**
- `copper_layers` ∉ {2, 4, 6};
- `layers` not exactly KiCad's names in order (`F.Cu`, `In1.Cu` … `In<n-2>.Cu`, `B.Cu`, measured S1), or of another length;
- an outer layer that is not `signal`;
- a plane whose `net` is not `GND` or a declared domain id;
- two planes on one net (the router stitches one plane per net);
- `dielectrics` not exactly the `n − 1` adjacent pairs in order;
- `min > nominal` or `nominal > max`, a zero thickness, `finished_thickness.min` below the sum of the dielectric minima;
- `copper_max_um` or `relative_permittivity_max_permille` zero;
- any rule below the engine floor (`FLOOR_NM`, 0.2 mm; via drill 0.3 mm, via 0.6 mm), or a via smaller than its drill plus twice the track floor;
- an empty `source`.

Today's two-layer constants become Sentinel-01's declared values (decision **D5**):
- finished 340–2000 µm, nominal 1600 µm. The nominal is what `pcb.rs` writes as `(general (thickness 1.6))` (L785), what `FAB-NOTES.md` prints as "Declared thickness 1.6 mm" (`eda/src/fab.rs` L89), what `extract::read` requires of the board, and the base of the via's height bound (nominal + 25 %, today 2.0 mm). A 4-layer board writes its own nominal there;
- one dielectric with a 200 µm minimum (0.34 mm − 2 × 70 µm);
- 70 µm copper;
- permittivity 5.4.

With those, `h_min` = `S` exactly, so extraction keeps today's path (S4), and `FAB-NOTES.md`'s conditions can print the same words.

**`RUL-14` and versions.** This is specification `schema_version` **2**. A version-1 file is refused by name (`UnsupportedSchemaVersion`, which already exists, `lib.rs` L860), and nothing defaults a `board` in. The bump must be shared with `4200`, which also changes `aethelgard-ir` (PR #118): one bump, landed by whichever merges first (decision **D6**).

**What migrates** (counted with `grep '"schema_version"'` on `9035203`):
- `examples/sentinel-01-minimal.json` and `examples/invalid-zero-domain.json`;
- `crates/aethelgard-cli/tests/fixtures/*.json` (1) and `fixtures/solve/*.json` (4);
- 10 inline specifications in Rust tests (`json!` with `schema_version`);
- `docs/work-orders/AET-WO-1200.repro/` (2). These are historical: leave them, or migrate them, by ruling.

The `scenarios/` files carry the scenario's own `schema_version`, not the specification's.

**The home of the stackup.** I recommend declaring it **in the specification, as bounds with sources**, as above. A fabricator's published stackup is a nominal value; the bound needs a minimum dielectric and a maximum copper weight. The two fabricators below show why:
- JLCPCB publishes no thickness tolerance on its stackup page;
- OSH Park does publish one (±0.0202 mm).

A table of named fabricator stackups (`4200`'s parts-table precedent) can come later as a convenience that fills these bounds, not as their home (decision **D2**).

**Furnace reaches the pages (measured, `curl`, HTTP 200, saved under `~/.cache/aet-4100/s2/`):**

| Page | Read there |
|---|---|
| `jlcpcb.com/impedance` (sha256 `b6854cd9…`) | **JLC04161H-7628**: top copper 0.035 mm, **prepreg 7628 × 1, 0.21040 mm**, core 1.065 mm, prepreg 0.21040 mm, bottom 0.035 mm. **JLC04161H-3313**: **prepreg 3313 × 1, 0.09940 mm**, core 1.265 mm. Prepreg Dk: 7628 4.4, 3313 4.1, 1080 3.91, 2116 4.16. **No tolerance stated** |
| `docs.oshpark.com/services/four-layer/` (sha256 `d7c8c17c…`) | 1 oz outer copper, clad and plated, **1.7 mil (0.0432 mm)**. **FR408HR 2113 prepreg 7.87 mil (0.1999 mm) ± 0.797 mil (0.0202 mm)**, Dk 3.61 at 1 GHz. Core 39 mil (0.9906 mm) ± 3.9 mil. 1.6 mm nominal |
| `pcbway.com/multi-layer-laminated-structure.html`, JLCPCB's impedance-calculator guide | Reached; prose only, not used |

So P2.3 was wrong: Scotty is not needed to fetch them.

### S3 — the router at N layers

The probe was built by a subagent (Sonnet) in its own worktree. I spot-checked the D1 claim by re-running its binary: 2 layers `Unroutable /SDA0` in 12.75 s; 4 layers routed in 0.48 s with 226 vias. The probe files are in `spike/4100/s3/`:
- `router-nlayer.patch`: +230/−120 lines in `route.rs`, 1 in `pcb.rs`;
- `run.sh`;
- `results.tsv`;
- `gen.py`.

**2 × 5 is a reconstruction** (`many(n_i2c=2, per_bus=5)`); only D1's generator was recovered. **D1 is the reviewer's own input.** Its generator was recovered verbatim from the 1302 reviewer's transcript (`many()`: 4 I2C buses × 6 devices, references `S0…S23`, one SPI flash `S1000`). The only change: `emit` now refuses an I2C bus with no rise-time constraint (`MissingBusConstraint`), so `gen.py` adds the reference's constraint (100 pF, 300 ns) to every line.

**The probe's design.** `AET_LAYERS` selects 2, 4 or 6 layers:
- **2 layers** are today's code path. The reference, 2 × 5 and 2 × 5-without-SPI boards are byte-identical to HEAD, and the route unit tests pass.
- **4 layers** are S, P, P, S: routing on F and B, a GND plane on In1 and a supply plane on In2, and no B.Cu zone.
- **6 layers** are S, P, S, S, P, S: routing on F, In2, In3 and B.

Plane nets are GND plus every `VDD*` net, a probe rule. Each goes through today's `route_ground`: a pad, then an F.Cu track, then a via that reaches the plane.

Vias are through vias, and each is an obstacle on every layer. `check` and `connectivity` use a per-layer mask. A via counts as joined when copper or its plane meets it. A plane's core is cut only by foreign vias and the edge, and each plane net's vias must lie in one piece of its plane.

**What the build must change (about 55 sites in `route.rs`, plus `pcb.rs` L833/860/865 and `fab.rs` L211).** By area:
- the `Layer` enum and `other` (L100–121);
- the per-layer arrays `[Vec<u32>; 2]` and `[BTreeSet; 2]` (L344–469);
- the search state `(l.index()·n + node)·5 + d` and `layer_of` (L589–590), which scale with n;
- the single via move and the F-only corridor (L636–672);
- `blockers`, `contest` and the `0..2` loops (L763–803);
- `commit`, `claim_via` and `rip` (L830–881);
- `route_net`, `route_ground` and the zone loop (L916–1413);
- `Item.layers` and `check` (L1444–1495);
- `connectivity`'s `via_layers` and zone core (L1551–1642);
- `read`'s `layer_of`, its exact-`F.Cu B.Cu` via check and its zone read (L1789–1859). The probe did not change these, so `read` still refuses a 4-layer board (S5).

`unhop` assumes a path returns to the layer it left, and needs a guard once vias span more than two routing layers.

| Spec | 2 layers (HEAD) | 4 layers (S-P-P-S) | 6 layers (S-P-S-S-P-S) |
|---|---|---|---|
| reference | routed, 1.0 s, 70 vias | routed, 0.1 s, 64 vias; F+B track 349 → 193 mm | routed, 0.6 s, 65 vias |
| 2 × 5 | routed, 5.8 s, 132 vias | routed, 0.4 s, 129 | routed, 0.9 s, 126 |
| 2 × 5, no SPI | routed, 4.4 s, 114 | routed, 0.3 s, 114 | routed, 0.6 s, 111 |
| **D1** (24 I2C) | **Unroutable `/SDA0`** (1302: `/S0_SDO`; same class), 500 rip-ups | **routed, 0.5 s, 226 vias, 13 rip-ups** | Unroutable `/SCL3` at 500 rip-ups. Routes with inner step cost 6 (9.6 s) or 3,000 rip-ups (55 s) |
| D3 (24 I2C + 4 SPI × 3) | Unroutable `/MOSI0` | **Unroutable `VDD_3V3`**, blocked by GND at U1 pads 39 and 31 (plane fan-out), even at 3,000 rip-ups | Unroutable, a different net each knob |

Times were measured with 5 runs in parallel, ±20 %. Only the probe's own `check` and `connectivity` judged the routed results, not KiCad DRC, because the probe emits no 4-layer KiCad board.

**S3 answers.**
- **Four layers route what two cannot.** D1 routes at 4 layers in 0.5 s, and boards that already route get 10–40× faster with about 45 % less track.
- **Six layers add nothing measurable over four.** Its default outcome on D1 is worse. My inference is that a through via must find all four routing layers open, so via sites get scarcer.
- **Escape planning is still needed for D3-class boards** (inferred from the failing pads): adjacent supply and ground pads on the MCU's LQFP each need their own escape to a plane via.

So P3.2 holds for D1 and fails for D3, and P3.3 holds.

### S4 — extraction's bound at a stackup

**Setting.** A piece is a prism: a convex base `B` (area `A`, perimeter `p`), height `c ≤ COPPER_MAX`, on an outer layer. Its bottom face is at `z = 0`, and the dielectric is below.
- Every foreign conductor on the same layer is at least `S` away laterally (`route::check`).
- Every via barrel lies inside its checked disk.
- The next copper layer down, plane or signal, is at depth at least `h` = the declared minimum thickness of that dielectric. That holds for every inner-copper item, and only the minimum is used.
- Nothing above the piece is in scope (part bodies are outside, as today).

**The trial.** `ψ = max(0, 1 − ρ)`, where:
- `ρ = d/S` for `z ≥ 0`, with `d` the Euclidean distance to the piece (today's trial);
- `ρ = √((d_B/S)² + (z/h)²)` for `z < 0`, with `d_B` the planar distance from `(x, y)` to `B`: an elliptical cap.

**Conditions, each checked:**
1. **ψ = 1 on the piece.** There `ρ = 0`.
2. **ψ is continuous and Lipschitz, so it is in H¹.** At `z = 0`, the distance from `(x, y, 0)` to the prism is `d_B`, so both forms give `d_B/S`.
3. **Its support lies inside today's `S`-shell** when `h ≤ S`. Below, `z² < h²(1 − d_B²/S²) ≤ S² − d_B²`, so `d² = d_B² + z² < S²`.

   So every conductor today's argument excludes is still excluded:
   - same-layer copper;
   - via barrels;
   - a plane's antipad: the plane keeps 0.5005 mm from a foreign via, measured in S1, and DRC judges it.
4. **Below, the support ends at `|z| < h`**, so ψ = 0 on the next copper layer, whatever it carries. At 6 layers that covers a signal layer under a plane as well.
5. **Floating conductors and the union of pieces are handled as today.** A floating conductor may take the constant 0. For the union, `max(ψ_a, ψ_b)` gives the sum of the pieces' bounds.

**The energy.** By Chew eq. 7.1.48, `C ≤ ε_max ∫ |∇ψ|²`. The integral has three parts.
- **Above and beside (`z ≥ 0`): today's terms.**
  - side: `c(pS + πS²)/S²`;
  - top cap: `A/S + πp/4 + 2πS/3`.
- **Below (`z < 0`):**
  - Directly under `B`, `|∇ρ|² = 1/h²` over volume `A·h`. That gives `A/h`.
  - Beyond `B`, the lateral annulus at `d_B = s` has area `(p + 2πs) ds` (Steiner). With `s = Su`, `|z| = hv`, and polar `u = r cos θ`, `v = r sin θ`, the integrand is `|∇ρ|² = (u²/S² + v²/h²)/(u² + v²) = cos²θ/S² + sin²θ/h²`.
  - So the annulus contributes `S·h ∫₀¹∫₀^{π/2} (p + 2πS r cos θ)(cos²θ/S² + sin²θ/h²) r dθ dr`.

The result:

  **`lower(A, p, h) = A/h + πph/(8S) + πpS/(8h) + 4πh/9 + 2πS²/(9h)`**

  **`C_piece ≤ ε0 ε_r,max · [ c(pS + πS²)/S² + A/S + πp/4 + 2πS/3 + lower(A, p, h) ]`, valid for `0 < h ≤ S`; for `h ≥ S` use `h = S`.**

- **At `h = S`, `lower` equals today's bottom cap** `A/S + πp/4 + 2πS/3`, coefficient by coefficient. This was checked with exact rationals, π kept as a symbol (`s4/check.out` §1). So the bound is continuous in `h`, and at `h_min ≥ S` it is today's bound.
- **Per length of track** (a stadium, `A = LW + πW²/4`, `p = 2L + πW`):
  `dC/dL = ε[(2cS + WS + πS²/2)/S² + W/h + πh/(4S) + πS/(4h)]`. At `h = S` this is today's `(2(c+W)S + πS²)/S²` (279.30 fF/mm, 280 rounded up).
- **A piece between two dielectrics** (an inner track) takes `lower(·, h_above)` and `lower(·, h_below)` in place of today's caps. The stripline check below is exactly that case.
- **Vias** keep today's isotropic shell. That needs every plane to keep at least `S` from a foreign via, which DRC judges (S1: 0.5005 mm). The via's height bound becomes the declared maximum finished thickness plus its tolerance condition.

**Numerical checks (`spike/4100/s4/check.py` → `check.out`):**

| Check | Result |
|---|---|
| Identity at `h = S` (exact rationals, π symbolic, three `(A, p, S)`) | equal in both coefficients |
| `lower` against a 3D finite-difference quadrature of the same ψ: boxes 0.2 × 1.0 and 0.6 × 0.5 mm, h = 0.075, 0.1, 0.15; grids S/20, S/40, S/80, first-order Richardson | agreement to **6 × 10⁻⁵ … 1.6 × 10⁻⁴** relative, all six cases |
| **Exact case for the new cap:** zero-thickness stripline, planes at `h` above and below (Cohn, `C' = 4ε K(k')/K(k)`, `k = sech(πW/4h)`). Modulus checked against the wide-strip limit: `C'/ε − 2W/h → 8 ln 2/π = 1.7651` at W = 2, 4 and 8 mm, as computed | the trial (two caps) **≥ exact** at every h from 0.05 to 0.2 mm (ratio 1.34–1.50) and every W from 0.1 to 1.0 mm |
| **The whole trial (microstrip):** W × c = 0.2 × 0.07 mm at h over a plane, against the P1 energy of the discrete minimiser in a 6 × 3 mm grounded box (5 µm grid). Any admissible P1 function has energy ≥ C(box) ≥ C(open), so trial ≥ P1 shows trial ≥ truth | holds at h = 0.075, 0.1, 0.15, 0.2 (trial/P1 = 1.42–1.67); the inscribed wire `2π/acosh(H/a)` (a lower reference) sits below both |

**The reference's I2C lines over a plane at `h`.** I took the reference's own routed pieces: length, tracks, vias, and each pad's box read from the board. I recomputed the trace, the track ends and the pads with `lower(·, h)`. The vias and pins do not change. At `h ≥ S`, the float model reproduces `extract`'s 2-layer terms to rounding (trace 9,116 against 9,146 integer, and so on).

Every row is the float model; deltas are float against float at `h ≥ S` (SDA 46,765, SCL 58,177). `extract`'s integer path gives 46,800 and 58,235 there, because it rounds each term up.

| `h_min` (mm), source | per-length (fF/mm) | SDA bound (fF) | SCL bound (fF) | rise vs `h ≥ S` | Margin to 100,000 |
|---|---|---|---|---|---|
| ≥ 0.2 (2 layers) | 279.3 | 46,765 | 58,177 | — | 53 % / 42 % |
| 0.1797 (OSH Park 2113 prepreg, 0.1999 − 0.0202) | 285.1 | 46,944 | 58,472 | +178 / +295 (+0.4 / +0.5 %) | 53 % / 42 % |
| 0.15 | 298.4 | 47,371 | 59,178 | +606 / +1,002 | |
| 0.1 | 345.9 | 48,987 | 61,841 | +2,222 / +3,664 | |
| **0.0994 (JLC04161H-3313, nominal; no tolerance published)** | **346.8** | **49,019** | **61,893** | **+2,253 / +3,717 (+4.8 / +6.4 %)** | **51 % / 38 %** |
| 0.075 | 398.1 | 50,804 | 64,834 | +4,039 / +6,657 | |
| 0.05 | 507.2 | 54,641 | 71,149 | +7,875 / +12,972 (+16.8 / +22.3 %) | 45 % / 29 % |

**S4 answers.**
- **A valid trial exists, and it is derived and checked.** The §7 stop "no trial bounds an outer piece over a near plane" does not trigger.
- **The reference's declared 100 pF still holds at 4 layers.** At the thinnest real prepreg (JLC 3313) the bound rises by 4.8 % (SDA) and 6.4 % (SCL), leaving 38 % margin on SCL. The pins' 30 pF dominate. P4.2 predicted about 345 fF/mm per length at 0.1 mm. The run printed 345.9 fF/mm at 0.1 mm and 346.8 fF/mm at 0.0994 mm. P4.4's ranges were close: SDA came in lower than predicted.
- **The 2-layer reference is unchanged by construction.** Its `h_min` = 0.34 − 2 × 0.07 = 0.2 mm = `S`. I recommend the build keep today's integer code path whenever `h_min ≥ S`, rather than evaluate the new formula at `h = S`: a different `div_ceil` placement could move an integer even where the reals agree. The `CONDITIONS` strings must also stay word for word for Sentinel-01, because `FAB-NOTES.md` prints them.
- **Six layers: refusing inner-layer tracks is not enough on its own.** The S3 probe routed I2C on In2 and In3 at 6 layers (D1: 24 mm on In2). Either the router keeps I2C on the outer layers, or extraction bounds inner pieces with two caps. The two-cap form is the stripline case already checked exactly, so it costs little (decision **D4**). P4.5 holds.

### S5 — the fabrication package at N layers

| Question | Measured |
|---|---|
| Can `verify --fab-out` take a 4-layer board today? | No. `verify` takes only `--spec` and regenerates a 2-layer board. The package's own read of the copper, `route::read`, refuses the 4-layer board: `aethelgard extract` on `b4` and `b6` → exit 3, `BoardUnreadable` "a zone on layer Some("In1.Cu")". **The DRC gate passes the same boards (S1).** So today a 4-layer board passes DRC and cannot be extracted or packaged, which fails closed |
| Gerber set at 4 / 6 layers | The package's export arguments (`export gerbers --layers … --check-zones`) give `F_Cu.gtl`, `In1_Cu.g1`, `In2_Cu.g2` (… `In4_Cu.g4`), `B_Cu.gbl`, deterministic once the two clock lines are removed (S1). `cli/src/fab.rs` `LAYERS` is a fixed 9-entry list, so the build must derive the copper entries from the declared layers. Otherwise a 4-layer package would silently lack its inner layers: KiCad exports only the layers asked for |
| Drill at 4 layers | `export drill --excellon-min-header` on `b4` is **byte-identical** to the 2-layer reference's (through vias only, the same holes) |
| `FAB-NOTES.md` | `eda/src/fab.rs` L86 prints "Two copper layers: F.Cu (top) and B.Cu (bottom).", a literal. At 4 layers it would lie. It must print the declared layer order with each layer's role and plane net, and the declared stackup table with its bounds and sources |
| `MANIFEST.json`, `--against-fab` | Both enumerate what the package writes. Once the Gerber list follows the declared layers, `--against-fab` names a missing inner Gerber as a missing file and a stackup mismatch in `FAB-NOTES.md` as a differing file. Neither works today because both stop at `route::read` |
| The job file (`.gbrjob`) | Not recommended. It carries a wall-clock `CreationDate` (JSON line 8), a place `RUL-03`'s amended rule does not name, so including it needs a new amendment. Worse, it says nothing the fabricator needs that the package would not already state. The deciding reasons: **the gate discards the board's `(setup …)`, so no declared stackup can reach the job file** through the package's export, and the layer order is already in every Gerber's `%TF.FileFunction`. (With my partial block its `MaterialStackup` printed no thickness either, S1.) |

**What a fabricator needs at 4 layers that the package lacks:**
- the inner Gerbers;
- the layer order (in `%TF.FileFunction` and in `FAB-NOTES.md`);
- the stackup, as the fabricator's named stackup or as thickness and copper bounds;
- the finished thickness;
- which inner layers are planes, on which net.

P5.1, P5.2 and P5.3 hold.

### S6 — scope check

`grep -rnE 'BOARD_THICKNESS|thickness|F\.Cu|B\.Cu|In[1-4]\.Cu|\bLayer::|stackup' crates --include=*.rs` finds hits in `aethelgard-eda` and `aethelgard-cli` only:
- eda `src`: `pcb.rs`, `route.rs`, `extract.rs`, `fab.rs`, `strict.rs`;
- cli `src`: `fab.rs`;
- 12 test files across the two.

There are no hits in `aethelgard-ir`, `-twin`, `-evolve`, `-chaos`, `-svd-hal` or `-kicad-lib`, nor in `bridge/` or `boards/`. The CLI's other `kicad_pcb` mentions are paths (`generate.rs` L107, `verify.rs` L625, `main.rs` help text).

The twin, the analog deck (`cli/src/analog.rs`), the campaign and `evolve` read the solved topology and the declared net constraints, never a board's copper. **Nothing outside the fence changes.** P6.1 holds.

---

## Predictions against results

| Id | Prediction | Result |
|---|---|---|
| P1.1 | inner ids 4, 6 (8, 10), type `signal` | **held** |
| P1.2 | no stackup required; none added on save | **held** |
| P1.3 | DRC and Gerbers ignore the stackup; only the job file carries it | **half:** ignored by DRC and Gerbers, but the job file carries **no** thickness either; and the gate drops `(setup …)` anyway |
| P1.4 | inner clearance reported; via through a plane not a violation | **held**; inner short also reported (`shorting_items`) |
| P1.5 | `In1_Cu.g1` …, `Copper,L2,Inr`, clock lines at 2 and 9 | **held**, all 5 runs byte-identical once stripped |
| P2.1 | `schema_version` 2, v1 refused, fixtures listed | **held** (proposal) |
| P2.2 | stackup in the specification as bounds | **held** (recommendation): JLCPCB publishes no tolerance |
| P2.3 | Furnace cannot read the fabricator pages | **wrong**: JLCPCB, OSH Park and PCBWay all reached, values read |
| P3.1 | layer index, per-layer sets, planes as stitched zones | **held** (probe) |
| P3.2 | 4 layers route the congested variants; no escape planning | **half:** D1 routes at 4; D3 does not (plane fan-out at the LQFP), so escape planning is still needed for D3-class boards |
| P3.3 | 6 adds nothing over 4 | **held** (worse by default on D1) |
| P3.4 | check/connectivity learn per-layer, via on every layer, plane joins | **held** (probe) |
| P4.1 | elliptical cap; equals today at h = S | **held**, derived and checked |
| P4.2 | ≈345 fF/mm at 0.1 mm | **held**: 345.9 at 0.1, 346.8 at 0.0994 |
| P4.3 | 7628 slightly under S with tolerance | **untested as stated:** JLC 7628 is 0.2104 nominal with no published tolerance; OSH's 0.1797 minimum gives +0.4 % / +0.5 % |
| P4.4 | SDA 50–55 pF, SCL 62–68 pF at 0.1 mm | **close:** 49.0 / 61.9 pF (slightly lower) |
| P4.5 | inner refusal enough only if I2C stays outer | **held**: the 6-layer probe routes I2C inside |
| P5.1–P5.3 | verify cannot take a 4-layer board; fabricator needs order and stackup; job file excluded | **held** |
| P6.1 | nothing outside the fence | **held** |

## Recommendation per question

- **S1. Adopt KiCad's measured 4- and 6-layer tables.** No inner-layer check of the gate's own is needed. `RUL-03`'s two-line rule covers the inner Gerbers as it stands. The board's `(setup …)`, and with it any stackup, stays discarded by the gate.
- **S2. Specification `schema_version` 2 with a required `board` section, shaped as above.** The stackup is declared in the specification as bounds with sources. Version 1 is refused by name. One version bump is shared with `4200`.
- **S3. Build 4 layers (S-P-P-S) with planes for ground and each declared domain, through vias, and the probe's generalisation.**
  - Do not build 6 layers in this work order. It adds nothing measurable, routes worse by default, and needs inner-layer extraction.
  - Escape planning is a separate work order, if D3-class boards matter.
- **S4. Build the elliptical-cap bound,** keyed on each outer layer's adjacent dielectric minimum.
  - Keep today's integer path when `h_min ≥ S`.
  - Put the derivation in the module documentation, with a test against the exact stripline.
  - If 6 layers are ruled in, add the two-cap inner case rather than refusing inner I2C.
- **S5. Derive the Gerber list and `FAB-NOTES.md`'s layer and stackup text from the declared board.** Keep the job file excluded.
- **S6. No change outside the fence.**

## Findings

| Finding | Where | Severity | Type | Recommendation | Decision needed |
|---|---|---|---|---|---|
| **F1.** `validate` accepts a specification that `solve` and `emit` refuse: the pre-fix D1 (no net constraint on its buses) gives `validate` exit 0, `{"ok":true,…}`, and `solve`/`emit` exit 2, `MissingBusConstraint` "I2C bus `B0` has no net constraint on `SDA0`". Measured on `main`'s binary. `RUL-14` says `validate` refuses what `generate` refuses; that sentence was written about identifiers, so this may be by design | `aethelgard-ir` validate vs solve | low | `main`, not this work order's | rule whether `validate` should run the solver's bus checks; not fixed here | yes |
| **F2.** A 4- or 6-layer board passes the DRC gate (`check drc --spec`, exit 0) while `route::read` refuses it (`BoardUnreadable`, "a zone on layer In1.Cu"), so `extract` and the package refuse it. This fails closed today; the build must make the readers follow the declared layers | `cli check drc` / `eda::route::read` | info | measured | the build's §2.2–2.4 | no |
| **F3.** D1's generator existed only in the 1302 reviewer's session transcript; it is now `spike/4100/s3/gen.py`, which the build deletes with `spike/` | 1302 B10 | low | provenance | at the build, the D1 specification becomes a committed fixture (§2.7) | no |
| **F4.** A board's stackup is inert: the gate discards `(setup …)`, DRC and the Gerbers ignore it, and the job file printed no thickness for the block tried | `eda::board::prepare_read` | info | measured | the stackup lives in the specification only | no |

## Decisions needed (spike) — ruled on PR #120

Ruled by the architect on PR #120: D1 (a), D2 as recommended (with the published-tolerance rule), D3 (a), D4 not applicable, D5 confirmed, D6 this work order's, D7 confirmed and narrowed, D8 as proposed, D9 left as history. The build below follows the ruling.

1. **D1. Layer counts in scope:**
   - **(a)** 2 and 4 now, with 6 deferred to `4500` or a successor. Recommended.
   - **(b)** 2, 4 and 6, which also needs the two-cap inner bound and a 6-layer fixture.
2. **D2. The stackup's home:** in the specification as bounds with sources, recommended, or named from a fabricator table.
3. **D3. 1302 B10.** D1 routes at 4 layers. D3-class boards (24 I2C + 12 SPI) still fail at 4 layers at the MCU's supply and ground pads.
   - **(a)** Accept D1 routing at 4 layers as B10's closure, and open escape planning as its own work order. Recommended.
   - **(b)** Build escape planning here.
4. **D4. Inner-layer I2C (only if D1 is (b)):** the router keeps I2C on the outer layers, or extraction bounds inner pieces with the two-cap form. I recommend the bound.
5. **D5. Sentinel-01's declared stackup** reproduces today's constants (nominal 1600 µm, finished 340–2000 µm, one dielectric ≥ 200 µm, 70 µm copper, 5.4), so its package and extraction stay byte-identical. Confirm.
6. **D6. Version bump sharing with `4200`:** whichever lands first takes specification `schema_version` 2, and the other merges onto it.
7. **D7. Which nets are planes.** The proposal makes each plane name its net, ground or a declared domain, one plane per net. A board with more domains than plane layers routes the extra domains as tracks, as today. Confirm.
8. **D8. Units:** micrometres and thousandths in the specification, as proposed, or nanometres like the code.
9. **D9. `AET-WO-1200.repro/` specifications:** migrate them, or leave them as history.

---

# Build (§2), to the ruling on PR #120

**Ruled:** D1 (a) 2 and 4 layers, 6 deferred; D2 stackup in the specification as bounds with sources, the 4-layer fixture on OSH Park's published stackup; D3 (a) with `D1` and `D3` committed as fixtures; D4 n/a; D5 confirmed; D6 this work order takes specification version 2 (`4200` is not ruled to bump it; it had not landed, so no merge was needed); D7 narrowed (exactly one `GND` plane at 4 layers, the other a declared domain); D8 micrometres and thousandths; D9 `AET-WO-1200.repro/` left as history (no test reads it). The rules field and the further requirements are below, item by item. `spike/4100/` is deleted; the spike tables above stay, and the probes are in this branch's history (`3549976`).

**Where everything ran.** Host: `cargo` under a capped user scope (8 GiB). KiCad, Renode and tier-2 tests: the pinned tools image under rootless podman, `--memory 8g --memory-swap 8g --pids-limit -1`, `TMPDIR` unset, one container at a time; every container's `memory.max=8589934592 memory.swap.max=0 pids.max=max` was read back after start (`~/.cache/aet-4100/caps-build.log`).

## What was built

| Item | Where | What it does |
|---|---|---|
| §2.1 `board` section, schema 2 | `aethelgard-ir/src/board.rs`, `lib.rs` step 13 | `copper_layers` (2 or 4), `layers` (KiCad's names in order; `role` signal or plane; a plane's `net`), `stackup` (finished thickness nominal/min/max, one dielectric per adjacent pair, copper maximum, permittivity maximum, each with a `source`), `rules`. Every key required (`INV-06`). Refusals by name: `UnsupportedLayerCount`, `LayerTable`, `LayerRole`, `PlaneNet`, `PlaneLayout`, `DielectricTable`, `FinishedThickness`, `StackupValue`, `InvalidSource`, `RuleBelowFloor`, `UnsupportedViaRule`, each with `path()` where it names a field. A version-1 file is refused as `UnsupportedSchemaVersion` before its shape is read (`cli/src/main.rs` `parse_spec`), at every entry that reads a specification. Seven committed specifications migrated; ten inline test specifications migrated (by a subagent; spot-checked) |
| The rules field | `eda/src/rules.rs`, `stack.rs`, `route.rs`, `extract.rs` | `Rules::from_specification` reads the declared clearance and track width; the verdict's label is `"specification"` (was `"engine-floor"`; I named it here as the ruling asks). The router routes with them (every reach is derived from `Board::stack`), the reader requires tracks of the declared width, and extraction's `S` and `W` are the declared ones. Measured: a 0.21 mm track width routes the 4-layer fixture and the reference with every track 0.21 mm (tier-1 test); 0.25 mm clearance on the reference fails closed, `Unroutable` (the LQFP's own pads are 0.2 mm apart); via drill and diameter other than 0.3/0.6 mm are refused by name |
| §2.2 writer, router, DRC files | `pcb.rs`, `route.rs` | The layer table carries KiCad's inner rows (`In1.Cu` 4, `In2.Cu` 6) between `F.Cu` and `B.Cu`, as KiCad saves them; `(general (thickness …))` is the declared nominal. At 4 layers each pad of a plane net is joined by an F.Cu stub to a through via in its plane (today's ground stitching, generalised); the planes are unfilled zones the gate refills (`RUL-03`); there is no B.Cu zone, so B.Cu routes only. A plane's core is cut only by foreign vias; a fence of vias that splits a plane is ripped up like today's zone fence. `connectivity` judges each zone or plane as one piece reaching every via of its net, and every via joined on at least two of the layers it crosses. The gate-written DRC files follow the declared rules |
| The reader follows the specification | `route::declared`, `route::read`, `extract::read`, `cli/src/fab.rs` | A board whose copper layer table (by KiCad id, whatever the name says), zones/planes or thickness is not the declared one is refused by name: `LayersDisagree`, `PlanesDisagree`, `ThicknessDisagrees`. An inner-layer track, a zone on an undeclared layer, a via other than the engine's through via are refused. F2's state (DRC passes, the reader refuses) is closed by teaching the reader; the gate still discards the board's `(setup …)` (F4) |
| §2.3 extraction | `extract.rs` (module doc: the derivation) | The elliptical cap for a piece whose next copper layer is nearer than `S`, keyed on each outer layer's adjacent dielectric minimum (F.Cu pieces and pads: the first dielectric; B.Cu tracks: the last). Implemented as today's shell plus the exact rational `cap = (S − h)·[…]/(72·113·h)`, which is 0 at `h = S` by its first factor. **When the minimum is at least `S`, today's integer path runs unchanged.** Via height: **the declared maximum finished thickness**; the quarter is dropped, because the maximum bounds the barrel and both pads (it is in the board), and the reference declares 2.0 mm, today's height. `CONDITIONS` became `conditions(&Stack)`: at 2 layers the same words, from the declared values; at 4 a line naming every dielectric minimum is added |
| §2.4 package | `cli/src/fab.rs` `gerbers`, `eda/src/fab.rs` | The Gerber list derives from the declared layers (`In1_Cu.g1`, `In2_Cu.g2` after `B_Cu.gbl`, creator line 9 as measured). `FAB-NOTES.md` at 4 layers prints the layer order with each plane's net, every dielectric minimum with its source, the thickness's and copper's sources, and the rounding rule (minima down, maxima up). At 2 layers the words are today's. The job file stays excluded |
| §2.6 fixtures | `cli/tests/fixtures/four-layer.json` (+ `four-layer-pullup.json`), `d1-24-i2c.json`, `d3-24-i2c-12-spi.json` | OSH Park 4-layer: dielectrics 179/891/179 µm (published value less published tolerance, rounded down), copper **70 µm** (the engine's stated condition; was 44 µm, the published 1.7 mil rounded up, a nominal: see the review fix below), finished 1.250–**1.851** mm (dielectric minima summed; every published dielectric and the solder resist at their maxima plus four copper layers at 70 µm; was 1.692), permittivity 5.4 (IPC). `D1` and `D3` are the 1302 reviewer's generator, recovered in S3, on that board |

## Acceptance (§4), measured

| Clause | Result |
|---|---|
| **Sentinel-01 byte-identical at 2 layers** (D5) | Emitted board, schematic and BOM: **identical** to `main`'s (cmp). `extract` output: identical except `input.path` and `specification_sha256`. `verify --fab-out` package, `main` (`a71bef5`) and this branch built and run in the image side by side: **13 of 15 files identical** (all 9 Gerbers, drill, placement, BOM, ELF); `FAB-NOTES.md` differs only in the specification's sha256; `MANIFEST.json` only in the hashes that carry it (`specification_sha256`, `topology_sha256`, `verdict_sha256`, and `FAB-NOTES.md`'s). The committed tree `boards/sentinel-01/**`: **8 files change, one line each**, the header line naming the specification's sha256. **Cause, named (§2.5):** the reference specification's bytes changed (`schema_version` 2 and the required `board` section, D5/D6), so its sha256 is `a277c933…` instead of `acd60081…`, and every artifact that names it changes there and nowhere else. The pinned solve and svd-hal digests moved for the same reason; the migration subagent checked that substituting the old hash into the new outputs reproduces every old pin |
| Old and new extraction paths equal on the reference (D5) | `stackups_tier1::the_old_and_new_paths_agree_on_the_reference`: every term of both lines equal; bounds 46,800 / 58,235 fF as `AET-WO-1305` |
| **4-layer fixture passes `verify --fab-out`** | In the image: every gate passes (ERC, DRC with schematic parity by the specification's rules, extract, prove ×2, build, simulate, campaign); the package carries `In1_Cu.g1` (GND region) and `In2_Cu.g2` (VDD_3V3 region); `--against-fab` derives it identical; a missing inner Gerber and a `FAB-NOTES.md` dielectric changed from 0.179 to 0.25 mm are each named (`board_tier2.rs`, 2 new tests, run in the image: 4 passed with the reference's two package tests) |
| Five-export determinism on the gate-prepared 4-layer copy (ruling) | Five `verify --fab-out` runs, 09:48 to 09:58: all 17 files byte-identical across the five, verdict `fab.files` identical; `strip_clock` found the two clock lines at lines 2 and 9 of both inner Gerbers every time |
| Plane gap on the gate-prepared copy (ruling) | From the package's inner Gerbers: smallest gap from plane copper to a foreign via's 0.6 mm disk **0.2005 mm** on In1 (47 foreign vias) and on In2 (45). **This corrects S1's 0.5005 mm**, which came from an unprepared copy (KiCad's default zone clearance); under the gate the fill keeps the declared clearance. The via term needs at least `S` and DRC judges it, so the bound stands; FEEDBACK S1 above keeps the unprepared number as measured |
| The reference's I2C margin at 4 layers (integer path; evidence for Sentinel-02, not a gate) | The 4-layer fixture as routed (its I2C routes are shorter than the 2-layer reference's: SDA 18.1 mm vs 24.3, SCL 29.4 vs 43.5): at OSH Park's 0.179 mm, **SDA 45,857 fF, SCL 48,827 fF** (was 44,953 / 47,792 with 44 µm copper and a 1.692 mm maximum); the same board judged at JLC 3313's 0.0994 mm (nominal, no tolerance published; evidence only), **SDA 47,688, SCL 51,327** (was 46,784 / 50,293). Declared 100,000 fF: margin ≥ 49 %. (S4's float estimate on the 2-layer reference's longer routes, 49.0 / 61.9 pF, is not this board) |
| A malformed `board` refused by name, per field | `aethelgard-ir/tests/board.rs` (12 tests), `stackups_tier1::a_version_one_specification_is_refused_by_name` (validate and emit) |
| 1302 B10, `D1` | `stackups_tier1`: `D1` routes at 4 layers (0.6 s release) and fails closed at 2 (`Unroutable /SDA0` after 500 rip-ups); `D3` fails closed at 4 naming its net (`VDD_3V3`, blocked by `/CS0_0`, `/S1000_HOLD`, `GND`): the escape-planning work order's target |
| Gates (`CLAUDE.md`) | host: `cargo fmt --all -- --check` ok; `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo test --workspace` **1,109 passed, 0 failed** (tier 2 compiles and runs 0 on the host). Image: tier-2 build with `--features aethelgard-cli/with-tools` ok |
| `TIER2_MIN` | **92**: `main`'s 90 (after `AET-WO-1909`, merged in as `7bf199e`) plus exactly the 2 new tier-2 tests (91 → 93 before that merge). `ci/test-weights.tsv`: 7 rows added (the 2 tier-2 tests at 460 s each, estimated from the reference's own two-run fab tests at 416–516 s on CI; 5 tier-1 at their host times). No refit: nothing was measured above the refit rule's thresholds yet |

## Adversarial pass (full, §5), by a fresh subagent

**The reviewer:** a fresh Opus subagent in its own worktree at `564b814`, given the work order, the ruling and the code, not this FEEDBACK. Evidence: `~/.cache/aet-4100-adv-evidence/` on Furnace. Six image containers, one at a time (the first logged `CAPS NOT READ`: it ran only `kicad-cli version` and five refusals that start no tool; the other five read `memory.max=8589934592 memory.swap.max=0`). **I spot-checked each finding below by reproducing it before fixing it**, and each fix has a test that fails without it (mutation run below).

| Id | Attack (§5) | Result at `564b814` | Severity | Now |
|---|---|---|---|---|
| A1 | §5.1: a board that lies about its stackup, judged by `check drc --spec` | **Broke.** `extract` and the package refused every lie by name, but `check drc --spec` compared the board with the declared board nowhere: the 2-layer reference passed under the 4-layer specification and the reverse, planes swapped passed, and a GND via typed `blind` or `micro` (F.Cu–In1.Cu) passed, each with 0 findings and `rules: "specification"` | high | **Fixed.** With `--spec`, `check drc` runs `route::declared` on the strict reading before any tool, refusing by name (exit 3): `LayersDisagree`, `PlanesDisagree`, `ThicknessDisagrees`, and the new `ViaNotThrough` (a via typed, or not `F.Cu`–`B.Cu`). `route::read` refuses a typed via too. Test `stackups_tier1::check_drc_with_spec_refuses_a_board_that_is_not_the_declared_one`. Two fixtures that ran `check drc --spec` on hand-written boards gained the declared layer table (`emit_tier1` `LOOSE_BOARD`, whose subject is the rule files; `with_tools` `LOOSE10`, `LOOSE` in KiCad 10's format, run in the image: pass). The corpus test (43 edits, `drc-parity` pinned) still passes in the image unchanged |
| A2 | §5.2: under-bound by overflow | `femtofarads` multiplied by the permittivity's numerator before dividing; an irreducible value (4.663) gave `Overflow` on a 3.3 mm pad. Refused by name, never a wrap, but a refusal at a realistic input; new in this work order (εr was the constant 5.4) | low | **Fixed.** When the exact product overflows, the same value in two divisions, each rounded up (only adds); the exact path is unchanged, so 5.4 gives today's integers. Test `an_irreducible_permittivity_still_bounds_a_large_pad` |
| A3 | §5.5: 2-layer behaviour against `main` | `connectivity` counted ground's B.Cu zone in a third slot, so a GND via met only by a B.Cu stub and the zone counted as joined on two layers; `main` refuses it (`NetLabelsDisagree`) | low | **Fixed.** Ground's zone is B.Cu copper (slot B); a plane is a layer of its own. Test `a_ground_via_met_only_on_b_cu_is_not_joined_on_two_layers` |
| A4 | §5.6: router against KiCad | A plane on a declared domain no pad draws from passed `validate`, `emit` and `connectivity`; KiCad reports `isolated_copper` (failed closed at DRC) | low | **Fixed** earlier than DRC: `connectivity` (and so the router) refuses a plane no via reaches. Test `a_plane_no_via_reaches_is_refused` |
| — | §5.1 other lies | plane on the wrong net, layer table renamed/reordered/added/dropped/typed `power`/id `04`, zone duplicated/filled/keepout/hatched/`connect_pads`/on B.Cu, a board `(stackup …)`: each refused by name by `extract` (and the board's setup is discarded by the gate, F4). An inner-layer track: all 11 seeds caught by DRC (dangling, short, clearance) and refused by `extract` | — | held |
| — | §5.2 the bound | thinner dielectric, higher permittivity, more copper, larger maximum thickness: each raised both lines' bounds. `per_mm_ff`, `segment_ff`, `pad_ff`, `via_ff` never fell below an f64 evaluation over 894,891 draws (rounding and overflow only, same formula); **independent**: a direct quadrature of the trial for a disk below `z = 0` matches `lower(A,p,h)` to 3 × 10⁻⁶ | — | held |
| — | §5.3 the specification | about 75 mutations, each refused by name with its path or id; a version-1 file through ten entries (validate, solve, emit, generate, verify ×3, extract, check drc, a campaign base): each exit 2 `UnsupportedSchemaVersion`. Passing, by design: planes swapped between `GND` and the domain (D7 allows either order), a 1 µm dielectric (a bound, conservative), 10 mm clearance (fails closed in routing), inline Markdown in a source (no newline possible) | — | held |
| — | §5.5 2-layer identity | its own build of `a71bef5`: `emit` identical for the reference and `two-bme`, `extract` identical but the specification hash, FAB-NOTES text identical; `generate` differs only in the hash line | — | held (and A3) |
| — | §5.6 `D1` at 4 layers in KiCad | DRC with parity: pass, 0 findings | — | held |
| I1 | info | `route::read` does not judge a zone's polygon (a 1 mm² plane passes connectivity), on `main` too | info | finding F7 below |
| I2 | info | "Laminate: FR-4" printed at 4 layers, while the fixture's core is FR408HR | info | fixed: at 4 layers the notes say the laminate is as the dielectrics' sources state; 2 layers unchanged |
| I3 | info | the package path reports a `declared` mismatch as a `ToolError`, not by its kind | info | as intended: the package's board is the generated one, bound by hash, so a mismatch there is an engine defect; the kind is in the message |
| I4 | info | a thinner dielectric can lower one piece's bound by about 1 fF when `S` ≥ 500 µm (the cap's `S − 2h` term); not at 200 or 300 µm. Not an under-bound: every value is a valid bound for its own `h` | info | noted |
| — | §5.4 package lies, extra file | not run by the reviewer; covered by the tier-2 tests here (missing inner Gerber, lying stackup) and by the reference's own extra-file test | — | covered |

**Mutation run** (one mutant per guard this work order adds, against the suite it lives in; `~/.cache/aet-4100/mutate.py`, at `eb28da7`): **12 of 12 killed**:
- `declared`: the via type, the layer table, the planes and the thickness;
- `connectivity`: the B.Cu slot and a plane with no via;
- extraction: the cap path under `S` and the staged fallback;
- `check drc` judging the declared board;
- `validate`: the rules floor and the one GND plane;
- the package's inner Gerbers.

**Reduction pass (diff): −10 net (`564b814` +23 −25, `1311f5e` +4 −12) · 2 duplicates removed · 0 dead paths removed · 1 protected kept · 0 findings below.**
- KiCad's copper-layer ids were stated in `pcb.rs` and in `route.rs`. They now live in one function, `stack::kicad_id`.
- The zone-layer check was stated in `route::read` and again in `route::declared`. Every caller runs `declared` first, and `connectivity` judges the zones again, so `read`'s copy went.
- Kept: `stack::reference()`, a literal copy of the reference's declared board for eda's unit tests (eda has no JSON dependency, which `workspace_shape` pins). `stackups_tier1::the_reference_declares_todays_two_layer_board` asserts it equals `Stack::of` on the file, so it cannot drift.

Each removal was proven with the same suites and the boards' bytes (the reference and the 4-layer fixture emit identical before and after).

## Gates, final tree

- **Host, at the final tree:**
  - `cargo fmt --all -- --check`: ok.
  - `cargo clippy --workspace --all-targets -- -D warnings`: clean, with and without `--features aethelgard-cli/with-tools`.
  - `cargo test --workspace`: **1,114 passed, 0 failed**.
- **Image, each at the commit named** (none at the final tree; CI is the final tree's tier-2 run):
  - at `adfcffa`'s tree (before A1–A4, both reductions and the merge): the two 4-layer tier-2 tests and the reference's two package tests, 4 passed;
  - at `017b38c` (A1–A4 applied): the corpus test and the loose-board gate test, passed;
  - at `4912049`–`adfcffa`: `verify --fab-out` on the 4-layer fixture ×5 (determinism), the D5 package comparison against `main`.
- **CI:** the runs on the pushed heads are on the PR; their shard times and `tier2_tests_passed` are reported there, not here.
- **The migration subagent's work** (`c7096e9`), checked by me: I reproduced the reference solve pin, `53ac16ac…` from `solve` on the new file, and `c655bf2e…` (the old pin) after substituting the old specification hash, byte for byte. Changes it made beyond the hash: two tests whose input was a bare `{"schema_version":1}` now say 2, so they still reach the shape error (`SchemaViolation`) they test rather than the new version refusal (`generate_tier1` `prove_with_spec_refuses_…`, `validate_cli` `solved_topology_document_exits_2`); the CLI test that expected `extract`'s `BoardThickness` now expects `ThicknessDisagrees` (a caller-visible rename at 2 layers, named in the CLI README).

## Findings

| # | Finding | Where | Severity | Type | Recommendation | Decision-needed |
|---|---|---|---|---|---|---|
| F1 | `validate` accepts what `solve` refuses for a missing bus constraint (spike) | `main` | low | backlog | ruled: roadmap backlog, not this build | no |
| F5 | **The specification's sha256 is in every artifact that names the specification**, so a required schema change moves 8 committed generated files (one line each), the solve and svd-hal pins, `MANIFEST.json`'s hashes and `FAB-NOTES.md`'s hash line, although nothing they generate changed. Every future schema bump will do the same | `generate`, `fab`, pins | info | design | none here; if it should not, the generators could name the solved topology's hash instead (it changes only when a solved value does). A ruling, not this work order's | yes (low) |
| F6 | **0.2005 mm, not 0.5005 mm**: the plane's gap to a foreign via under the gate is the declared clearance; S1's figure came from an unprepared copy. The bound needs only ≥ `S` and DRC judges it | `extract` conditions | info | correction | none: S1's row stays as measured; this row corrects its reading | no |
| F7 | `route::read` does not judge a zone's outline (I1): a 1 mm² zone passes the router's connectivity; KiCad judges the fill. On `main` too | `route` | low | pre-existing | backlog: compare each zone's polygon with the outline | no |
| F8 | `D3`-class boards (24 I2C + 12 SPI flashes) fail closed at 4 layers on the MCU's adjacent supply and ground pads | `route` | info | ruled (D3) | the escape-planning work order; `d3-24-i2c-12-spi.json` is its target | no |
| F9 | The two new tier-2 tests' CI weights (460 s each) are estimates from the reference's two-run fab tests, not measured alone on CI | `ci/test-weights.tsv` | info | process | refit by the rule in `ci/shard.sh` if a shard passes 20 minutes or the spread passes 6 | no |

## Decisions needed

1. **D5 as delivered (confirm).** The ruling says byte-identical to `main`. Delivered: byte-identical except the specification's sha256 and the hashes that carry it (8 one-line changes in `boards/`, `MANIFEST.json`'s hashes, `FAB-NOTES.md`'s hash line), because the reference's own bytes had to change (version 2 and its `board`). The work order allows a named change (§2.5), so this is not a §7 stop; I ask it be confirmed.
2. **"A via joined on each layer it crosses" (reading).** Built as: every via is met by its net's copper on at least two of the layers it crosses (tracks on F.Cu or B.Cu, or its net's zone or plane). A through via crosses all four layers but a signal via cannot touch a plane of another net, so "each layer" read literally refuses every signal via. Confirm the reading.
3. **D2 and the fixture's maxima** (ruled in the review: fix them; done below). OSH Park publishes tolerances for its dielectrics (used) but **none for copper or the finished thickness** (the 1.6 mm is "nominal"). So the fixture's copper maximum, 44 µm, is the published 1.7 mil rounded up (the ruling's "outer copper as published"), and its finished maximum, 1.692 mm, is my own sum of every published layer at its maximum with copper at nominal. That maximum is also the via-height bound. Accept these two as declared, or name a source with a tolerance (IPC-6012's copper and thickness classes) for the fixture.
4. **F5 (low):** whether generated artifacts should keep naming the specification's sha256 (so every schema bump touches them), or name the solved topology's.
5. **The PR title.** `gh-write` has no edit command, and a minted token is spent only through it (Furnace contract), so I cannot retitle PR #120 from here. Proposed title: `AET-WO-4100: stackups at 2 and 4 layers — the board a specification declares, routed, judged, bounded and packaged`. The architect sets it at merge (roadmap §6), or `gh-write` gains `edit`.

## The review's fix (`~/.cache/aet-4100-review.md`)

**Ruled in the review:**
- D5 as delivered: confirmed.
- The via reading: confirmed as built.
- F5: keep naming the specification's hash.
- F7, F9 and an IPC-6012 copper class: to the backlog.
- The title: set by the architect.

**The one change, the fixture's maxima (D2).** OSH Park publishes no copper tolerance, so the fixture's 44 µm was a nominal value, not a bound.

| Fixture field (`four-layer.json`, and the same board in `d1-24-i2c.json`, `d3-24-i2c-12-spi.json`) | Before | Now |
|---|---|---|
| `copper_max_um` | 44 (1.7 mil rounded up; a nominal) | **70**, sourced as the engine's stated condition (`AET-WO-1305`, as the reference: 2 oz, twice KiCad's 35 µm), not a fabricator tolerance |
| `finished_thickness_um.max` | 1692 (copper at nominal) | **1851**: 2 × 0.2201 (prepreg maximum) + 1.0897 (core maximum) + 2 × 0.0203 (solder resist maximum) + 4 × 0.070 (copper condition) = 1.8505 mm, rounded up; the source says so |
| `finished_thickness_um.min`, dielectrics, permittivity, rules | unchanged | unchanged |

**Re-measured.** The board is byte-identical: no routing input changed. Only the bound's inputs did: copper height, and via height, which is the declared maximum. Both terms rose, as they should:

| Measurement | Before | Now |
|---|---|---|
| SDA bound at OSH's 0.179 mm (trace / vias / pads / pins, fF) | 44,953 (8,081 / 5,336 / 1,536 / 30,000) | **45,857** (8,532 / 5,716 / 1,609 / 30,000) |
| SCL bound at OSH's 0.179 mm | 47,792 (10,920 / 5,336 / 1,536 / 30,000) | **48,827** (11,502 / 5,716 / 1,609 / 30,000) |
| The same board at JLC 3313's 0.0994 mm (evidence only) | 46,784 / 50,293 | **47,688 / 51,327** |
| `extract`'s copper condition, and `FAB-NOTES.md`'s | "finished copper at most 44 um thick; finished board from 1.25 mm to 1.692 mm thick" | "… at most 70 um …; … from 1.25 mm to 1.851 mm thick" |

Two test files pin these values, and both were updated to the new numbers:
- `stackups_tier1` (the bounds and the condition);
- `board_tier2` (the notes' lines).

Results: host `cargo test --workspace` 1,114 passed, 0 failed; the two 4-layer tier-2 tests were re-run in the image on this tree: 2 passed in 530.75 s, four `verify` runs of about 132 s each. CI on the pushed head is on the PR.
