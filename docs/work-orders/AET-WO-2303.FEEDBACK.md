# AET-WO-2303 — FEEDBACK (spike; stops at §2)

Branch `wo/aet-2303-dynamic-volume`, from `main` at `ecc6be95`. The work order is read from `origin/claude/argon-repo-docs-synthesis-rjrb0d` (docs PR #198) and is not committed here. Prototypes live under `spike/2303/`; no crate changes.

Every answer is labelled **measured**, **read at source** or **inferred**.

## §1.1 The question first, at source: the faults `4500`'s campaign needs

**Where the list comes from (read at source):**
- roadmap §5, `AET-WO-4500`: "a fault campaign that includes an SPI fault". The P4 exit gate's second clause asks for "a **static** fault campaign whose electrical and SPI faults are attributed correctly (`INV-05`)".
- `AET-WO-3303`'s coverage rule, `crates/aethelgard-cli/src/verify.rs` L872–946: `--fab-out` refuses a campaign that leaves a declared I2C bus, SPI bus, serial port or rail uncovered. Rails other than the MCU's supply read `no fault kind droops it`, and so does `battery` once a power tree is declared.
- The override kinds in `crates/aethelgard-chaos/src/campaign.rs` L565–577: `pull_up_ohms`, `max_capacitance`, `line_held_low` (SDA, SCL), `droop`, `spi_device_absent`, `serial_rx_open`.
- `2301`'s fault kinds (`docs/work-orders/AET-WO-2301.FEEDBACK.md`): the declared droop (built), and the interrupt storm (P3, deferred to the first firmware that takes an interrupt).
- The fixture `crates/aethelgard-cli/tests/fixtures/sentinel-02-shaped.json`:
  - parts: BME280, TMP117 and BMP581 on I2C `SENSORS`; W25Q64JV on SPI `FLASH`; ICM-42688-P on SPI `IMU`;
  - four `dshot600` motor outputs on J3;
  - serial ports `MAVLINK` and `CRSF`;
  - domains `VDD_3V3` (the MCU's supply, fed from J2 pin 1), `VDD_5V` and `VDD_1V8`;
  - `power: null`, so it declares no battery.
- The roadmap rows owned by `4500` or the battery-sag work order: L559, L627, L646–648, L658, L673 and L674.

**Three facts that bound the table (read at source):**
- **F-a.** The fixture declares no power tree. A campaign over one that does is refused: `PowerNotSimulated`, `crates/aethelgard-cli/src/campaign.rs` L379–391. `4500`'s Sentinel-02 flies a 3S–4S pack (roadmap §5), so its specification will declare one.
- **F-b.** The P4 exit gate's own word is "static".
- **F-c.** The generated firmware's DShot output is "a continuous zero-throttle pattern, with no DMA":
  - after `CR1 = ARPE | CEN` nothing reads or writes the timer (`crates/aethelgard-svd-hal/README.md` L64; `AET-WO-4300a.md` L54);
  - the timer's `CEN` is set once per boot, and a rendered reset (`machine Reset`) clears it.

  So what the firmware commands during a run is: whether frames go out (`CEN`), and the frame's throttle (`CCR`, constant zero).

**The classification:**
- **static:** the analog side is computed once, before the boot, from declared values.
- **declared:** the analog side moves in time, but its waveform is a function of declared inputs only (`2301`'s droop).
- **firmware-dependent:** the electrical effect depends on what the firmware does during the run.

| # | Fault | Where it comes from | Fault kind today | Class | Why |
|---|---|---|---|---|---|
| 1 | Pull-up outside its band on `SENSORS` | 3303 coverage (I2C bus) | `pull_up_ohms` | **static** | The rise time is computed before the boot (`RUL-17`, §2.17) |
| 2 | A line's capacitance over its bound | 3303 coverage (I2C bus) | `max_capacitance` | **static** | As row 1 |
| 3 | SDA or SCL held low | 3303 coverage (I2C bus) | `line_held_low` | **static** | Rendered as the device's absence before the boot (`RUL-17`) |
| 4 | An SPI device absent (`U4` or `U5`) | `4500`'s "SPI fault"; 3303 coverage (SPI buses `FLASH`, `IMU`) | `spi_device_absent` | **static** | The device is left out of the platform |
| 5 | A serial port's RX line open | 3303 coverage (`MAVLINK`, `CRSF`); roadmap L647 | `serial_rx_open` | **static** | The line's state holds for the whole run. The peer's frames are a digital stimulus timed from the boot word; nothing analog moves |
| 6 | `VDD_3V3` (the MCU's supply) droops | 3303 coverage (MCU rail); `2301` | `droop` | **declared** | `2301` D1 and D8, ratified: solved once, applied at its crossings, nothing per quantum (`OQ-03`, §8) |
| 7 | `VDD_5V` or `VDD_1V8` droops | 3303 coverage: `no fault kind droops it` | none | **declared** (inferred) | Each rail's loads are the specification's declared loads and part-table rows (`aethelgard-ir/src/power.rs` L37–48); the firmware draws nothing from them that it chooses at run time |
| 8 | The battery sags under the motors' current, and through the regulator `VDD_3V3` with it | 3303 coverage (`battery`); roadmap L559, L648, L658, L673; `4500`'s 3S–4S pack | none (`PowerNotSimulated`) | **firmware-dependent** | See below |
| 9 | An interrupt storm | `2301` P3 | none (deferred) | not analog: **outside `OQ-03`** | The generated firmware enables no interrupt (`2301` P3, measured). A storm is digital stimulus; no solver runs per quantum |
| 10 | A motor pin's off-board load (ESC input plus cable) | roadmap L627 (4300a Q3, owned by `4500`) | none | **static** | A capacitance on a timer output: a rise time and Fmax check before the boot |
| 11 | J2's current rating against the battery sag | roadmap L673 (4300g K7) | none | **static** | A rating judged against a declared or computed current, with nothing in time |
| 12 | A covering fault moved past its declared tolerance | roadmap L646 (3303 F3) | rule over rows 1–2 | **static** | A rule on the static kinds |
| 13 | An off-board device absent (GPS compass on DS-009) | roadmap L635 (4300f D9) | `spi_device_absent`-like; none on I2C yet | **static** | As row 4 |

**Row 8: why the battery sag is firmware-dependent (read at source, with one inference marked).**
- On a quadcopter, the battery's current is the four ESCs' current. An ESC's current follows the throttle it is commanded on DShot, and it goes to zero when the commands stop.
  - That this is the dominant load is **inferred**: no ESC or motor part is declared anywhere in the repository.
- Who commands it is the firmware, during the run:
  - today, by setting the timer's `CEN` once per boot (F-c);
  - under ArduPilot, every control loop.

  Two firmwares on the same board draw different battery waveforms.
- **The loop closes.** A sag deep enough to cross the regulator's dropout drops `VDD_3V3` below BOR. The rendered reset clears `CEN`, the commands stop, the current falls and the battery recovers. The firmware boots again, sets `CEN`, and the load returns.
  - The waveform therefore depends on when the firmware boots and starts its timer: a reboot loop.
  - It cannot be solved before the boot, as `2301`'s droop is, without leaving the coupling out.
- **It can be declared instead, at a cost.** Roadmap L559 words the battery-sag chain as "fed into `--rail` from declared parts": declare a current profile as a stimulus, as `2301` declares its load.
  - That is honest as a stimulus.
  - It cannot show the reboot loop, because a reset that stops the motors would not relieve a declared load.

  Which reading `4500` takes is decision D1 below.
- **Today's firmware makes the dependence weak.** The command is a constant zero throttle, so a real ESC would draw its idle current. What the firmware controls today is only *when* frames are present.
  - **Nothing in the repository declares an ESC's supply current per throttle or per frame presence**: no part row, and 4300a Q3's motor load is the ESC's *input*.
  - The prototype therefore uses a **spike-declared** load map, labelled as such, to close the loop. Its magnitudes are not sourced.

**Result of §1.1: one row (8) is firmware-dependent, so §1.2 runs.** `OQ-03` is not left answered by default.

## Predictions — written before any measured run

### The prototype, as designed (`spike/2303/`)

- **Starting point.** `2301`'s lock-step probe at `a451cd15` (`spike/2301/lockstep`, `spike/2301/ngspice`), restored under `spike/2303/` with its FFI (outside the workspace, as `2301`'s was).
  - It runs the `ioc` route: libngspice in-process, a breakpoint at every 10 µs boundary, and Renode as the probe's child, run one quantum at a time from `GetSyncData`.
  - Each quantum is sent as one repeated `;`-joined monitor line with a constant sentinel (`2301` F15).
- **The board and the firmware.**
  - The Sentinel-02-shaped fixture, generated and built by the unmodified CLI (`aethelgard generate`, `aethelgard build`).
  - Its Renode command line is captured from the unmodified CLI's own `simulate` run, then driven by the probe one quantum at a time instead of in one `RunFor`.
  - The judge's DShot stop at the anchor plus 2 ms (`2302`) is left out, because the load must follow the firmware's timer for the whole run. The DShot observers' logging is silenced, as `2302` measured. Both are recorded as deviations.
- **The deck (spike-declared, not sourced):**
  - the battery: a 3S pack at 11.1 V with a series resistance R_pack (the fault: a failing pack or connector) into a bulk capacitor;
  - the motors' current: a current source driven by an external control voltage the probe sets at each boundary, through a first-order lag (τ, the ESC and motor's current rise);
  - the regulator: behavioural, `VDD_3V3 = min(3.3 V, V_bat − dropout)` through a small output resistance into the emitted BOM's decoupling (400 nF), with the MCU's 100 mA load as in `2301`;
  - BOR3, DS12117 Rev 7 Table 26 typical (2.61 V falling, 2.70 V rising), as `2301`.
- **Each boundary:**
  1. Renode runs the quantum.
  2. The motor timer's `CR1` and one `CCRk` are read.
  3. The load is set from them: I_run when `CEN` is set, 0 otherwise. The `CCR` is recorded, and with the zero-throttle pattern it does not scale the load.
  4. ngspice's `V_bat` and `VDD_3V3` are sampled.
  5. The bridge applies `2301`'s held-reset rendering on a BOR crossing: `machine Reset`, the cold writes and `VTOR`, halted, then released with the 10 ns nudge.
- **The archive.** It is written with the real `aethelgard-chaos` `TraceWriter` in several shapes, and replayed with `replay_dir` and with the CLI's `replay` (below).

### Cited, not re-measured (read at source, `2301` FEEDBACK P4 and F11)

- One kind 3 (`AnalogSample`) per quantum: 20.0 B, **2.0 MB per simulated second** at 10 µs.
- One labelled record per quantum (kind 8 as the stand-in): 31.0 B, **3.1 MB/s**.
- The fold: about 428 B per record. The writer refuses the 65,537th record (`MAX_RECORDS`), so one record per quantum stops at **0.66 s**.
- Lock-step cost: 1.4 ms per quantum with one read per quantum, so 14–15 s per 100 ms.

### Predicted

| id | Quantity | Prediction |
|---|---|---|
| Q1 | Per-quantum quantities | Three: `V_bat`, `VDD_3V3`, and the commanded load (the timer state). The new one against `2301` is the third: what the firmware commanded |
| Q2 | Bytes per simulated second, as kind 3 samples (three per quantum) | About 60 B per quantum: **6.0 MB/s** ±15 %. Two per quantum: 4.0 MB/s |
| Q3 | Bytes per simulated second, as stand-in records (three per quantum) | About 93 B per quantum: **9.3 MB/s** ±15 % |
| Q4 | Bytes per simulated second, one combined record per quantum | 35–45 B: **3.5–4.5 MB/s** |
| Q5 | Event records (resets, releases, `CEN` changes, crossings) | Under 1 KB per reboot cycle, and tens of cycles per second at most: under 100 KB/s, negligible beside Q2–Q4 |
| Q6 | Records per run | Records shape, three per quantum: 300,000 per simulated second. Combined: 100,000 |
| Q7 | Where `MAX_RECORDS` (65,536) stops a run | Three records per quantum: **0.22 s**. Combined: 0.66 s. Kind 3 samples: never (kind 3 folds to one map entry per net) |
| Q8 | Where 256 MiB (`MAX_TRACE_BYTES`) stops a run | Kind 3, three per quantum: about **45 s** simulated. Combined record: about 65 s, but `MAX_RECORDS` stops it first |
| Q9 | Wall time per simulated second | 1.4 ms per quantum for lock-step, plus Renode's DShot cost (0.10 s per ms of DShot running, `2302`) for the fraction of time `CEN` is set: **150–250 s per simulated second** |
| Q10 | Renode memory | 4.7 MiB of RSS per ms of DShot (`2302`): **the CLI's 1 GiB bound is passed before about 0.25 s of DShot running.** Inferred to be the first limit any run meets, before `MAX_RECORDS` for the kind 3 shape |
| Q11 | Determinism | Two runs of the same case give byte-identical traces (as `2301`'s lock-step did) |
| Q12 | `replay_dir` | Reproduces the state-sequence hash. A byte flipped in a mid-run sample is refused (`HashMismatch`), because every event's delta enters the hash chain (`replay.rs` L26–63, read at source) |
| Q13 | The CLI's `aethelgard replay` | **Refuses** a campaign run directory holding a kind 3 sample: `NotACampaignRun`, "a campaign run holds an event that is not a campaign record" (`campaign.rs` `check_archived`, read at source). The campaign archive has no place for per-quantum state today. Only a new record kind and its run rules would give it one, and that is a crate change |
| Q14 | The verdict from the records | The bridge's actions (each reset and release) can be re-derived from the recorded per-quantum `VDD_3V3` samples alone, with no solver. A spike re-judge reproduces them exactly |
| Q15 | The loop | With R_pack large enough that the motors' current pulls `V_bat` under the regulator's dropout, the run shows a reboot loop: reset, `CEN` cleared, recovery, boot, `CEN` set, sag, reset. A period of a few ms to tens of ms, set by the firmware's time from release to `CEN` |
