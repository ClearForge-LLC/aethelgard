# AET-WO-4408 — results, judged against `PREDICTIONS.md`

Measured 2026-11-23 on Furnace, on `wo/aet-4408-matrix` from `main` `7fbe5d11`. Every container ran under `~/bin/capped-run` (memory capped, swap 0, both read back). Scratch is in `~/.cache/aet-4408`. Labels: **[measured]**, **[read]**, **[inferred]**.

## D. Digests

As recorded before predicting:
- **No tools image has been published since #339** (run 45105034798, smoke flake). WO §7's second stop holds for `container/IMAGE`, which stays at `e5fc7ddb`. **Since cleared:** the architect re-ran it at 07:58Z (ruling 08:00Z), and attempt 2 published `1e0ab105…9c63` at 08:25Z, which `IMAGE` now names (§B).
- **ArduPilot:** the newest publish is **`aethelgard-ardupilot@sha256:e1809b909b02c7fd01b593a65f32d191169a5027ef61ceeae8e74cc08117c2b6`**, from `ab9d4663` (run 45127867881, `publish` job 137539166537, 07:13Z), and `ARDUPILOT-IMAGE` now names it.

| # | Prediction | Result |
|---|---|---|
| A1 | revision label = its commit; dpkg list = `160f9955`'s (171), `linux-libc-dev 6.12.117-1` | **held** [measured]: label `ab9d46631a13…`; 171 packages, `diff` empty; `linux-libc-dev 6.12.117-1`. Its `Created` is 03:18:27Z, the `9958f594` build: the `ab9d4663` publish reused that build's layers from the GHA cache (adversarial §5 (b)) |
| A2 | sentinel-01's ELF byte-identical under `160f9955` and `e1809b90` | **held** [measured]: sentinel-01 ELF `4f0c0f5f…`, `hwdef.h` `cb2fbb4e…`; four-port ELF `5d931274…`, `hwdef.h` `b05e75cc…`; each identical under both images |

## H. The four-port board

The fixture is `crates/aethelgard-cli/tests/fixtures/four-port.json`, sha256 `ce7b2b05…`. From it:
- topology `03358dc4…`;
- `hwdef.dat` `0db1d65c…`, committed as `crates/aethelgard-hwdef/tests/fixtures/four-port.{topology.json,hwdef.dat}`.

| # | Result |
|---|---|
| H1 | **held**: MAVLINK USART1 PA9/PA10 (BRR 1111); GPS USART2 PA2/PA3 (556); CRSF USART3 PB10/PB11 (152); GPS2 UART4 PA0/PA1 (556); FLASH_CS PA11; I2C1 PB6/PB7; SPI1 PA5–7 |
| H2–H7 | **held**, line for line. Not predicted: the header also carries 4404's "Serial pulls" line, and the TMP117 note |
| H8 | **held**: two generations, `cmp` equal |
| H9 | **held**: the three existing fixtures pass unchanged (`generate.rs` 16/16, `hwdef_tier1.rs` 7/7) |
| H10 | **held** [measured]: built by unmodified ArduPilot in `e1809b90` under `--network none` in 40 s (8 jobs). `aethelgard check pinout` against its own topology exits 0 with `"decodes":"elf and hwdef.h agree"`, `"identity":"judged"`, `findings: []`; 14 configured pins, each named by the topology (UART4 AF8, USART1–3 AF7, I2C1 AF4, SPI1 AF5, PA11 output) |

## P. The pin table

The DS12117 data was transcribed by two subagents, each forbidden to read anything but the PDF.

