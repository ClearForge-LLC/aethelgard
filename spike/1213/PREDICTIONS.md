# AET-WO-1213 — predictions, committed before the measurement

Written 2026-11-20, before `measure.py` ran.

1. **Every committed specification's console passes the round trip**,
   `map_baudrate(b / 1000) == b` on `AP_SerialManager.cpp:737-770` at
   `Copter-4.7.2`. The bauds are 115200, 57600 and 9600, each of which
   divides to a listed code (115, 57, 9). No committed specification would be refused.
2. **Only the `console` role is reached.** ArduPilot opens only SERIAL0 at boot from its
   define (`init_console`, `:413-423`), and only `0_BAUD` stores the define divided by
   1000 (`:172`). `1_BAUD` to `9_BAUD` store `DEFAULT_SERIALn_BAUD` unchanged
   (`:199`-`:390`), so a `mavlink` or `gps` baud above 2000 maps to itself. `rc_crsf` and
   `rc_sbus` emit no baud define (hwdef F3, PR #189).
3. **The two example bauds in the WO, 100000 and 250000, round-trip.** `100` is a listed
   code (100000), and 250 is "any other kbaud" (250 × 1000). So the failing tests use bauds
   that do fail: 76800 (76 → 76000), 111000 (111 → 111100), 115000 (115 → 115200),
   3000000 (3000 → 3000), and 999 (0 → 57600).
