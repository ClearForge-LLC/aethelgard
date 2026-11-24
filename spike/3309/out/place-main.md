| group | scenario | main | R1A | R1B | R2A | R2B | R2B-line | R2B-dev | R2B-min | R3B | R3B-min |
|---|---|---|---|---|---|---|---|---|---|---|---|
| bmp | bmp-baseline | N1 [] | = | = | = | = | = | = | = | = | = |
| bmp | bmp-held-low-sda | E1 [5:9] | = | = | = | = | = | = | = | = | = |
| bmp-0x47 | bmp-mutant | S1 [4:7,5:7,30:13,32:13] | **S1 [4:7,5:7,30:13,31:13]** | **S1 [4:7,5:7,30:13]** | **S1 [4:7,5:7,30:13,31:13]** | **S1 [4:7,5:7,30:13]** | **I1 []** | **I1 []** | **I1 []** | **S1 [4:7,5:7,30:13]** | **I1 []** |
| c0 | motors-dshot600-droop | N1 [] | = | = | = | = | = | = | = | = | = |
| c0 | motors-dshot600 | N1 [] | = | = | = | = | = | = | = | = | = |
| c1 | motors-pwm | N1 [] | = | = | = | = | = | = | = | = | = |
| c2 | baseline | N1 [] | = | = | = | = | = | = | = | = | = |
| c2 | battery-sag-tolerated | N1 [] | = | = | = | = | = | = | = | = | = |
| c2 | droop-bor3 | N1 [] | = | = | = | = | = | = | = | = | = |
| c2 | droop-tolerated | N1 [] | = | = | = | = | = | = | = | = | = |
| c2 | held-low-sda | E1 [20:9] | = | = | = | = | = | = | = | = | = |
| c2 | serial-rx-open-console | E3 [96:28,99:29] | = | = | = | = | = | = | = | = | = |
| c2 | serial-rx-open-crsf | E3 [97:28,100:29] | = | = | = | = | = | = | = | = | = |
| c2 | serial-rx-open-mavlink | E3 [98:28,101:29] | = | = | = | = | = | = | = | = | = |
| c2 | spi-absent-u4 | E2 [85:22] | = | = | = | = | = | = | = | = | = |
| c2 | spi-absent-u5 | E2 [86:22,87:22,88:22,89:22,90:22] | = | = | = | = | = | = | = | = | = |
| c2 | vdd5v-out-of-band | E3 [98:28,99:28,101:29,102:29] | = | = | = | = | = | = | = | = | = |
| c3 | serial-four-open-crsf | E3 [54:28,58:29] | = | = | = | = | = | = | = | = | = |
| c3 | serial-four-open-mavlink | E3 [52:28,56:29] | = | = | = | = | = | = | = | = | = |
| c3 | serial-four | N1 [] | = | = | = | = | = | = | = | = | = |
| c4 | baseline | N1 [] | = | = | = | = | = | = | = | = | = |
| c4 | battery-sag-tolerated | N1 [] | = | = | = | = | = | = | = | = | = |
| c4 | vdd1v8-out-of-band | E2 [36:22,37:22,38:22] | = | = | = | = | = | = | = | = | = |
| c5 | droop-bor3 | N1 [] | = | = | = | = | = | = | = | = | = |
| c5 | droop-tolerated | N1 [] | = | = | = | = | = | = | = | = | = |
| c5 | pullup-2k2 | N1 [] | = | = | = | = | = | = | = | = | = |
| c5 | pullup-47k | E1 [5:7,6:7,7:9] | = | = | = | = | = | = | = | = | = |
| c5 | cap-220p-2k2 | E1 [5:7,6:7,7:9] | = | = | = | = | = | = | = | = | = |
| c5 | cap-390p-sda-2k2 | E1 [6:7,7:9] | = | = | = | = | = | = | = | = | = |
| c5 | held-low-scl | E1 [6:9] | = | = | = | = | = | = | = | = | = |
| c5 | held-low-sda | E1 [6:9] | = | = | = | = | = | = | = | = | = |
| c5 | pullup-100k | E1 [5:7,6:7,7:9] | = | = | = | = | = | = | = | = | = |
| c5 | pullup-10k | E1 [5:7,6:7,7:9] | = | = | = | = | = | = | = | = | = |
| c5 | pullup-3k3 | N1 [] | = | = | = | = | = | = | = | = | = |
| c5 | pullup-3k9 | E1 [5:7,6:7,7:9] | = | = | = | = | = | = | = | = | = |
| c5 | pullup-4k7 | E1 [5:7,6:7,7:9] | = | = | = | = | = | = | = | = | = |
| c5 | serial-rx-dead | N1 [] | = | = | = | = | = | = | = | = | = |
| c5 | serial-rx-open-console | E3 [36:28,37:29] | = | = | = | = | = | = | = | = | = |
| c5 | spi-absent-u4 | E2 [35:22] | = | = | = | = | = | = | = | = | = |
| c6 | droop-bor3 | N1 [] | = | = | = | = | = | = | = | = | = |
| c6 | pullup-2k2 | N1 [] | = | = | = | = | = | = | = | = | = |
| c6 | pullup-3k3 | N1 [] | = | = | = | = | = | = | = | = | = |
| c6 | serial-rx-open-console | E3 [40:28,41:29] | = | = | = | = | = | = | = | = | = |
| c6 | spi-absent-u4 | E2 [39:22] | = | = | = | = | = | = | = | = | = |
| ctl-c2 | baseline | N1 [] | = | = | = | = | = | = | = | = | = |
| ctl-c2 | held-low-sda | E1 [20:9] | = | = | = | = | = | = | = | = | = |
| ctl-ext-cfg37 | ext-mutant | I4 [6:7,7:7] | **S1 [6:7,7:7,42:13]** | **S1 [6:7,7:7,42:13]** | **S1 [6:7,7:7,42:13]** | **S1 [6:7,7:7,42:13]** | **I1 []** | **I1 []** | **S1 [6:7,7:7,42:13]** | **S1 [6:7,7:7,42:13]** | **S1 [6:7,7:7,42:13]** |
| ext | ext-baseline | N1 [] | = | = | = | = | = | = | = | = | = |
| ext | ext-held-low-sda | E1 [9:9] | = | = | = | = | = | = | = | = | = |
| ext-cfg37 | ext-mutant | I4 [6:7,7:7] | **S1 [6:7,7:7,42:13]** | **S1 [6:7,7:7,42:13]** | **S1 [6:7,7:7,42:13]** | **S1 [6:7,7:7,42:13]** | **I1 []** | **I1 []** | **S1 [6:7,7:7,42:13]** | **S1 [6:7,7:7,42:13]** | **S1 [6:7,7:7,42:13]** |
| seeded-console-rx-af-wrong | serial-rx-dead | S5 [14:11,37:29] | = | = | = | = | = | = | = | = | = |
| seeded-cs-never | pullup-2k2 | S3 [35:22] | = | = | = | = | = | = | = | = | = |
| seeded-gpiob-clock-skipped | pullup-2k2 | S1 [5:7,6:7,15:11,16:11,20:11] | = | = | = | = | = | = | = | = | = |
| seeded-i2c-clock-skipped | pullup-2k2 | S1 [5:7,6:7,22:11] | = | = | = | = | = | = | = | = | = |
| seeded-leave-firmware | pullup-2k2 | S2 [5:7,6:7,7:10] | = | = | = | = | = | = | = | = | = |
| seeded-leave-firmware | pullup-47k | I3 [5:7,6:7,7:9] | = | = | = | = | = | = | = | = | = |
| seeded-leave-firmware | cap-390p-sda-2k2 | I3 [6:7,7:9] | = | = | = | = | = | = | = | = | = |
| seeded-motor-af-wrong | motors-pwm | S4 [31:11,53:26] | = | = | = | = | = | = | = | = | = |
| seeded-no-boot-word | pullup-2k2 | S2 [5:7,6:7,7:10] | = | = | = | = | = | = | = | = | = |
| seeded-scl-af-wrong | pullup-2k2 | S1 [5:7,6:7,16:11] | = | = | = | = | = | = | = | = | = |
| seeded-scl-mode-output | pullup-2k2 | S1 [5:7,6:7,15:11] | = | = | = | = | = | = | = | = | = |
| seeded-sda-af-wrong | pullup-2k2 | S1 [5:7,6:7,16:11] | = | = | = | = | = | = | = | = | = |
| seeded-wrong-address | pullup-2k2 | S1 [5:7,6:7,33:13] | = | = | = | = | = | = | = | = | = |
| seeded-wrong-address | pullup-47k | E1 [5:7,6:7,7:9] masked [34:13] | = | = | = | = | = | = | = | = | = |
| seeded-wrong-address | pullup-3k3 | S1 [5:7,6:7,33:13] | = | = | = | = | = | = | = | = | = |
| seeded-wrong-nbytes | pullup-2k2 | S1 [5:7,6:7,33:13] | = | S1 [5:7,6:7,33:13] (kind 13 moved) | = | S1 [5:7,6:7,33:13] (kind 13 moved) | **I1 []** | **I1 []** | S1 [5:7,6:7,33:13] (kind 13 moved) | S1 [5:7,6:7,33:13] (kind 13 moved) | S1 [5:7,6:7,33:13] (kind 13 moved) |
| seeded-wrong-register | pullup-2k2 | S1 [5:7,6:7,33:13] | = | S1 [5:7,6:7,33:13] (kind 13 moved) | = | S1 [5:7,6:7,33:13] (kind 13 moved) | **I1 []** | **I1 []** | S1 [5:7,6:7,33:13] (kind 13 moved) | S1 [5:7,6:7,33:13] (kind 13 moved) | S1 [5:7,6:7,33:13] (kind 13 moved) |
| t2-baro | baro-baseline | N1 [] | = | = | = | = | = | = | = | = | = |
| t2-baro-pf | baro-mutant | I4 [4:7,5:7] | I4 [4:7,5:7] (kind 13 moved) | I4 [4:7,5:7] (kind 13 moved) | I4 [4:7,5:7] (kind 13 moved) | I4 [4:7,5:7] (kind 13 moved) | **I1 []** | **I1 []** | I4 [4:7,5:7] (kind 13 moved) | = | = |
| t2-collide | two-bme-collide | S1 [4:7,5:7,28:13,29:13] | = | = | = | = | = | = | = | = | = |
| t2-ext-bor | ext-bmp581-droop-bor3 | N1 [] | = | = | = | = | = | = | = | = | = |
| t2-logger | logger-baseline | N1 [] | = | = | = | = | = | = | = | = | = |
| t2-logger-bor | logger-droop-bor3 | N1 [] | = | = | = | = | = | = | = | = | = |
| t2-s02s | s02s-baseline | N1 [] | = | = | = | = | = | = | = | = | = |
| t2-s02s-bor | s02-shaped-droop | N1 [] | = | = | = | = | = | = | = | = | = |
| two | two-baseline | N1 [] | = | = | = | = | = | = | = | = | = |
| two | two-held-low-sda | E1 [5:9] | = | = | = | = | = | = | = | = | = |
| two-0x75 | two-mutant | S1 [4:7,5:7,28:13] | = | = | = | = | = | = | = | = | = |
| two-0x77 | two-mutant | S1 [4:7,5:7,28:13,29:13] | = | = | = | = | = | = | = | = | = |

