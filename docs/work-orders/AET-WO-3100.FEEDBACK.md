# AET-WO-3100 — FEEDBACK: stopped at §7 before the build

**Status:** stopped on the first §7 condition: *attribution needs a fact the archive does not carry.* No code is written and nothing is pushed. Base `034fcfd`, branch `wo/aet-3100-attribution`.

## S1 · The archive does not say which bus a symptom is on · §7 · decision needed

Every rule in §1.2 is per bus. The E rule says "a symptom **on that bus**". The S rule says "**its bus** has a `RiseMeasured` for every line", and asks for "a `RegisterObserved` mismatch on **that peripheral's** clock gate, GPIO mode or alternate function". The `indeterminate` rule includes "the only fault is on a **different bus** from the symptom", and §3.4 tests that case in tier 1.

The archive cannot join a symptom to a bus. Each field below is quoted from `crates/aethelgard-chaos/src/record.rs`.

| Kind | Fields | Names a bus? |
|---|---|---|
| 7 `RiseMeasured` | `bus, net, rise_30_70_ps, complete, ceiling_ps, exceeds` | yes, by id (`SENSORS`) |
| 8 `LineStuckHigh` | `bus, net, microvolts_at_release, ceiling_ps` | yes |
| 9 `BusFault` | `bus, fault, source, from, nets, firmware_sees` | yes |
| 11 `RegisterObserved` | `name` (`GPIOB.AFRL`, `RCC.APB1LENR`), `reading` | no |
| 12 `I2cCheck` (the symptom) | `reference, address, register, expected, observed, status` | **no, and no peripheral either** |
| 13 `I2cWitness` | `peripheral` (`I2C1`), `reference`, `cr2`, `txdr` | **no**, only the controller |
| 14 `Verdict` | `[kind, severity, count]` | no; the judge's message text is not archived |

What is in the archive:
- `reference → peripheral`, from kind 13, and only for the last device on each controller.
- `bus → net`, from kinds 7, 8 and 9.

What is not in the archive:
- `peripheral → bus`, or `reference → bus`;
- `net → pin → peripheral`, which the GPIO and alternate-function rule needs, because `GPIOB.AFRL` covers both PB6 and PB7.

Those joins live in the `SolvedTopology` (`i2c_devices[ref].bus`, `assignments[net]`). The archive holds only the topology's sha256, in kind 6 and the header, and §1.3 forbids `attribute` to read anything outside the run directory.

I checked that the gap is not one the CLI drops while archiving. `verdict::I2cReport` and `WitnessReport` (`crates/aethelgard-cli/src/verdict.rs`) carry no bus either, and `firmware_sees` is the constant `analog::RENDERING`, which names no device. So the archive cannot gain the fact unless what `campaign` records changes, and `campaign` is read-only under §2.

**Why I did not build around it.** The reference board has one bus. A rule such as "a run that names exactly one bus in kinds 7 to 9 has every symptom on that bus" would turn tier 2 green, but it fails in two ways:
1. It is a conclusion by inference, from the absence of any other bus in the records. A re-sealed two-bus run whose faulted bus's analog records were dropped consistently would be attributed `software` on the healthy bus. That is the confident wrong answer the roadmap §3 names, and §5.1's own attack.
2. The §3.4 tier-1 case "a fault on another bus" could not be decided for its stated reason.

Building it would mean a test that goes green only by being weakened, the third §7 condition.

**What does work without the join.** From `I2cWitness.peripheral`, `aethelgard-svd-hal`'s `cmsis::register_facts().enable_bit(peripheral)` gives the `RCC` register and bit that gate the controller. That can be matched against a `RegisterObserved` by address and mask. Likewise `position("I2C_CR2_SADD_Pos")` and `position("I2C_CR2_NBYTES_Pos")` give the witness's fields from the header. So the clock-gate rule and the witness rule (wrong SADD, NBYTES or TXDR register byte) are buildable, but only for a device that has a witness. The bus-health half of the S rule, and the whole E rule, are not.

**Smallest amendments, for the architect to pick:**
- **(a)** Add `bus` to kind 12, or to kind 13, or to both. Add the pin per peripheral line too if the GPIO and AF rule is to be per peripheral. This is a format change under the chaos README's version rule (trace header version 2 → 3, golden constants re-pinned) and a `campaign` change, so it is a separate work order, or an amendment to this one's §2 fence.
- **(b)** Allow `attribute` to read the solved topology, checked against kind 6's `topology_sha256`. This drops §1.3's "reads nothing outside the run directory", and with it the archive-alone property the gate clause relies on (`INV-04`).
- **(c)** An interim ruling that I offer but have not taken: a run whose kinds 7 to 9 name exactly one bus attributes every I2C symptom to it; any other run is `indeterminate` on every I2C symptom; the gap is recorded as a residual. Under (c), §3.4's "a fault on another bus" would be `indeterminate` because the bus is unknown, not because the fault is elsewhere, and attack 1 above stays open. I do not recommend it without (a) to follow.

I recommend **(a)**, with `bus` on kind 12, so each symptom carries its own bus. It makes the per-bus rules exact, and attack 1 then needs a forged join, not a dropped record.

## What was read

The work order, the kickoff, northstar `INV-04` to `INV-07`, architecture `RUL-06`, `RUL-16`, `RUL-17`, §2.16 and §2.17, the roadmap's P3 gate and §6, `aethelgard-chaos` (README, `record.rs`, `campaign.rs`, `replay.rs`), the CLI's `campaign.rs`, `verdict.rs`, `analog.rs` and the judge in `renode.rs`, `aethelgard-twin` `boot.rs`, the `AET-WO-2300` and `AET-WO-2202` FEEDBACK files, and the `aethelgard-evolve` stub.

## Not done, by reason of the stop

The build, the seeded set, tier-1 and tier-2 tests, the adversarial pass, the reduction pass, and the PR. A replacement goal is needed with the ruling (roadmap §6: a ruling that adds a stop comes with a replacement goal).
