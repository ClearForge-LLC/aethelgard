# AET-WO-1917 §1.1 — predictions, written before any measurement (2026-11-23)

Each row is stated before it is measured; `measurement.md` records what was found against it.

| # | Item | Prediction |
|---|---|---|
| P1 | Files the `ardupilot-image.yml` jobs read, missing from both `paths:` blocks | `container/IMAGE` (foreign: `docker pull "$(cat container/IMAGE)"`) and `bridge/**` (foreign builds `bridge/Cargo.toml`). Also predicted missing: `container/Dockerfile` or another Dockerfile that `test-check-pins.sh` reads ("the three real Dockerfiles"), and something under `ci/` or `models/renode` that `ci/foreign-judge.sh` reads beyond itself (models/** is listed). `ci/foreign-judge.sh` itself is already listed. 2 to 4 misses in all. |
| P2 | The two `paths:` blocks | identical, item for item. |
| P3 | Generated matrix against the pinout judge's list | the matrix has 5 rows (4 `generated-*` artifacts and 1 `mutant-*`); the judge's hand-written list names the same 4. No miss today; the gap is only that a new row is not judged. |
| P4 | `examples/sentinel-01.json` | `validate` and `solve` pass; `generate hwdef` refuses it, most likely with `SenseNotEmitted` (it declares power sense; `4410` owns that). If it refuses, (3) stops and waits for `4410`. |
| P5 | `route.sh inventory` misses | `format!("docs/work-orders/{}.md", …)` and `"docs/reviews/"` (trailing slash) both produce no row today; also missed: a named placeholder (`"docs/{dir}/x.md"`), a `fab/{board}/ORDER.md` form. Fewer than 5 literals in today's tree carry a placeholder or trailing slash that the new rule would newly report. |
| P6 | `with-tools` test files against `TIER2_RE` | 11 files carry `#![cfg(feature = "with-tools")]` and one (`attribute_seeded.rs`) an item-level cfg; all 12 are in `TIER2_RE`, and all 12 names in it exist. No miss today. |
| P7 | Split-line forms in run `45198295391` shard 5 (attempt 1) | one form: `test NAME ... TOOLTEXT` then a later line `ok` alone. Across the run's other shards, no other form; the existing `TOOLTEXT test NAME ... ok` form appears at least once. |
