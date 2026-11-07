# AET-WO-1200 adversarial reproducers

The inputs for finding **A1** in `../AET-WO-1200.FEEDBACK.md`: two infeasible specifications on which the built search spends its whole 10,000,000-node budget and refuses `BudgetExceeded`, instead of `Infeasible` naming the colliding buses. No test runs them, because they take about 45 s in a debug build. Run them with:

```
aethelgard solve --file docs/work-orders/AET-WO-1200.repro/fuzz-natural-6spi.json
aethelgard solve --file docs/work-orders/AET-WO-1200.repro/core-infeasible.json
```

- `fuzz-natural-6spi.json` was found by random fuzzing, with no hand peripherals.
- `core-infeasible.json` was built by hand.
