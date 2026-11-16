# AET-WO-4211 — The W25Q64JV is NRND: its lifecycle cited, and every board that uses it acknowledges it

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-4211-flash-nrnd`.
**Author:** Claude (architect) · **Builder:** strong tier, with a targeted adversarial pass.
**Phase:** backlog (roadmap §7.1), written 2026-11-16. **Cadence: build only.** It runs once `4300g` and `4230` have merged, because it touches the fixtures they renumber.

**Grounds:**
- **`parts.json`** (`crates/aethelgard-ir/data/parts.json`, the `W25Q64JVxIQ` row) says `"lifecycle": "active"`. Its citation is `pending`: Winbond's catalog page loads its status column by script, so the status was not held (2026-11-15).
- **Scotty's evidence, 2026-11-16, held read-only** in `furnace:~/.cache/aet-ds/scotty-2026-11-16/` (README there):
  - `winbond-W25Q64JV-catalog-export-2026-11-16.csv` (sha256 `f10526ae…`), Winbond's own catalog export: **Status-Industrial = N (NRND)** for W25Q64JVSSIQ and every industrial code;
  - `winbond-W25Q64JVSSIQ-longevity-2026-11-16.jpg` (`7091ccdc…`): Longevity Y, WPLP since 2016/03;
  - `winbond-W25Q64JVSSIQ-status-2026-11-16.jpg` (`73aa6c75…`): the status as N.
  - DigiKey shows the part as "Active". The maker's word wins, and the row says so.
- **`4210`'s machinery** (PR #143, Scotty's D1): a part that is not `active` must be named in the specification's `accept_lifecycle`, and every verdict carries a banner; EOL is refused for fabrication. NRND is the middle state, already in `Lifecycle`.

> **What this is:** the row becomes `nrnd`, cited to the export by sha256, file name and column. The reference and every fixture that places the part name it in `accept_lifecycle`, and the verdict's banner says NRND.
>
> It is NOT:
> - choosing Sentinel-02's flash, which is `AET-WO-4500`'s, from parts in mass production;
> - a change to the flash model, its script or its witness.

## 1. Build

1. **The row:**
   - `lifecycle` becomes `nrnd`;
   - its citation names the CSV (file name, sha256, the Status-Industrial column and its value N), with the longevity image as supporting evidence;
   - `pending` is removed;
   - the DigiKey disagreement is noted in the citation's text.
2. **Read what NRND does today.** Find how `validate`, `verify --fab-out` and the banner treat `nrnd`, and check it against `4210`'s ruling D1. It must be acknowledged like EOL, but not refused for fabrication. **STOP** if the code treats `nrnd` differently from that ruling.
3. **The boards.** Every example and fixture that places a `W25Q64JVxIQ` gains `accept_lifecycle: ["W25Q64JVxIQ"]`. About 19 files reference it today; count them at kickoff.
   - Their emitted boards stay byte-identical except the specification-hash lines.
   - Their verdicts gain the banner.
4. **The README quickstart.** If the reference specification changes, check the quickstart against roadmap §6's rule (the pin moves only at a phase close). Report what the quickstart job does with the changed reference, and do not move the pin.

## 2. Acceptance

- A specification placing the part without `accept_lifecycle` is refused by name; with it, it passes with the NRND banner.
- Every example and fixture passes as before.
- Board files are byte-identical except the specification-hash lines, and the list of changed files is in FEEDBACK.
- **The CLAUDE.md gates pass.** CI is green, and `TIER2_MIN` is exact.

## 3. Invariants and fences

- **Invariants and rulings:** `INV-01`, `INV-06`; `4210` D1 (lifecycle).
- **Fence:** `aethelgard-ir/data/parts.json` (this row only), examples and fixtures (`accept_lifecycle` only), tests, FEEDBACK.
- No schema or specification version: the field and the state exist.
- One PR, left unmerged. Write `~/.cache/aet-4211/pr-title-body.md`.

## 5. Adversarial pass, targeted

- `accept_lifecycle` naming the part in another case.
- Naming it twice.
- Naming a part the board does not place.

Each must be refused, or behave as `4210`'s rules say.

## 7. Flag-and-stop conditions

- **§1.2: the code's treatment of `nrnd` differs from `4210` D1.**
- **A fixture's emitted board changes beyond its specification-hash lines.**
