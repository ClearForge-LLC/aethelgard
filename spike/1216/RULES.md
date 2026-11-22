# AET-WO-1216 — the options as rules, committed before they ran

`judge.py` is the rule; this page is its summary. Both were committed before the first run of `judge.py`. The paths it judges are walk 1's rows (`out/spec/*.tsv`), which walk 2 (`out/drawn/*.tsv`) reconciles on every drawn board (`out/compare.tsv`).

- **The limit at VDD = 0** (DS12117 Rev 10 Table 10, p. 103, rendered and read): BOOT0 9.0 V; every other pin 4.0 V. FT_xxx is `Min(Min(VDD, VDDA, VDD33USB, VBAT) + 4.0, 6 V)`, which is 4.0 V at 0; TT_xx is 4.0 V; any other pin is 4.0 V. Table 11 note 3 (p. 104): positive injection "is not possible on these I/Os and does not occur for input voltages lower than the specified maximum value". So a voltage at or under the limit injects nothing by the datasheet's own words, and voltage is the whole test.
- **A path** is a row whose source can be up while `mcu.supply` is below 1 V on what the specification declares (`can_lead` = yes): the battery, a domain with no declared source or a sibling of `mcu.supply`, an off-board end (connector, test point, ESC, foreign clock).
- **(c) VDD = 0 for every path.** `c1` refuses a stated worst above the limit and leaves an unbounded path unjudged. `c2` also refuses every unbounded path (`OffDomainUnbounded`, `INV-06`'s reading).
- **(b) the order from EN/PG wiring alone.** A domain follows `mcu.supply` only if it is downstream of it or its EN is wired to a PG on `mcu.supply`'s chain. Otherwise it is `SourceUndeclared` (no tree) or `RailsRace` (a sibling). The battery leads by definition and is judged as in `c1`. `b1` refuses every undeclared or racing domain. `b2` refuses one only when it reaches an MCU pin, by a row on its rail or a connector it powers.
- **(a) a declared `power.sequence`.** None is committed.
  - `a1` requires one of every board with a domain other than `mcu.supply`, or with a tree.
  - `a2` requires one only of a board with a path from a non-supply rail.
  - `a-sidecar` judges a hand-declared sequence (`sequences/<board>.json`, for the four hand-checked boards). It refuses a step with an empty source, and judges every path led by a rail in a step at or before `mcu.supply`'s at VDD = 0.
