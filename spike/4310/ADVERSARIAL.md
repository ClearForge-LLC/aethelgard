# AET-WO-4310: adversarial pass (full), read-only

Reviewed against HEAD 215c7a53 (base adbd5c66), the work order, and the bus
rulings (06:58Z to 08:55Z). Line numbers are confirmed at HEAD with
`git show HEAD:<file> | grep -n`.

**What I read.** The diffs of:

- `supervisor.rs`, `power.rs`, `power_parts.rs`, `radio.rs` and
  `power-parts.json`;
- `aethelgard-eda` `supply.rs`, `extract.rs` and `fab.rs`;
- `aethelgard-cli` `campaign.rs`, `verify.rs`, `readiness.rs`, `extract.rs`,
  `fab.rs`, `rebind.rs` and `main.rs`;
- `aethelgard-chaos` `campaign.rs` and `record.rs`;
- `aethelgard-evolve` `lib.rs`;
- `aethelgard-twin` `platform.rs`;
- the committed droop scenarios;
- `parse_ladder_record` in `schematic.rs`.

**What I did not read.** The `.github` diff, the new motor-open and held-low-sda
scenario files, the test diffs, and the rest of `schematic.rs` (the ladder-record
writer).

**Not reviewed.** The working tree has an uncommitted reduction edit to
`crates/aethelgard-eda/src/supply.rs` (helper dedupe only).

No cargo was run.

The four lines, in brief:

- **Omission.** There is one false "no minimum" claim (F1), one judgement that
  can be skipped silently (F3), and latent ground entry gaps (F4). Off-board
  peers on connectors are out of scope, as ruled (F11). F2 was withdrawn on
  checking.
- **Wrong number.** The SPI timing and floor comparisons use the reset at the
  monitor, not at the part's pin (F3, F5). The release floor cites a VOH row
  outside its stated VDD range, though the number happens to be the same (F6).
- **Coverage.** The ADC input still has a tolerable cover with no tolerance (F7).
  E4 reads back the twin's own disconnection (F8, named, ruled design).
- **Package.** The writer accepts an absent `built`, and only the comparer
  refuses it (F9). Rebind admits more than the ruling named (F10).

Clean (checked, no finding) is listed at the end.

---

## F1. Pin devices are named "no supply minimum is held", but their rows hold one. Real defect: a false verdict text, in the strict direction.

- `crates/aethelgard-ir/src/supervisor.rs:354-363` (`powered`) pushes each pin
  device's supply rails with `min: None`. The doc comment says "whose rows hold no
  minimum", and `spike/4310/RESULTS.md` P2.6 says the same.
- That is false. `crates/aethelgard-ir/data/parts.json` holds each supply pin's
  `range_mv`, which `pin_devices.rs:953` (`supplies`) already reads as
  `PinKind::Supply { range_mv }` and judges against the domain:
  - DRV8833PWPR (`parts.json:1052`): `VM` [2700, 10800], "VM 2.7 to 10.8 V, p. 5";
  - DRV8876PWPR (`parts.json:1328`): `VM` [4500, 37000], "VVM 4.5 to 37 V, p. 4".
- **Failure.** Take `d2-drv8833`-shaped VM on the MCU's 3V3 rail at BOR3:
  - The real judgement is `Floor` (2700 > 2540).
  - The verdict instead says "no supply minimum is held for it, so it is never
    passed". The reason is wrong.
  - At BOR2 (2250 floor) the same part should also be a `Floor`. It is named
    instead, with a false reason.
  - No drawn board carries a pin device today, so no committed verdict moves.
    The spike's measurement row is still wrong, and so is the readiness text
    every future driver board will get.
- **Smallest fix.** In `powered`, take `range_mv.0` of each supply pin bound to
  that rail, the lowest when several pins share it. Cite it with the row's
  document and `page`. Fix the doc comment and the RESULTS row. Add one mutant
  (`min: None`).

## F2. Withdrawn: the release-floor skip when the module's rail has no drop limit is not a defect.

- `radio_lines` returns `Ok(())` when `line_supply` gives no `rs`
  (`power.rs:1865-1870`).
