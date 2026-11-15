# AET-WO-4300f FEEDBACK — off-board connectors: spike

**Builder:** Claude (strong tier), worktree `~/aethelgard-4300f`, branch `wo/aet-4300f-connectors` from `main` `cb44872`. The work order is read from `origin/claude/argon-repo-docs-synthesis-rjrb0d` (docs PR #132) and is not committed here.
**Cadence:** spike (§1), then STOP for the architect's ruling.

The predictions in §1 were committed before any fetch, any container (created or run) and any measured run. They are never edited afterwards; corrections go in a later section.

Labels:
- **[read]**: read at source, in this repository at `cb44872`, in a fetched document held by sha256, or in the pinned image's filesystem.
- **[measured]**: run on Furnace: host cargo under a capped user scope, KiCad in the pinned image under rootless podman with the caps read back.
- **[inferred]**: neither; reasoning from the two.

## 0. Source reads that shape the predictions

Each item is **[read]**, this repository at `cb44872`.

- **Vendored library.** `aethelgard-kicad-lib` holds 6 footprints and 10 symbols (README, `data/kicad-10.0.7/`). None is a connector. `aethelgard-eda::library` refuses a file that is missing, unpinned or altered, so a connector cannot reach the emitter without a vendoring change.
- **Nets in the specification today.** Every net a specification names is a bus line (`i2c_buses[].sda/scl`, `spi_buses[].sck/mosi/miso`) or an SPI chip select (`spi_devices[].cs`), plus the voltage domains. `SolvedTopology.gpio` is filled only from SPI chip selects (`solve.rs:287`). No field declares a lone net.
- **The emitter** (`schematic.rs`): every MCU pin the topology solves gets a 2.54 mm stub ending in a **local label** with the net's name. Every unused pin gets a `no_connect`; every rail a `PWR_FLAG`.
- **The board** (`pcb.rs`): each pad takes the net KiCad's netlist gives its pin (`/<label>`). A pad with no symbol pin carries no net.
- **Placement** (`pcb.rs` `place`) puts the MCU at `ORIGIN`, then each IC beside the placed IC it shares most nets with, then each passive beside its IC. The outline is derived from the placed parts plus `MARGIN_NM` (2 mm). There is no edge concept: the edge follows the parts. Every footprint is at rotation 0 on the front.
- **The router** (`route.rs`, `1307`'s fence, read only): it routes only nets with two or more pads (`route`, the `order` filter). A one-pad net gets no copper.
- **The gate** (`aethelgard check erc|drc`, `cli/src/kicad.rs`): it runs `kicad-cli` with `--exit-code-violations`, accepts exit 0 with no findings or 5 with some, and fails on any finding of any severity. The gate's project (`rules.rs:106`) sets ERC's `lib_symbol_issues`, `lib_symbol_mismatch` and `footprint_link_issues` to ignore, and nothing else.

## 1. Predictions (committed before any run)

**P0, the caps.** A no-op container started with `--memory 8g --memory-swap 8g --pids-limit -1`, with no cgroup write, reads `memory.max` 8589934592, `pids.max` max, and **`memory.swap.max` 8589934592, not 0**, as `4210` measured (F10 on PR #131). If so, the kickoff's rule stops every KiCad run. The 0 read on `1307`'s live container (`aet1307ci-240016`) carries a 02:10:18 mtime, which `4210` attributes to the pre-correction runner, so it is not evidence either way.

**P1 (S1), what exists.**
- KiCad 10.0.7's `Connector_JST.pretty` holds `JST_GH_SM02B-GHS-TB_1x02-1MP_P1.25mm_Horizontal` through `…SM15B…` and the vertical `BM0xB-GHS-TBT` set. Each SMD horizontal footprint has N signal pads and **two mounting pads that share the name `MP`**.
- The symbols are `Connector_Generic:Conn_01x04` (and siblings), plain, with no `MP` pin; `Connector:Conn_01x04_MountingPin` or similar carries one `MountingPin` pin.
- All are CC-BY-SA 4.0 with KiCad's design exception, so they vendor under `RUL-11` as the existing ones did.
- The JST-GH datasheet gives 1.25 mm pitch, 1 A per contact (AWG 26), and a mated height near 4.25 mm.
- DS-009 gives 6-pin TELEM (VCC, TX, RX, CTS, RTS, GND), 6-pin GPS/UART with I2C, 4-pin I2C (VCC, SCL, SDA, GND), 6-pin POWER (VCC, VCC, CURRENT, VOLTAGE, GND, GND), and a 10-pin or 6-pin debug port. Pin 1 is VCC on each and the last pin is GND.

**P2 (S2), an off-board net today.**
- No specification field yields a one-pin net except a bus with no device on it. An SPI bus declared with no `spi_devices` passes `validate` and `solve` (**prediction**: no rule refuses it), and `emit` draws SCK, MOSI and MISO as labels on single MCU pins and nets one pad each. An I2C bus with no device still gets its two pull-ups, so it is not one-pin.
- The board carries each such net on one MCU pad and no copper (the router skips one-pad nets).
- **DRC: exit 0, no findings** (no ratsnest for a one-pad net, and parity matches).
- **ERC: findings.** KiCad reports each single-use local label as `isolated_pin_label` (warning), so `check erc` exits 5 and **the gate fails**. That fails the "honestly" test of `4300a` §1.8 and `4300b` §1.6.

**P3 (S3), the shape.** Recommend **(a) explicit `connectors`**: each names a part, a DS-009 port name (or an explicit pin map) and each pin's net. Under (b) the I/O group would choose the connector, which leaves the power and ground pins, and a second port, implicit. `validate` refuses a pin map that contradicts the cited port by name, with the pin number and both nets. Ground pins join `GND`, and VCC pins join the declared domain they name; a VCC pin with no domain is refused.

**P4 (S4), placement and routing.**
- Today's placer puts the connector beside the IC it shares most nets with, so it lands inside the board and not at an edge. The outline follows it.
- A hand-placed JST-GH 4-pin at the reference's edge, rotated so the cable leaves outward, routes with the existing router (four two-pad nets), and DRC is clean except where the `MP` pads, with no net, block an escape. **Prediction:** routed, DRC clean.
- The 2-layer reference gains connectors on F.Cu at an edge, with ground through the B.Cu zone. The 4-layer fixture gains the same, with VCC and GND pins on the planes by via.

**P5 (S5), the twin.** A connector adds no peripheral, no `.repl` line and no witness: the twin is emitted from a `SolvedTopology` and `svd-hal`'s register facts, and a connector is in neither. **The candidate exception** is extraction (`extract.rs`), which bounds a line's capacitance on the board alone. A line that continues into a cable is under-counted, so `4300c`'s speed check would be optimistic. The twin itself is unchanged.

## 2. What was done, in order

1. Predictions committed (`75d66b3`), before any fetch or container.
2. DS-009 and the JST-GH datasheet fetched from their publishers, then JST-SH's (DS-009's debug ports use it, §3.1). Held in `~/.cache/aet-4300f/ds/` by sha256.
3. KiCad's connector files read from the pinned image by `podman create` + `podman cp` (never started: no process, no cgroup), and fetched from KiCad's GitLab at tag `10.0.7`, the vendoring source of every file in `aethelgard-kicad-lib`.
4. S2's host half run through the unmodified CLI (`25e887a`).
5. The caps read back from a no-op container (`spike/4300f/img.sh -- true`). **Swap read 8 GiB, so nothing ran** (§3.0), and no KiCad step was run after it.
6. KiCad 10.0.7's ERC source read for S2's gate half.
7. S4's host half measured by a probe crate (`spike/4300f/s4/`), built by a subagent and spot-checked (§3.4).
8. S3 and S5 read at source.

## 3. Results

### 3.0 The caps (P0: as predicted)

- **[measured]** `spike/4300f/img.sh -- true` started `podman run --memory 8g --memory-swap 8g --pids-limit -1 --network none` and wrote no cgroup file.
- It sampled the container's cgroup every 0.25 s and read `memory.max=8589934592`, **`memory.swap.max=8589934592`**, `pids.max=max` (`spike/4300f/out/caps.log`, first sample 02:42:37Z). The runner logs only a change of value. None was logged before it gave up at 02:43:22Z, and its last read was still 8 GiB.
- The runner releases the command only after four consecutive reads of swap 0. It never released, so `true` never ran (exit 99).
- This is `4210`'s F10, which is D8 on PR #131. **The kickoff's rule 3 therefore stops every KiCad step of this spike:**
  - S2's ERC and DRC;
  - S4's DRC.

  Both are answered below as far as the host and KiCad's source go. The harness that measures them is committed, so they run as soon as D8 is ruled (§5, D10).

### 3.1 S1, what exists

**The documents** (each **[read]**, held in `~/.cache/aet-4300f/ds/` with `SHA256SUMS`):

| Document | Fetched from | sha256 |
|---|---|---|
| DS-009 Pixhawk Connector Standard, rev 0.1.0, 12 pp. | `raw.githubusercontent.com/pixhawk/Pixhawk-Standards/master/DS-009%20Pixhawk%20Connector%20Standard.pdf`. The file's only commit is `48dd6d1d` (2020-04-24). | `dbc6b94e0367ef0700f3ae2ed5958fd00e8938435a12c7cf528a772ec1a5bbdb` |
| JST GH series, `eGH.pdf`, 6 pp. | `www.jst-mfg.com/product/pdf/eng/eGH.pdf` | `b1dcb317b6b9a4fbbedd2dbf42c64c95306a252d23de8933b85fcf161240b722` |
| JST SH series, `eSH.pdf`, 4 pp. (added: DS-009's debug ports) | `www.jst-mfg.com/product/pdf/eng/eSH.pdf` | `ea3071ca5ee5a6069eba534fa39a10f34c9fb742ee42135a69ab4156bfa0f5de` |

JST's text layer uses a font whose glyph codes are offset by 0x1F; it was decoded by that offset, and the dimension figure was read from a rendered page (`ds/png/page2.png`).

**JST GH** **[read]**:
- 1.25 mm pitch; pad pattern tolerance ±0.05 mm, non-accumulating.
- Current 1.0 A AC/DC at AWG #26; voltage 50 V; −40 to +105 °C; wire AWG #30 to #26.
- Headers: side entry `SM02B`–`SM15B-GHS-TB`, top entry `BM02B`–`BM15B-GHS-TBT`; housing `GHR-0xV-S`; contact `SSHL-002T-P0.2`.
- **Mated height (reference values):** side entry 4.35 mm above the board, with the plug body (7.15) mm long and the cable leaving parallel to the board. Top entry is (7.3) mm.
- **[inferred]** The side-entry cable leaves on the side away from the signal tails: JST's side view shows the cable leaving one end, but nothing in the figure ties that end to the pad view. In KiCad's footprint frame that is +y, the mounting-pad side, so at rotation 0 the cable leaves toward the board's bottom edge (+y). The courtyard is symmetric, so the footprint does not settle this. **Check the 3D model or JST's drawing before the build commits to an edge.**

**JST SH** **[read]**: 1.0 mm pitch; 1.0 A at AWG #28; 50 V.

**DS-009's ports** **[read]**. Pin 1 is the red wire. Signal voltages are as printed: +3.3 V signal levels, VCC as stated.

| Port | Pins | Pinout |
|---|---|---|
| Telemetry (any serial port) | 6 | VCC +5V · TX (out) · RX (in) · CTS (in) · RTS (out) · GND. CTS and RTS float when there is no flow control. |
| Full GPS + safety switch | 10 | VCC +5V · TX · RX · I2C1 SCL · I2C1 SDA · SAFETY SWITCH IN · SAFETY LED · VDD_3V3 · BUZZER (+5–24 V) · GND |
| Basic GPS | 6 | VCC +5V · TX · RX · I2C1 SCL · I2C1 SDA · GND |
| CAN | 4 | VCC +5V · CAN_H · CAN_L · GND |
| I2C | 4 | VCC +5V · SCL · SDA · GND. "1.5K pullup on autopilot". |
| SPI (optional) | 7 | VCC +5V · SCK · MISO · MOSI · !SS1 · !SS2 · GND |
| Analog power | 6 | VCC +5.3V · VCC +5.3V · CURRENT · VOLTAGE · GND · GND. "The VCC lines have to offer at least 2.5A continuous." |
| Debug full | 10 | **JST SH, `SM10B`.** Target voltage · console TX · console RX · SWDIO · SWCLK · SWO · TRACE · TRACE · TRACE · GND |
| Debug mini | 6 | **JST SH, `SM06B`.** Target voltage · console TX · console RX · SWDIO · SWCLK · GND |

What the read changes against the prediction (P1):
- **DS-009's debug ports are not JST GH.** They are JST SH (1.0 mm). SWD for Sentinel-02 needs a second connector series.
- **DS-009 has no power-port wording "POWER".** It names it "Analog Power". Its VCC needs 2.5 A continuous over two pins, but JST GH rates a contact at 1.0 A at AWG #26, so two carry 2.0 A **[read]**, both documents. DS-009 does not reconcile this. It is 4300d's problem, recorded here.
- **There is no motor-output, PWM or USB port in DS-009 rev 0.1.0.** `4300a`'s motor outputs and `4300e1`'s USB have no published pinout to validate against, so they need an explicit pin map.
- **Every external DS-009 port's pin 1 is +5 V** (+5.3 V on power). The two debug ports' pin 1 is the target processor voltage, 1.8–3.3 V. Sentinel-01 declares only `VDD_3V3` (§5, D5).
- **DS-009's licence** (p. 3): "Redistribution and use in products, without modification, are permitted provided that the following conditions are met: ● The trademark shall only be used for compliant products and in combination with a signed adopter agreement. ● Implementations of the standard must be compliant with the full specification. ● A royalty-free, non-exclusive license is provided to adopters with a valid adopter agreement for schematics and drawings based on the standard documentation." Copyright 2020, Pixhawk SIG. So the PDF is not vendored, and the "Pixhawk" name is not used for a port. Whether a schematic emitted to a DS-009 pinout needs an adopter agreement is a question for Scotty (D2).

**KiCad's connector files** **[read]**:
- The image at `c69ff9bc…` holds `Connector_JST.pretty` with 28 GH footprints and 29 SH footprints. Symbols ship as merged `.kicad_sym` files (`Connector_Generic.kicad_sym`, `Connector_Generic_MountingPin.kicad_sym`).
- Each candidate below was fetched from GitLab at tag `10.0.7`, the source `aethelgard-kicad-lib`'s README names, as one file per symbol in `.kicad_symdir`.
- Each footprint is byte-identical to the image's copy.
- **Licence:** `LICENSE.md` at the tag is byte-identical to the vendored one (`45d2bce7…`): CC-BY-SA 4.0 with the design exception. That fits `RUL-11` exactly as the 16 existing files do.

| File (KiCad 10.0.7) | sha256 | git blob |
|---|---|---|
| `Connector_JST.pretty/JST_GH_SM04B-GHS-TB_1x04-1MP_P1.25mm_Horizontal.kicad_mod` | `d28e139f60db2bef02272b37d5075e87f3e804aef0b442d0fd81a3f7cfe07ebe` | `d152b6f8` |
| `Connector_JST.pretty/JST_GH_SM06B-GHS-TB_1x06-1MP_P1.25mm_Horizontal.kicad_mod` | `d1855caf77704056d0af0775660442c74af40b18ae3e40bd244db5997fdc072f` | `d1315a6b` |
| `Connector_JST.pretty/JST_GH_SM10B-GHS-TB_1x10-1MP_P1.25mm_Horizontal.kicad_mod` | `5767bc29104c4797f67b898b1db76f1e1afb5ac65f432afaabda2c56769c7da7` | `f15fb25e` |
| `Connector_JST.pretty/JST_GH_BM04B-GHS-TBT_1x04-1MP_P1.25mm_Vertical.kicad_mod` | `c0e5c962366864b39ad3a295bb6419547e18f0f5ac49060d75ea51d0e7079732` | `842df399` |
| `Connector_JST.pretty/JST_SH_SM06B-SRSS-TB_1x06-1MP_P1.00mm_Horizontal.kicad_mod` | `090e9bd7350aed224a938b922cc7d49020aa5235cc7e647a82cc3c08f37e881e` | `106a8a1f` |
| `Connector_JST.pretty/JST_SH_SM10B-SRSS-TB_1x10-1MP_P1.00mm_Horizontal.kicad_mod` | `722ee628302edc008d93a3b20cb4e8f597dcc85c5e7c185fc85ebeb32817bbc5` | `04182ed3` |
| `Connector_Generic_MountingPin.kicad_symdir/Conn_01x04_MountingPin.kicad_sym` | `370c2845e6ad1e61ea1b6b5c6a0784dea59f4aedc6d16eedda5ea3108c5105ac` | `1f1d2fbc` |
| `Connector_Generic_MountingPin.kicad_symdir/Conn_01x06_MountingPin.kicad_sym` | `43b18535b89d110da0b5750f4aeff7ed5c1ccb7def712d79451b292f55ce3c53` | `e9d132ee` |
| `Connector_Generic_MountingPin.kicad_symdir/Conn_01x10_MountingPin.kicad_sym` | `29f8c6d312d1a9a13225066d864c58b4e1e08d001360fb5bcc196a08be14caba` | `42396e1e` |
| `Connector_Generic.kicad_symdir/Conn_01x04.kicad_sym` | `6047a81578c35a3eb39e5bb055d85acb20523327c75cf327fa436cb33acbd69b` | `d07c05d6` |

**The footprints' shape** **[read]**, using `SM04B` (the others follow the same pattern):
- Signal pads `1`–`4`: SMD roundrect 0.6 × 1.7 mm, at x = −1.875 … +1.875 and y = −1.85.
- **Two mounting pads, both named `MP`**: 1 × 2.7 mm at (±3.725, +1.35).
- Courtyard ±4.73 × (−3.2 … +3.2) mm.
- Every pad is front SMD, so `route::part` accepts the shape.
- The `(model "${KICAD10_3DMODEL_DIR}/…step")` line is the only one with a variable. All 6 vendored footprints carry the same kind of line.

**The symbols** **[read]**:
- `Conn_01x04` has passive pins `1`–`4` and no `MP` pin. Its footprint filter is `Connector*:*_1x??_*`, which does **not** match the GH names (`_1x04-1MP_`).
- `Conn_01x04_MountingPin` adds a passive pin `MP` (name `MountPin`). Its filter `Connector*:*_1x??-1MP*` does match. The gate's project ignores `footprint_link_issues`, so the filter is not judged either way.
- With `_MountingPin`, both `MP` pads take that pin's net.
- With the plain symbol, they carry none. `pcb.rs` already allows that for TMP117's exposed pad, which parity accepts.

### 3.2 S2, an off-board net today

**The only specification-legal one-pin net is a bus line with no device on it** **[read]** (§0). An SPI bus with no device yields three: no rule refuses it (P2 held). An I2C bus with no device still gets its two pull-ups.

**Host half** **[measured]**:
- The probe spec `spike/4300f/s2/offboard.json` is the reference plus SPI bus `EXT`, which has nets `EXT_SCK`, `EXT_MOSI` and `EXT_MISO` and no device.
- It ran through the unmodified CLI, built at `75d66b3`:
  - `validate`: exit 0;
  - `solve`: exit 0, with `SPI2` on PA12, PB14 and PB15, AF5;
  - `emit`: exit 0.
- The emitted files' hashes are in `spike/4300f/s2/emitted.sha256`, and the files themselves are in `~/.cache/aet-4300f/s2-out/`.
- **The schematic:** each `EXT_*` net is one local label on one wire stub on one MCU pin. There is no `no_connect` and no other label of that name.
- **The board:** each is one MCU pad carrying `(net "/EXT_…")`, with no segment, via or zone on it. The router routes only nets with two or more pads (`route.rs`, the `order` filter).

**Gate half: not measured** (§3.0).

**ERC** **[read]**, KiCad 10.0.7 at GitLab tag `10.0.7`, sha256s in `~/.cache/aet-4300f/kicad-src/SHA256SUMS`:
- `CONNECTION_GRAPH::ercCheckLabels` (`eeschema/connection_graph.cpp` ~l. 4640) reports `ERCE_LABEL_SINGLE_PIN` for a label whose net, across the schematic, has exactly one pin and no no-connect.
- That is `isolated_pin_label`, "Label connected to only one pin" (`erc_item.cpp:110`).
- Its default severity is warning (`erc_settings.cpp:108`).
- The gate's project overrides only `lib_symbol_issues`, `lib_symbol_mismatch` and `footprint_link_issues` (`rules.rs:106`), and `check erc` counts errors and warnings alike (`kicad.rs:182`).
- **[inferred] `check erc` exits 5 with three `isolated_pin_label` findings, so the gate does not pass an off-board net.**
- A `no_connect` on the pin would silence it, but it would mark a used pin unused. That is a waiver by another name, which `4300a` §1.8 and `4300b` §1.6 rule out ("no waived check, no dangling label").

**DRC** **[inferred]**: no finding.
- There is no ratsnest for a one-pad net, and the pad keeps its clearances because it is the emitter's own pad.
- Parity matches, because the board's pad net is the netlist's.

**So the answer to `4300a` §1.8 and `4300b` §1.6** is:
- **[measured]** `emit` draws the net honestly;
- **[read]** ERC will fail it;
- the gate result itself is unmeasured.

It went to both inboxes at 02:50Z, with the same labels.

### 3.3 S3, the specification's shape

**Recommended: (a), explicit `connectors`** **[inferred]** from §0, §3.1 and §3.2.

**The proposed shape** (specification version N+1, taken once, after `4300a`'s):

```json
"connectors": [
  { "reference": "J2", "part": "JST-GH-SM06B", "standard": "ds009-telemetry",
    "pins": [ {"pin": "1", "net": "VDD_5V"}, {"pin": "2", "net": "UART7_TX"},
              {"pin": "3", "net": "UART7_RX"}, {"pin": "4", "net": null},
              {"pin": "5", "net": null}, {"pin": "6", "net": "GND"} ] },
  { "reference": "J5", "part": "JST-GH-SM04B", "standard": null,
    "pins": [ {"pin": "1", "net": "MOTOR1"}, {"pin": "2", "net": "MOTOR2"},
              {"pin": "3", "net": "MOTOR3"}, {"pin": "4", "net": "GND"} ] }
]
```

**The pieces:**
- **`part`** names a row in a connectors table in `aethelgard-ir` (`INV-01`), cited data like `parts.json`. Each row holds the pin count, the KiCad symbol and footprint, the mounting-pad rule (D3), the contact current rating and the datasheet sha256.
- **`standard`** names a row in a ports table transcribed from DS-009, each port citing DS-009's sha256 and page. It is either one of those rows or `null`. `null` is the explicit pin map, for ports DS-009 does not cover: motor outputs, USB, SWD by choice.
- **Every pin is listed**, with `net: null` for a pin the port leaves floating. CTS and RTS without flow control are one example. The emitter gives such a pin a `no_connect`, which is honest because the pin really is unconnected.

**Why not (b), implied by each I/O group:**
- It leaves VCC, GND and the floating pins implicit.
- It cannot put two groups on one port (DS-009's GPS ports carry a UART and I2C).
- It cannot choose between DS-009's two GPS ports or two serial ports on one connector size.
- It hides the connector's reference from the bill of materials' author. (c), a group-to-port shorthand that expands into (a), could come later as sugar; it adds nothing (a) cannot say.

**How `validate` refuses a contradiction** (exit 2, by name; `INV-06`):
- **`ConnectorPinContradictsStandard`** (`reference`, `pin`, the standard's role, and the net given). With `standard` set, each pin's net must have the role the port's row names:
  - `TX`/`RX` must be a serial port's TX or RX net (`4300b`);
  - `SCL`/`SDA` must be an I2C bus's lines;
  - `SCK`/`MISO`/`MOSI` must be an SPI bus's lines;
  - `GND` must be `GND`;
  - `VCC` must be a voltage domain;
  - a "may float" pin must be `null` or its role's net.
- **`ConnectorStandardPinCount`**: the part's pin count is not the port's.
- **`ConnectorSupplyContradictsStandard`**: a VCC pin's domain nominal is outside the standard's stated voltage, for example DS-009's +5 V against `VDD_3V3`. Sentinel-01 has no 5 V domain, so a DS-009 port on it is refused until a 5 V domain is declared (D5).
- **`ConnectorPinUnknown` / `ConnectorPinRepeated` / `ConnectorPinMissing`**: the pin list is not exactly the part's pins.
- **`ConnectorNetUnknown`**: a net no bus, port, domain or `GND` declares. **A connector cannot invent a net.**
- **`ConnectorCurrentExceeded`** is for when a domain's declared load on a connector's VCC pins exceeds pins × the contact rating. This needs `4300d`'s loads; until then it is unbuilt, and the verdict says so.

**How ground and power pins join their domains:**
- A `GND` pin is drawn with the emitter's ground symbol, the same `GND` net as every other ground pin. At 2 layers it joins the B.Cu zone by a via (`route_ground`); at 4 layers, the GND plane.
- A VCC pin naming domain `D` is drawn with `D`'s power symbol, so it is on `D`'s flagged rail. Connector pins are passive, which ERC allows on a flagged rail (`schematic.rs`, the header's rules).
- At 4 layers, a pin on the plane's domain joins the plane by via, as other plane-net pads do.

**Where connectors sit in the IR** **[read]**, `RUL-16`:
- The emitter already takes the `Specification` and the `SolvedTopology`.
- The twin takes only the latter.
- Keeping connectors in the `Specification` only, never in the `SolvedTopology`, keeps them out of the twin by construction (§3.5).

### 3.4 S4, placement and routing

**Today's placement** **[read]**, `pcb.rs`:
- **No edge.** `place` puts each part beside the IC it shares most nets with, and the outline is the placed keep-out boxes plus `MARGIN_NM` (2 mm). A connector given to today's placer lands beside the MCU, inside the board, with the outline following it, as P4 predicted.
- **No rotation.** Every footprint is at rotation 0 (`pcb.rs` header), and **`route::part` refuses a rotated footprint** ("J1 is rotated", also measured). At rotation 0 the GH side-entry footprint's cable leaves toward +y (§3.1), so **the bottom edge is the only edge a connector can face today.**

**The probe** **[measured]** (host, `spike/4300f/s4/`, output `result.txt`), built by a subagent and spot-checked against `result.txt` and the written boards:
- **The reference**, emitted unmodified: 15 parts, 360 segments, 70 vias and 1 zone.
  - Re-routing it with `route::route` gives identical copper, with 37 rip-ups, in 6.5 s (a debug build).
  - `check` and `connectivity` pass.
- **J1** is `JST_GH_SM04B-GHS-TB` at rotation 0, origin (94.8, 118.275) mm, x-centred.
  - It is placed by the placer's own rule. Its keep-out box comes from `pcb::geometry`: the courtyard, ±4.755 × ±3.225 mm with its line width, plus the visible silkscreen `Reference` text at y −3.9, giving a top of −4.55. That box must clear every placed keep-out box by `GAP_NM`, or by `IC_GAP_NM` (1 mm) when both are ICs. A connector is an IC to `is_ic`, because its lib id is not `Device:`.
  - The first candidate below the old edge clears every box. The nearest box overlapping its x range is U1's, 2.05 mm away.
  - The outline's bottom moves from 116.55 to 121.5 mm (+4.95 mm), so the housing's front (the courtyard's bottom) is flush with the edge.
- **J1's pads** follow DS-009's I2C port: 1 `VDD_3V3` (DS-009 says +5 V, and Sentinel-01 has none), 2 `/I2C_SCL`, 3 `/I2C_SDA`, 4 `GND`.
- **The two MP pads** sit 0.525 mm from the bottom edge (`EDGE_NM` is 0.5 mm).

| | A: MP no net | B: MP on GND |
|---|---|---|
| `route` | Ok, 390 segments, 78 vias, 37 rip-ups, 6.3 s | Ok, 396 segments, 80 vias, 37 rip-ups, 6.4 s |
| `check` / `connectivity` | Ok / Ok | Ok / Ok |
| board written, read back by `strict` + `route::read` | equal, Ok | equal, Ok |
| sha256 (`~/.cache/aet-4300f/s4-out/`) | `000550fa…` | `a459f8e2…` |

- **Placement legal:** yes, by the placer's keep-out boxes and gaps and by `EDGE_NM` **[measured]**.
  - The first version of this probe compared J1's courtyard against pads only. The adversarial pass caught that, and the probe now uses `pcb::geometry`.
  - The position moved by 25 µm, and the routing counts are unchanged.
- **Routed, with the router's exact `check` and `connectivity`:** yes, both variants. Two runs gave identical output (wall times aside) and identical boards **[measured]**.
- **DRC:** not measured (§3.0). `spike/4300f/gates.sh` runs it on both boards, unrun.
  - The boards carry no schematic symbol for J1, so the DRC there omits parity.
  - **[inferred]** the risks DRC would judge that the router does not: courtyard and silkscreen against the edge (the housing is flush with it), and mask slivers between the 0.65 mm-gap GH pads.

**Proposal** **[inferred]**:
1. **Placement.** Place the core as today, then each connector last, on an edge the specification names (`"edge": "bottom"` and so on, or the placer's default), ordered along that edge by reference and spaced by `GAP_NM`. Then grow the outline so each housing's front is flush with its edge, keeping `EDGE_NM` from every pad.
   - **For v1, bottom only** needs nothing new.
   - **Other edges** need `route::part` and `pcb.rs` to accept 90°, 180° and 270°. Pads stay axis-aligned at right angles, so the router's exact box arithmetic holds. That is `1307`'s fence: after it merges, and by the architect's split (D7).
2. **Routing.** No change. A connector's nets are ordinary multi-pad nets; GND pins reach the B.Cu zone (2 layers) or the GND plane (4) by the existing via path.
   - VCC pins are tracked at the one declared width. A connector that carries supply current (DS-009 VCC, up to 2.5 A) needs a width for its current, which belongs with `4300d`.
3. **What each board gains.**
   - **The 2-layer reference:** connectors on F.Cu at the bottom edge, ground by via to the B.Cu zone, and rails as tracks.
   - **The 4-layer fixture:** the same, plus each VCC pin on the plane's domain by via to the plane, and shorter signal escapes, because the plane frees B.Cu.

### 3.5 S5, the twin

**A connector is invisible to the twin** **[read]**:
- `aethelgard-twin` emits the `.repl`, the analog decks and the boot plan from a `SolvedTopology` and `svd-hal`'s register facts. It never reads a `Specification` (`RUL-16`, architecture §3 table).
- Under §3.3 a connector lives only in the `Specification`. So it adds no peripheral, no `.repl` line, no deck and no witness, and **no witness changes**.
- Firmware is unaffected too. The board crate is generated from the solved topology, and a connector solves no pin.

**Three exceptions, none of them in the twin itself:**
1. **Extraction refuses the board** **[read]**, `extract.rs` `line_with`. Every part with a pad on an I2C line must be one the specification puts on that line, so a connector pad on `I2C_SDA` (the DS-009 I2C or GPS port) is refused with `UnmodelledPart`. `lines()` must learn connector pins: their copper counted, no `Ci`.
2. **The bound stops being a bound** **[inferred]**, from `RUL-19` (1). The cable and the off-board device's pins add capacitance the board's copper does not show. A line that leaves the board therefore needs a declared off-board budget (cable plus remote `Ci`, with a source). Extraction adds that budget, and the claim names it. Without it, "the campaign's declared value bounds the board's" is false for that line. This touches `4300c`'s `Fmax(C)` check, because SPI on a DS-009 SPI port, and UART lines, leave the board the same way (D8).
3. **An off-board device** **[read]**, `schematic.rs` `design`. A GPS's compass on a DS-009 GPS port's I2C pins is a real I2C device the twin should model and the firmware should read. Today, though, every declared `i2c_device` is placed on the board by `emit`. A device behind a connector needs a way to say "modelled in the twin, not placed" (D9). This is the one case where a connector would change what the twin sees: the device would, not the connector.

**DS-009's I2C port says "1.5K pullup on autopilot".** A bus on that port could have its declared `pull_up_ohms` checked against 1500 by the same `ConnectorPinContradictsStandard` path. That is optional and is recorded for the ruling.

## 4. The stop

### 4.1 The table

| Question | Answer | Label |
|---|---|---|
| S1: DS-009 fetchable from Furnace | yes, from the Pixhawk-Standards repository (only commit `48dd6d1d`), `dbc6b94e…` | measured |
| S1: JST-GH datasheet fetchable | yes, from jst-mfg.com, `b1dcb317…`: 1.25 mm, 1.0 A at AWG #26, 50 V, mated 4.35 mm (side) / 7.3 mm (top) | measured / read |
| S1: DS-009 pinouts | §3.1 table: 9 ports. Debug ports are **JST SH**, not GH. Every VCC is +5 V. No motor, PWM or USB port. | read |
| S1: KiCad GH/SH footprints and symbols vendorable under `RUL-11` | yes: CC-BY-SA 4.0 + design exception, the same licence file byte for byte; GitLab = image bytes | read |
| S2: `validate`/`solve`/`emit` with a one-pin net | all exit 0; one label on one MCU pin; one pad, no copper | measured |
| S2: ERC | `isolated_pin_label` warning per net, so the gate fails (exit 5 expected) | read (KiCad source), **run blocked** |
| S2: DRC | no finding expected | inferred, **run blocked** |
| S3: shape | (a) explicit `connectors`, with `standard` or `null`; seven named refusals | inferred |
| S4: placer puts a connector at an edge | no: beside an IC; no rotation (`route::part` refuses it) | read, measured |
| S4: hand-placed GH 4-pin at the bottom edge | placement legal, routed, `check` and `connectivity` Ok, both MP variants, deterministic | measured (host) |
| S4: DRC of that board | **not run**; harness `spike/4300f/gates.sh` | blocked |
| S5: twin | unchanged; no peripheral, no witness change | read |
| S5: exceptions | extraction refuses a connector pad on an I2C line (`UnmodelledPart`); the bound ignores cable and remote `Ci`; an off-board device is placed by `emit` | read, inferred |
| caps | `memory.swap.max` = 8 GiB under `--memory 8g --memory-swap 8g` | measured |

**Predictions against results:**
- **P0 held.** Swap read 8 GiB.
- **P1 held in part, with three misses:**
  - debug is SH;
  - the power port is "Analog Power", with a current contradiction;
  - there is no motor or USB port.
- **P2 held** where it could be checked: the device-less SPI bus is admitted, there is one pad and no copper, and the ERC rule exists as predicted (read). The ERC and DRC predictions are unmeasured.
- **P3 stands as the recommendation.**
- **P4 held on the host.** It missed that rotation is refused, which confines v1 to one edge. DRC is unmeasured.
- **P5 held.** The exception is wider than predicted: extraction refuses outright, rather than under-counting, and there is the off-board-device case.

### 4.2 Recommended shape

§3.3 option (a): `connectors[]`, each holding:
- `reference`;
- `part` (a cited connectors table in `aethelgard-ir`);
- `standard` (a cited DS-009 ports table row, or `null`);
- every pin with its net or `null`.

It sits in the `Specification` only. GND and VCC pins join through the emitter's ground and rail symbols. Placement is on a named edge, bottom-only until rotation lands.

### 4.3 Vendoring list (for the build, if ruled)

Into `aethelgard-kicad-lib`, verbatim from KiCad GitLab tag `10.0.7`, each with its sha256 pinned in `aethelgard-eda::library` (§3.1 table):
- **Footprints:**
  - `JST_GH_SM04B-GHS-TB_1x04-1MP_P1.25mm_Horizontal` (DS-009 I2C, CAN);
  - `JST_GH_SM06B-GHS-TB_1x06-1MP_P1.25mm_Horizontal` (TELEM, basic GPS, power);
  - `JST_GH_SM10B-GHS-TB_1x10-1MP_P1.25mm_Horizontal` (full GPS).
  - For Sentinel-02's SWD, if ruled: `JST_SH_SM06B-SRSS-TB_1x06-1MP_P1.00mm_Horizontal` (debug mini).
  - The 7-pin SPI port and the 10-pin SH debug are left out until a board needs them.
  - The vertical `BM…` parts are left out: horizontal is cable-friendly at an edge, and one orientation keeps the table small.
- **Symbols:** `Connector_Generic_MountingPin.kicad_symdir/Conn_01x04_MountingPin`, `…01x06…` and `…01x10…` (D3).
- **Licence:** no new file. The existing `LICENSE.md` is byte-identical.
- **The 3D-model line** stays as KiCad wrote it, as for the 6 vendored footprints.

## 5. Decisions needed (for the architect; D2 is Scotty's)

- **D1. Shape:** (a) explicit `connectors` (recommended), (b) implied by I/O group, or (c) (a) plus a shorthand.
- **D2. DS-009's licence and trademark (Scotty).** The standard grants a schematic and drawing licence "to adopters with a valid adopter agreement", and reserves the Pixhawk mark for compliant, signed adopters.
  - Options: (a) cite DS-009 pinouts as facts, never use the mark, and vendor no PDF; (b) get an adopter agreement; (c) explicit pin maps only, with no `standard` field.
  - Recommended: (a) if Scotty accepts the reading, else (c).
- **D3. Mounting pads:**
  - (a) the `Conn_01xNN_MountingPin` symbol with `MP` on `GND`: its filter matches, and the pads are ground-stitched (measured: routes, +2 vias);
  - (b) `MP` with a `no_connect`;
  - (c) the plain `Conn_01xNN`, with no net on `MP` (measured: routes).
  - Recommended: (a).
- **D4. The vendoring list in §4.3.** Should SH come in now for Sentinel-02's SWD, or wait?
- **D5. DS-009 VCC is +5 V; Sentinel-01 has only `VDD_3V3`.**
  - Options: (a) refuse a DS-009 port without a 5 V domain (recommended, `INV-06`), so Sentinel-01's connectors are explicit maps until `4300d` declares 5 V; (b) allow VCC `null` on a DS-009 port; (c) allow 3V3 on pin 1 with a named deviation. DS-009's licence requires that "implementations of the standard must be compliant with the full specification", so (c) could not be called DS-009.
- **D6. DS-009's power port needs 2.5 A over two GH contacts rated 1.0 A each.** Should `4300d` take a different connector for power-in?
- **D7. Rotation.** For v1, bottom edge only (no router change), or rotation in `route::part` and `pcb.rs` after `1307` merges (its fence)?
- **D8. Lines that leave the board.**
  - Extraction must count connector pads (not refuse them).
  - Who declares the off-board capacitance budget (cable plus remote `Ci`, with a source), so that the bound in `RUL-19` (1) and `4300c`'s `Fmax(C)` stay true?
  - Options: (a) a per-connector-pin declared budget, required on I2C and SPI lines (recommended); (b) the line's `max_capacitance` covers it, so extraction checks board + 0, which is weaker and should be stated.
- **D9. Off-board devices** (a GPS's compass on a DS-009 GPS port): modelled in the twin, not placed on the board. Is that in `4300f`'s build or a later work order?
- **D10. The caps stop (= 4210's D8 on PR #131).** S2's ERC and DRC and S4's DRC are unmeasured because swap read 8 GiB.
  - The harness is committed: `bash spike/4300f/img.sh -- bash spike/4300f/gates.sh`.
  - Once D8 is ruled, it runs in under a minute and answers `4300a`/`4300b` by measurement. Both inboxes already have the host half and the source read.
- **D11. Sequencing for `4300a` and `4300b`.** By KiCad's source, their off-board nets fail ERC honestly, so both will STOP at their §1 measured stop unless connectors land first. Should `4300f`'s build come before their board side, or should they carry a bottom-edge connector themselves?

## 6. Adversarial pass, reduction pass, gates

**Adversarial pass:** a fresh subagent was given the sources, not this document's conclusions, and asked to falsify §3–§5. What it found:
- **Confirmed**, at source:
  - S2's ERC reading, including that no pin type, label type or `has_nc` path spares a single MCU pin's local label;
  - the gate counting warnings;
  - the DS-009 table, pin by pin;
  - the JST figures;
  - every KiCad file hash;
  - the rotation refusal and the one-pad skip;
  - extraction's `UnmodelledPart`;
  - the twin never reading a `Specification`.
- **Wrong, fixed:** S4's "placement legal by `GAP_NM`" was unchecked. The probe compared pads, not keep-out boxes, which include silkscreen text.
  - The probe now places by `pcb::geometry` and the placer's gaps.
  - That measured the placement legal, 2.05 mm from U1 (the subagent's inferred overlap with U1 did not hold when measured), and moved J1 by 25 µm (§3.4).
- **Wrong, fixed:** "every DS-009 port's pin 1 is +5 V". The debug ports' pin 1 is the target voltage.
- **Overstated, fixed:**
  - the cable direction was listed as read; it is now inferred, with a check named for the build;
  - "held 45 s" is now worded as the log supports it;
  - the licence quote's ellipsis dropped the full-compliance condition, which bears on D2 and D5 (c), and is now quoted whole;
  - "second run byte-identical" had no record, and is now measured.

**Reduction pass (diff):** nothing to remove in the repository. There are 2 findings, both spike-only, which the build deletes:
- `spike/4300f/img.sh` is `4210`'s runner pattern copied. A shared runner would be one copy, but spikes never reach `main`.
- The S4 probe copies `pcb.rs`'s private `copper()` to write boards. If the build needs a probe-like path, it should reach the function rather than copy it.

Outside the repository, an 802 MB scratch copy of KiCad's share tree was deleted from `~/.cache/aet-4300f`.

**Gates** **[measured]**, Furnace, host, under a capped user scope (`systemd-run --user --scope -p MemoryMax=32G -p MemorySwapMax=0`, nice 10), at `fbbfd4f`, whose code is unchanged from `main` `cb44872`:
- `cargo fmt --all -- --check`: 0;
- `cargo clippy --workspace --all-targets -- -D warnings`: 0;
- `cargo test --workspace`: 0, with 1200 passed, 0 failed and 0 ignored across 77 test binaries.

The spike adds no workspace code. Its probe crate has its own `[workspace]`. Tier 2 is CI's.
