# AET-WO-4411 — A radio module as a part: an ESP32 WiFi module on a declared serial port, its power and straps judged, its antenna keep-out honoured, and its firmware named as not judged

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-4411-radio-module`.
**Author:** Claude (architect) · **Builder:** strong tier, with a full adversarial pass.
**Phase:** P4. Written 2026-11-24. **Cadence:** a measurement, a stop, then the build.

**Grounds:**
- **Scotty, 2026-11-24:** "Path A to start. Path B at phase 5."
  - **Path A:** a pre-certified ESP32 WiFi module on the flight-controller board, linked to the STM32 by a UART. WiFi's range is for testing, not the end goal; the long-range link comes later on the same kind of port.
  - **Path B** (two microcontrollers co-designed) is P5's (roadmap §5 P5, northstar §3.1). This work order must not become it.
- **The roadmap's standing ruling:** "One microcontroller family, and no second processor on the board in v1" (§5 P4). A module is a part with its own certified radio and its own firmware. Aethelgard judges its power, its pins, its link and its copper, and **states that the module's firmware is not judged**. That keeps the ruling intact. `INV-06` applies: the gap is named, never filled with a default.
- **Why a module and not the bare chip:** the module's RF section (crystal, matching and antenna) is certified as a unit. A bare chip's RF layout is antenna and certification work that a later antenna capability would own (a roadmap row).
- **`AET-WO-4400h`** (#393): ArduPilot armed over MAVLink on a UART. The module is where that MAVLink will come from on the real board. `AET-WO-4400i` builds the twin's side of the same link.
- **`RUL-03`'s note** (2026-11-15, `4300d2` D2): a keep-out zone inside a vendored footprint is admitted and judged by DRC. A module's antenna keep-out is that case.
- `INV-01`, `INV-02`, `INV-06`, `INV-07` (cite them; do not restate them).

> **What this is:**
> - **(1) Measure first, with predictions committed:**
>   - **Pick one module.** Criteria: a held datasheet (fetch it from the vendor; record the sha256); a footprint in KiCad's own `RF_Module` library (vendored as `RUL-03`'s libraries are); an antenna keep-out drawn or dimensioned in the datasheet; an onboard PCB antenna, and if the family offers one, a u.FL variant named beside it.
>   - **The module's datasheet facts the board needs,** each cited by page:
>     - the supply range and its peak current during transmit;
>     - the enable and boot straps, and their required levels at reset;
>     - the UART pins and their levels;
>     - the antenna keep-out, and the placement rule at the board edge;
>     - the ground pads.
>   - **How ArduPilot is told the port speaks MAVLink** with no defaults file (`4406`): measure whether the emitted hwdef can declare the port's protocol and baud as compiled defaults, and whether `check pinout` judges it. If it cannot, the port is declared and the protocol arrives over MAVLink at run time (`4400i`). Say which.
>   - **Which committed board carries it.** Sentinel-03 is in fabrication readiness (`AET-WO-4300k`) and about to be ordered. Adding the module there moves its package. Measure what a derived `sentinel-03-wifi` example costs, against adding it to Sentinel-03 itself. **This is a stop question for Scotty**, not a builder's choice.
> - **(2) The part** (after the ruling):
>   - a specification key for a radio module: its part, its serial port, its power rail and peak current, and its straps. The specification version bumps (`INV-01`);
>   - the solver gives the module its UART pins and checks the straps' levels at reset;
>   - the supply gate counts its transmit peak;
>   - the schematic and layout emit it from the vendored footprint, with its keep-out, placed at the board's edge as its datasheet says;
>   - DRC judges the keep-out: copper, tracks or pours inside it fail;
>   - the verdict carries one line naming the module, its firmware as **not judged**, and its radio as **not simulated**.
> - **Each refusal comes with a test, and each test with a mutant it kills.**
>
> It is NOT:
> - a second microcontroller (P5's Path B);
> - a bare-chip RF layout, an antenna design, or any RF simulation;
> - the module's firmware;
> - the long-range radio (a later work order on the same kind of port);
> - the twin's MAVLink injection (`AET-WO-4400i`).

## 1. Build

1. **Measure** (1). **Stop** with:
   - the module chosen and why, with its datasheet's sha256;
   - the fact table, each fact cited;
   - the ArduPilot protocol answer;
   - Sentinel-03 or a derived board, with the cost of each;
   - the specification key as proposed.
2. **Build** (2) to the ruling. Merge main before READY.

## 2. Acceptance

- **A board with the module** passes validate, ERC, DRC and the supply gate, and its verdict names the module's firmware as not judged.
- **Planted failures, each refused by name:**
  - a pour or track in the keep-out;
  - the module placed away from the board's edge, where its datasheet requires the edge;
  - a strap at the wrong level at reset;
  - a rail that cannot supply the transmit peak;
  - the module's UART on pins the solver did not give it.
- **Every committed board without a module** is byte-identical before and after (`INV-08`).
- **The CLAUDE.md gates pass, and CI is green,** with every CI guard run locally first (`ci/route.sh`, `ci/test-shard.sh`, the shard plan).

## 3. Fences

- **Fence:**
  - `crates/aethelgard-ir` (the key, its refusals and the version bump);
  - the solver, the schematic and layout emitters, `check drc`, the supply gate and the hwdef emitter, where the module touches them;
  - `crates/aethelgard-kicad-lib` (the vendored footprint and symbol);
  - `examples/` (the derived board, if ruled);
  - their tests and goldens;
  - `spike/4411/`, `docs/work-orders/AET-WO-4411.*`.
- **Not:**
  - the twin, `renode.rs` and `renode/foreign.rs` (`AET-WO-4400i`'s);
  - `fab/` (a moved package is a stop);
  - the steering documents.
- **Running beside you:** `AET-WO-4300k` (fab readiness, `validate`, `fab/`) and `AET-WO-4400i` (the twin's MAVLink). Add, never rewrite, and merge main before READY.
- One PR, left unmerged. Write `~/.cache/aet-4411/pr-title-body.md`.

## 5. Adversarial pass, full

- **A keep-out that leaks:** a pour on an inner layer, a via under the antenna, or a ground pour beside it that the datasheet forbids.
- **A strap that is right only sometimes:** a level set by a pull that another net overrides, or a strap pin shared with a UART pin.
- **A power budget that hides the peak:** an average current declared where the datasheet gives a transmit peak.

## 7. Flag-and-stop conditions

- **After §1.1,** with the measurement.
- **A committed board's package or verdict moves.** Stop with the diff.
- **The datasheet does not dimension the keep-out.** Stop, and name the document that would.
