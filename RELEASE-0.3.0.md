# Releasing 0.3.0: Scotty's checklist

Run this from your own workstation, never Furnace, with your own crates.io token. `docs/releasing.md` is the procedure behind each step, and `AET-WO-1802` prepared it. Each command is followed by what it should print.

## 0. Before you start

- **The commit.**
  1. Run `git switch main && git pull && git status`. It should say `nothing to commit, working tree clean`.
  2. Run `git log -1 --format=%H`. It should name the commit CI passed on: the 0.3.0 bump commit (`AET-WO-1802`), or a later one before `main` moves to `0.4.0-dev`.
  3. Run `grep -m1 '^version' Cargo.toml`. It should print `version = "0.3.0"`. **If it ends in `-dev`, stop:** that is `main` between releases, and a `-dev` version is never published (`docs/releasing.md`).
- **The token,** made on crates.io under Account Settings, API Tokens:
  - scopes `publish-new` and `publish-update` (one crate is new: `aethelgard-hwdef`, which answered 404 on 2026-11-26);
  - crate scope `aethelgard-*`;
  - a short expiry.
- **Log in.** Run `cargo login` and paste the token at the prompt, never as an argument. It should print `Login token for crates-io saved`.
- **The whole release, without uploading.** Run it in a fresh cargo home and target directory, so that no earlier dry run's cached 0.3.0 can stand in for today's (cargo treats a name and version as immutable; `AET-WO-1801` §9):
  ```
  d=$(mktemp -d); CARGO_HOME="$d/home" CARGO_TARGET_DIR="$d/target" cargo publish --workspace --dry-run --locked; echo "exit $?"; rm -rf "$d"
  ```
  It needs no token and downloads the dependencies again. For each of the eleven crates it should print `Uploading aethelgard-<name> v0.3.0` and then `warning: aborting upload due to dry run`, and it should exit 0. *(Run on Furnace on 2026-11-26, in this hermetic form, on the trial bump: this branch with 0.3.0 applied, before #410. Exit 0 for all eleven, with no token. Re-run at the bump commit.)*

## 1. Publish, in this order

Before each crate from step 4 on, check that its dependencies are live:
`curl -s -o /dev/null -w '%{http_code}\n' https://crates.io/api/v1/crates/<dependency>/0.3.0` should print `200`.

| # | Command | Needs live first |
|---|---|---|
| 1 | `cargo publish -p aethelgard-ir --locked` | none |
| 2 | `cargo publish -p aethelgard-kicad-lib --locked` | none |
| 3 | `cargo publish -p aethelgard-renode-models --locked` | none |
| 4 | `cargo publish -p aethelgard-svd-hal --locked` | 1 |
| 5 | `cargo publish -p aethelgard-eda --locked` | 1, 2 |
| 6 | `cargo publish -p aethelgard-twin --locked` | 1, 3, 4 |
| 7 | `cargo publish -p aethelgard-chaos --locked` | none |
| 8 | `cargo publish -p aethelgard-evolve --locked` | 4, 7 |
| 9 | `cargo publish -p aethelgard-pincheck --locked` | 1 |
| 10 | `cargo publish -p aethelgard-hwdef --locked` | 1 |
| 11 | `cargo publish -p aethelgard-cli --locked` | 1, 4, 5, 6, 7, 8, 9, 10 |

Each should print, in turn:
- `Packaging aethelgard-<name> v0.3.0`, `Verifying …`, `Uploading …`;
- `Uploaded aethelgard-<name> v0.3.0 to registry crates-io`;
- `Waiting for aethelgard-<name> v0.3.0 to be available at registry crates-io`;
- `Published aethelgard-<name> v0.3.0 at registry crates-io`.

**Dry runs.** On Furnace, 2026-11-26, on the trial bump, each in a fresh cargo home: `cargo publish -p <crate> --dry-run --locked` exited 0 for steps 1, 2, 3 and 7. Steps 4–6 and 8–11 cannot be dry-run one at a time until their dependencies are live, because the dry run resolves them on crates.io. The workspace dry run in step 0 covers all eleven. To dry-run one of them anyway, add `--dry-run` once the dependencies in its row answer 200.

**If the first publish of the new name (step 10) is refused for a rate limit:** nothing was uploaded. Wait the time it names, then run the same command again.

**If a publish times out waiting, or the next one cannot find a dependency:** the index is lagging. Wait until the `curl` check answers `200`, then run the same command again. Never publish a crate twice. See `docs/releasing.md`, "If crates.io's index lags".

## 2. Verify the release

In a clean directory, outside this repository's target directory:

```
cargo install aethelgard-cli --version 0.3.0 --locked --root /tmp/aethelgard-0.3.0
/tmp/aethelgard-0.3.0/bin/aethelgard --version
```

The second command should print `aethelgard 0.3.0 (commit unknown)`. A crates.io build is not told its commit, and needs no telling: the published version is the snapshot (`AET-WO-1802`).

```
/tmp/aethelgard-0.3.0/bin/aethelgard validate < examples/sentinel-03.json; echo "exit $?"
```

It should print one JSON line starting `{"ok":true,"project_name":"sentinel-03",`, then `exit 0`. *(The same `validate`, run on Furnace from a clean `cargo install --path crates/aethelgard-cli --locked` of the trial bump, gave exit 0. Re-run at the bump commit.)*

## 3. Afterward

- Revoke the token on crates.io.
- Tell the architect. The README's install line is a docs PR after the publish, and `main` moves to `0.4.0-dev` by its own PR (`AET-WO-1802` (4)).

## What 0.3.0 can do that 0.2.0 could not

0.2.0 was published on 2026-11-17, after `AET-WO-1801` (merged as `7452bf4b`), and read specification 13. 0.3.0 reads specification 31 (`crates/aethelgard-ir/src/lib.rs`, `SPECIFICATION_SCHEMA_VERSION`), so a specification written for today's engine cannot be read by 0.2.0. Each version from 14 to 31 came with one work order: `AET-WO-1312` (14), `AET-WO-1313` (15), `AET-WO-4400d` (16), `AET-WO-4400g` (17), `AET-WO-2304b` (18), `AET-WO-4240v` (19), `AET-WO-4240a` (20), `AET-WO-1210` (21), `AET-WO-1211a` (22), `AET-WO-4240b` (23), `AET-WO-1211e` (24), `AET-WO-1216b` (25), `AET-WO-1217` (26), `AET-WO-1218` (27), `AET-WO-4300i` (28), `AET-WO-4300j` (29), `AET-WO-4300k` (30) and `AET-WO-4411` (31). An older specification is refused with every change it lacks, listed in the message and in `error.entries` (`AET-WO-1215`).

**The engine's identity**
- `aethelgard --version` names the commit when the build states one, as `aethelgard 0.3.0 (commit <sha>, stated by the build)`, and otherwise `aethelgard 0.3.0 (commit unknown)`. `doctor` reports `engine_commit`, and the MCP server's info carries the same identity. The engine image states its commit, the one its `org.opencontainers.image.revision` label names, and CI checks that they agree. A crates.io install says `commit unknown`: the version is the snapshot. No committed or hashed byte names the commit, so a package never goes stale because a commit moved. A run's archive header still names the version alone, since its bytes seed the state-sequence hash that the verdict binds (`AET-WO-1802`; `AET-WO-2205` F4 stays open).

**Specification and solving**
- A driver IC is part of a specification: `pin_devices` (GPIO, ADC and bridge devices), `pwm.idle` and `pwm.resolution_ps`. The motor pull-down is a declared part, derived by AN4899 and switchable by a drawn link (`AET-WO-1211a`, `AET-WO-4300i`, `AET-WO-4300j`).
- A board declares its outline bound (`board.max_outline_um`), its forward axis and each IMU's orientation, test points on any declared net, and BOOT0 on a connector through a judged pull-down (`AET-WO-1313`, `AET-WO-4400d`, `AET-WO-1312`).
- A specification declares the MCU's clock source (`mcu.hse`: crystal, external or null), its core supply, a battery's series resistance, and a board that is only a twin stand-in (`board.drawn`) (`AET-WO-4240a`, `AET-WO-4240v`, `AET-WO-2304b`, `AET-WO-1210`).
- `validate` refuses more:
  - a power load naming no part, a part off its rail, or the MCU or a regulator counted twice (`AET-WO-1209`, `AET-WO-1209b`, `AET-WO-1209c`);
  - a pull no MCU pin takes, and a VREF that is wrong for its ADC (`AET-WO-1211e`);
  - a mavlink, gps or console baud that ArduPilot cannot read back (`AET-WO-1213`, `AET-WO-1214`);
  - a trip sense tied to GND, a digital signal on PC2_C or PC3_C, and an unknown MCU, once, at `mcu.part` (`AET-WO-1217`);
  - any path from a rail that can lead the MCU supply at VDD = 0 (`AET-WO-1216b`).
- The generated clock tree and bring-up are judged: HSE to PLL1, VOS scale 2 with bounded waits, and a kernel-clock switch, so no board starts SPI1 to SPI3 without a kernel clock (`AET-WO-4240b`, `AET-WO-4240v`, `AET-WO-4240e`).
- The reset supervisor and input protection come from held datasheets (`AET-WO-4300k`).

**Board generation**
- Under a declared bound, the placer is order-independent. It puts the bucks first and the connectors on the side edges, retries a missed placement, and refuses by name when the board cannot fit (`AET-WO-1313`, `AET-WO-1313c`, `AET-WO-1317`). Compaction ends at a bound it derives (`AET-WO-1313d`).
- The router keeps each via's declared ring at the fabricator's worst hole and keeps a supply-gate net's vias apart. It refuses by name when its work bound is exceeded (`AET-WO-1314`, `AET-WO-1324`, `AET-WO-1323`).
- The crystal and its load group are drawn and routed with no via on the oscillator nets (`AET-WO-4240c`). Pin devices are drawn: the DRV8876 and DRV8833 on the project's own symbols, with the declared bulk capacitor (`AET-WO-1211d`, `AET-WO-1211f`, `AET-WO-1321`). The hot-plug bulk capacitor and each control are drawn as declared (`AET-WO-1315`).
- `aethelgard init --template i2c-spi-bench` writes a bench specification with ten placeholders, each refused by its own kind and path until it is filled (`AET-WO-1608`).
- Example boards: Sentinel-01 as ordered, and Sentinel-02 and Sentinel-03 with a bench specification and package (`AET-WO-3308`, `AET-WO-3401`, `AET-WO-3402`, `AET-WO-3403`). Sentinel-03 carries an ESP32-C3-WROOM-02 module on USART6, on its own LMR33620 rail, behind a TXU0304 UART crossing (`AET-WO-4411`).

**Firmware and the twin**
- Generated firmware drives pin devices (GPIO levels, input pulls, ADC1 and ADC3 bring-up, a bridge's enable), and the twin witnesses them (`AET-WO-1211b`, `AET-WO-1211c`). It starts from the crystal and runs at its solved clock (`AET-WO-4240b`, `AET-WO-4400g`).
- Every board boots 100 ms in the twin. Each witness's window starts where its evidence starts, and each `RunFor` runs exactly its nanoseconds (`AET-WO-2302`, `AET-WO-4202`). Each SPI controller the twin adds wires its receive DMA request (`AET-WO-4400c2`).
- The twin judges each I2C step's status word per device, so a two-device bus fault is detected (`AET-WO-3305`). A bring-up reads every identity, and a device with no model is named `identity_not_witnessable` (`AET-WO-4241`).
- The ICM-42688-P model has its FIFO, from DS-000347 (`AET-WO-4220c`). Renode's resident memory is capped at 1.5 GiB (`AET-WO-2313`).
- `aethelgard-bridge --version` and `doctor` name the ngspice banner, and `build` names the rustc that cargo ran (`AET-WO-1606`).

**Foreign firmware (ArduPilot)**
- `aethelgard generate hwdef` writes ArduPilot's `hwdef.dat` from the solved topology alone, and unmodified ArduPilot builds it. This is the new crate `aethelgard-hwdef` (`AET-WO-4400b`, `AET-WO-4408`). The identity judge holds ArduPilot's SPI devices, motor map, I2C clock, serial order, bauds and barometer probes to the topology (`AET-WO-4400b2`, `AET-WO-4400d`).
- `simulate --foreign` boots unmodified ArduPilot on the emitted platform. It probes and reads the ICM-42688-P through its FIFO and both barometers, and judges each declared serial port's `USART_CR2` options (`AET-WO-4400e`, `AET-WO-4400e3`, `AET-WO-2311`). A foreign firmware's clocks come from the specification (`AET-WO-4400g`).
- Sentinel-03 runs unmodified ArduPilot on ArduPilot's own clock. `generate hwdef` writes a DShot board's `defaults.parm` (`MOT_PWM_TYPE`), so its motors run DShot600 from boot, over the twin's timer and DMA models, with each frame judged. The reboot is judged, and a foreign run's hang guard scales to its window (`AET-WO-4412`).
- A foreign firmware hears MAVLink as a declared input, witnessed from its own heartbeat, archived and replayed (`AET-WO-4400i`, `AET-WO-4400j`). A foreign fault is named at the fault handler's entry, before the memory cap (`AET-WO-2314`), and ArduPilot's `AP_HAL::panic` by name (`AET-WO-1609`).
- `check pinout` judges each serial port's protocol, options and baud from `var_info` in the ELF. It refuses another MCU, an ELF that can apply parameter defaults at boot, and a third GPS port (`AET-WO-4404`, `AET-WO-4405`, `AET-WO-4406`, `AET-WO-1218`). It also reads the battery monitor and the 16 MHz crystal, judges SWD and a chip select's ODR, and refuses a function-like `#define` (`AET-WO-4410`).
- **A firmware must be rebuilt from the hwdef of the engine that checks it.** A DShot board's `defaults.parm`, which ArduPilot embeds in ROMFS, names the engine version that wrote it, and `check pinout` admits only the file this engine writes, byte for byte. So an ELF built from the hwdef of another engine version, `0.4.0-dev` included, is refused by 0.3.0's `check pinout`, and the refusal names both versions (`AET-WO-4412`, `AET-WO-1802`).

**Fault campaigns and evolution**
- Campaigns plant events from the undisturbed boot's own word, and a BOR droop's recovery is judged by the boot the twin ran (`AET-WO-2310`, `AET-WO-2305`). The battery-sag chain is solved once, with a battery-sag scenario (`AET-WO-2304b`).
- `attribute`:
  - names the twin's window findings by software rules (`AET-WO-2302b`);
  - judges each I2C device's whole log against its script, so a step sent to another device's address that answers is the software's (`AET-WO-3306`);
  - counts a BMP581's witness as evidence (`AET-WO-3307`), and compares counts as well as values (`AET-WO-3309`).
- `replay` and `attribute` are bound to the records, and `attribute` refuses past 1,024 records by name (`AET-WO-2312`). A trace refuses at its size bound, `TraceFull` (`AET-WO-2309`).

**Verification and fabrication**
- `extract` bounds every MCU supply and return pin's resistance by a certified flow, with each fabrication tolerance priced (`AET-WO-1303b`, `AET-WO-1314`, `AET-WO-1314b`).
- Every `verify` verdict carries one table: which gates ran, which were off and why, and which were not reached. A gate that judged nothing says `nothing_in_scope`, and `prove` with no harness is `not_reached: no_harness` (`AET-WO-1609`, `AET-WO-2306`, `AET-WO-1610`).
- `verify --fab-out` judges a readiness gate, and re-binds an ordered package only when nothing but its binding moves. A stand-in is labelled in every fab file or refused by name (`AET-WO-4300k`, `AET-WO-2308`). `verify` on a board that is not drawn runs but is never verified (exit 7) (`AET-WO-1607`). `check drc` judges the IMU seats and the declared outline (`AET-WO-1610`).
- `check support` is pinned, and every verdict carries the lifecycle banner (`AET-WO-1607`). The JLCPCB BOM and placement files are written as ruled (`AET-WO-1604`).
- The schedule harness is split on every board with two or more parts, so `prove` peaks lower (`AET-WO-3303b`, `AET-WO-1918`).

**Images**
- The tools image's Renode carries upstream's NVIC fixes (`AET-WO-1605`). Every Dockerfile installs apt packages from a dated snapshot, and `check-pins` refuses the moving index (`AET-WO-4407`).

P3's exit gate still waits on bench rows, and P4 is under way, so this release claims no phase beyond P2 as closed.