- That case is refused upstream. `power.rs:1607` raises `DropLimitUndeclared` for
  a module on its own rail with no drop limit, before `radio_lines` is called at
  `:1711`.
- The `continue` at `:1604` needs a domain with no derived regulator, which
  `tree()` (`DomainWithoutSource`) rules out.

**Related shape: a module on the MCU's rail with no crossing.** That is
`sentinel-03-radio`: U30 on `VDD_3V3`, with no `uart_crossings`.

- The `None` arm (`power.rs:1912-1920`) adds no release-floor lines.
- The floor rule judges U30 as a `Floor` (3000 > 2540), so readiness refuses the
  board anyway.
- Plain `verify` judges those lines only over the rail's band. That is a gap to
  name at most, not a defect.

## F3. `spi_timing` can pass by omission, and judges at the wrong voltage. Gap to name (latent: TXU0304 has one SPI row, at 3.0 V).

`crates/aethelgard-ir/src/supervisor.rs:417-470`:

- **(a) It is skipped silently in two cases** (`:438`, `:440-446`):
  - when neither side is on `mcu.supply`;
  - when no `supplies` row is non-`uart_only`.

  The second case matters because `supplies_for(.., uart=false)` then falls back
  to `supplies[0]`, a `uart_only` row (`power_parts.rs:374-384`). An SPI
  crossing would then read UART timing, and the floor rule would never name it.
- **(b) It takes the minimum over all non-UART rows (`min_by_key`), not the row
  the crossing actually reads** (`supplies_for`, which needs both domains in
  band). A lower row whose VCCB band excludes the B domain would let the crossing
  pass when the row it reads starts above the reset.
- **(c) It compares the row's VCCA minimum with `reset_min` at the source
  (`:449`).** The ruled release floor for lines is `reset_min − drop`, and
  `radio_lines` uses exactly that (`power.rs:1963`). A row at 2500 mV with
  BOR3 at 2540 and a 110 mV drop passes here, though the pin sees 2430 mV.
- **Smallest fix.**
  - Use the row `supplies(ctx, ..)` / `supplies_for` returns for the crossing's
    two domains.
  - Compare it with `reset_min − drop(mcu.supply)`.
  - Turn both `continue`s into `Named`.

## F4. Ground charge: some currents are charged at the wrong entry pads, or at a 0 mA load. Gap to name. (a) and (b) are latent, since every committed regulator is battery-fed or a rail-fed LDO. (c) applies to Sentinel-03's `VDD_3V3_RF`, at microvolt scale.

`crates/aethelgard-eda/src/supply.rs:1806-1840`:

- **(a) A child buck fed from rail `k` is undercharged** (`:1821`):
  - `ma` (`:1808`) is `k`'s solved load, which includes the child's draw.
  - `k`'s entry pads exclude any regulator with `input == k`.
  - The child buck's input current enters the ground copper at the child's PGND.
    That is not among `k`'s entries, and the battery loop (`:1843`) sees only
    battery-fed sinks.
  - If the child's PGND is farther from the return pin than every other `k`
    consumer, the charge is `ma × max(other consumers' bounds)`, below what the
    principle allows. That is an undercharge, which is not certified.
  - A rail-fed LDO is fine: its current enters at its own domain's consumers,
    which the child's own loop charges.
- **(b) `map_or(0, …)` at `:1808` zeroes a domain missing from
  `checked.domains`.** That is unreachable today, but it is the INV-06 pattern
  the codebase refuses elsewhere ("never zeroed").
- **(c) A consumer of `k` with no GND pad is skipped, while its current stays in
  `ma`** (`:1826`, `if !g.is_empty()`).
  - On Sentinel-03, `VDD_3V3_RF` has four such loads, each declared 1 mA:
    - R90, U30's EN pull-up;
    - R91, the IO2 strap;
    - R30, the top of U13's FB divider;
    - R32, U13's PG pull-up.
  - Their current enters the ground copper at another part's pads: U13's GND
    through PG, R31's ground pad, U30's pins.
  - It is charged at the largest bound of U30's pads instead. If those pads sit
    nearer the return pin than U13's or R31's, this is an undercharge of at most
    4 mA × the difference in bounds. That is microvolts, but uncertified.
  - The all-padless case is handled: the current is charged at `r11` and named.
    The mixed case is not.
  - `load_milliamps` is "everything the regulator delivers"
    (`power.rs:735-736`), which confirms that `ma` includes the child
    regulators' draw (a).
