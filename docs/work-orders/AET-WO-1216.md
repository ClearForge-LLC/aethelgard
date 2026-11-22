# AET-WO-1216 — Spike: the power sequence. Which domains can be up while the MCU is not, what that puts on its pins, and how a specification should say it

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1216-power-sequence`.
**Author:** Claude (architect) · **Builder:** strong tier, with a targeted adversarial pass.
**Phase:** P4, before the P4 baseline's bench. Written 2026-11-22. **Cadence:** a design spike that stops for the schema ruling. No crate code.

**Grounds** (`1211e` F1 and F5, roadmap §7.1):
- **`1211e`'s A1 judges every pull with the MCU powered.** No specification declares a power sequence.
- **What DS12117 Rev 10 states for an unpowered MCU:**
  - Table 10 (p. 103): FT inputs at `Min(Min(VDD, VDDA, VDD33USB, VBAT) + 4.0, 6 V)`, which is **4.0 V** at VDD = 0;
  - note 1: every main power and ground pin "must always be connected to the external power supply, in the permitted range";
  - §3.5.1 (p. 27) sequences only the supplies;
  - Table 11 note 4 (p. 104) calls any VIN above VDD "positive injection", and Table 48 limits the injected current.
- **The bench risk:** a board whose motor rail comes up before 3V3 can hold a pin-device output, pulled to a domain above 4.0 V, on an unpowered MCU pin beyond its rating.
- Read `1211e` FEEDBACK §5 (F1, F5), `4300d`'s power-tree ruling (two LMR33630 bucks from the battery, the LDO, `crossings`) and `1211a`'s pin-device schema first.

> **What this is:**
> - **A measurement** on every committed specification with a power tree or a pin device:
>   - every path from a domain other than `mcu.supply` onto an MCU pin: a pull, a divider, a pin device's output, a crossing, a connector pin;
>   - for each, the most the pin can see while `mcu.supply` is below 1 V, with its source.
> - **What the held parts state about order:** for each regulator, its enable, power-good and soft-start behaviour, page-cited from its held datasheet. Whether the declared tree already fixes an order (an EN wired to a PG), and where it cannot (two bucks from one battery, each self-enabled).
> - **Options for the schema,** each with its cost and the files it would touch. At least:
>   - (a) a declared order, `power.sequence`, with a source per step, judged by A1 at each step;
>   - (b) the order derived from the tree's EN/PG wiring alone, with an undeclared or racing pair refused by name;
>   - (c) no sequence; every off-domain path to an MCU pin judged at VDD = 0 (4.0 V), the most conservative reading.
> - **A lean, with its reason, and a stop** (§7).
>
> It is NOT:
> - any crate change, schema change or specification bump. That is the build the ruling orders.
> - a dynamic power-up simulation in the twin;
> - USB's VBUS (`4300e1`'s) or VBAT, unless the measurement finds a path through them, which is then reported.

## 1. Measure

1. **Predictions first,** committed in `spike/1216/` before any run: which committed boards you expect to have an off-domain path, and how many.
2. **The paths,** by a scratch reader of the solved topology and the drawn netlist, cross-checked by hand on Sentinel-02, Sentinel-03, D1 and D2.
3. **The parts:** each regulator's start-up and enable facts, page-cited. Never assume a soft-start time; a figure no held page states is "not stated".
4. **Option (c) applied today,** in scratch: which committed boards it would refuse, and why.
5. **The options and the lean** in FEEDBACK.

## 2. Acceptance

- **Every committed board's off-domain paths are listed,** each with its worst pin voltage at VDD < 1 V and its source, or "not stated".
- **Each option names what it refuses** on the committed boards, measured, not estimated.
- **The lean names its reason** and what it leaves unjudged.
- **No crate file changes.** The gates are not run on a spike-only branch; say so.

## 3. Fences

- **Fence:** `spike/1216/` and FEEDBACK. Scratch readers live under `spike/1216/`, built against `main`'s crates as a path dependency, never inside a crate.
- **Not:** any crate, specification, `fab/`, workflow or `ci/` file.
- **Running beside you:**
  - `1211c` (twin, chaos, `verify`, four IR solved fields);
  - `1606` (bridge, `doctor`, `build`);
  - `1211f` (eda, the bulk capacitor).
  None conflicts with a spike-only branch.
- One PR, spike-only, left unmerged. Write `~/.cache/aet-1216/pr-title-body.md`.

## 5. Adversarial pass, targeted

- **A path the reader misses:** a connector pin, a crossing's low side, an ESD diode's clamp, a pin device's open-drain output pulled elsewhere.
- **A start-up figure that no held page states.**
- **An option whose refusal count was estimated, not run.**

## 7. Flag-and-stop conditions

- **Stop after §1.5 with the options and the lean.** The schema is the architect's and Scotty's ruling.
- **A committed board already exceeds 4.0 V on an unpowered MCU pin by a path no option judges.** Stop at once and report it: it is a bench risk now.
