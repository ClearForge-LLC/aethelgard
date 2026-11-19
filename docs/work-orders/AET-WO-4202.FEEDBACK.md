# AET-WO-4202 FEEDBACK

Base: `main` `ee835bde` (post-`4400g`, #238). D1 ruled R (U2 removed), D2 as drafted.

## 0. Predictions (committed before the first run)

Labels: **measured** (a run shown here or in `4201`), **read** (a file read), **inferred** (neither).

| # | Question | Prediction | Label |
|---|---|---|---|
| P1 | Does the stamp fault reproduce on `ee835bde`? | Yes, on `droop-bor3` alone, on an `AETHELGARD-INJECT` line, 1 ns early. The numbers differ from `4201`'s (`4400g` rebuilt the timers and USARTs at 64 MHz, so the injection's count moves). | inferred |
| P2 | Its cause | H1: the BMP581 model's 1 GHz one-shot timer is a sync point off the 10 ns instruction grid. A sync point that is not a multiple of 10 ns leaves the core's count and virtual time out of step by a fraction of an instruction across the reset, so the restarted count's origin is 1 ns off `released_ns + NUDGE_NS`. | inferred |
| P3 | Does the ICM-42688-P model (also ours, also timed) show it? | Yes, if its timer's events fall off the 10 ns grid in the boot after a release. Sentinel-02 carries both and showed no stamp refusal in `4201` (`s02-droop.txt`), so the phase matters; the fault needs an event off the grid in the window that sets the origin. | inferred |
| P4 | Sentinel-03 outline and least `extract` slack | Move: U2 and its decoupling leave the placement. | inferred |
| P5 | Sentinel-03 supply-gate figures | Move by U2's 1 mA load row. | inferred |
| P6 | `sentinel-02` hwdef lines | The `BARO BMP280 I2C:0:0x76` line goes; `BARO BMP581 I2C:0:0x46` stays. Nothing else in the hwdef moves. | read (`4201` §4.1, generator `hwdef/src/lib.rs` L686) |
| P7 | Foreign judge witness times and window margin | Move: ArduPilot probes one barometer fewer, so later witnesses come earlier. | inferred |
| P8 | Campaign `state_sequence_hash`es | Those of scenarios on `sentinel-02-own` move (D1). If the fix is in the twin's reset path, every droop scenario's hash could move, Sentinel-01's included, which is a §7 stop; a fix that renders the same commands for a BME280-only board leaves Sentinel-01's unchanged. | inferred |
| P9 | Tier-2 tests added | One (the regression run through the reset with a timed model of ours), plus one tier-1 test of the mechanism. | inferred |