- **Smallest fix.**
  - For each regulator `q` with `q.input == k` whose row is a buck, add `q`'s
    PGND pad to `k`'s entry set.
  - Charge the declared mA of `k`'s padless consumers at `r11`, which the
    principle bounds for any entry pad, and name them.
  - Replace `map_or(0, …)` with a refusal.

What I checked and found sound:

- The maximum-principle bound itself, and that a sink anywhere other than `b`
  only lowers `a`.
- That the victim's own domain's other consumers are covered: `own` charges the
  whole domain load at `r11`.
- Skipping the victim's own buck input. Its current is injected at `b`, so it can
  only lower `a`.
- 2-layer boards. Every return is refused there, so the drop is `None` and the
  gate fails. Skipping `others` loses nothing.
- Battery-domain loads. `tree()` refuses a domain without a regulator, and
  `battery_side` sinks are bucks only, so the `continue`s at `:1847-1851` are
  unreachable.
- `v_mv = pack floor − input drop`. That is the right direction: a lower V gives
  a higher current, which is conservative.

## F5. The floor rule compares a part's minimum with the reset at the monitor, not at the part's pin. Gap to name (pre-existing for bus devices; 4310 widened it to every part).

- `supervisor.rs:395` (`judged`): `min > reset_min`. The BOR senses VDD at the
  MCU's pin. Another consumer on the same rail sees its own copper drop, and the
  gate does not bound that drop (the MCU is the only load).
- **Failure.** On the MCU rail at BOR3 (2540), with a 110 mV drop limit, a part
  with a 2500 mV minimum is judged `Pass`, though its pin may sit below 2500
  while the MCU runs.
- Committed parts on the MCU rail have wide margins, so no verdict moves today:
  - BMP581 and ICM at 1710;
  - TXU0304 VCCA at 1080;
  - VBOR0 at 1580;
  - W25Q64JV at 2700, already a floor.

  `radio_lines` already subtracts the drop for the same physical question.
- **Smallest fix.** Judge `min > reset_min − drop_limit(rail)`, or name the
  assumption in the floor text.

## F6. The release floor's MCU driver cites a VOH/VOL row stated only for 2.7–3.6 V. Not a numerical defect; a wrong citation and load. Gap to name.

- `power.rs:2024-2031` reuses `mcu_drv` (`o.high_below_supply_millivolts`). Its
  cite is "DS12117 Table 50, IIO = −8 mA, 2.7 V ≤ VDD ≤ 3.6 V", applied at the
  release floor of about 2.43–2.54 V (BOR3 less drop).
- The held DS12117 Rev 10, p. 139, Table 50, has the row that does hold there:
  "VOH … IIO = −4 mA, 1.62 V ≤ VDD < 3.6 V, VDD − 0.4", with VOL 0.4 at 4 mA.
  - The figure is the same, so the 4310 margins stand.
  - The cite, and the 8000 µA `load_ua`, are out of range at the operating point.
- This is the same class as D3's "margin from the wrong number". It holds only
  because the figures coincide.
- **Smallest fix.** At the release floor, use a `Driver` with
  `load_ua: Some(4000)` and cite Table 50's 1.62–3.6 V row. Ideally that row
  goes in power-parts as data.

## F7. An ADC input is still covered by a tolerable `analog_value` that moves nothing past any tolerance. Real gap against D7 (i) (latent: `git grep -l analog_value -- '*.json'` finds no JSON scenario; it appears only inline in `.rs` tests).

- `verify.rs:1721-1731` (`stress`): any `analog_value` whose `millivolts × 1000`
  differs from the idle's microvolts yields `Target::Analog`. That includes
  1 mV, or the idle value merely rounded to whole mV.
