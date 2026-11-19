# AET-WO-3305 build: reproduction predictions, written 2026-11-19T23:41:53Z before the branch differential

Main's CLI (35d23eae) against the branch's, on one firmware per group (diff.sh): 31 runs in 7 groups.

1. Every trace differs from main's in the version byte (14 -> 16), so its sha256 and state hash differ too.
2. With the version byte normalised, every trace is byte-identical, except where a kind 13 `absent` held an
   address from more than one step:
   - min, s01, dshot, pwm, serial: one I2C step on the bus, so no content moves. Held-low and weak-pull-up runs
     carry [118] for U2 before and after.
   - ext-held-low-sda: EXT's BMP581 has three steps (identity, configure, data), each meeting 0x47. Main's three
     kind 13s carry [71, 71, 71] each; the branch's carry [71] each. The bytes move; the attribution does not
     (E1, nothing masked before or after, since every address was the device's own).
   - two-held-low-sda: main [118, 119] on both witnesses -> branch [118] and [119]; the attribution moves from
     E1 with masked [28,13], [29,13] to E1 with nothing masked (the defect fixed).
   - ext-baseline, two-baseline: no absent address, so no content moves.
3. Campaign lines, replays and attributions are otherwise identical but for run ids and hashes.
