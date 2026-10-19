# CLAUDE.md — how to work in this repository

Read, in this order, before changing anything:

1. `README.md` — what this is and whether it works yet.
2. `docs/northstar.md` — why it exists and the invariants `INV-01` through `INV-07`. Cite them by number; never restate them.
3. `docs/architecture.md` — the shape, the rulings `RUL-01` onward with their reasons, and the open questions.
4. `docs/roadmap.md` — what order things get built in and the exit gate for each phase.
5. Your work order in `docs/work-orders/AET-WO-PSNN[a].md`. It cites the slice of the above it touches.

Rules that hold in every work order:

- Rust for everything in this repository (`RUL-12`). No `unsafe` in any workspace crate.
- `aethelgard validate` and every later subcommand refuse malformed input with a JSON error and a nonzero exit; nothing substitutes a default (`INV-06`).
- Shared specification types live only in `crates/aethelgard-ir` (`INV-01`).
- The steering documents, `README.md`, `LICENSE-*`, and `docs/inputs/**` are protected: findings about them go in your `FEEDBACK.md`, never in a feature branch.
- Gates before any PR: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`.
- Close every work order with the adversarial pass, the reduction pass, then `docs/work-orders/<id>.FEEDBACK.md`, then one PR left unmerged. The builder never merges.
- On Furnace: no `sudo`, no system packages, no credentials anywhere. Push with `gh-write`.

This file points; the documents above define.
