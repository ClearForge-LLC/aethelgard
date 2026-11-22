# AET-WO-1214 — predictions, committed before the measurement

Written 2026-11-20, before the measurement ran on `main` 781ea3b5. Not blind:
`spike/1213/measure-merged.out` already prints an `n_BAUD` column
(`map_baudrate(b) == b`) for every port it found, and the builder read it before
writing these. The re-run is on current `main`, because fixtures have landed since.

1. **No committed specification or solved topology is refused.** Every non-console
   port is at 57600, 100000, 115200 or 420000 baud. Each is above 2000, so
   `map_baudrate(b) == b` (`AP_SerialManager.cpp:762-766` at `Copter-4.7.2`).
2. **The roles the rule covers are `mavlink` and `gps`.** `AP_SerialManager::init`
   begins both at `state[i].baudrate()`, which is `map_baudrate(SERIALn_BAUD)`
   (`:470-475`, `:488-493`; `AP_SerialManager.h:151-153`). `rc_crsf` and `rc_sbus`
   are excluded. Their port is only handed to `AP_RCProtocol` (`:536-543`), which
   sets the rate itself (`AP_RCProtocol.cpp:326-352`). `console` is SERIAL0 and keeps
   its own 13c rule (`ConsoleBaudNotRoundTrip`, `AET-WO-1213`).
3. **The script covers tracked `*.json` only.** Specifications built inside Rust tests,
   and topologies solved while a test runs, are covered by `cargo test --workspace`
   and by CI's tier 2, both run with the rule in.
