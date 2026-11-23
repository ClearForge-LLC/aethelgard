# AET-WO-4300j — The motor pull-down, weakened and made switchable: a derived value that survives BLHeli passthrough, and a declared either/or fitment

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-4300j-motor-pull-fitment`.
**Author:** Claude (architect) · **Builder:** strong tier, with a full adversarial pass.
**Phase:** P4, before any Sentinel-03 fabrication. Written 2026-11-23. **Cadence:** a measurement, then a build with one stop.

**Grounds:**
- `AET-WO-4300i` FEEDBACK §5 D2 and F7. ArduPilot's BLHeli passthrough sets the internal pull-up on a motor line (`RCOutput.cpp:2300-2313`). Against 4300i's 40.2 kΩ pull-down, that pull-up (DS12117 RPU, 30–50 kΩ) holds the idle line near mid-rail.
- **Scotty's ruling, 2026-11-23:**
  - choose a weaker pull-down, so that passthrough works with it fitted;
  - keep a built-in toggle, so that either fitment is an easy rework, not new copper.
- **Held sources, newly held on Furnace in `~/.cache/aet-held/`:**
  - ST AN4899 Rev 4 (May 2026), `st-AN4899-rev4.pdf`, sha256 `a04463891ce210d5fefe4f9d08ee79ecbe87225728b143ca7fddabfa76e56bd6`. §5.1.5 gives the pull-resistor bound (p. 18): `(VDDmax − VOLmax)/(IOL + Ilkg) < RPU < (VDDmin − VOHmin)/(IIH + Ilkg)`. It states this for a pull-up on an open-drain output.
  - Microchip DS70005664A, the drone ESC reference design user guide (dsPIC33CDVC256MP506), `microchip-DS70005664A-drone-esc-ref.pdf`, sha256 `bef7e27501613f289d31ef0cb3e89ef80015fd3e2f305b7301710ab810bbef39`. §2.1.2 (p. 7) and the J2 schematic give one real ESC's signal input: J2 pin 1 `AD_PWM_IN1`, "PWM or DShot", through `R11` 0 Ω to the dsPIC, with no input pull drawn.
  - A text extract of each is beside it (`.txt`, made with pypdf). Cite the PDF page, never the extract's line.
  - DS12117 Rev 10 (sha256 `607b9ead…`) for RPU, RPD, VIH, VIL and Ilkg, as already held.

> **What this is:**
> - **(1) The value, derived rather than chosen.** Write the bound for a pull-down on a motor line in `spike/4300j/`.
>   - AN4899's inequality is for a pull-up. Its pull-down form is your derivation: label it as one, step by step.
>   - **The floor:** with the line idle in passthrough, the internal pull-up at its weakest (RPU max) against the pull-down must leave the line at or above VIH min, with a stated margin. That sets R_pd's lower bound.
>   - **The ceiling:** at reset, with the pin Hi-Z, `Ilkg` max times R_pd must stay below VIL max of the ESC input, with a margin. No held document states an ESC's VIL, so use the STM32's own VIL as the stand-in and name that as an assumption. That sets the upper bound.
>   - **The value:** pick an E96 value inside both bounds, from a part row that exists in `parts.json` or one you add with a held source.
> - **(2) The rule moves from a band to a bound.** 4300i judges the value inside RPD's 30–50 kΩ. Replace that with the derived bound, held as data with its citations.
> - **(3) The either/or fitment, declared.** Add a series link on every motor pull: a normally-closed solder jumper, or a 0 Ω 0402, whichever eda already draws as a part. Cut it, or leave it unfitted, and the pull is gone with no copper change.
>   - The specification states which fitment the board is built as: `motor_outputs[].pull.fitment`, either `fitted` or `open`. It is required, with no default (`INV-06`).
>   - **`fitted`:** both generators write no internal motor pull, as 4300i built.
>   - **`open`:** the board relies on firmware, so svd-hal writes `PUPDR` pull-down and hwdef writes `PULLDOWN`, as before 4300i. `implied_pupd`, `check pinout` and the witness ledger follow the declared fitment.
>   - eda draws the link on every motor net in both fitments. The BOM marks the link `DNP` when `open`.
> - **(4) Passthrough named in the generated hwdef.** Generate no `HAL_SERIAL_ESC_COMM_ENABLED` override. Write a comment line in the hwdef naming the fitment and the bound it was checked against, so that whoever reads the hwdef sees why passthrough is safe.
> - **These are specification keys,** so they take the next specification version. Each refusal comes with its test, and each test with a mutant it kills.
>
> It is NOT:
> - (b), the off-board drive bound;
> - any pin move;
> - a firmware change to ArduPilot itself.
> - **It is not a decision on an ESC's input threshold.** That waits for a held ESC MCU datasheet. Record it as a row in FEEDBACK.

## 1. Build

1. **Measure first.** Commit predictions in `spike/4300j/` covering:
   - the validate/solve/emit baseline on every committed specification;
   - the derivation of (1), with each number cited by document, page and table;
   - which bytes move, by board.
2. **Stop with the bound, the value, and (3)'s schema shape.** Name the field, its version, and the link part. Then build to the ruling.
3. **Build (1)–(4),** with goldens moved by their generators.
4. **Run the null sweep.** No pin moves.

## 2. Acceptance

- **Every committed motor board** declares a pull-down inside the derived bound, with a declared fitment. The generated firmware's motor pull agrees with that fitment in both generators.
- **A value outside the bound is refused,** and so is a missing fitment, each with a test.
- **eda draws the link on every motor net,** and the BOM's `DNP` follows the fitment.
- **No committed board's pins move.** Every other byte moves only as measured and ruled.
- **The CLAUDE.md gates pass, and CI is green.** If a `fab/` byte moves, queue for a package dispatch.

## 3. Fences

- **Fence:**
  - `crates/aethelgard-ir` (motor, validate, versions, held data, README, tests);
  - `crates/aethelgard-svd-hal`'s generator (motor `PUPDR`);
  - `crates/aethelgard-hwdef` (`PULLDOWN` and the comment line);
  - `crates/aethelgard-pincheck` (`implied_pupd`);
  - `crates/aethelgard-eda` (the link, the BOM's `DNP`);
  - `examples/*.json` and the version literals and goldens a bump moves, by their generators;
  - `spike/4300j/` and FEEDBACK.
- **Not:** the twin, chaos, `verify`; workflows or `ci/`.
- **Running beside you:** the backlog wave in roadmap §5. Merge `main` before READY.
- One PR, left unmerged. Write `~/.cache/aet-4300j/pr-title-body.md`.

## 5. Adversarial pass, full

- **A motor line pulled two ways:** an `open` board whose firmware writes no pull, or a `fitted` board whose firmware still does, on any path (svd-hal, hwdef, the bootloader, `start_outputs`).
- **A bound that cannot fail:** a value just outside each edge must be refused. Plant both.
- **A drawn link or resistor** whose net, value, footprint or `DNP` disagrees with the declaration.
- **A citation that does not say what is claimed:** read each cited page in the PDF, not the extract.

## 7. Flag-and-stop conditions

- **After §1.2:** stop with the bound, the value and the schema shape.
- **No E96 value fits inside both bounds** with the stated margins. Stop with the numbers.
- **A held page does not support a number you need.** Stop, and name the document a ruling would need.
- **Any committed board's pin moves.** Stop with the diff.
