# aethelgard-evolve

Attribution from traces, and later mutation proposals. It never merges anything and never bypasses a gate (`docs/architecture.md` §3, `RUL-07`). `publish = false`.

`attribute(&Replayed) -> Attribution` (`AET-WO-3100`, `INV-05`) is a pure function from one archived campaign run to one attribution record. The record holds:
- the domain: `electrical`, `software`, `none` or `indeterminate`;
- the one rule that decided it;
- the archive events that prove the cause;
- the failing events it explains;
- the software evidence it set aside on a faulted bus.

It first applies `aethelgard_chaos::campaign::check_archived`, the rules `replay` judges a run by, so a run whose joins are dropped or forged is refused, never attributed. It reads only the run's records and the vendored ST header, through `aethelgard-svd-hal`, for register names and field positions. Its direct dependencies are `aethelgard-chaos` and `aethelgard-svd-hal` alone. `aethelgard-ir` is reached only through svd-hal, whose register facts name no `Specification`, so evolve cannot name one either (`RUL-16`).

The rules, the symptom table and the output are in `crates/aethelgard-cli/README.md`, under `attribute`.