- `classify` (`:1864-1870`) filters only `Target::I2c` from tolerable targets.
- D7 (i): "a quantity with no declared tolerance needs a detected fault to cover
  it". An ADC input declares no tolerance, and no fault kind detects one. So
  `campaign_faults_every_bus_and_rail` can be met for a pin-device board's ADC
  inputs by a scenario that changes nothing material.
- The other tolerable targets are sound:
  - the MCU rail (a droop below the band that must cross the BOR level);
  - the battery (`R > healthy`).
- **Smallest fix.** Filter `Target::Analog` as `Target::I2c` is filtered. Each
  ADC input is then unmet until a detected fault exists, or name it in
  `no_device` as inputs are. Ask the architect which.

## F8. E4 reads back the twin's own disconnection. Name it in FEEDBACK (the ruled design, D7 (iii)).

- Two mechanisms produce the symptom without the firmware or the board taking
  part:
  - `platform.rs` (`emit_observers`, the `open` branch) leaves the observer off
    the line.
  - `evolve/src/lib.rs:1537` (`motor_open`) accepts its timing-check record plus
    a witness with `rises: 0, falls: 0`.

  The fault run alone would "detect" an output the firmware never drives. That
  is the INV-07 shape `verify.rs` itself cites for held inputs ("its gate would
  read back the twin's own stimulus").
- It does not become a false cover, because coverage needs a passing baseline
  (`verify.rs:1938`, `baseline && ran(i)`). The post-gate call is
  `verify.rs:2709`, `coverage(.., Some(&outcome))`.
  - A baseline passes only with every output witnessed. In `renode.rs:2069-2071`,
    fewer than 2 rises is `unwitnessed`, which raises
    `motor_output_unwitnessed` (`chaos/campaign.rs:562`).
  - So no healthy `MotorWitness` is 0/0. A static-level output is a different
    record (`MotorLevelWitness`, kind 32), which E4 does not match.
  - The open run cannot pass with another output silent either, since subjects
    are limited to `output:<label>` (`verify.rs:2170-2183`).
- What is left is that the motor target asserts no more than the baseline
  already does. Outputs with no observer, and PWM outputs, are not a hole:
  - `timer::observers` gives every motor output an observer, PWM included;
  - an output with no witness record gets no E4, and stays unmet.
- **Action.** Record it in FEEDBACK as a stated limit of D7 (iii). It also fits
  with the twin's stuck-output alternative.

## F9. A package field the writer accepts when absent. Gap to name (latent today).

- `fab.rs:109` (`built_lines`) returns `[]` when the extract gate's `built` lacks
  `outline_mm` or `capacitance_ladder`.
- `verify.rs:2692` passes it to `derive`, which writes FAB-NOTES without either
  line. Only `--against-fab`'s `states_built` (`verify.rs:2420`,
  `rebind.rs:202`) refuses that package later.
- `extract`'s `Report` always serializes both fields when it reports, so this is
  unreachable now. It is still exactly attack line 4 on the writer side.
- **Smallest fix.** In `conclude` or `derive`, refuse `PackageBuiltUnstated` when
  `built_lines(..).len() != 2`, so a package is never written that the reader
  would refuse.

## F10. Rebind admits more than the 08:22Z ruling named. Gap to name, borderline real.

Ruling (2): rebind "admits, by key: the outline line, the ladder line, and the
scenario entries D7 adds", and "every other line must be equal".

- **(a) Any new scenario is admitted.** `without_new_scenarios`
  (`rebind.rs:150-196`) drops every derived entry whose name the committed
  manifest lacks, not only D7's `held-low-sda`. A later re-bind could add any
  number of scenarios to the ordered board's manifest without a `RebindMoved`.
  - Renames and removals are caught: the held name goes missing and the arrays
    differ. I checked both.
- **(b) The two notes lines are admitted by key forever, with their content
  never compared** (`admitted`, `:139`). They are stripped from the held side
  too. After this WO's write, the committed FAB-NOTES carries them, so a later
  re-bind whose outline or ladder text differs passes the notes comparison
  unseen.
  - A changed outline or rung moves the Gerbers, which `tree::against` still
    refuses, so the hardware stays guarded.
  - The FAB-NOTES text itself is no longer "every other line equal".
- `ORDERED.sha256` binds only the ten Gerber and drill files, not FAB-NOTES or the
  MANIFEST. So this does not trip D6's stop.
