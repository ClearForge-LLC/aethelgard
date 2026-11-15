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