## Devices whose placement differs from the count rule's, or with (1), (2), (4) findings

- **bmp/bmp-held-low-sda U5** (Bmp581, 3 steps, 0 lines, whole log = script: False):
  - (2) {'extra': [], 'held': 0, 'none_lines': 0, 'of': 3}
- **bmp-0x47/bmp-mutant U5** (Bmp581, 3 steps, 5 lines, whole log = script: False):
  - (2) {'extra': ['[NOISY] i2c1.u5: read 0x50 -> 0x00'], 'held': 3, 'none_lines': 1, 'of': 3}
  - R1A: moved [{"boot_placed": false, "boot_today": false, "placed": ["[WARNING] i2c1.u5: access to unmodelled register 0x50", "[NOISY] i2c1.u5: read 0x50 -> 0x00", "[NOISY] i2c1.u5: read 0x01 -> 0x50"], "step": "U5.identity", "today": ["[WARNING] i2c1.u5: access to unmodelled register 0x50"]}, {"boot_placed": true, "boot_today": false, "placed": ["[NOISY] i2c1.u5: write 0x36 <- 0x40 0x81"], "step": "U5.configure", "today": ["[NOISY] i2c1.u5: read 0x50 -> 0x00"]}, {"boot_placed": true, "boot_today": false, "placed": ["[NOISY] i2c1.u5: read 0x1D -> 0x00 0x00 0x19 0x40 0xF3 0x62"], "step": "U5.data", "today": ["[NOISY] i2c1.u5: read 0x01 -> 0x50", "[NOISY] i2c1.u5: write 0x36 <- 0x40 0x81", "[NOISY] i2c1.u5: read 0x1D -> 0x00 0x00 0x19 0x40 0xF3 0x62"]}] unplaced ['[WARNING] i2c1.u5: access to unmodelled register 0x50', '[NOISY] i2c1.u5: read 0x50 -> 0x00']
  - R1B: moved [{"boot_placed": true, "boot_today": false, "placed": ["[NOISY] i2c1.u5: read 0x01 -> 0x50"], "step": "U5.identity", "today": ["[WARNING] i2c1.u5: access to unmodelled register 0x50"]}, {"boot_placed": true, "boot_today": false, "placed": ["[NOISY] i2c1.u5: write 0x36 <- 0x40 0x81"], "step": "U5.configure", "today": ["[NOISY] i2c1.u5: read 0x50 -> 0x00"]}, {"boot_placed": true, "boot_today": false, "placed": ["[NOISY] i2c1.u5: read 0x1D -> 0x00 0x00 0x19 0x40 0xF3 0x62"], "step": "U5.data", "today": ["[NOISY] i2c1.u5: read 0x01 -> 0x50", "[NOISY] i2c1.u5: write 0x36 <- 0x40 0x81", "[NOISY] i2c1.u5: read 0x1D -> 0x00 0x00 0x19 0x40 0xF3 0x62"]}] unplaced ['[WARNING] i2c1.u5: access to unmodelled register 0x50', '[NOISY] i2c1.u5: read 0x50 -> 0x00']
  - R2A: moved [{"boot_placed": false, "boot_today": false, "placed": ["[WARNING] i2c1.u5: access to unmodelled register 0x50", "[NOISY] i2c1.u5: read 0x50 -> 0x00", "[NOISY] i2c1.u5: read 0x01 -> 0x50"], "step": "U5.identity", "today": ["[WARNING] i2c1.u5: access to unmodelled register 0x50"]}, {"boot_placed": true, "boot_today": false, "placed": ["[NOISY] i2c1.u5: write 0x36 <- 0x40 0x81"], "step": "U5.configure", "today": ["[NOISY] i2c1.u5: read 0x50 -> 0x00"]}, {"boot_placed": true, "boot_today": false, "placed": ["[NOISY] i2c1.u5: read 0x1D -> 0x00 0x00 0x19 0x40 0xF3 0x62"], "step": "U5.data", "today": ["[NOISY] i2c1.u5: read 0x01 -> 0x50", "[NOISY] i2c1.u5: write 0x36 <- 0x40 0x81", "[NOISY] i2c1.u5: read 0x1D -> 0x00 0x00 0x19 0x40 0xF3 0x62"]}] unplaced ['[WARNING] i2c1.u5: access to unmodelled register 0x50', '[NOISY] i2c1.u5: read 0x50 -> 0x00']
  - R2B: moved [{"boot_placed": true, "boot_today": false, "placed": ["[NOISY] i2c1.u5: read 0x01 -> 0x50"], "step": "U5.identity", "today": ["[WARNING] i2c1.u5: access to unmodelled register 0x50"]}, {"boot_placed": true, "boot_today": false, "placed": ["[NOISY] i2c1.u5: write 0x36 <- 0x40 0x81"], "step": "U5.configure", "today": ["[NOISY] i2c1.u5: read 0x50 -> 0x00"]}, {"boot_placed": true, "boot_today": false, "placed": ["[NOISY] i2c1.u5: read 0x1D -> 0x00 0x00 0x19 0x40 0xF3 0x62"], "step": "U5.data", "today": ["[NOISY] i2c1.u5: read 0x01 -> 0x50", "[NOISY] i2c1.u5: write 0x36 <- 0x40 0x81", "[NOISY] i2c1.u5: read 0x1D -> 0x00 0x00 0x19 0x40 0xF3 0x62"]}] unplaced ['[WARNING] i2c1.u5: access to unmodelled register 0x50', '[NOISY] i2c1.u5: read 0x50 -> 0x00']
  - R2B-line: moved [{"boot_placed": true, "boot_today": false, "placed": ["[NOISY] i2c1.u5: read 0x01 -> 0x50"], "step": "U5.identity", "today": ["[WARNING] i2c1.u5: access to unmodelled register 0x50"]}, {"boot_placed": true, "boot_today": false, "placed": ["[NOISY] i2c1.u5: write 0x36 <- 0x40 0x81"], "step": "U5.configure", "today": ["[NOISY] i2c1.u5: read 0x50 -> 0x00"]}, {"boot_placed": true, "boot_today": false, "placed": ["[NOISY] i2c1.u5: read 0x1D -> 0x00 0x00 0x19 0x40 0xF3 0x62"], "step": "U5.data", "today": ["[NOISY] i2c1.u5: read 0x01 -> 0x50", "[NOISY] i2c1.u5: write 0x36 <- 0x40 0x81", "[NOISY] i2c1.u5: read 0x1D -> 0x00 0x00 0x19 0x40 0xF3 0x62"]}] unplaced ['[WARNING] i2c1.u5: access to unmodelled register 0x50', '[NOISY] i2c1.u5: read 0x50 -> 0x00']
  - R2B-dev: moved [{"boot_placed": true, "boot_today": false, "placed": ["[NOISY] i2c1.u5: read 0x01 -> 0x50"], "step": "U5.identity", "today": ["[WARNING] i2c1.u5: access to unmodelled register 0x50"]}, {"boot_placed": true, "boot_today": false, "placed": ["[NOISY] i2c1.u5: write 0x36 <- 0x40 0x81"], "step": "U5.configure", "today": ["[NOISY] i2c1.u5: read 0x50 -> 0x00"]}, {"boot_placed": true, "boot_today": false, "placed": ["[NOISY] i2c1.u5: read 0x1D -> 0x00 0x00 0x19 0x40 0xF3 0x62"], "step": "U5.data", "today": ["[NOISY] i2c1.u5: read 0x01 -> 0x50", "[NOISY] i2c1.u5: write 0x36 <- 0x40 0x81", "[NOISY] i2c1.u5: read 0x1D -> 0x00 0x00 0x19 0x40 0xF3 0x62"]}] unplaced ['[WARNING] i2c1.u5: access to unmodelled register 0x50', '[NOISY] i2c1.u5: read 0x50 -> 0x00']
  - R2B-min: moved [{"boot_placed": true, "boot_today": false, "placed": ["[NOISY] i2c1.u5: read 0x01 -> 0x50"], "step": "U5.identity", "today": ["[WARNING] i2c1.u5: access to unmodelled register 0x50"]}, {"boot_placed": true, "boot_today": false, "placed": ["[NOISY] i2c1.u5: write 0x36 <- 0x40 0x81"], "step": "U5.configure", "today": ["[NOISY] i2c1.u5: read 0x50 -> 0x00"]}, {"boot_placed": true, "boot_today": false, "placed": ["[NOISY] i2c1.u5: read 0x1D -> 0x00 0x00 0x19 0x40 0xF3 0x62"], "step": "U5.data", "today": ["[NOISY] i2c1.u5: read 0x01 -> 0x50", "[NOISY] i2c1.u5: write 0x36 <- 0x40 0x81", "[NOISY] i2c1.u5: read 0x1D -> 0x00 0x00 0x19 0x40 0xF3 0x62"]}] unplaced ['[WARNING] i2c1.u5: access to unmodelled register 0x50', '[NOISY] i2c1.u5: read 0x50 -> 0x00']
  - R3B: moved [{"boot_placed": true, "boot_today": false, "placed": ["[NOISY] i2c1.u5: read 0x01 -> 0x50"], "step": "U5.identity", "today": ["[WARNING] i2c1.u5: access to unmodelled register 0x50"]}, {"boot_placed": true, "boot_today": false, "placed": ["[NOISY] i2c1.u5: write 0x36 <- 0x40 0x81"], "step": "U5.configure", "today": ["[NOISY] i2c1.u5: read 0x50 -> 0x00"]}, {"boot_placed": true, "boot_today": false, "placed": ["[NOISY] i2c1.u5: read 0x1D -> 0x00 0x00 0x19 0x40 0xF3 0x62"], "step": "U5.data", "today": ["[NOISY] i2c1.u5: read 0x01 -> 0x50", "[NOISY] i2c1.u5: write 0x36 <- 0x40 0x81", "[NOISY] i2c1.u5: read 0x1D -> 0x00 0x00 0x19 0x40 0xF3 0x62"]}] unplaced ['[WARNING] i2c1.u5: access to unmodelled register 0x50', '[NOISY] i2c1.u5: read 0x50 -> 0x00']
  - R3B-min: moved [{"boot_placed": true, "boot_today": false, "placed": ["[NOISY] i2c1.u5: read 0x01 -> 0x50"], "step": "U5.identity", "today": ["[WARNING] i2c1.u5: access to unmodelled register 0x50"]}, {"boot_placed": true, "boot_today": false, "placed": ["[NOISY] i2c1.u5: write 0x36 <- 0x40 0x81"], "step": "U5.configure", "today": ["[NOISY] i2c1.u5: read 0x50 -> 0x00"]}, {"boot_placed": true, "boot_today": false, "placed": ["[NOISY] i2c1.u5: read 0x1D -> 0x00 0x00 0x19 0x40 0xF3 0x62"], "step": "U5.data", "today": ["[NOISY] i2c1.u5: read 0x01 -> 0x50", "[NOISY] i2c1.u5: write 0x36 <- 0x40 0x81", "[NOISY] i2c1.u5: read 0x1D -> 0x00 0x00 0x19 0x40 0xF3 0x62"]}] unplaced ['[WARNING] i2c1.u5: access to unmodelled register 0x50', '[NOISY] i2c1.u5: read 0x50 -> 0x00']