- **Smallest fix.**
  - Admit a key only when the held FAB-NOTES has no line with it, and compare it
    when held has one.
  - Admit only scenario names in an explicit list (`["held-low-sda"]`), or scenario
    files new in this WO's diff.
  - Add a mutant: a planted extra scenario entry must be `RebindMoved`.

## F11. Off-board peers on a connector are not named in the verdict. Gap to name; ruled out of scope.

- `powered()` does not enumerate connectors. On Sentinel-03:
  - J3 draws 50 mA on `VDD_3V3`, declared as "the EXT port's budget for
    off-board devices; no source".
  - J2 draws 1 mA, the SWD target sense.
- An off-board device powered from a board rail has a supply minimum that
  nothing judges. Spike RESULTS P2.6 puts "connectors and off-board peers" out of
  scope, the architect accepted the table, and no committed verdict moves.
- But under (2)'s "a part with no declared minimum is named, never passed", the
  verdict says nothing about it.
- **Smallest fix.** Add a condition line per powered connector naming the rail
  and the budget, and stating that its peers' minimum is not judged. This is not
  a refusal. Otherwise, state the scope in FEEDBACK.

---

## Checked and sound (no finding)

- **Translator rows and the UART crossing.**
  - A missing translator row, radio row, parts row or supervisor row gives
    `min: None`, so the part is named, never passed.
  - A UART crossing whose A side is not on the MCU's supply is refused by
    `uart_crossings_shape` (`power.rs:1782`).
- **The lower band (679/243).**
  - Each is the worse of SCES935A's 2.3 V and 3.0 V rows as a fraction of VCCI.
  - 678.3 is rounded up, which is conservative.
  - I checked this arithmetic against the rows' own citations (1.56 V and
    0.56 V at 2.3 V; 0.89 V at 3 V), not against the PDF. No text extraction of
    the held SCES935A was available here.
  - If VT± is linear in absolute volts between the rows, the ratio is monotone,
    so the 2.3 V row bounds the whole band.
  - The reader enforces that the band meets `supply_millivolts.min` and is no
    better than it.
- **The release-floor receiver.** It applies the lower-band permille at every
  corner up to the rail's maximum, which is conservative. The translator's VOH
  and VOL are stated over 1.1–5.5 V, so they hold at the floor.
- **Headroom pack series.** It is added to the gate-drive drop, the RDS(on)
  iteration and the path drop: three places, one direction. Undeclared, it is
  screened by name.
- **Droop sources.**
  - A bare number, or a v6 droop, is refused by name.
  - A source kind other than held/stimulus, or text that is empty, too long or
    non-ASCII, is refused.
  - `droop_load_short` compares with the effective topology's solved load on the
    droop's own domain, exactly. The domain is the MCU's supply, by
    `DroopNotMcuSupply`.
  - Sentinel-03's 67 mΩ × 750 mA keeps the 50.25 mV I·R. That is a stimulus by
    the ruling, stated as such.
  - Limit: nothing checks a `held` text against what it cites, for example that
    a held R equals `battery.series_milliohms`. That is text-only by design.
- **Unmodelled buses.**
  - `witnessed` uses the declared devices plus `model::kind`.
  - A bus with some modelled devices, faulted on an unmodelled one, is limited
    after the gates by `outcome`.
  - EXT stays `no_device`.
- **The motor override.**
  - An unknown label is refused (`UnknownMotorOutput`).
  - It is intolerable in an expected-pass scenario.
  - It needs scenario version 7.
  - `booted_is_emitted` and `analog_pass` both thread `motors_open`.
- **extract's `title_block`.**
  - It is read once (the `once` set).
  - Its only child is `(comment 1 "…")`, parsed strictly with no leading zeros
    and bounded by `RUNGS` and `POUR_STEPS`.
  - A record with no ladder, or a ladder with no record, is refused
    `LadderUnrecorded`.
- **Readiness.**
  - The named parts and the floors are listed together, after the specification's
    own gaps.
  - `floors_above_reset` has no other callers now. The old "named in every other
    verdict" path was readiness-only at base too.
