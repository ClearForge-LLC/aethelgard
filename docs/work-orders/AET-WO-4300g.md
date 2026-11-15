# AET-WO-4300g — The MCU's support network, drawn from the parts table and judged from the netlist

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-4300g-support`.
**Author:** Claude (architect) · **Builder:** strong tier, with a full adversarial pass.
**Phase:** P4. From the second Mythos 5.2 review (`docs/reviews/2026-11-15-mythos-5-2.md`, A2 and B1; response `…/2026-11-15-mythos-5-2.response.md`). **Cadence: build, with one measured stop (§1.1).** It runs after `AET-WO-4300d2` merges, because both change the schematic emitter and the placer's anchors.

**Grounds, on `main` (each checked at source by the architect, 2026-11-15):**
- **A2.** The emitter decouples the MCU once: `crates/aethelgard-eda/src/schematic.rs:644`, `b.decouple(&mcu_ref, &spec.mcu.supply, "100n")`. The reference board's `VDD_3V3` carries 16 `U1` pins and four 100 nF capacitors for the whole board.
  - `NRST` (U1.25) is unconnected.
  - `PA13`/`PA14` (SWDIO/SWCLK) reach no connector, so nothing can program the part.
  - `VDD_3V3` has no source.
  - ERC, DRC, the proofs and the twin all pass it, and `verify --fab-out` writes its package.
- **DS12117 Rev 10 §6.1.6, p. 102, Figure 14 and its notes:** "N corresponds to the number of VDD pins available on the package". "Each power supply pair (VDD/VSS, VDDA/VSSA ...) must be decoupled with filtering ceramic capacitors as shown above."
- **The pattern to follow:** the regulator and translator rows in `power-parts.json` already list `external` parts, and `4300d2` draws and places them by owning pin.
- **DS-009's debug ports** are JST SH (`4300f` FEEDBACK §3.1). The 6-pin "debug mini" carries target voltage, console TX/RX, SWDIO, SWCLK and GND, and no NRST. Its footprint is vendored under `RUL-11` if it is used.

> **What this is:** the MCU's row in the parts data lists its required support network, each value cited from DS12117. The emitter draws it. A gate re-reads the emitted netlist and refuses a board that lacks any of it, independently of the emitter: the support-network `INV-07`.
>
> It is NOT:
> - the regulators' own support (`4300d`, `4300d2`);
> - the crystal (backlog, 4400 D1);
> - a programming tool or flow.

## 1. Build

1. **Read the figure first (measured stop).** Render DS12117 p. 102 (Figure 14) and the NRST, BOOT0, PDR_ON, VCAP, VDDA and VREF+ sections, and transcribe every value with page and figure.
   - The figure's values are in a graphic, so render them; never use decrypting extraction.
   - **STOP** with the transcription if any value the network needs cannot be read from the held document.
2. **The MCU's row gains `external` roles**, as regulator rows have them:
   - one 100 nF per VDD pin, owned by that pin;
   - bulk on VDD;
   - VDDA and VREF+ decoupling as the figure gives them;
   - the two VCAPs (already drawn, now from data);
   - the NRST capacitor;
   - BOOT0's strap;
   - PDR_ON's strap.

   Each is cited.
3. **The emitter draws the network from the row.** The placer anchors each per-pin capacitor by its owning pin, with `4300d2`'s anchors. The distance a gate can check is checked, with its citation. The rest are named conditions.
4. **Two whole-board requirements**, each a named refusal at `validate`:
   - **A programming path.** SWDIO, SWCLK, GND and the target voltage reach one declared connector: a DS-009 debug port, or an explicit pin map (`4300f`'s `standard: null`). Refusal: `NoProgrammingPath`.
     - Whether NRST also reaches it is recorded in the verdict. It is not required, because DS-009's debug ports carry none.
     - Sentinel-01's reference gains a debug-mini connector.
   - **A supply source.** The MCU's supply net is driven by a declared regulator output (`power`) or a declared input connector pin. Refusal: `SupplyHasNoSource`.
     - For Sentinel-01, which has no `power` tree, this is its 3.3 V input pin on a declared connector.
5. **The support gate.** It re-reads the emitted schematic's netlist through the strict reader and judges each role from the row:
   - present;
   - on the right nets;
   - the right value class;
   - its owner within the checkable distance.

   It does not import the emitter's choices. A missing role is a named failure. It runs in `verify` with ERC, and its findings are in the verdict.
6. **FAB-NOTES** names the support network as judged, and lists each named condition.

## 2. Acceptance

- **Sentinel-01's reference:**
  - emits 16 per-pin capacitors, the bulk, VDDA/VREF+, NRST, the straps, a debug-mini connector and a declared 3.3 V input;
  - passes the support gate, ERC and DRC with parity.
- **The power fixture and every other fixture pass.** Each fixture that lacks a programming path or source is refused, by name, until it declares one, and FEEDBACK lists them.
- **Each refusal and each support-gate failure has a tier-1 test.**
- **The README's quickstart pin moves** in the architect's next docs PR, because the reference board changes.
- **The CLAUDE.md gates pass.** CI is green with every shard under 20 minutes, and `TIER2_MIN` is exact.

## 3. Invariants and fences

- **Invariants and rulings:** `INV-01`, `INV-06`, `INV-07`; `RUL-03`, `RUL-11`, `RUL-14`, `RUL-19`.
- **Fence:**
  - `aethelgard-ir`: the MCU row's roles, the two refusals;
  - `aethelgard-eda`: the emitter, anchors, the support gate;
  - `aethelgard-kicad-lib`: the JST SH footprint and symbol;
  - the CLI: `verify`'s gate list, FAB-NOTES;
  - tests and fixtures; `ci/`; FEEDBACK.
- **Next free versions:** take them from `main` at kickoff. The second to merge renumbers.
- One PR, left unmerged. Write `~/.cache/aet-4300g/pr-title-body.md`.

## 5. Adversarial pass, full

1. **Remove one per-pin capacitor,** move one to another pin's net, or give one 10 nF. Each must fail the support gate.
2. **Program-proof the board:** swap SWDIO and SWCLK on the connector, or drop GND from it.
3. **Leave the supply sourceless** by renaming the regulator output's net.
4. **Fool the gate with the emitter:** an emitter mutant that draws a capacitor the gate then accepts on the wrong net must fail.

## 7. Flag-and-stop conditions

- **A value in Figure 14 cannot be read from the held document** (§1.1).
- **A per-pin capacitor cannot be placed within its checkable distance** on the reference board by any placement the placer can make.
- **The JST SH footprint has no vendorable source** under `RUL-11`.
