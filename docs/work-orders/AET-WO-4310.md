# AET-WO-4310 — P4 slice D: power, package and coverage honesty

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main` after `AET-WO-4411` merges, named in the kickoff.
**Branch:** `wo/aet-4310-power-coverage`.
**Author:** Claude (architect) · **Builder:** strong tier (the `4411` builder, who knows the power tree), with a full adversarial pass.
**Phase:** P4, slice D (roadmap §5, "The remaining P4 work, as slices"). Written 2026-11-26. **Cadence:** a measurement, a stop, then the build. One more stop if (1) moves a committed verdict.

**Grounds.** Roadmap §7.1, the rows tagged **P4 slice D**, cited by their text. Exit clause 2 needs each one: a P4 verdict that passes while one of these is open would be a wrong pass.
- **Power:**
  - "The supervisor's floor rule covers bus devices only" (`4411` Q-SUP).
  - "The droop's band check against the loaded floor (less load × source resistance), not the declared floor" (`3303` F5).
  - "Other domains' return currents in the shared ground copper" (`1303b`).
  - "IR's static headroom counts no input-path drop" (`2304b` K6).
- **Package:** `4411`'s FEEDBACK rows:
  - the capacitance ladder's rung and the pour step reach neither the verdict nor FAB-NOTES;
  - FAB-NOTES carries no dimension line.
- **Coverage:**
  - "A covering tolerated fault must move its quantity past the declared tolerance" (`3303` F3);
  - "`3303`'s coverage rule counts SENSORS as covered by `pass` scenarios with no modelled device on the bus" (`4201` F5);
  - "A bus whose only devices are unmodelled counts as covered" (`4600` F4);
  - "Coverage has no motor-output target and no fault kind reaches one" (`4600` F3).
- `INV-03`, `INV-05`, `INV-06`, `INV-08`; `RUL-13`, `RUL-20`.

> **What this is.** One work order for the slice, built as six parts. Each has its rule, a planted board or scenario that fails by name, and a mutant per branch.
> - **(1) Measure first, predictions committed in `spike/4310/`.** For every committed board (the 25 `INV-08` boards, Sentinel-01, Sentinel-03 and the bench packages), state what each part below would change: which verdicts move, which new findings appear, and by how much each margin moves. **Stop** with the table. A new rule that fails a committed board is a finding about that board. Say which board, why, and what fixing it would take. It is never a reason to soften the rule.
> - **(2) The supervisor floor for every powered part.** `supervisor::floors_above_reset` judges every part with a declared minimum operating supply against the supervisor's release, not only bus devices. That covers the module, the flash, the sensors and the translator (`4411`). A part with no declared minimum is named, never passed.
> - **(3) The supply bounds, loaded.**
>   - The droop band is judged against the loaded floor: declared minimum less load × source resistance.
>   - The static headroom counts the input path's I·R drop.
>   - The ground bound counts other domains' return currents in shared copper, or names each one it cannot count.
>   - Each figure comes from the solved topology and held data, with its source.
> - **(4) The package says what it was built at.**
>   - The capacitance ladder's rung and the pour step go through `extract` into the verdict, and into FAB-NOTES' binding.
>   - FAB-NOTES gains the board's outline as built, in millimetres, from the copper.
>   - Both strict readers take the new fields, and a package that lacks them is refused by name.
> - **(5) Coverage that means what it says.**
>   - A tolerated fault covers its quantity only if it moves that quantity past the declared tolerance.
>   - A bus with no modelled device, or with only unmodelled ones, is never counted as covered by a fault nothing witnesses. It is named as uncovered.
>   - Motor outputs become a coverage target, with at least one fault kind that reaches one (an output stuck or open), judged by the observer edges `4300a` already witnesses.
>   - `campaign_faults_every_bus_and_rail` is true only when all of that holds.
> - **(6) One reduction pass over `supervisor`, the supply bounds and the coverage rule,** since each grew by accretion.
>
> It is NOT:
> - a change to any board's design to make it pass (a row, or the board's own work order);
> - a new part or a new fault mechanism in the twin beyond (5)'s output fault;
> - flight in the twin (slice C);
> - Sentinel-02's specification (slice E).

## 1. Build

1. **Measure (1).** Stop with:
   - the table, by board and by part;
   - each new finding on a committed board, and what fixing it would take;
   - the cost in CI time.
2. **Build (2)–(6) to the ruling.** The committed packages are re-derived by the write dispatch on your head. Name every byte that moves and why. Merge `main` before READY.

## 2. Acceptance

- **(2)–(5):** each rule refuses its planted board or scenario by name, and passes its clean twin. Kill a mutant per branch.
- **Committed boards:**
  - every committed board's verdict either is unchanged, or changes as (1)'s ruling says, named;
  - `INV-08`'s boards are byte-identical except where the ruling names a field;
  - both bench packages pass `--against-fab` after the write dispatch.
- **A package without (4)'s fields** is refused by `verify --against-fab`, by name.
- **CI is green.** Every new test is in a shard at its measured weight, and `bench-package-s03` stays inside `1919`'s target.

## 3. Fences

- **Fence:**
  - `aethelgard-ir`'s power and coverage types (`INV-01`);
  - `aethelgard-cli`'s `supervisor`, the supply-bound and ground-bound code, `extract`, `verify`'s coverage rule and the fab package writer and readers;
  - the campaign's fault kinds for (5);
  - the planted boards and scenarios;
  - `spike/4310/`;
  - `docs/work-orders/AET-WO-4310.*`.
- **Not:**
  - the router and placer;
  - the twin's models, except (5)'s output fault;
  - `ci/` beyond weights;
  - the steering documents.
- One PR, left unmerged. Write `~/.cache/aet-4310/pr-title-body.md`.

## 5. Adversarial pass, full

- **A rule that passes by omission:** a part, a bus, a domain or a quantity that the new rule does not see, and so does not judge.
- **A margin computed from the wrong number:** a declared value where the loaded or measured one is required, or the reverse.
- **Coverage claimed by a fault that moves nothing,** or that nothing in the twin witnesses.
- **A package field the reader accepts when absent.**

## 7. Flag-and-stop conditions

- **After (1),** with the table.
- **A committed board fails a new rule.** It's in (1)'s table, ruled before any build.
- **A figure the rule needs is held nowhere.** Stop with the part and the page you'd need.
