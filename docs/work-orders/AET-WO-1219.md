# AET-WO-1219 — Audit: how STM32-specific is the IR? Every vendor-bound name listed and counted, with no abstraction built

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1219-stm32-coupling-audit`.
**Author:** Claude (architect) · **Builder:** standard tier, with a targeted adversarial pass.
**Phase:** P4, before the P4 baseline. Written 2026-11-22. **Cadence:** a measurement that stops. No crate code.

**Grounds:**
- **Roadmap §7.1, the STM32-coupling audit row.** It comes from the Grok review of 2026-11-21, §5, and the response's §2.5: designing a seam with no second chip in hand is speculative, so measure first.
- **`RUL-08`:** the RP2350 is the planned second target, *because* it is a different vendor. Northstar §6: a second vendor is what keeps the IR from becoming one vendor's register map.
- **The P4 schema epoch** (roadmap §5) fixes the IR's shape. Coupling found after the epoch is a bump, and a bump is about 50 minutes of package queue. Coupling found before it is a finding.

> **What this is:**
> - **A list** of every place in `crates/aethelgard-ir` (types, specification keys, refusal kinds, held data such as `power-parts.json` and the pin tables) whose meaning is specific to the STM32H7 or to ST. Examples:
>   - alternate-function numbers;
>   - register or bit names;
>   - DS12117 table structures (FT/TT, `io.analog_pads`);
>   - VOS ranges;
>   - RCC clock names;
>   - `mcu.part` strings;
>   - HSE/MCO.
> - **Each entry classed** by one rule, written in `spike/1219/` before the count:
>   - **(V) vendor-neutral in name and meaning;**
>   - **(N) neutral meaning, ST name.** A rename would make it neutral.
>   - **(S) ST-only meaning.** A second vendor has no equivalent, or a different one.
>   - **(D) data, not schema.** It lives in a held table, and a second vendor would add rows, not keys.
> - **For each (N) and (S) entry,** whether the RP2350 has an equivalent. Answer from the RP2350 datasheet if a copy is held, else write "not held". Never answer from memory.
> - **The count:** per class, and per specification key against the version that introduced it.
> - **A lean:** which (S) entries, if any, must change before the epoch, and why. Then stop.
>
> It is NOT:
> - any crate, specification, `fab/`, workflow or `ci/` change;
> - an abstraction, a trait, or a second-vendor model;
> - a rename. Renames are what the ruling may order.

## 1. Measure

1. **The class rule and predictions** (the expected count per class) are committed in `spike/1219/` before any reading.
2. **The list,** read from the source and the held data. Each entry cites a file and line, and for held data, a document and a page.
3. **A scratch reader** (no `Cargo.toml` under `spike/`, as `1216` D6) cross-checks the specification keys against a serde dump of every committed specification, so that no key is missed.
4. **The RP2350 column, the count and the lean** go in FEEDBACK.

## 2. Acceptance

- **Every specification key and refusal kind in `aethelgard-ir` is classed.** The reader shows that none is missing.
- **Every (N) or (S) entry has an RP2350 column:** an equivalent, "none", or "not held".
- **No crate file changes.** The gates are not run on a spike-only branch; say so.

## 3. Fences

- **Fence:** `spike/1219/` and FEEDBACK.
- **Not:** any crate, specification, `fab/`, workflow or `ci/` file. Other crates (eda, twin, svd-hal, hwdef) are out of scope; note a coupling seen there in one line, and do not audit it.
- **Running beside you:** `4300h` (a spike, stopped for Scotty's ruling). Spike-only branches do not conflict.
- One PR, spike-only, left unmerged. Write `~/.cache/aet-1219/pr-title-body.md`.

## 5. Adversarial pass, targeted

- **A key the reader misses:** an optional key absent from every committed specification, or an enum variant no committed specification uses.
- **An entry classed (V) whose meaning is ST's:** a "neutral" name whose valid values are ST's alone, such as a speed grade or a pin-structure letter.
- **An RP2350 answer from memory** where no held document states it.

## 7. Flag-and-stop conditions

- **Stop after §1.4** with the count and the lean.
- **No RP2350 datasheet is held.** Do not fetch one. Write "not held" in every cell, and name the document a ruling would need.
