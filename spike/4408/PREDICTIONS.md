# AET-WO-4408 — predictions (§1.1), written before measuring

Written 2026-11-23 between 07:15Z and 07:25Z on `wo/aet-4408-matrix` at `main` `7fbe5d11`. At that point nothing in this directory had run: no solve or generate of the four-port board, no table diff, no verify. Each prediction is judged in `RESULTS.md`.

Labels:
- **[measured before predicting]**: I read it first, so it is not a prediction (the digests, below).
- **[read]**: at source.
- **[inferred]**: neither.

## D. The published digests after #339 [measured before predicting]

I read these from the run logs before writing this file, so they are facts, not predictions:
- **tools-image**, run 45105034798 (push, `9958f594` = #339's merge): **failed** in `image`'s smoke (`FAIL libngspice: ldconfig does not list libngspice.so.0`). `publish` and `source` were skipped. No later `main` push touched `container/**`, so **no tools digest exists after #339**, and WO §7's second stop holds for `container/IMAGE`.
  - The cause is a flake. `container/smoke.sh:115` runs `ldconfig -p | grep -qF` under `pipefail`, and ldconfig dies of SIGPIPE when grep exits first.
  - In 200 runs it failed once in the pinned `e5fc7ddb` and once in `main`'s tools image built `--no-cache` on Furnace. Both images give 62,785 bytes, with the match at byte 18,612.
- **ardupilot-image publishes:**
  - `aethelgard-ardupilot@sha256:8824dafd06a57d8a21e3add28ae1fdd1e6c2b495db859c8b6e70f268dc34f409` (from `9958f594`, run 45105034787);
  - `@sha256:98d1bab531d8475819d606e58f104b769aeb530f325c7536f0f3ccb0d6fad8f3` (from `c7a62975`, run 45117816963);
  - `ab9d4663`'s run 45127867881 was publishing at 07:08Z.
- **The GtkSharp mirror** `aethelgard-tools@sha256:03cf1db8…29fe` is **not on ghcr**: `gtksharp-source.sh fetch` gives `HTTP 404`, exit 2. Its publish step is in the skipped `publish` job.

**Predictions on the ArduPilot re-pin** [inferred]:
- **A1.** The newest published ArduPilot image, pulled by digest, carries its own commit as `org.opencontainers.image.revision`, and its dpkg list equals `160f9955`'s package for package (171), `linux-libc-dev 6.12.117-1` among them. Both builds resolved apt at snapshot 20261117T120000Z, and the 11-16 build already held 117-1 (`spike/4407`).
- **A2.** Under the new image, sentinel-01's ArduPilot ELF is byte-identical to the one `160f9955` builds: the same ArduPilot tree, the same toolchain, the same Python set.

## H. The four-port board's generated `hwdef.dat`

**The fixture** (`crates/aethelgard-cli/tests/fixtures/four-port.json`, specification 28) is `4404`'s `fc-base` (FEEDBACK §2.2):
- `serial-four.json` with MAVLINK made the console;
- its SBUS port made a second GPS port (`GPS2`, 8N1 115200, not inverted, its nets renamed `GPS2_TX`/`GPS2_RX`);
- the I2C bus at 100 kHz.

It differs from `serial-four` in 13 lines. `serial-four` itself cannot be the board: the generator refuses it with `NoConsole`, and with a console added, `RcInputTwice` (`hwdef_tier1.rs:135-160`) [read].

**Solve** [inferred from `solve_cli.rs:389-417`: the role and net-name changes do not move the search]:
- **H1.** Ports in order:
  - MAVLINK USART1 PA9/PA10;
  - GPS USART2 PA2/PA3;
  - CRSF USART3 PB10/PB11;
  - GPS2 UART4 PA0/PA1.
- **H1 (continued).** `FLASH_CS` on PA11; I2C1 on PB6/PB7; SPI1 on PA5/PA6/PA7.

**Generate** [inferred from `hwdef/src/lib.rs` `serial()` and `4404` FEEDBACK §2.2]:
- **H2.** `SERIAL_ORDER USART1 USART2 USART3 UART4`.
- **H3.** SERIAL0 is the console:
  - `define DEFAULT_SERIAL0_BAUD 57600`;
  - no SERIAL0 protocol or options line.
- **H4.** SERIAL1 (GPS):
  - `define DEFAULT_SERIAL1_PROTOCOL SerialProtocol_GPS`;
  - `_BAUD 115200`;
  - `_OPTIONS 0`.
- **H5.** SERIAL2 (CRSF):
  - `define DEFAULT_SERIAL2_PROTOCOL SerialProtocol_RCIN`;
  - no baud or options line;
  - a `# Not emitted: CRSF (rc_crsf) on SERIAL2` header line naming 416666 baud and the declared 420000.
- **H6.** SERIAL3 (GPS2): GPS, 115200, options 0.
- **H7.** The other lines:
  - `define HAL_I2C_MAX_CLOCK 100000`;
  - `BARO BMP280 I2C:0:0x76`;
  - TMP117 not emitted;
  - `SPIDEV dataflash SPI1 …`;
  - `define HAL_LOGGING_DATAFLASH_ENABLED 1`;
  - `define HAL_USE_ADC FALSE`;
  - no motor, no IMU line.
- **H8.** Generating twice gives the same bytes.
- **H9.** No committed `hwdef.dat` or topology fixture moves (none of them changes).

**Build and judge** [inferred, `4404` §2.3 built fc-base in the pinned image]:
- **H10.** Unmodified ArduPilot builds it under `--network none`.
- **H10 (continued).** `aethelgard check pinout` against its own topology exits 0. Every `identity` fact passes, the four serial protocols included (`4405`'s judge).

## P. ST's pin table against ArduPilot's `STM32H753xx.py` and DS12117

The sides, as the solver can assign them: `I2Cn_SCL/SDA`, `SPIn_SCK/MISO/MOSI`, `USARTn`/`UARTn` `_TX/_RX`, `TIMn_CH1..4` (`solve.rs` `instances`, `motor::timer_channel`, the `{p}_TX`/`{p}_RX` at :1785) [read].

**ST (`svd_hal::pin_table("STM32H753ZITx")`) against ArduPilot** [inferred from `4400` FEEDBACK §S2's measurement on the raw XML, 597 shared (pin, signal) pairs with 0 AF disagreements]:
- **P1.** Over the LQFP144 GPIO pins, every (pin, signal) the two tables share carries the same AF number: **0 disagreements**.
- **P2.** In the solver's families, ArduPilot has no entry ST lacks. That is `4400`'s "every I2C, SPI, USART/UART data line equals ST's", and TIM channels included.
- **P3.** In the solver's families, ST has at most 5 entries ArduPilot lacks. The ArduPilot-only entries outside those families are the 10 hand-added CAN lines at AF9 and the 12 name merges (`USARTn_CTS_NSS`, `TIMx_BKIN_COMP12`), as `4400` found.

**ST against my transcription of DS12117 Rev 10** (sha256 `607b9ead…c032`):
- **P4.** All 144 LQFP144 positions carry the same pin name as `pin_table`'s `position`, and its GPIO positions are exactly the `P[A-K]n` pins `pin_table` holds, 112 of them (`4400` FEEDBACK §S2 counted 112 GPIO pins on the package). This is the Sentinel-01 review's 144/144 (`docs/reviews/2026-11-19-sentinel-01-as-ordered.md`, its pad check, L260).
- **P5.** In the solver's families, DS12117's AF tables (pp. 86–91 per the review) give the same AF number as ST's for every (pin, signal) on the package: 0 disagreements.
- **P6.** Planting one AF change in any of the three sources fails the test, naming that row and both values.
- **P7.** So does a row on which ST and ArduPilot agree but DS12117 differs (the §5 adversarial row). No two-of-three vote.

## B. The bench package under the re-pinned tools image

- **B1** [inferred]. `verify --against-fab fab/sentinel-01-bench/package` under `main`'s tools image passes, and no `fab/` byte moves.
  - Between `e5fc7ddb` and that image, 4407 moved only `linux-libc-dev` 6.12.115-2 → 6.12.117-1, plus labels and the source mirror.
  - Kernel headers reach neither the firmware (thumbv7em) nor KiCad, ngspice, Renode or Kani.
  - This is a rehearsal on Furnace. The acceptance is CI's `bench-package` job once `container/IMAGE` can name a published digest.

## G. The GtkSharp rehearsal

- **G1** [inferred]. Seeded on `gcc-source.sh`'s `CANON` pattern, the rehearsal publishes to the runner's registry from GitHub's archive while ghcr has no mirror (exit 2 from the canonical fetch). Once `main` publishes the mirror, it seeds from it without asking GitHub.
