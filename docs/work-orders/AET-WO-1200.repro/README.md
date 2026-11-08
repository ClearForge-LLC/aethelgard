# AET-WO-1200 adversarial reproducers

The inputs for finding **A1** in `../AET-WO-1200.FEEDBACK.md`: two infeasible specifications on which the built search spends its whole 10,000,000-node budget and refuses `BudgetExceeded`, instead of `Infeasible` naming the colliding buses. No test runs them, because they take about 45 s in a debug build. Run them with:

```
aethelgard solve --file docs/work-orders/AET-WO-1200.repro/fuzz-natural-6spi.json
aethelgard solve --file docs/work-orders/AET-WO-1200.repro/core-infeasible.json
```

- `fuzz-natural-6spi.json` was found by random fuzzing, with no hand peripherals.
- `core-infeasible.json` was built by hand.

**Since `AET-WO-1202`** (2026-11-08), every I2C line needs a net constraint, so each line in both files now carries 100 pF and 300 ns. Without them, both refuse `MissingBusConstraint` before the search starts. **Since `AET-WO-1201`** (PR #48), both refuse `Infeasible` over the six SPI buses: 198 and 186 nodes, about 4 ms in a release build. Before it, both ran to the 10 M-node budget and refused `BudgetExceeded`, in 5.5 and 6.9 s.
