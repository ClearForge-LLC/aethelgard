# AET-WO-4300f — Off-board connectors: where flight I/O leaves the board

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-4300f-connectors`.
**Author:** Claude (architect) · **Builder:** strong tier, with a full adversarial pass at the build.
**Phase:** P4. Found at the architect's desk while writing `4300b`, 2026-11-15. **Cadence: spike (§1), then STOP for the architect's ruling, then build (§2).**
- The spike runs now; it writes nothing outside `spike/4300f/`.
- The build waits for `1307` (the router) to merge, because connectors change placement and routing.

**Grounds, read on `main` at `cb44872`:**
- **`aethelgard-kicad-lib` vendors 6 footprints and 10 symbols.** None is a connector. Every net today joins parts on the board.
- **P4's flight I/O leaves the board:**
  - motor outputs (`4300a`);
  - serial ports (`4300b`);
  - power in and battery sense (`4300d`);
  - USB (`4300e1`);
  - and on Sentinel-02, SWD for the factory flash.

  Without a connector, such a net has one pin, and the fabricated board has nowhere to plug anything.
- **The flight-controller world has a published connector standard:** the Dronecode/Pixhawk connector standard (DS-009). It specifies JST-GH 1.25 mm connectors and per-port pinouts (TELEM, GPS, I2C, CAN, POWER, debug). Fetch it, hold it by sha256, and cite it. Do not recall it.

> **What this is:** connectors as declared parts, with standard pinouts where a published standard exists. Each off-board net ends at a connector pin the specification declares; the emitter places connectors where a cable can reach them; and the gates judge them like any other part.
>
> It is NOT:
> - cable harnesses;
> - mechanical enclosure fit;
> - a connector library beyond what P4's Sentinel-02 needs.

## 1. Spike

Probes under `spike/4300f/`. **Predictions go into FEEDBACK before any run.**

**S1. What exists:**
- KiCad 10.0.7's shipped libraries in the tools image: the `Connector_JST` JST-GH footprints (`SM0xB-GHS-TB` and others) and the `Connector_Generic` / `Connector` symbols. Record each one's name, licence (`RUL-11`'s vendoring rule) and sha256.
- The JST-GH datasheet: fetch it and hold it by sha256, with its pitch, current rating and mating height.
- DS-009's port pinouts for TELEM, GPS, POWER and debug.

**S2. What `emit` and the gates do today with an off-board net,** a single MCU pin and no other part:
- the schematic and the board;
- ERC and DRC messages, measured in the image.

That answers `4300a` §1 and `4300b` §1.6, so write the answer into both builders' inboxes (`~/.cache/aet-bus/`) as soon as it is measured.

**S3. The specification's shape.** Propose how a connector is declared:
- **(a)** explicit `connectors`: a part, a standard pinout name or an explicit pin map, and each pin's net;
- **(b)** implied by each I/O group (a serial port's role selects a DS-009 port);
- **(c)** another shape.

Say how `validate` refuses a pin map that contradicts the cited standard, and how ground and power pins on a connector join their domains.

**S4. Placement and routing.** Read how placement works today (`aethelgard-eda`) and propose:
- how a connector is placed at a board edge, oriented for a cable;
- how its nets route;
- what the 2-layer reference and the 4-layer fixture each gain.

Measure one hand-placed JST-GH 4-pin on the reference in a probe: placement legal, routed, DRC clean, or each failure named.

**S5. The twin.** Confirm that a connector is invisible to the twin (it adds no peripheral) and that no witness changes. Name any exception.

**Stop** with:
- the table;
- the recommended shape;
- the vendoring list;
- the decisions needed.

## 2. Build (after the ruling; expected, and the ruling confirms)

1. **The ruled connector parts,** vendored with their licence and sha256, symbol and footprint.
2. **The specification's connectors,** validated against DS-009 where it applies.
3. **The emitter, placement and router** handle them, and every gate judges them.
4. **Sentinel-01 gains the connectors its declared off-board nets need.** Its committed files change by those named consequences only.

## 3. Invariants and fences

- **Invariants and rulings:** `INV-01`, `INV-03`, `INV-06`, `INV-07`; `RUL-03`, `RUL-11`, `RUL-14`.
- **The spike fence:** `spike/4300f/` and FEEDBACK only. Datasheets and standards go in `~/.cache/aet-4300f/ds/`.
- **Other builders on Furnace:** `1307`, `4300c`, `1303`, `4210`, `4400a`. Never touch their worktrees. One container of yours at a time.
- One PR, left unmerged. Write `~/.cache/aet-4300f/pr-title-body.md`. **Push and revoke by the FUR-WO-0301 F7 route.**

## 7. Flag-and-stop conditions

- **DS-009 or the JST-GH datasheet cannot be fetched from Furnace.** Stop with the URL, for Scotty.
- **No vendorable connector footprint fits `RUL-11`.**
