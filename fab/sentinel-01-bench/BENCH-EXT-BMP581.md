# Sentinel-01: a BMP581 breakout on J4 (EXT), the bench sheet

U2 is not fitted on the boards ordered 2026-11-19 (`BENCH.md`, "U2 empty"). A BMP581 breakout on J4 needs no copper, but the ordered ELF never addresses EXT. This sheet runs a second ELF, built from a bench specification, on the same boards.

Every prediction here was written by the `AET-WO-3403` builder **before** any bench, from `main` at `47ec18f6` and re-run on `main` at `ee835bde` (after `4400g`, specification 17; §1), in the pinned tools image (`aethelgard-tools@sha256:e5fc7ddb…1b58`) on Furnace, 2026-11-19. Fill the blank columns at the bench. **Never edit a prediction: a disagreement is the result** (`INV-07`; `northstar.md` §2). Source labels are `BENCH.md`'s: twin, rule, generator, repeat, not predicted.

## 0. What this is, and what it is not

- **The specification is `examples/sentinel-01-ext-bmp581.json`** (sha256 `ac54b8064055721f62dab54306c0e07b860376bcc3f1c75a87253de8b7c71470`, specification 22). It is `examples/sentinel-01.json` with four changes:
  - **U5, a BMP581 on bus `EXT`, at address `0x47`** (71), with Sentinel-02's declared pin capacitance (`examples/sentinel-02-bench.json`'s U7 row);
  - **EXT's pins pinned to PB10 (SCL) and PB11 (SDA)**, so the solver cannot move the bus off J4's copper;
  - `project_name` `sentinel-01-ext-bmp581`;
  - `acknowledge_unwitnessed` emptied. Sentinel-01's entries for I2C2 say "the firmware starts no transfer on EXT", which is false for this ELF. Empty, the list also means no fabrication package can be written from this specification.
- **It describes a board that was never made.** To `emit`, U5 is an on-board part with a footprint, because a specification cannot place a device behind a connector (the review's U3; `I2cDevice` has no placement field). Its `verify` verdict judges that board. **It says nothing about J4's copper with a breakout plugged in.** The ordered package's verdict does not cover this ELF either. Only the ELF comes to the bench.
- **Why the ELF still fits the ordered copper** [measured]: `solve` on this specification and on `examples/sentinel-01.json` gives the same `assignments` (every net on the same pin and function: EXT_SCL PB10 and EXT_SDA PB11 at I2C2 AF4, I2C_SCL/SDA PB6/PB7 at I2C1 AF4, SPI PA5–PA7, FLASH_CS PA0, the console PA9/PA10), the same `gpio`, `clock`, `i2c_timings`, `pin_speeds` and `spi_controllers`. The two topologies differ only in `i2c_devices` (U5 added) and the specification hash.
- **The firmware's difference from the ordered ELF** [measured: `diff -r` of the generated board crates]: `main.rs`, and `proofs.rs`'s Kani bounds (the largest transfer is now 3 bytes out and 6 in); every other difference is the crate name or the specification hash in a comment. `main.rs` keeps the ordered ELF's I2C1 read of `0x76` and its SPI read of the flash, and adds three I2C2 transactions to `0x47` and one 10 ms wait. The board crate's driver code is the same.
- **Not the ordered board's verdict, and not an engine change.** A device declared behind a connector, so that the ordered specification itself could name the breakout, is a roadmap §7.1 row (the review's U3 (b)).

## 1. The firmware

**ELF:** `sentinel-01-ext-bmp581.elf`, sha256 **`a60306c9ac32fbbf13bea1e610c546c94307bd04b080166651fef9953febb761`** [measured].
- Generated and built by `main`'s CLI (`cargo build --locked -p aethelgard-cli` and the bridge, as CI's `bench-package` job builds them), `generate --spec` then `build --crate`, under `capped-run` with `--network none`.
- Built twice, at `/cache/runs/gen1` and `/cache/runs/deep/path/gen2`, giving one sha.
- **Unchanged by specification 17.** After `4400g` merged (`ee835bde`), the specification moved to 17 and nothing else; generated and built again, twice, by `main`'s CLI: the same sha. The generated tree differs only in the specification hash in comments and in the `.repl`'s new clock section.
- **On Furnace:** `~/.cache/aet-3403/bench/`, beside `SHA256SUMS`, the specification, the emitted `.repl` and the `simulate` verdict. Copy it and check it before flashing:

```
scp furnace:/home/forge/.cache/aet-3403/bench/sentinel-01-ext-bmp581.elf .   # as the forge user: its home is 0700
echo "a60306c9ac32fbbf13bea1e610c546c94307bd04b080166651fef9953febb761  sentinel-01-ext-bmp581.elf" | sha256sum -c -
```

**The source runs** (Furnace, `~/.cache/aet-3403/`):
- **S1:** `simulate --firmware … --for 100ms` on the emitted platform (`.repl` sha256 `0c47a7e4…6a6f`, with `4400g`'s clock section). Verdict **`pass`** (`bench/simulate-verdict.json`, sha256 `d346c9fe…3722`). Boot word at 19,523,760 ns virtual; 25 register rows equal under their masks; U2 identity `0x60`; U5 identity `0x50`, configure, data; U4 `EF4017`; the console's pattern witnessed; peak RSS 503 MiB. Every field equals the run on specification 16 (`.repl` `17fd7067…`), except the specification and `.repl` hashes, the paths, the RSS and the Renode build it now names (`4400g`'s pin).
- **S2:** `verify` on the specification, in the image under a 16 GiB cap, 18–23 minutes each. Result, reported as it is [measured], on specification 16 and again on specification 17 after `4400g` merged (the same result both times, below):
  - **With the ordered package's five scenarios** (`pullup-3k3`, `droop-bor3`, `spi-absent-u4`, `pullup-2k2`, `serial-rx-open-console`, each copied with its `base` pointed at this specification and nothing else changed): **refused** at the campaign, exit 5, no verdict. `droop-bor3` alone refuses the same way: the twin stamps the console injection after the brown-out reboot at 20,855,539 ns where its count places it at 20,855,540 ns. That is `4201`'s fault (a BMP581 through `droop-bor3` is stamped 1 ns early, #237), a roadmap §7.1 row in `4400g`'s area; the twin was not touched. The ordered ELF passes the same scenario (the package's verdict).
  - **Architect's note (2026-11-20), after `4202` (#257):**
    - The 1 ns stamp above was Renode truncating `RunFor`'s decimals, and `4202` fixed it. `droop-bor3` now runs, and fails as `brownout_unrecovered`.
    - The cause is the recovery probe: it sits at release + 100 µs and wants a one-line witness, while the BMP581's 10 ms settle delays the boot word. That is `4600` F1, filed under `2303` F11's row.
    - It is a twin-judge limit. It says nothing about the copper or this ELF.
  - **With the other four:** verdict **`fail`**, on one gate. **`extract` fails:** `SPI_MOSI`'s bound is 47.4 pF (trace 17.2, vias 19.7, pads 2.5, pins 8.0) against the declared 30 pF. ERC, DRC, support, all eight proofs, build, simulate, both `attribute` runs and the campaign pass; all four scenarios matched their expectations (`pullup-3k3` and `pullup-2k2` pass; `spi-absent-u4` and `serial-rx-open-console` fail as expected). None of the four touches EXT or U5: they move SENSORS' pull-up, remove U4 and open the console's RX. Only `droop-bor3` reached U5, and it is the one refused. Verdict `bench/verify-verdict.json`, sha256 `1ee281f4…c3e3`, ELF `a60306c9…`, topology `4d81fc80…`.
  - **Specification 17** (`main` `ee835bde`, `4400g`'s twin clocks): the refusal is byte-for-byte the same file. The four-scenario verdict differs from specification 16's (`a9245654…4bdb`) only in hashes: the specification, the topology, the generated firmware crate (the specification hash in its comments), the `.repl`, and the campaign's run ids and state sequences. The emitted KiCad board and schematic are byte-identical (the inputs of ERC, DRC, support and `extract`), and every gate's result and value, and every scenario's outcome, is the same.
  - **What the failure is about** [measured: `extract` on each specification's emitted board]: with U5 as an on-board footprint, the board is placed and routed differently. Its SPI_MOSI is 49.9 mm and 47.4 pF, against 20.9 mm and 24.6 pF on the ordered board's emitted copper (whose input sha256 `401b9e1e…` is the one the package's verdict judged). That board was never made. The ordered board's `extract` passes (the package's verdict), and the ELF does not depend on the copper. So the failure says nothing about the ordered boards, and nothing about this ELF on them.
  - **So, plainly: this specification's `verify` verdict fails,** because its `extract` judges copper that does not exist, and its campaign cannot run `droop-bor3`, the known BMP581 twin fault (`4201`). **The verdict that judges the copper on the bench is the ordered package's** (`package/MANIFEST.json`). What comes to the bench from this specification is its ELF, and its solved topology equals the ordered one in every assignment, timing, clock and pin speed. Ruled by the architect (D1, accept), 2026-11-19. A specification that names the breakout behind J4 and passes is the review's U3 (b), engine work.

**After this sheet, flash the ordered ELF back** (`BENCH.md` B1): every row of `BENCH.md` assumes `package/sentinel-01-bench.elf`.

## 2. The breakout: address, pull-ups, cable

### 2.1 The address

**Assumed: `0x47`.** Both common BMP581 breakouts strap `0x47` by default:
- Adafruit's (BMP580/581/585 guide, "Pinouts", read 2026-11-19): "The default I2C address on each of the boards is 0x47"; connecting SDO to GND, or soldering the back's `addr` jumper closed, makes it `0x46`.
- SparkFun's Qwiic BMP581: `0x47` default, `0x46` alternate, by its ADR jumper [inferred: product listings and search summaries; its own hookup page refused the read (HTTP 403), and one mirror of it says `0x77`].

**Confirm it on the board, with the ordered ELF still flashed** (it leaves I2C2 enabled and idle) and the breakout plugged in (§2.3 first):

```
$P write --chip STM32H753ZITx b32 0x4000581C 0x00000030   # I2C2.ICR: clear STOPF, NACKF
$P write --chip STM32H753ZITx b32 0x40005804 0x0200208E   # I2C2.CR2: address 0x47, write, NBYTES 0, AUTOEND, START
$P read  --chip STM32H753ZITx b32 0x40005818 1            # I2C2.ISR
$P write --chip STM32H753ZITx b32 0x4000581C 0x00000030
$P write --chip STM32H753ZITx b32 0x40005804 0x0200208C   # the same at 0x46
$P read  --chip STM32H753ZITx b32 0x40005818 1
```

`ISR & 0x00008030` reads `0x00000020` at the address that answers (ACK) and `0x00000030` at the one that does not (`NACKF`): RM0433 Rev 9 p. 1975, as `BENCH.md` B9. **Exactly one of the two should answer.** `probe-rs`'s `write` syntax was not run on Furnace: check `--help` first.

**Read the strap before power, and record it.** The specification addresses `0x47` (ruled by the architect, D2: the common breakouts' default). `3304`'s Nucleo sheet assumed SDO to GND (`0x46`), so this breakout may be strapped that way. **Both straps are predicted:**
- **SDO high, `0x47`:** the probe above ACKs at `0x47`, and §3's records and §4's decode are the `0x47` case.
- **SDO low, `0x46`:** the probe ACKs at `0x46` only, and the bench ELF's three U5 transfers are NACKed at `0x47`: §3's "SDO low" column, and the decoder shows `Start`, `Write`, `Address write: 47`, `NACK`, `Stop` three times.

Do not edit the specification at the bench. Either strap is a result.

Breakout (maker, name, revision): ________ · SDO strap, read before power (high / low): ________ · `0x47` ISR: ________ · `0x46` ISR: ________

### 2.2 The pull-ups and the load

The breakout's own pull-ups sit in parallel with R3/R4 (10 kΩ, measured in `BENCH.md` Before power, step 3). Read the breakout's values off its resistors or its documentation, or measure them unpowered, breakout unplugged, from its SCL pad and its SDA pad to its supply-in pad.

| Line | R3/R4, measured | Breakout's pull-up | Parallel, `R_eff = R·R_b / (R + R_b)` | Level shifter on the breakout? |
|---|---|---|---|---|
| EXT_SCL | | | | |
| EXT_SDA | | | | |

**The arithmetic** [computed], for reference:

| Breakout pull-up | `R_eff` with 10 kΩ | Largest C for a 1000 ns rise at 100 kHz, `1000 ns / (0.8473 · R_eff)` |
|---|---|---|
| none | 10.0 kΩ | 118 pF |
| 10 kΩ | 5.00 kΩ | 236 pF |
| 4.7 kΩ | 3.20 kΩ | 369 pF |
| 2.2 kΩ | 1.80 kΩ | 655 pF |

- At `R_eff` ≤ 10 kΩ and any plausible breakout and cable load, the rise stays under EXT's 1000 ns ceiling: the bus works electrically.
- **With the breakout fitted, EXT is outside the bound the gates judged:** the breakout's board, regulator and any level shifter add capacitance that J4's declared 20 pF does not cover. So `BENCH.md` B6's EXT lines and B7 run with J4 **empty**, never with the breakout.
- **Adafruit's breakout level-shifts SDA and SCL** ("All pins going into the breakout have level shifting circuitry", its guide). A level-shifted line is not an RC line: a rise time read on TP11/TP12 with it fitted is **not comparable** with any figure here. Record it, and say so.
- Source: **not predicted** for the rise (the breakout's load is unknown); the table is arithmetic.

### 2.3 The cable: J4 to the breakout, by function

**Never a straight-through adapter.** J4 is JST GH 1.25 mm in DS-009 order (`examples/sentinel-01.json` J4: 1 `VDD_3V3`, 2 `EXT_SCL`, 3 `EXT_SDA`, 4 `GND`). A Qwiic/STEMMA QT breakout's JST SH 1.0 mm socket is in a different order, so a GH-to-SH lead wired pin 1 to pin 1 puts 3.3 V on the breakout's ground and reverses its supply.

**Name each wire by what it touches on the board, never by its colour or its position.** Plug a 4-pin JST GH lead (flying leads at the far end) into J4, nothing powered, and find each wire's far end with the DMM on continuity:

| The wire continuous with | Carries | Goes to the breakout pad labelled |
|---|---|---|
| TP4 | J4 pin 1, `VDD_3V3`, +3.3 V | **VIN** (Adafruit; **not** its **3Vo**, the regulator's output) or **3V3** (SparkFun) |
| TP11 | J4 pin 2, `EXT_SCL`, clock | **SCL**, also labelled **SCK** |
| TP12 | J4 pin 3, `EXT_SDA`, data | **SDA**, also labelled **SDI** (**not** **SDO**) |
| TP1 | J4 pin 4, `GND` | **GND** |
| — | — | **SDO** and **CS**: nothing, **only if the breakout straps them on its own board** (Adafruit's does: "Leave the CS pin disconnected for I2C use", and its `addr` jumper sets SDO). Bosch: "SDO must not be floating" (DS 5.6) and CSB high selects I2C (DS Fig. 26), as `AET-WO-3304.bench.md` B4 quotes them. On a breakout that leaves them free, wire SDO to its supply pad (`0x47`) and CS to its supply pad |

Tag each wire as you find it, then solder or plug it to the pad the table names, reading the label printed beside the pad.

**If J4 is not fitted** (3 GH headers are on order for 9 positions, backordered; `BENCH.md`, "What is on order"), there is no lead to trace. Solder four wires from the board's first column to the breakout's third column, one row at a time, at TP4, TP11, TP12 and TP1 or at J4's bare pads on those nets. The check below is unchanged ("lead in J4" then means "wires soldered").

(For orientation only: J1, J2, J3 and J4 sit in a row on one edge, left to right, J2 being the 6-pin SH. Looking at the component side with that edge nearest you, J4's pin 1 is its end nearest J3: the review's footprint pin 1 at −x, the opening toward the edge, JST eGH p. 3. The continuity step above does not depend on it.)

**Check the cable before power. This is the step that can fail.** Breakout wired, lead in J4, nothing powered. Put the DMM's probe on the breakout's pad **by its printed label**, not on the wire you remember soldering there. "Not a short" means above 100 Ω once the reading stops changing (the board's decoupling charges for a few seconds).

| From | To (the pad labelled) | Pass |
|---|---|---|
| TP4 (VDD_3V3) | VIN or 3V3 | under 1 Ω |
| TP1 (GND) | GND | under 1 Ω |
| TP11 (EXT_SCL) | SCL/SCK | under 1 Ω |
| TP12 (EXT_SDA) | SDA/SDI | under 1 Ω |
| TP11, then TP12 | SDO, CS, 3Vo (if present) | not continuous |
| TP4 | GND | not a short |
| TP11, then TP12 | TP1 | not a short |
| TP11 | TP12 | not a short |

A reversed supply fails the first, second and sixth rows; crossed data lines fail the third and fourth; a wire on SDO or CS fails the fifth; the supply wire on 3Vo fails the first. Then power up as `BENCH.md` says (3.30 V, 100 mA limit) and read VIN or 3V3 against GND on the breakout: **positive, and within 0.05 V of TP4** (the same rail through the lead). §2.1's address probe is the powered check that the data lines work.

Lead: ________ · continuity rows all pass? ____ · breakout supply, powered: ________ V

## 3. The records

Flash the bench ELF, clear the records (`BENCH.md`, "What the ordered ELF puts where": a reset leaves the last boot's words in RAM), reset, wait at least 1 s, then read:

```
$P download --chip STM32H753ZITx sentinel-01-ext-bmp581.elf
$P write --chip STM32H753ZITx b32 0x20000000 0 0 0 0 0 0 0 0 0 0 0 0 0
$P read  --chip STM32H753ZITx b32 0x20000000 13   # all zero
$P reset --chip STM32H753ZITx
$P read  --chip STM32H753ZITx b32 0x20000000 13
```

(`download --verify` if this `probe-rs` offers it; syntax as `BENCH.md` says, not run on Furnace.)

The layout is the generated `main.rs`'s (`record(at, words, …)`): status at the word named, value in the words after it, most significant first. Status `0x22020000` is OK, `0x22020001` is TXIS (address unanswered).

| Word | Address | What | Predicted, SDO high (`0x47`) | Predicted, SDO low (`0x46`) | Source | Bench |
|---|---|---|---|---|---|---|
| 0 | `0x20000000` | boot word | `0x2100B007` | `0x2100B007` | twin (S1) | |
| 1 | `0x20000004` | `FLASH_OPTSR_CUR` | bits 3:2 = `11` after `BENCH.md` B3 | the same | as `BENCH.md` B3 | |
| 2, 3 | `0x20000008`, `0x2000000C` | U2 identity (I2C1, `0x76`, register `0xD0`) | `0x22020001` / `0x00000000` | the same | generator and RM0433 (`BENCH.md`, "U2 empty"); **repeat in kind** (`3302`). The twin's S1 read `0x60` because its U2 is fitted | |
| 4, 5 | `0x20000010`, `0x20000014` | U5 identity (I2C2, `0x47`, register `0x01`) | `0x22020000` / `0x00000050` | `0x22020001` / `0x00000000` (TXIS) | twin (S1: `read 0x01 -> 0x50`); the BMP581's chip ID | |
| 6, 7 | `0x20000018`, `0x2000001C` | U5 configure (write `0x36` ← `0x40 0x81`) | `0x22020000` / `0x00000000` | `0x22020001` / `0x00000000` | twin (S1: `write 0x36 <- 0x40 0x81`) | |
| 8, 9 | `0x20000020`, `0x20000024` | U4 identity (SPI1, `0x9F`) | `0x22020000` / `0x00EF4017` | the same | twin; Winbond (`BENCH.md` B4). **Its address differs from the ordered ELF's** (`0x20000010`) | |
| 10 | `0x20000028` | U5 data status (6 bytes from `0x1D`, after the 10 ms wait) | `0x22020000` | `0x22020001` | twin (S1) | |
| 11, 12 | `0x2000002C`, `0x20000030` | U5 data value | **not predicted:** the bench reads the room. The twin's stimulus gives `0x00000000`, `0x1940F362` (25 °C, 101,325 Pa) | `0x00000000`, `0x00000000` | twin's value is its stimulus | |

The SDO-low column is the generated code's own path for an unanswered address (`main.rs`: `Err(I2cError::Txis) => (STATUS_TXIS, 0)`, each wait bounded) and RM0433's NACK (p. 1975), as `BENCH.md`'s "U2 empty" traces for I2C1: **generator**; the twin was not run with U5 absent.

**What fails here, and for what:** a U5 record that differs from the column of the strap read before power. With SDO high, TXIS means the breakout did not answer `0x47`: the cable (§2.3), the breakout's supply, or PB10/PB11. With SDO low, an OK record means the strap was misread. Words 8–9 still reading `EF4017` shows the board and the SPI path are unaffected by the breakout.

To read the data for the record: T = signed(`t2 t1 t0`) / 65,536 °C and P = (`p2 p1 p0`) / 64 Pa, the bytes in the order read (`0x1D`–`0x22`; `AET-WO-3304.bench.md` B5). Bench T ____ °C · P ____ Pa.

## 4. The I2C2 transaction, as `sigrok-cli` prints it

**Capture** with the FX2 analyzer: `D0` on TP11 (EXT_SCL), `D1` on TP12 (EXT_SDA), ground to J4 pin 4 or TP1. Untriggered, with the reset within about a second of the start (`AET-WO-3304.bench.md` B5, A1: a triggered capture loses the first START, and the identity read's `Start repeat` would print as `Start`).

```
sigrok-cli -d fx2lafw --config samplerate=24m --channels D0,D1 --time 4s -o s01-ext.sr
# second terminal, within ~1 s:
~/tools/probe-rs/bin/probe-rs reset --chip STM32H753ZITx
sigrok-cli -i s01-ext.sr -P i2c:scl=D0:sda=D1 -A i2c=addr-data
```

**The decoder** is libsigrokdecode 0.5.3's `i2c/pd.py` (sha256 `bd28a545…`) through `sigrok-cli` 0.7.2, read at source by `3304` (`AET-WO-3304.bench.md` B5). Check the installed hashes as that sheet says; if they differ, compare by meaning.

**The line format** (as `3304`): `Write` or `Read` prints before the `Address …` line. `Start repeat` prints only for a START with no STOP since the last one, so `Stop` then `Start` before a `Read` is a **disagreement**. The repeated starts are the **generator**'s: the transfer writes `CR2` with `START` after `TC`, `AUTOEND` 0 in the write phase (RM0433), and Renode's I2C model cannot tell Sr from P then S.

**Predicted output** (`#` lines are sources, not decoder output). This is the SDO-high case. With SDO low (§2.1), each of the three blocks is instead `Start`, `Write`, `Address write: 47`, `NACK`, `Stop`.

```
# U5 identity: twin read 0x01 -> 0x50 (S1)
i2c-1: Start
i2c-1: Write
i2c-1: Address write: 47
i2c-1: ACK
i2c-1: Data write: 01
i2c-1: ACK
i2c-1: Start repeat
i2c-1: Read
i2c-1: Address read: 47
i2c-1: ACK
i2c-1: Data read: 50
i2c-1: NACK
i2c-1: Stop
# U5 configure: twin write 0x36 <- 0x40 0x81 (OSR_CONFIG 0x40, ODR_CONFIG 0x81)
i2c-1: Start
i2c-1: Write
i2c-1: Address write: 47
i2c-1: ACK
i2c-1: Data write: 36
i2c-1: ACK
i2c-1: Data write: 40
i2c-1: ACK
i2c-1: Data write: 81
i2c-1: ACK
i2c-1: Stop
# U5 data, at least 10 ms after the flash read: six bytes from 0x1D
i2c-1: Start
i2c-1: Write
i2c-1: Address write: 47
i2c-1: ACK
i2c-1: Data write: 1D
i2c-1: ACK
i2c-1: Start repeat
i2c-1: Read
i2c-1: Address read: 47
i2c-1: ACK
i2c-1: Data read: t0
i2c-1: ACK
i2c-1: Data read: t1
i2c-1: ACK
i2c-1: Data read: t2
i2c-1: ACK
i2c-1: Data read: p0
i2c-1: ACK
i2c-1: Data read: p1
i2c-1: ACK
i2c-1: Data read: p2
i2c-1: NACK
i2c-1: Stop
```

| Transaction | Predicted | Source | Bench (paste) | Agree? |
|---|---|---|---|---|
| U5 identity | the 13 lines above | twin (bytes); generator (Sr) | | |
| U5 configure | the 11 lines above | twin | | |
| U5 data, framing (Sr, 6 reads, 5 ACK, last NACK, Stop) | as above | twin (count); generator (Sr) | | |
| U5 data, `t0..p2` | **not predicted** (the room); the twin's stimulus is `00 00 19 40 F3 62` | — | | — |
| SCL period inside a byte | **at least 10.037 µs** (at most 99.63 kHz): 648 programmed kernel cycles of `TIMINGR` `0x50D32A40` plus ≥ 4 synchronisation cycles, at HSI's fastest 64.958976 MHz | **rule** (`silicon.rs`, as `BENCH.md` B9's floor). It can fail only on a wrong `TIMINGR` or clock; the real period is longer by the rise and the synchronisation, and is not predicted | | |

No other transaction appears on EXT. On SENSORS (TP13/TP14, if a second capture is made), the ordered ELF's U2 NACK appears unchanged (`BENCH.md` B9).

## 5. What the twin cannot say here

- **U2 absent beside U3 present,** on SENSORS: the twin renders an absent device only as the whole bus failing (`BENCH.md`, "U2 empty"). Words 2–3 are the code trace and `3302`'s repeat.
- **The breakout:** its pull-ups, its level shifter, its regulator, its cable. None is in the specification, and with U5 declared as an on-board part, the twin's EXT line is the emitted board's, not J4 with a lead.
- **The repeated start** and **every data byte** (as `3304` §4).
- **`droop-bor3` with a BMP581:** the twin refuses it (S2).
- **Any time on silicon** beyond the SCL floor: the 10 ms wait is the generator's lower bound at HSI's fastest; silicon's cycles per spin are not modelled.