- **c2/held-low-sda U7** (Bmp581, 3 steps, 0 lines, whole log = script: False):
  - (2) {'extra': [], 'held': 0, 'none_lines': 0, 'of': 3}
- **ctl-c2/held-low-sda U7** (Bmp581, 3 steps, 0 lines, whole log = script: False):
  - (2) {'extra': [], 'held': 0, 'none_lines': 0, 'of': 3}
- **ctl-ext-cfg37/ext-mutant U5** (Bmp581, 3 steps, 4 lines, whole log = script: False):
  - (4) ['U5.data: Write from tx 0, Read from tx 1']
  - (2) {'extra': ['[NOISY] i2c2.u5: write 0x37 <- 0x40 0x81', '[NOISY] i2c2.u5: read 0x1D -> 0x7F 0x7F 0x7F 0x7F 0x7F 0x7F'], 'held': 1, 'none_lines': 1, 'of': 3}
  - R1A: moved [{"boot_placed": false, "boot_today": true, "placed": ["[NOISY] i2c2.u5: read 0x01 -> 0x50", "[WARNING] i2c2.u5: write to read-only register 0x38 ignored", "[NOISY] i2c2.u5: write 0x37 <- 0x40 0x81"], "step": "U5.identity", "today": ["[NOISY] i2c2.u5: read 0x01 -> 0x50"]}, {"boot_placed": false, "boot_today": false, "placed": [], "step": "U5.configure", "today": ["[WARNING] i2c2.u5: write to read-only register 0x38 ignored"]}, {"boot_placed": false, "boot_today": false, "placed": ["[NOISY] i2c2.u5: read 0x1D -> 0x7F 0x7F 0x7F 0x7F 0x7F 0x7F"], "step": "U5.data", "today": ["[NOISY] i2c2.u5: write 0x37 <- 0x40 0x81", "[NOISY] i2c2.u5: read 0x1D -> 0x7F 0x7F 0x7F 0x7F 0x7F 0x7F"]}] unplaced ['[WARNING] i2c2.u5: write to read-only register 0x38 ignored', '[NOISY] i2c2.u5: write 0x37 <- 0x40 0x81']
  - R1B: moved [{"boot_placed": false, "boot_today": false, "placed": [], "step": "U5.configure", "today": ["[WARNING] i2c2.u5: write to read-only register 0x38 ignored"]}, {"boot_placed": false, "boot_today": false, "placed": ["[NOISY] i2c2.u5: read 0x1D -> 0x7F 0x7F 0x7F 0x7F 0x7F 0x7F"], "step": "U5.data", "today": ["[NOISY] i2c2.u5: write 0x37 <- 0x40 0x81", "[NOISY] i2c2.u5: read 0x1D -> 0x7F 0x7F 0x7F 0x7F 0x7F 0x7F"]}] unplaced ['[WARNING] i2c2.u5: write to read-only register 0x38 ignored', '[NOISY] i2c2.u5: write 0x37 <- 0x40 0x81']
  - R2A: moved [{"boot_placed": false, "boot_today": true, "placed": ["[NOISY] i2c2.u5: read 0x01 -> 0x50", "[WARNING] i2c2.u5: write to read-only register 0x38 ignored"], "step": "U5.identity", "today": ["[NOISY] i2c2.u5: read 0x01 -> 0x50"]}, {"boot_placed": false, "boot_today": false, "placed": ["[NOISY] i2c2.u5: write 0x37 <- 0x40 0x81"], "step": "U5.configure", "today": ["[WARNING] i2c2.u5: write to read-only register 0x38 ignored"]}, {"boot_placed": false, "boot_today": false, "placed": ["[NOISY] i2c2.u5: read 0x1D -> 0x7F 0x7F 0x7F 0x7F 0x7F 0x7F"], "step": "U5.data", "today": ["[NOISY] i2c2.u5: write 0x37 <- 0x40 0x81", "[NOISY] i2c2.u5: read 0x1D -> 0x7F 0x7F 0x7F 0x7F 0x7F 0x7F"]}] unplaced ['[WARNING] i2c2.u5: write to read-only register 0x38 ignored']
  - R2B: moved [{"boot_placed": false, "boot_today": false, "placed": ["[NOISY] i2c2.u5: write 0x37 <- 0x40 0x81"], "step": "U5.configure", "today": ["[WARNING] i2c2.u5: write to read-only register 0x38 ignored"]}, {"boot_placed": false, "boot_today": false, "placed": ["[NOISY] i2c2.u5: read 0x1D -> 0x7F 0x7F 0x7F 0x7F 0x7F 0x7F"], "step": "U5.data", "today": ["[NOISY] i2c2.u5: write 0x37 <- 0x40 0x81", "[NOISY] i2c2.u5: read 0x1D -> 0x7F 0x7F 0x7F 0x7F 0x7F 0x7F"]}] unplaced ['[WARNING] i2c2.u5: write to read-only register 0x38 ignored']
  - R2B-line: moved [{"boot_placed": false, "boot_today": false, "placed": ["[NOISY] i2c2.u5: write 0x37 <- 0x40 0x81"], "step": "U5.configure", "today": ["[WARNING] i2c2.u5: write to read-only register 0x38 ignored"]}, {"boot_placed": false, "boot_today": false, "placed": ["[NOISY] i2c2.u5: read 0x1D -> 0x7F 0x7F 0x7F 0x7F 0x7F 0x7F"], "step": "U5.data", "today": ["[NOISY] i2c2.u5: write 0x37 <- 0x40 0x81", "[NOISY] i2c2.u5: read 0x1D -> 0x7F 0x7F 0x7F 0x7F 0x7F 0x7F"]}] unplaced ['[WARNING] i2c2.u5: write to read-only register 0x38 ignored']
  - R2B-dev: moved [{"boot_placed": false, "boot_today": false, "placed": ["[NOISY] i2c2.u5: write 0x37 <- 0x40 0x81"], "step": "U5.configure", "today": ["[WARNING] i2c2.u5: write to read-only register 0x38 ignored"]}, {"boot_placed": false, "boot_today": false, "placed": ["[NOISY] i2c2.u5: read 0x1D -> 0x7F 0x7F 0x7F 0x7F 0x7F 0x7F"], "step": "U5.data", "today": ["[NOISY] i2c2.u5: write 0x37 <- 0x40 0x81", "[NOISY] i2c2.u5: read 0x1D -> 0x7F 0x7F 0x7F 0x7F 0x7F 0x7F"]}] unplaced ['[WARNING] i2c2.u5: write to read-only register 0x38 ignored']
  - R2B-min: moved [{"boot_placed": false, "boot_today": false, "placed": ["[NOISY] i2c2.u5: write 0x37 <- 0x40 0x81"], "step": "U5.configure", "today": ["[WARNING] i2c2.u5: write to read-only register 0x38 ignored"]}, {"boot_placed": false, "boot_today": false, "placed": ["[NOISY] i2c2.u5: read 0x1D -> 0x7F 0x7F 0x7F 0x7F 0x7F 0x7F"], "step": "U5.data", "today": ["[NOISY] i2c2.u5: write 0x37 <- 0x40 0x81", "[NOISY] i2c2.u5: read 0x1D -> 0x7F 0x7F 0x7F 0x7F 0x7F 0x7F"]}] unplaced ['[WARNING] i2c2.u5: write to read-only register 0x38 ignored']
  - R3B: moved [{"boot_placed": false, "boot_today": false, "placed": ["[NOISY] i2c2.u5: write 0x37 <- 0x40 0x81"], "step": "U5.configure", "today": ["[WARNING] i2c2.u5: write to read-only register 0x38 ignored"]}, {"boot_placed": false, "boot_today": false, "placed": ["[NOISY] i2c2.u5: read 0x1D -> 0x7F 0x7F 0x7F 0x7F 0x7F 0x7F"], "step": "U5.data", "today": ["[NOISY] i2c2.u5: write 0x37 <- 0x40 0x81", "[NOISY] i2c2.u5: read 0x1D -> 0x7F 0x7F 0x7F 0x7F 0x7F 0x7F"]}] unplaced ['[WARNING] i2c2.u5: write to read-only register 0x38 ignored']
  - R3B-min: moved [{"boot_placed": false, "boot_today": false, "placed": ["[NOISY] i2c2.u5: write 0x37 <- 0x40 0x81"], "step": "U5.configure", "today": ["[WARNING] i2c2.u5: write to read-only register 0x38 ignored"]}, {"boot_placed": false, "boot_today": false, "placed": ["[NOISY] i2c2.u5: read 0x1D -> 0x7F 0x7F 0x7F 0x7F 0x7F 0x7F"], "step": "U5.data", "today": ["[NOISY] i2c2.u5: write 0x37 <- 0x40 0x81", "[NOISY] i2c2.u5: read 0x1D -> 0x7F 0x7F 0x7F 0x7F 0x7F 0x7F"]}] unplaced ['[WARNING] i2c2.u5: write to read-only register 0x38 ignored']
