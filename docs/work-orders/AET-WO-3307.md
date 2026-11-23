# AET-WO-3307 — A BMP581's witness counts as evidence in attribution

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-3307-bmp581-witness`.
**Author:** Claude (architect) · **Builder:** standard tier, with a targeted adversarial pass.
**Phase:** P4, backlog wave 1. Written 2026-11-23. **Cadence:** a measurement, then a build with one stop.

**Grounds** (roadmap §7.1, `3306` F1; read `3306` FEEDBACK first):
- A collide onto a BMP581 stays I4. `aethelgard-evolve`'s `witness_fields` reads only `Write XX`/`Read XX`, and the BMP581 model logs `read 0x01 -> 0x50`, so no BMP581 witness is ever evidence.
- Sentinel-03's U7 and Sentinel-02's barometers are BMP581s.
- `3306` pins the case at I4 with `a_collide_onto_a_bmp581_stays_i4_because_no_bmp581_witness_is_evidence`.

> **What this is:**
> - `witness_fields` reads the BMP581 model's witness form, from the line format the model emits today.
> - The pinning test is inverted to the attribution the evidence now supports.
> - A test that fails without the change.
>
> It is NOT:
> - a change to the C# model, whose log line is pinned by sha256;
> - a change to any other part's attribution.

## 1. Build

1. **Measure first,** with predictions in `spike/3307/`: every committed seeded attribution case, today and after the change.
2. **Stop if any committed case other than the pinned BMP581 case re-attributes,** or if a case on the bench board re-attributes, since its verdict is in the bench package.
3. **Build:** the change, its tests, and a mutant it kills.

## 2. Acceptance

- **The BMP581 collide case** attributes by its witness.
- **Every other committed case** is unchanged, shown by the sweep.
- **The CLAUDE.md gates pass, and CI is green.**

## 3. Fences

- **Fence:** `crates/aethelgard-evolve` (`witness_fields`, `tests/attribute.rs`); `spike/3307/` and FEEDBACK.
- **Not:** `models/renode`, the twin, chaos, the CLI, the IR; workflows, `ci/` (except a test-weights row).
- **Running beside you:** `4300i`, `4407`, `1607`, `2309`. Merge `main` before READY.
- One PR, left unmerged. Write `~/.cache/aet-3307/pr-title-body.md`.

## 5. Adversarial pass, targeted

- **A witness read from a line another part's model also emits.**
- **A malformed BMP581 line** taken as evidence.

## 7. Flag-and-stop conditions

- **Another committed case re-attributes, or a `fab/` byte moves.** Stop with the diff.
