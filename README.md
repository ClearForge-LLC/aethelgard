# Aethelgard

Generates a circuit board and the firmware that drives it from one specification, and refuses to call either finished until the pair has survived simulated electrical and software faults together, before any board is manufactured.

**Status:** phase P0 of 4 in progress. The workspace builds and its first subcommand, `aethelgard validate`, runs; nothing is generated or simulated yet. Licensed `MIT OR Apache-2.0` (see `LICENSE-MIT` and `LICENSE-APACHE`).

## Where to go

| I want to know | Read |
|---|---|
| Why this exists and what must stay true | [`docs/northstar.md`](docs/northstar.md) |
| What it is and how it is shaped | [`docs/architecture.md`](docs/architecture.md) |
| What order it gets built in, and what "done" means | [`docs/roadmap.md`](docs/roadmap.md) |
| The planning conversation these were synthesized from | [`docs/inputs/argon-session/`](docs/inputs/argon-session/) |

## Running it

Check a specification. It prints one line of JSON and exits 0 when the specification is valid, nonzero otherwise; the codes are in [`crates/aethelgard-cli/README.md`](crates/aethelgard-cli/README.md).

```bash
cargo run -q -p aethelgard-cli -- validate < examples/sentinel-01-minimal.json
```

---

This README points; it does not define. The invariants, the rulings, and the phases each have exactly one home above, and a copy here would be a fourth place for the same fact to drift.