- **ext/ext-held-low-sda U5** (Bmp581, 3 steps, 0 lines, whole log = script: False):
  - (2) {'extra': [], 'held': 0, 'none_lines': 0, 'of': 3}
- **ext-cfg37/ext-mutant U5** (Bmp581, 3 steps, 4 lines, whole log = script: False):
  - (4) ['U5.data: Write from tx 0, Read from tx 1']
  - (2) {'extra': ['[NOISY] i2c2.u5: write 0x37 <- 0x40 0x81', '[NOISY] i2c2.u5: read 0x1D -> 0x7F 0x7F 0x7F 0x7F 0x7F 0x7F'], 'held': 1, 'none_lines': 1, 'of': 3}
  - R1A: moved [{"boot_placed": false, "boot_today": true, "placed": ["[NOISY] i2c2.u5: read 0x01 -> 0x50", "[WARNING] i2c2.u5: write to read-only register 0x38 ignored", "[NOISY] i2c2.u5: write 0x37 <- 0x40 0x81"], "step": "U5.identity", "today": ["[NOISY] i2c2.u5: read 0x01 -> 0x50"]}, {"boot_placed": false, "boot_today": false, "placed": [], "step": "U5.configure", "today": ["[WARNING] i2c2.u5: write to read-only register 0x38 ignored"]}, {"boot_placed": false, "boot_today": false, "placed": ["[NOISY] i2c2.u5: read 0x1D -> 0x7F 0x7F 0x7F 0x7F 0x7F 0x7F"], "step": "U5.data", "today": ["[NOISY] i2c2.u5: write 0x37 <- 0x40 0x81", "[NOISY] i2c2.u5: read 0x1D -> 0x7F 0x7F 0x7F 0x7F 0x7F 0x7F"]}] unplaced ['[WARNING] i2c2.u5: write to read-only register 0x38 ignored', '[NOISY] i2c2.u5: write 0x37 <- 0x40 0x81']
  - R1B: moved [{"boot_placed": false, "boot_today": false, "placed": [], "step": "U5.configure", "today": ["[WARNING] i2c2.u5: write to read-only register 0x38 ignored"]}, {"boot_placed": false, "boot_today": false, "placed": ["[NOISY] i2c2.u5: read 0x1D -> 0x7F 0x7F 0x7F 0x7F 0x7F 0x7F"], "step": "U5.data", "today": ["[NOISY] i2c2.u5: write 0x37 <- 0x40 0x81", "[NOISY] i2c2.u5: read 0x1D -> 0x7F 0x7F 0x7F 0x7F 0x7F 0x7F"]}] unplaced ['[WARNING] i2c2.u5: write to read-only register 0x38 ignored', '[NOISY] i2c2.u5: write 0x37 <- 0x40 0x81']
  - R2A: moved [{"boot_placed": false, "boot_today": true, "placed": ["[NOISY] i2c2.u5: read 0x01 -> 0x50", "[WARNING] i2c2.u5: write to read-only register 0x38 ignored"], "step": "U5.identity", "today": ["[NOISY] i2c2.u5: read 0x01 -> 0x50"]}, {"boot_placed": false, "boot_today": false, "placed": ["[NOISY] i2c2.u5: write 0x37 <- 0x40 0x81"], "step": "U5.configure", "today": ["[WARNING] i2c2.u5: write to read-only register 0x38 ignored"]}, {"boot_placed": false, "boot_today": false, "placed": ["[NOISY] i2c2.u5: read 0x1D -> 0x7F 0x7F 0x7F 0x7F 0x7F 0x7F"], "step": "U5.data", "today": ["[NOISY] i2c2.u5: write 0x37 <- 0x40 0x81", "[NOISY] i2c2.u5: read 0x1D -> 0x7F 0x7F 0x7F 0x7F 0x7F 0x7F"]}] unplaced ['[WARNING] i2c2.u5: write to read-only register 0x38 ignored']
  - R2B: moved [{"boot_placed": false, "boot_today": false, "placed": ["[NOISY] i2c2.u5: write 0x37 <- 0x40 0x81"], "step": "U5.configure", "today": ["[WARNING] i2c2.u5: write to read-only register 0x38 ignored"]}, {"boot_placed": false, "boot_today": false, "placed": ["[NOISY] i2c2.u5: read 0x1D -> 0x7F 0x7F 0x7F 0x7F 0x7F 0x7F"], "step": "U5.data", "today": ["[NOISY] i2c2.u5: write 0x37 <- 0x40 0x81", "[NOISY] i2c2.u5: read 0x1D -> 0x7F 0x7F 0x7F 0x7F 0x7F 0x7F"]}] unplaced ['[WARNING] i2c2.u5: write to read-only register 0x38 ignored']
  - R2B-line: moved [{"boot_placed": false, "boot_today": false, "placed": ["[NOISY] i2c2.u5: write 0x37 <- 0x40 0x81"], "step": "U5.configure", "today": ["[WARNING] i2c2.u5: write to read-only register 0x38 ignored"]}, {"boot_placed": false, "boot_today": false, "placed": ["[NOISY] i2c2.u5: read 0x1D -> 0x7F 0x7F 0x7F 0x7F 0x7F 0x7F"], "step": "U5.data", "today": ["[NOISY] i2c2.u5: write 0x37 <- 0x40 0x81", "[NOISY] i2c2.u5: read 0x1D -> 0x7F 0x7F 0x7F 0x7F 0x7F 0x7F"]}] unplaced ['[WARNING] i2c2.u5: write to read-only register 0x38 ignored']
  - R2B-dev: moved [{"boot_placed": false, "boot_today": false, "placed": ["[NOISY] i2c2.u5: write 0x37 <- 0x40 0x81"], "step": "U5.configure", "today": ["[WARNING] i2c2.u5: write to read-only register 0x38 ignored"]}, {"boot_placed": false, "boot_today": false, "placed": ["[NOISY] i2c2.u5: read 0x1D -> 0x7F 0x7F 0x7F 0x7F 0x7F 0x7F"], "step": "U5.data", "today": ["[NOISY] i2c2.u5: write 0x37 <- 0x40 0x81", "[NOISY] i2c2.u5: read 0x1D -> 0x7F 0x7F 0x7F 0x7F 0x7F 0x7F"]}] unplaced ['[WARNING] i2c2.u5: write to read-only register 0x38 ignored']
  - R2B-min: moved [{"boot_placed": false, "boot_today": false, "placed": ["[NOISY] i2c2.u5: write 0x37 <- 0x40 0x81"], "step": "U5.configure", "today": ["[WARNING] i2c2.u5: write to read-only register 0x38 ignored"]}, {"boot_placed": false, "boot_today": false, "placed": ["[NOISY] i2c2.u5: read 0x1D -> 0x7F 0x7F 0x7F 0x7F 0x7F 0x7F"], "step": "U5.data", "today": ["[NOISY] i2c2.u5: write 0x37 <- 0x40 0x81", "[NOISY] i2c2.u5: read 0x1D -> 0x7F 0x7F 0x7F 0x7F 0x7F 0x7F"]}] unplaced ['[WARNING] i2c2.u5: write to read-only register 0x38 ignored']
  - R3B: moved [{"boot_placed": false, "boot_today": false, "placed": ["[NOISY] i2c2.u5: write 0x37 <- 0x40 0x81"], "step": "U5.configure", "today": ["[WARNING] i2c2.u5: write to read-only register 0x38 ignored"]}, {"boot_placed": false, "boot_today": false, "placed": ["[NOISY] i2c2.u5: read 0x1D -> 0x7F 0x7F 0x7F 0x7F 0x7F 0x7F"], "step": "U5.data", "today": ["[NOISY] i2c2.u5: write 0x37 <- 0x40 0x81", "[NOISY] i2c2.u5: read 0x1D -> 0x7F 0x7F 0x7F 0x7F 0x7F 0x7F"]}] unplaced ['[WARNING] i2c2.u5: write to read-only register 0x38 ignored']
  - R3B-min: moved [{"boot_placed": false, "boot_today": false, "placed": ["[NOISY] i2c2.u5: write 0x37 <- 0x40 0x81"], "step": "U5.configure", "today": ["[WARNING] i2c2.u5: write to read-only register 0x38 ignored"]}, {"boot_placed": false, "boot_today": false, "placed": ["[NOISY] i2c2.u5: read 0x1D -> 0x7F 0x7F 0x7F 0x7F 0x7F 0x7F"], "step": "U5.data", "today": ["[NOISY] i2c2.u5: write 0x37 <- 0x40 0x81", "[NOISY] i2c2.u5: read 0x1D -> 0x7F 0x7F 0x7F 0x7F 0x7F 0x7F"]}] unplaced ['[WARNING] i2c2.u5: write to read-only register 0x38 ignored']
