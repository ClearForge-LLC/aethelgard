# aethelgard-evolve

Attribution from traces, and proposals from attributions. It never merges anything and never bypasses a gate (`docs/architecture.md` §3, `RUL-07`). It is published, because `aethelgard-cli` depends on it (`docs/releasing.md`).

`attribute(&Replayed) -> Attribution` (`AET-WO-3100`, `INV-05`) is a pure function from one archived campaign run to one attribution record. The record holds:
- the domain: `electrical`, `software`, `none` or `indeterminate`;
- the one rule that decided it;
- the archive events that prove the cause;
- the failing events it explains;
- the software evidence it set aside on a faulted bus.

It first applies `aethelgard_chaos::campaign::check_archived`, the rules `replay` judges a run by, so a run whose joins are dropped or forged is refused, never attributed. It reads only the run's records and the vendored ST header, through `aethelgard-svd-hal`, for register names and field positions. Its direct dependencies are `aethelgard-chaos` and `aethelgard-svd-hal` alone. `aethelgard-ir` is reached only through svd-hal, whose register facts name no `Specification`, so evolve cannot name one either (`RUL-16`).

The rules, the symptom table and the output are in `crates/aethelgard-cli/README.md`, under `attribute`.

`view(&Replayed) -> RunView` is a read-only view of a run through the same join (`AET-WO-3200` §8): for an event, its bus, and for a register or witness event, each field's solved value against the observed one. `attribute` does not use it, so its output is unchanged.

`propose` (`AET-WO-3200`, `RUL-07`, `RUL-18`) is the proposer interface. A proposer takes an `Attribution`, its evidence through the view, and the solved facts the CLI hands it as plain integers and identifiers: each bus's pull-up, rail and floor, the floor taken from `aethelgard_ir::electrical` by the CLI, because this crate keeps no copy of it. It answers exactly one of a `Proposal` (changes of the form `{bus, pull_up_ohms}`, and no other form exists), a `GeneratorReport`, or `NoProposal { reason }`. Proposers run in a fixed order and the first that answers wins; a proposal stands only for an `electrical` run at or above each bus's floor, and a generator report only for a `software` run, whoever answered. Today there is one proposer, `Rules`, a closed table. Its rows, and `aethelgard propose`, are in `crates/aethelgard-cli/README.md`, under `propose`.
