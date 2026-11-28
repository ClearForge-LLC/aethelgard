# AET-WO-4413 part (1): results, scored against PREDICTIONS.md

Probes ran on Furnace, 2026-11-27, with this branch's engine (`86a1f280`'s build). The probe specifications are scratch copies of `examples/sentinel-03.json` in `~/.cache/aet-4413/probe/`; nothing committed moved. The raw outputs are in `out/`.

| Prediction | Result |
|---|---|
| **P1.1** TLV803S cannot monitor the radio rail | **Right.** VIT− 2.87 / 2.93 / 2.99 V, read at source (SBVS157E p. 6) |
| **P1.2** TPS3840 cannot | **Right.** VHYS 175 / 200 / 225 mV for VIT− from 3.1 V, guaranteed (SNVSB03E p. 6), so the release is up to 3,371.5 mV |
| **P1.3** TPS3839G33 fits | **Right, at source.** VIT− 3.003 / 3.08 / 3.126 V at −40 to 85 °C; Vhys 31 mV typical (SBVS193D p. 7, §7.5); push-pull active-low RESET (p. 4). Not yet built |
| **P1.4** The IR cannot express a second monitor | **Right** (read; the survey, spot-checked). `Mcu.reset_supervisor` is one `Option`, and `judged` holds only `mcu.supply` |
| **P2.1** At today's feedback, U30's pin can fall to 3,000.7 mV, under TPS3839G33's 3,157 mV release | Right (arithmetic) |
| **P2.2** 100k/41.2k solves to 3,303.185–3,589.075 mV; the domain declaration must change | **Right, with one correction.** `solve`: 3,303,185–3,589,075 µV. The domain must also fit U31's stated VCCB rows (3.0–3.6 V, SCES935A p. 14, §7.11). 3,430 mV ± 5 % (3,258–3,602 mV) is refused by name, `DomainOutsideConsumer`; ± 4.9 % (3,261.9–3,598.1 mV) validates. `radio.rs:720` judges U30's 3.6 V the same way. So the planted case "the rail set above a consumer's maximum" is already refused by name |
| **P2.3** VDD_3V3_RF's limit is 146 mV | Right, arithmetic: 3,303.185 − 3,157 |
| **P3.1** The split cannot be expressed in the specification today | **Right** (read, spot-checked at `supply.rs:1676` and IR `lib.rs:166`). Not probed: there is nothing to declare |
| **P3.2** A split drawn with the gate unchanged, about 334 mV | Not probed (inexpressible). Arithmetic only |
| **P3.3** Split plus gate change: 118.0 and 152.2 mV, both failing | Arithmetic only |
| **P3.4** Widening `VDD_3V3_RF`'s class lowers its supply side | **Wrong: it cannot be drawn.** At 1,500 µm the router refuses `VDD_3V3_RF`, `Unroutable`: "U30 pad 1 has no node clear to start a track". At 1,000, 800, 700 and 600 µm it refuses at "U31 pad 14 has no node clear to end a track". A class has one width, and U31's TSSOP pad takes only 500 µm. The lever needs a neck-down at a fine-pitch pad, which is router work |
| **P3.5** The ladder still passes on a widened board | Moot: no wider board was drawn |
| **P4.1** `emit` 104 s, `extract` 1,078 s | `emit` 97–176 s (163 s for a refused route); `extract` 952 s on the raised-rail board |
| **P4.2** `bench-package-s03` refuses at readiness in 6 min | Right (dispatch 46020469752: 14:57:17 → 15:03:26) |

## One probe the predictions did not make: raising VDD_3V3 too

U11 was raised to the same 100k / 41.2k (3,303.185–3,589.075 mV). VDD_3V3's consumers all validate: the MCU's VDD is 1.62–3.6 V (DS12117 Table 23, the held text line 7584), and U31's VCCA rows run to 3.6 V.

VDD_3V3's drop limit, re-derived by its own rule (the solved minimum less TLV803S's 3,030 mV release, strictly above, `SupervisorAboveRail`), becomes 272 mV:
- 273 mV is refused by name: 3,303 − 273 = 3,030 is not above 3,030.
- The limit's source text is unchanged in kind, the rule `4300k` D6 ruled.

| Board (raised rails, no other change) | VDD_3V3 | VDD_3V3_RF |
|---|---|---|
| Bound (µV) | 271,747 | 241,218 |
| Limit (mV) | 272 | 146 |
| Verdict | **pass, by 0.25 mV** | fail, by 95.2 mV |

- **The board moved by its four feedback resistors' values only:** R20, R21, R30 and R31 in the BOM, the schematic and the board's value properties. Every copper byte is the same, as expected.
- **VDD_3V3's bound rose 0.45 mV** (271,293 → 271,747 µV): the higher rails draw more battery current through U10's and U13's input loops.

Raw: `out/rail3-extract.json`, `out/wide-emit-refusals.txt`, `out/rail-validate.txt`.