- **seeded-wrong-nbytes/pullup-2k2 U2** (WriteRead, 1 steps, 3 lines, whole log = script: False):
  - R1A: moved [] unplaced ['[WARNING] i2c1.u2: Unhandled read from offset 0xD1.']
  - R1B: moved [{"boot_placed": false, "boot_today": false, "placed": ["[NOISY] i2c1.u2: Write D0", "[NOISY] i2c1.u2: Read 60 0"], "step": "U2.identity", "today": ["[NOISY] i2c1.u2: Write D0", "[WARNING] i2c1.u2: Unhandled read from offset 0xD1.", "[NOISY] i2c1.u2: Read 60 0"]}] unplaced ['[WARNING] i2c1.u2: Unhandled read from offset 0xD1.']
  - R2A: moved [] unplaced ['[WARNING] i2c1.u2: Unhandled read from offset 0xD1.']
  - R2B: moved [{"boot_placed": false, "boot_today": false, "placed": ["[NOISY] i2c1.u2: Write D0", "[NOISY] i2c1.u2: Read 60 0"], "step": "U2.identity", "today": ["[NOISY] i2c1.u2: Write D0", "[WARNING] i2c1.u2: Unhandled read from offset 0xD1.", "[NOISY] i2c1.u2: Read 60 0"]}] unplaced ['[WARNING] i2c1.u2: Unhandled read from offset 0xD1.']
  - R2B-line: moved [{"boot_placed": false, "boot_today": false, "placed": ["[NOISY] i2c1.u2: Write D0", "[NOISY] i2c1.u2: Read 60 0"], "step": "U2.identity", "today": ["[NOISY] i2c1.u2: Write D0", "[WARNING] i2c1.u2: Unhandled read from offset 0xD1.", "[NOISY] i2c1.u2: Read 60 0"]}] unplaced ['[WARNING] i2c1.u2: Unhandled read from offset 0xD1.']
  - R2B-dev: moved [{"boot_placed": false, "boot_today": false, "placed": ["[NOISY] i2c1.u2: Write D0", "[NOISY] i2c1.u2: Read 60 0"], "step": "U2.identity", "today": ["[NOISY] i2c1.u2: Write D0", "[WARNING] i2c1.u2: Unhandled read from offset 0xD1.", "[NOISY] i2c1.u2: Read 60 0"]}] unplaced ['[WARNING] i2c1.u2: Unhandled read from offset 0xD1.']
  - R2B-min: moved [{"boot_placed": false, "boot_today": false, "placed": ["[NOISY] i2c1.u2: Write D0", "[NOISY] i2c1.u2: Read 60 0"], "step": "U2.identity", "today": ["[NOISY] i2c1.u2: Write D0", "[WARNING] i2c1.u2: Unhandled read from offset 0xD1.", "[NOISY] i2c1.u2: Read 60 0"]}] unplaced ['[WARNING] i2c1.u2: Unhandled read from offset 0xD1.']
  - R3B: moved [{"boot_placed": false, "boot_today": false, "placed": ["[NOISY] i2c1.u2: Write D0", "[NOISY] i2c1.u2: Read 60 0"], "step": "U2.identity", "today": ["[NOISY] i2c1.u2: Write D0", "[WARNING] i2c1.u2: Unhandled read from offset 0xD1.", "[NOISY] i2c1.u2: Read 60 0"]}] unplaced ['[WARNING] i2c1.u2: Unhandled read from offset 0xD1.']
  - R3B-min: moved [{"boot_placed": false, "boot_today": false, "placed": ["[NOISY] i2c1.u2: Write D0", "[NOISY] i2c1.u2: Read 60 0"], "step": "U2.identity", "today": ["[NOISY] i2c1.u2: Write D0", "[WARNING] i2c1.u2: Unhandled read from offset 0xD1.", "[NOISY] i2c1.u2: Read 60 0"]}] unplaced ['[WARNING] i2c1.u2: Unhandled read from offset 0xD1.']
