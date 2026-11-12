# AET-WO-1203 — FEEDBACK

**Branch:** `wo/aet-1203-electrical-rules` · **Base:** `origin/main` `bf869b4` · **Work order:** `docs/work-orders/AET-WO-1203.md` · **Builder:** Claude Code (Sonnet 5.5) as `forge` on Furnace · **Result:** built to §1 and §3, no §7 stop. One open question for the architect (F1).

Entry schema: `finding · where · type · recommendation · decision-needed`.

| # | Finding | Where | Type | Recommendation | Decision-needed |
|---|---|---|---|---|---|
| **F1** | **The low-rail row of UM10204 table 10 is unconfirmed.** I could not read table 10: the PDF I fetched has compressed, font-encoded text, and no PDF text tool is installed on Furnace. A web search confirmed IOL 3 mA (Standard, Fast) and 20 mA (Fast-mode Plus), VOL 0.4 V, Cb 550 pF for Fast-mode Plus, and that the low-rail condition is VDD ≤ 2 V. It did **not** confirm VOL3 = 0.2·VDD. The adversarial subagent recalled that the VDD ≤ 2 V row carries IOL **2 mA**, not 3 mA. If so, the Standard and Fast floor at 1.8 V is 1440 mV / 2 mA = 720 Ω, not 480 Ω. I built §1.1 as written and did not guess: the 3.3 V rail, which every committed specification uses, is unaffected | `electrical.rs` `i2c_pull_up_floor_ohms`, `sink_microamps` | constant unconfirmed | Someone with the document reads table 10 and its footnotes and rules. If 2 mA applies below 2 V, it is a one-constant change plus the 1.8 V rows of `the_floor_for_each_mode_at_1v8_and_3v3` and `the_pull_up_floor_follows_the_bus_rail_and_mode`, and `AET-WO-3200` follows because it calls the function | **yes** |
| F2 | `DeviceNotPlaced` removed (`AET-WO-2203` F3). `Model::new` puts every I2C and SPI bus in the search, and `solve` assigns every signal of every bus or returns a refusal, so each device's bus `sda` net is always assigned when `i2c_devices` runs. `i2c_devices` no longer returns a `Result`; the lookup is an `expect` naming the invariant. No test named the variant | `solve.rs` | reduction | none | no |
| F3 | `SolvedTopology::validate` does not re-check either rule; only `solve` does. A hand-written topology given to the analog stage without `solve` is unchecked. I did not establish whether any command accepts one | `lib.rs` | gap (theoretical) | Leave: the gate is `solve` (`RUL-16`). Raise it if a command is found that reads a topology file without solving | no |
| F4 | Both rules read the rail as the bus domain's `nominal_millivolts`, as `analog_lines` does. A rail at its tolerance maximum draws more current than nominal; the floor is at nominal by the WO's wording | `solve.rs` `i2c_electrical` | scope | A later work order may judge the floor at `nominal × (1 + tolerance)` | no |
| F6 | `boot_tier1::a_rise_unfinished_in_the_window_or_a_line_stuck_high_is_a_fault` built its "stuck high" specification with a 10 Ω pull-up, which `solve` now refuses (the review's A2 case). The pull-up becomes 967 Ω, the lowest that solves. Every assertion is unchanged: the stub bridge answers what the real bridge answers for a 10 Ω pull-up, and `bridge/tests/edges.rs` still tests the real bridge at 10 Ω. This changes a test because the work order changes what `solve` accepts; it does not weaken one | `boot_tier1.rs` | test follows rule | none | no |
| F5 | Check order: ceilings, then the pull-up floor of each bus, then each bus's SDA and SCL capacitance, then the part check and the pin search. A specification breaking several rules is refused on the first in that order | `solve.rs` | note | none | no |

## Build

- `aethelgard_ir::electrical` (new): `I2cMode`, `i2c_mode`, `i2c_pull_up_floor_ohms` (integer, scaled by 5 so `0.2 × VDD` stays exact, `div_ceil`), `i2c_bus_capacitance_limit_femtofarads`. Names as `AET-WO-3200` calls them.
- `SolveError::PullUpBelowSinkFloor { bus, pull_up_ohms, floor_ohms }` (`id` and `claimants`: the bus) and `BusCapacitanceOverLimit { net, capacitance_femtofarads, limit_femtofarads }` (`id` and `claimants`: the net). The CLI maps both through `kind()`, `id()` and `claimants()`, with no CLI source change; its tests pin the messages.
- `crates/aethelgard-ir/README.md`: refusal rows 1b.
- Seeded set: `cap-470p-sda-2k2` is now `cap-390p-sda-2k2` (SDA 390 pF, 2.2 kΩ). `cases.json` and the case file are updated; no test named the old case by string. Every other seeded scenario has a pull-up of at least 2.2 kΩ and a capacitance of at most 390 pF, so each still passes `solve`.

## Measured

| Seeded case | Old rise (SDA) | New rise (SDA) |
|---|---|---|
| `cap-470p-sda-2k2` (470 pF, 2.2 kΩ) | 876.105 ns (measured, `AET-WO-3100` FEEDBACK) | removed: 470 pF is over the 400 pF limit |
| `cap-390p-sda-2k2` (390 pF, 2.2 kΩ) | none | 726.97 ns **by scaling, not remeasured**: the rise is linear in C, and 186.402 ns at 100 pF measured in 3100 gives 186.402 × 3.9 |

The new case fails its 300 ns ceiling and is attributed `electrical E1` by `attribute_seeded` in the tools image (the tier-2 run, Gates). I did not print its measured rise.

## Tests

- Unit table of the three modes at 1.8 V and 3.3 V, the 2 V boundary (2000 and 2001 mV) and the extremes: `electrical::tests`.
- `solve`: 966 Ω refused and 967 Ω solves at 3.3 V and 400 kHz; 24, 50 and 100 Ω refused; Fast-mode Plus 144/145 Ω; 1.8 V 479/480 Ω; 400 pF solves and 401 pF is refused at 400 kHz; 550 pF solves and 550 001 fF is refused at 1 MHz; `u64::MAX`; the check order. The old `pull_up_and_capacitance_are_not_read` asserted the opposite of this work order and is replaced (it now checks that in-rule values leave the ceiling alone).
- CLI: `solve` exit 2 for a 24 Ω pull-up and for a 401 pF SDA; a campaign with 24, 50 and 100 Ω overrides is refused with exit 2 before any run (`campaign_tier1`).

## Adversarial pass (targeted, fresh subagent, Sonnet 5.5)

The subagent could not break any of the three targets. It swept 100 kHz, 400 kHz and 1 MHz over every rail from 1 to 6000 mV and the extremes, against an independent `u128` computation: no mismatch, including 2000 and 2001 mV. A rail of 0 is refused by `validate` first. A second domain is refused by `SupplyMismatch`. Duplicate nets and constraints are refused by `validate`. SDA and SCL are both checked, and `u64::MAX` is refused. It also found F1 and F3 and, by reading the diff, no change to any emitted byte; the regeneration guard is under Gates. Its report is a claim: I read the diff and the sweep result before adopting it.

## Reduction pass

`DeviceNotPlaced`, its four match arms and the `Result` wrapper of `i2c_devices` are removed (F2). Nothing else in the diff could be cut: each added test pins a distinct row of §3.

## Gates

Run in the pinned image (`container/IMAGE`, digest `2df68259…`) under rootless podman, `--memory 16g --memory-swap 16g --pids-limit -1`, swap then set to 0; read back: `memory.max=17179869184 memory.swap.max=0 pids.max=max`. The CI tools job's steps, at `b75f5dc`: bridge build and tests, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked --offline --features aethelgard-cli/with-tools -- -D warnings`, `cargo test --workspace --locked --offline --features aethelgard-cli/with-tools --no-fail-fast`: **56 suites, 882 passed, 0 failed.** The first run, at `00c406b`, had one failure (F6, fixed). The seeded set (`attribute_seeded`, tier 2) and the regeneration guards pass with no change to any committed generated file, so no emitted byte moved (§7). CI on the PR: see the PR.
