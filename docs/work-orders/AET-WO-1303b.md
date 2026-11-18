# AET-WO-1303b — Extraction's build: supply and ground resistance from the regulator's pad to each MCU supply pin, the stackup minima it reads, and the USB pair's capacitance

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1303b-extraction-build`.
**Author:** Claude (architect) · **Builder:** strong tier (the `1313d` builder, reused), with a full adversarial pass.
**Phase:** due before `4500` (roadmap §7.1, "Extraction, first step, then full"). Written 2026-11-18. **Cadence: build, with one measured stop (§1.1).**

**Grounds:**
- **The ruling,** comment 6888926396 on PR #133 (the `1303` spike, merged as the record):
  - **D4:** pads are equipotential; a pass through a pad is priced; the source is the regulator's output pad; the drop is per-pin R × the declared total load (DS12117 Table 120: 658 mA at Tj 125 °C).
  - **D5 (a):** the closed-form pieces, each refusing by name, with coverage reported per pin.
  - **D7:** the ground return is bounded the same way, and the drop limit applies to the VDD and VSS sum.
  - **D3 and D6:** fields only for what a ruled gate reads, each with a source or refused by name when a gate needs it:
    - outer copper minimum;
    - plane copper minimum;
    - plating minimum;
    - the temperature.

    k = 1.181 is cited (IPC-MF-150F §3.8.1.2); its use on plated copper is a stated condition. 125 °C is the load's Tj. No nominal is declared as a minimum (`RUL-14`).
  - **D2 (b):** the USB full-speed device budget, ≤ 25 pF per conductor, from the existing upper-bound machinery widened to the pair. The < 10 % mismatch is a stated condition of symmetric routing, named as unverified.
  - **D1 (c):** no impedance bound. Closed forms are never a bound in this engine.
- **Its preconditions now hold:**
  - `AET-WO-1308` merged (#142): per-net widths, the via-pad option, coupled pairs.
  - `AET-WO-4300d2` merged (#149): the power fixture emits its regulators, so a regulator output pad exists to be the source.
  - `1308`'s ruling took D5 (a) (comment 6891478212).
- **The §7.1 rows owned by this build:**
  - a through-hole barrel priced as S4 prices a via's;
  - a relieved pad on a supply path refused until its spokes are priced (`1309` D2);
  - a cited current-to-width source for power classes (`4300d2` K11, A7);
  - a stackup bound below the fabricator's published minimum copper (`copper_max_um` is accepted down to 1).

> **What this is:** the supply-path bounds `1303`'s ruling scoped, built from the spike's closed-form pieces (`spike/1303/` at `d5bc8416` is the oracle), read by a `verify` gate.
>
> It is NOT:
> - a field solver, the certified discrete flow (D5 (b), unless §1.1 rules it in), or an impedance bound;
> - a router change (that is `1308`'s, merged);
> - USB's static checks (`4300e1`, unwritten): this order delivers the quantity they will read, tested on the existing pair fixture.

## 1. Build

1. **Measure first, then STOP.** On every specification in the corpus with a regulator (the power fixture at two and four layers, and any other):
   - each MCU supply pin's path from the regulator's output pad, and each VSS pin's return;
   - whether D5 (a)'s closed-form pieces bound it, using the router's via-pad option and supply widths where the specification can declare them;
   - the bound in mΩ, with the minima stated as symbols (R ≤ ρ_max·(a/t_out + b/t_plane + c/A(t_p))), and the coverage per pin.

   **STOP** with:
   - the coverage table;
   - each refused pin with the piece that refused it;
   - the specification fields you propose, with names, units and the schema version;
   - the verdict's shape (`supplies` beside `lines`);
   - every golden that would move. Sentinel-01's bounds must not move.

   **If coverage is below every pin** with the router's remedies applied, D5 (b) is the ruling's own trigger: say so with the numbers, and do not build the discrete flow before the ruling.
2. **After the ruling:**
   - the supply and ground R bounds, each with its derivation in the module documentation and an exact-case test (the spike's closed forms);
   - the minima and temperature fields, refused by name when a gate needs one and it is absent (`INV-06`);
   - `verify` gates the VDD + VSS drop against the declared limit;
   - the USB pair C (D2), on the existing pair fixture, with the mismatch condition named in the verdict;
   - each §7.1 row above, closed or re-owned with a reason.

## 2. Acceptance

- **Each bound** has an exact-case test, and a mutant that a test kills.
- **The power fixture** gets supply and ground bounds, or named refusals, per pin, as ruled at §1.1.
- **Sentinel-01's existing bounds are byte-identical,** and so is every board without a regulator.
- **A missing minimum** is refused by name when the gate reads it, and never defaulted.
- **The CLAUDE.md gates pass.** CI is green, and `TIER2_MIN` is exact.

## 3. Invariants and fences

- **Invariants and rulings:** `INV-03`, `INV-06`, `INV-07`; `RUL-03`, `RUL-12`, `RUL-14`.
- **Fence:**
  - `aethelgard-eda` (`extract.rs`, and `stack.rs` for its inputs);
  - the CLI's `extract` and `verify`;
  - the specification's new fields and its schema version, as ruled;
  - tests and fixtures;
  - FEEDBACK.
- **Running beside you:** the Renode upgrade (`1605`, `container/` and the twin). You share no file with it.

  You run KiCad: one container of yours at a time, under `capped-run`.
- One PR, left unmerged. Write `~/.cache/aet-1303b/pr-title-body.md`.

## 5. Adversarial pass, full

- **A bound that is not admissible:** a constant from the wrong side, a nominal used as a minimum, a pad assumed and not priced.
- **A gate that passes with a field absent.**
- **A board whose copper the reader cannot bound,** passing silently instead of refused.
- **Sentinel-01's bytes moving.**

## 7. Flag-and-stop conditions

- **§1.1.**
- **A quantity the gate needs has no honest bound** from the copper the reader can read: stop. It is declared as a condition, or the gate is dropped (`1303` §7).