- **seeded-wrong-register/pullup-2k2 U2** (WriteRead, 1 steps, 3 lines, whole log = script: False):
  - R1A: moved [] unplaced ['[NOISY] i2c1.u2: Write D1', '[WARNING] i2c1.u2: Unhandled read from offset 0xD1.', '[NOISY] i2c1.u2: Read 0']
  - R1B: moved [{"boot_placed": false, "boot_today": false, "placed": [], "step": "U2.identity", "today": ["[NOISY] i2c1.u2: Write D1", "[WARNING] i2c1.u2: Unhandled read from offset 0xD1.", "[NOISY] i2c1.u2: Read 0"]}] unplaced ['[NOISY] i2c1.u2: Write D1', '[WARNING] i2c1.u2: Unhandled read from offset 0xD1.', '[NOISY] i2c1.u2: Read 0']
  - R2A: moved [] unplaced ['[WARNING] i2c1.u2: Unhandled read from offset 0xD1.']
  - R2B: moved [{"boot_placed": false, "boot_today": false, "placed": ["[NOISY] i2c1.u2: Write D1", "[NOISY] i2c1.u2: Read 0"], "step": "U2.identity", "today": ["[NOISY] i2c1.u2: Write D1", "[WARNING] i2c1.u2: Unhandled read from offset 0xD1.", "[NOISY] i2c1.u2: Read 0"]}] unplaced ['[WARNING] i2c1.u2: Unhandled read from offset 0xD1.']
  - R2B-line: moved [{"boot_placed": false, "boot_today": false, "placed": ["[NOISY] i2c1.u2: Write D1", "[NOISY] i2c1.u2: Read 0"], "step": "U2.identity", "today": ["[NOISY] i2c1.u2: Write D1", "[WARNING] i2c1.u2: Unhandled read from offset 0xD1.", "[NOISY] i2c1.u2: Read 0"]}] unplaced ['[WARNING] i2c1.u2: Unhandled read from offset 0xD1.']
  - R2B-dev: moved [{"boot_placed": false, "boot_today": false, "placed": ["[NOISY] i2c1.u2: Write D1", "[NOISY] i2c1.u2: Read 0"], "step": "U2.identity", "today": ["[NOISY] i2c1.u2: Write D1", "[WARNING] i2c1.u2: Unhandled read from offset 0xD1.", "[NOISY] i2c1.u2: Read 0"]}] unplaced ['[WARNING] i2c1.u2: Unhandled read from offset 0xD1.']
  - R2B-min: moved [{"boot_placed": false, "boot_today": false, "placed": ["[NOISY] i2c1.u2: Write D1", "[NOISY] i2c1.u2: Read 0"], "step": "U2.identity", "today": ["[NOISY] i2c1.u2: Write D1", "[WARNING] i2c1.u2: Unhandled read from offset 0xD1.", "[NOISY] i2c1.u2: Read 0"]}] unplaced ['[WARNING] i2c1.u2: Unhandled read from offset 0xD1.']
  - R3B: moved [{"boot_placed": false, "boot_today": false, "placed": ["[NOISY] i2c1.u2: Write D1", "[NOISY] i2c1.u2: Read 0"], "step": "U2.identity", "today": ["[NOISY] i2c1.u2: Write D1", "[WARNING] i2c1.u2: Unhandled read from offset 0xD1.", "[NOISY] i2c1.u2: Read 0"]}] unplaced ['[WARNING] i2c1.u2: Unhandled read from offset 0xD1.']
  - R3B-min: moved [{"boot_placed": false, "boot_today": false, "placed": ["[NOISY] i2c1.u2: Write D1", "[NOISY] i2c1.u2: Read 0"], "step": "U2.identity", "today": ["[NOISY] i2c1.u2: Write D1", "[WARNING] i2c1.u2: Unhandled read from offset 0xD1.", "[NOISY] i2c1.u2: Read 0"]}] unplaced ['[WARNING] i2c1.u2: Unhandled read from offset 0xD1.']