**How it was transcribed** [measured, agents' notes in `~/.cache/aet-4408/ds/`]:
- **Pads, Table 8, LQFP144 column, pp. 64–88:**
  - read from the PDF's own glyph positions and ruling rectangles, and cross-checked against a text extraction;
  - 144 positions, each once;
  - 12 wrapped or footnoted names cleaned by an explicit map, every other name asserted free of spaces and parentheses.
- **AF, Table 9, pp. 89–100:**
  - the PDF's text words assigned to cells by coordinates;
  - then every page checked by eye from 200 dpi renders: pp. 89 and 99 by the first agent, the other ten by a second agent with no stake in the first's work, 164/164 lines confirmed and 0 discrepancies;
  - I spot-checked p. 90 myself (PB6, PB7: all rows equal).
- **So "hand-entered" is, exactly:** read from the PDF and checked by eye. It is not typed key by key. It is independent of ST's XML and of ArduPilot's tabula extraction, but both it and they come from ST.

| # | Prediction | Result |
|---|---|---|
| P1 | 0 AF disagreements on shared (pin, signal) | **held**: 603 shared rows, 0 disagreements (4400's raw-XML count was 597; `pin_table`'s own parsing gives 603) |
| P2 | no ArduPilot-only row in the solver's families | **held**: 0 |
| P3 | at most 5 ST-only rows in the families | **held, at 0**: ST, ArduPilot and DS12117 each give the same **167** (pin, signal, AF) rows on the package |
| P4 | 144/144 pads equal | **held for 142. Pads 28 and 29 differ in name**: DS12117 prints `PC2_C`/`PC3_C` (Table 8 note 4), and `pin_table` holds `PC2`/`PC3` there. See the finding below |
| P5 | 0 DS12117 AF disagreements | **held**: 167/167 |
| P6, P7 | planted changes fail, naming the row; ST+ArduPilot-agree-DS-differs fails | **held**: tier-1 tests, and the mutants below |

**Finding, PC2_C/PC3_C** [read]:
- On LQFP144 only the `_C` pads are bonded. DS12117 note 4: "Pxy alternate functions are available on Pxy_C when the analog switch is closed."
- RM0433 Rev 9 §12.3.2 p. 549 (`SYSCFG_PMCR`): "PXnSO reset value is 0 when the corresponding PXn_C pin is available on the package but PXn is not." So the switch is closed from reset, and ST's XML, which names the pad `PC2_C` and gives it PC2's signals, matches silicon at reset.
- The test carries the pair as a cited, exact allowance (`SWITCHED`: pad, printed name, pin). It is not a normalisation.
- **Residual:** a board on PC2 or PC3 (SPI2_MISO, SPI2_MOSI, I2S2, …) relies on firmware never setting `PC2SO`/`PC3SO`. Nothing checks that. FEEDBACK F4.

**ArduPilot leg, refusals** [measured]:
- unset variable: panics naming `AETHELGARD_ARDUPILOT_AF_TABLE`;
- a one-byte-changed file: "is not the pinned STM32H753xx.py";
- with the hash constant moved to that file: `["PB6 I2C1_SCL: st AF4, ardupilot AF6"]`.

## B. The bench package

The rehearsal [measured]:
- **Setup:** in the tools image, a tree exported from this branch's HEAD, `cargo build -p aethelgard-cli` plus the bridge (24–27 s warm). Then, under `--network none` with 16 GiB, CI's exact `verify … --against-fab fab/sentinel-01-bench/package`. It ran twice:
  - **control:** the pinned `e5fc7ddb`;
  - **new:** `main`'s tools image, built `--no-cache` on Furnace.
- **Both builds** give the same `aethelgard` (`98ab9780…`) and bridge (`691cd8be…`).
- **Both verifies** pass every gate and all four fab preconditions, and derive the same 17 files, hash for hash.
- **16 of 17 equal the committed `fab/` package byte for byte,** the ELF `f3e871bb…` among them. The 17th, `MANIFEST.json`, is `d47031ce…` in both runs. It differs from the committed one because the verdict it binds records the sandbox: `uid_boundary: false (user_namespace)` on rootless podman. ci.yml says the committed package is the one CI writes, "never one written on another machine". The control shows the same, so the image re-pin moves nothing.
- **Exit 1 in both,** for `against_fab: differs` on `MANIFEST.json` only.
- **After the architect's re-run** (run 45105034798, attempt 2, 08:25Z), `main` published `aethelgard-tools@sha256:1e0ab105ec42bdba66b773464f0b6679d622e4afdafef30ec4931e4326f49c63` from `9958f594`.
  - Pulled by digest, it carries revision label `9958f594…` and `Created` 03:20:17Z: attempt 1's layers, from the GHA cache.
  - Its dpkg list equals the `--no-cache` Furnace build's, all 669 lines.
  - The same verify under it builds the same binaries (`98ab9780…`, `691cd8be…`) and derives the same 17 files as the control and the local build.
- **B1 held as a rehearsal:** no `fab/` byte moves under `main`'s tools content. The acceptance remains CI's `bench-package` job on a published digest.

## G. The GtkSharp seed

Against a loopback registry (127.0.0.1:5408, `capped-run`) [measured]:
- **(a)** the committed script, ghcr holding no mirror: `… from GitHub: https://github.com/…`, then a second publish `already mirrored`;
- **(b)** a scratch copy with `CANON` set to the local registry: `… seeded from localhost:5408/clearforge-llc/aethelgard-tools`;
- **(c)** `CANON` unreachable: exit 1, "GitHub not asked".

The workflow step itself, extracted and run locally with its port rewritten: exit 0 on the GitHub branch. **G1 held.** The step's `seeded from ghcr…` branch cannot run in CI until `main` publishes the mirror. Case (b) is its local stand-in.

## Mutants (one per guard this WO adds, against its own suite) [measured]

**11/11 killed:**
- the `paths:` line dropped from `push` only;
- the same, from `pull_request` only;
- a DS12117 AF changed (PA9 USART1_TX 7 → 8);
- a DS12117 row dropped (PB10 I2C2_SCL);
- two DS12117 pads swapped (101 PA9 → PA10);
- a `SWITCHED` name changed;
- `TIMn_CH4` dropped from the families;
- a four-port `hwdef.dat` byte (`generate.rs`);
- a four-port topology pin (`hwdef_tier1.rs`);
- the ArduPilot leg with no variable;
- the ArduPilot leg with a planted AF.