- **t2-baro-pf/baro-mutant U6** (Icp, 3 steps, 5 lines, whole log = script: False):
  - R1A: moved [{"boot_placed": false, "boot_today": true, "placed": ["[DEBUG] i2c1.u6: Received command: [0x80, 0x5D]", "[DEBUG] i2c1.u6: Received command: [0x48, 0xA3]", "[DEBUG] i2c1.u6: Reading data with CRC: [0x30, 0x3D], current command: 0x48A3 (NormalPressureTemperature), requested: 2 bytes"], "step": "U6.configure", "today": ["[DEBUG] i2c1.u6: Received command: [0x80, 0x5D]"]}, {"boot_placed": false, "boot_today": false, "placed": [], "step": "U6.data", "today": ["[DEBUG] i2c1.u6: Received command: [0x48, 0xA3]", "[DEBUG] i2c1.u6: Reading data with CRC: [0x30, 0x3D], current command: 0x48A3 (NormalPressureTemperature), requested: 2 bytes"]}] unplaced ['[DEBUG] i2c1.u6: Received command: [0x48, 0xA3]', '[DEBUG] i2c1.u6: Reading data with CRC: [0x30, 0x3D], current command: 0x48A3 (NormalPressureTemperature), requested: 2 bytes']
  - R1B: moved [{"boot_placed": false, "boot_today": false, "placed": [], "step": "U6.data", "today": ["[DEBUG] i2c1.u6: Received command: [0x48, 0xA3]", "[DEBUG] i2c1.u6: Reading data with CRC: [0x30, 0x3D], current command: 0x48A3 (NormalPressureTemperature), requested: 2 bytes"]}] unplaced ['[DEBUG] i2c1.u6: Received command: [0x48, 0xA3]', '[DEBUG] i2c1.u6: Reading data with CRC: [0x30, 0x3D], current command: 0x48A3 (NormalPressureTemperature), requested: 2 bytes']
  - R2A: moved [{"boot_placed": false, "boot_today": true, "placed": ["[DEBUG] i2c1.u6: Received command: [0x80, 0x5D]", "[DEBUG] i2c1.u6: Received command: [0x48, 0xA3]", "[DEBUG] i2c1.u6: Reading data with CRC: [0x30, 0x3D], current command: 0x48A3 (NormalPressureTemperature), requested: 2 bytes"], "step": "U6.configure", "today": ["[DEBUG] i2c1.u6: Received command: [0x80, 0x5D]"]}, {"boot_placed": false, "boot_today": false, "placed": [], "step": "U6.data", "today": ["[DEBUG] i2c1.u6: Received command: [0x48, 0xA3]", "[DEBUG] i2c1.u6: Reading data with CRC: [0x30, 0x3D], current command: 0x48A3 (NormalPressureTemperature), requested: 2 bytes"]}] unplaced ['[DEBUG] i2c1.u6: Received command: [0x48, 0xA3]', '[DEBUG] i2c1.u6: Reading data with CRC: [0x30, 0x3D], current command: 0x48A3 (NormalPressureTemperature), requested: 2 bytes']
  - R2B: moved [{"boot_placed": false, "boot_today": false, "placed": [], "step": "U6.data", "today": ["[DEBUG] i2c1.u6: Received command: [0x48, 0xA3]", "[DEBUG] i2c1.u6: Reading data with CRC: [0x30, 0x3D], current command: 0x48A3 (NormalPressureTemperature), requested: 2 bytes"]}] unplaced ['[DEBUG] i2c1.u6: Received command: [0x48, 0xA3]', '[DEBUG] i2c1.u6: Reading data with CRC: [0x30, 0x3D], current command: 0x48A3 (NormalPressureTemperature), requested: 2 bytes']
  - R2B-line: moved [{"boot_placed": false, "boot_today": false, "placed": [], "step": "U6.data", "today": ["[DEBUG] i2c1.u6: Received command: [0x48, 0xA3]", "[DEBUG] i2c1.u6: Reading data with CRC: [0x30, 0x3D], current command: 0x48A3 (NormalPressureTemperature), requested: 2 bytes"]}] unplaced ['[DEBUG] i2c1.u6: Received command: [0x48, 0xA3]', '[DEBUG] i2c1.u6: Reading data with CRC: [0x30, 0x3D], current command: 0x48A3 (NormalPressureTemperature), requested: 2 bytes']
  - R2B-dev: moved [{"boot_placed": false, "boot_today": false, "placed": [], "step": "U6.data", "today": ["[DEBUG] i2c1.u6: Received command: [0x48, 0xA3]", "[DEBUG] i2c1.u6: Reading data with CRC: [0x30, 0x3D], current command: 0x48A3 (NormalPressureTemperature), requested: 2 bytes"]}] unplaced ['[DEBUG] i2c1.u6: Received command: [0x48, 0xA3]', '[DEBUG] i2c1.u6: Reading data with CRC: [0x30, 0x3D], current command: 0x48A3 (NormalPressureTemperature), requested: 2 bytes']
  - R2B-min: moved [{"boot_placed": false, "boot_today": false, "placed": [], "step": "U6.data", "today": ["[DEBUG] i2c1.u6: Received command: [0x48, 0xA3]", "[DEBUG] i2c1.u6: Reading data with CRC: [0x30, 0x3D], current command: 0x48A3 (NormalPressureTemperature), requested: 2 bytes"]}] unplaced ['[DEBUG] i2c1.u6: Received command: [0x48, 0xA3]', '[DEBUG] i2c1.u6: Reading data with CRC: [0x30, 0x3D], current command: 0x48A3 (NormalPressureTemperature), requested: 2 bytes']

## Each run's whole-log class per device

- bmp/bmp-baseline: U2:script U5:script; verdict pass=True
- bmp/bmp-held-low-sda: U2:0 lines U5:0 lines; verdict pass=False
- bmp-0x47/bmp-mutant: U2:0 lines U5:5 lines; verdict pass=False
- c0/motors-dshot600-droop: U2:script; verdict pass=True
- c0/motors-dshot600: U2:script; verdict pass=True
- c1/motors-pwm: U2:script; verdict pass=True
- c2/baseline: U7:script; verdict pass=True
- c2/battery-sag-tolerated: U7:script; verdict pass=True
- c2/droop-bor3: U7:script; verdict pass=True
- c2/droop-tolerated: U7:script; verdict pass=True
- c2/held-low-sda: U7:0 lines; verdict pass=False
- c2/serial-rx-open-console: U7:script; verdict pass=False
- c2/serial-rx-open-crsf: U7:script; verdict pass=False
- c2/serial-rx-open-mavlink: U7:script; verdict pass=False
- c2/spi-absent-u4: U7:script; verdict pass=False
- c2/spi-absent-u5: U7:script; verdict pass=False
- c2/vdd5v-out-of-band: U7:script; verdict pass=False
- c3/serial-four-open-crsf: U2:script; verdict pass=False
- c3/serial-four-open-mavlink: U2:script; verdict pass=False
- c3/serial-four: U2:script; verdict pass=True
- c4/baseline: U2:script; verdict pass=True
- c4/battery-sag-tolerated: U2:script; verdict pass=True
- c4/vdd1v8-out-of-band: U2:script; verdict pass=False
- c5/droop-bor3: U2:script; verdict pass=True
- c5/droop-tolerated: U2:script; verdict pass=True
- c5/pullup-2k2: U2:script; verdict pass=True
- c5/pullup-47k: U2:0 lines; verdict pass=False
- c5/cap-220p-2k2: U2:0 lines; verdict pass=False
- c5/cap-390p-sda-2k2: U2:0 lines; verdict pass=False
- c5/held-low-scl: U2:0 lines; verdict pass=False
- c5/held-low-sda: U2:0 lines; verdict pass=False
- c5/pullup-100k: U2:0 lines; verdict pass=False
- c5/pullup-10k: U2:0 lines; verdict pass=False
- c5/pullup-3k3: U2:script; verdict pass=True
- c5/pullup-3k9: U2:0 lines; verdict pass=False
- c5/pullup-4k7: U2:0 lines; verdict pass=False
- c5/serial-rx-dead: U2:script; verdict pass=True
- c5/serial-rx-open-console: U2:script; verdict pass=False
- c5/spi-absent-u4: U2:script; verdict pass=False
- c6/droop-bor3: U2:script; verdict pass=True
- c6/pullup-2k2: U2:script; verdict pass=True
- c6/pullup-3k3: U2:script; verdict pass=True
- c6/serial-rx-open-console: U2:script; verdict pass=False
- c6/spi-absent-u4: U2:script; verdict pass=False
- ctl-c2/baseline: U7:script; verdict pass=True
- ctl-c2/held-low-sda: U7:0 lines; verdict pass=False
- ctl-ext-cfg37/ext-mutant: U2:script U5:4 lines; verdict pass=False
- ext/ext-baseline: U2:script U5:script; verdict pass=True
- ext/ext-held-low-sda: U2:script U5:0 lines; verdict pass=False
- ext-cfg37/ext-mutant: U2:script U5:4 lines; verdict pass=False
- seeded-console-rx-af-wrong/serial-rx-dead: U2:script; verdict pass=False
- seeded-cs-never/pullup-2k2: U2:script; verdict pass=False
- seeded-gpiob-clock-skipped/pullup-2k2: U2:script; verdict pass=False
- seeded-i2c-clock-skipped/pullup-2k2: U2:0 lines; verdict pass=False
- seeded-leave-firmware/pullup-2k2: U2:script; verdict pass=False
- seeded-leave-firmware/pullup-47k: U2:0 lines; verdict pass=False
- seeded-leave-firmware/cap-390p-sda-2k2: U2:0 lines; verdict pass=False
- seeded-motor-af-wrong/motors-pwm: U2:script; verdict pass=False
- seeded-no-boot-word/pullup-2k2: U2:script; verdict pass=False
- seeded-scl-af-wrong/pullup-2k2: U2:script; verdict pass=False
- seeded-scl-mode-output/pullup-2k2: U2:script; verdict pass=False
- seeded-sda-af-wrong/pullup-2k2: U2:script; verdict pass=False
- seeded-wrong-address/pullup-2k2: U2:0 lines; verdict pass=False
- seeded-wrong-address/pullup-47k: U2:0 lines; verdict pass=False
- seeded-wrong-address/pullup-3k3: U2:0 lines; verdict pass=False
- seeded-wrong-nbytes/pullup-2k2: U2:3 lines; verdict pass=False
- seeded-wrong-register/pullup-2k2: U2:3 lines; verdict pass=False
- t2-baro/baro-baseline: U2:script U6:script; verdict pass=True
- t2-baro-pf/baro-mutant: U2:script U6:5 lines; verdict pass=False
- t2-collide/two-bme-collide: U2:0 lines U5:4 lines; verdict pass=False
- t2-ext-bor/ext-bmp581-droop-bor3: U2:script U5:script; verdict pass=True
- t2-logger/logger-baseline: U20:script; verdict pass=True
- t2-logger-bor/logger-droop-bor3: U20:script; verdict pass=True
- t2-s02s/s02s-baseline: U2:script U7:script; verdict pass=True
- t2-s02s-bor/s02-shaped-droop: U2:script U7:script; verdict pass=True
- two/two-baseline: U2:script U5:script; verdict pass=True
- two/two-held-low-sda: U2:0 lines U5:0 lines; verdict pass=False
- two-0x75/two-mutant: U2:0 lines U5:script; verdict pass=False
- two-0x77/two-mutant: U2:0 lines U5:4 lines; verdict pass=False
