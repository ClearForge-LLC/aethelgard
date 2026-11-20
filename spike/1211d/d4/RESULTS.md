# AET-WO-1211d, 4600 D4: the device arms as a table and one interpreter (spike only)

## Question
Can the schematic emitter's per-part device arms (I2C: BME280, BMP581, TMP117; SPI: W25Q64JVxIQ,
ICM20948, ICM42688P) be row data, without moving a byte of any existing part's output?

## Answer
Yes. Six rows and one interpreter produce byte-identical schematics, boards and bills of
materials for every committed specification. The data fits; what it cannot express is a short
list of behaviours, each a named interpreter feature (below). Nothing was merged; the code is in
`table-interpreter.patch` (against 76e8cafe).

## Method
- Base: 76e8cafe. Table and interpreter live in `crates/aethelgard-eda/src/schematic.rs` (a new file
  under `src/` would meet `strict_fence`, so the spike stayed in the one file). The IR crate and
  `parts.json` are untouched.
- `cargo build --release -p aethelgard-cli`; `sweep.sh` into `~/.cache/aet-1211d/d4/sweep-table`;
  `same.sh` against `sweep-base`.

## Sweep result
`same.sh`'s own output is NOT the single word "identical". It prints 24 lines of the form
`Files .../sweep-base/<spec>/emit.out and .../d4/sweep-table/<spec>/emit.out differ`, exit 1.

Why, and what was checked:
- `emit.out` is the CLI's JSON, which embeds the absolute output path (`"path":"/home/forge/.cache/aet-1211d/<dir>/..."`).
  `sweep-base` and `sweep-table` are different directories, so every `emit.out` that reports success
  differs in its path strings (24 of 31 specs; the other 7 refuse, and their `emit.out` has no path).
  The `sha256` fields beside those paths are equal (checked on examples/sentinel-01.json: schematic
  aa9d78af..., bom a0ff99dd..., board 401b9e1e...).
- Every other file (all `.kicad_sch`, `.kicad_pcb`, `.bom.csv`, every `validate.*`, `solve.*`,
  `*.exit`, `specs.txt`): `diff -rq` lists no difference. 259 files in the base.
- With the two directory prefixes normalised by `sed`, all 31 `emit.out` are byte-equal (0 differ).
- So: identical in content; same.sh as written cannot say "identical" when the two output
  directories differ, because of `emit.out`'s embedded paths. The builder should either sweep
  both into the same-named directory (move one aside) or have same.sh normalise the prefix.
  I did not edit same.sh (not mine).

## Interpreter features (each is a thing a plain row cannot say)
1. **Address strap choice (`Role::Addr`).** The connection depends on the specification's address,
   chosen by position against the parts row's address list (`strapped`), so the interpreter needs
   the declared device. It has two forms: through a named resistor (BME280/BMP581 `SDO`, an own net
   `<ref>_SDO`), or wired direct to one of four targets (TMP117 `ADD0`: GND, V+, SDA, SCL). The target
   list is row data; the choice and its refusals (`UnsupportedAddress`, count mismatch) are code.
2. **Strap to the supply through a resistor (`Role::Strap`).** An own net plus a resistor, drawn in
   pin order (reference numbers depend on it). Data, but needs the own-net claim and the passive
   builder.
3. **Own net closed by a capacitor (`Role::Own` + `After::OwnCap`).** ICM20948 REGOUT: the net is
   claimed before the part, the capacitor drawn after it, and the two must name each other.
4. **Translator substitution (`behind`).** For SPI the four bus lines resolve through the crossing
   map, falling back to the bus's own nets. A row says `Cs`, `Sck`, `Mosi`, `Miso`; the substitution
   is the interpreter's (a closure), computed by `power::behind` before any SPI device is drawn.
   I2C has no crossing.
5. **Ordering code and note override (`stand_in`).** W25Q's symbol is `W25Q32JVSS`; the Value and BOM
   note say `W25Q64JVSSIQ` and name the stand-in. A row field, but one the old `device()` keyed on the
   lib_id string; it is now an argument.
6. **Supply decoupling list (`After::Decouple`).** Values and count per part (BMP581 two 100n,
   ICM20948 two 100n, ICM42688P 100n/2u2/10n). Data. Note ICM20948's REGOUT capacitor is drawn
   BEFORE its decouplers, so passives are one ordered list of two kinds.
7. **Open pins.** A symbol pin not in the row is left open (the arms did this too). Needs no feature;
   a `NoConnect` role is unnecessary for byte identity.
8. **Order is data.** Straps are drawn in pin-list order, and reference designators (R/C numbers)
   follow it; the W25Q row puts WP (pin 3) before HOLD (pin 7) and the address strap first. A row author must
   know this; a test should pin it.
9. **Bus kind.** A row is I2C or SPI (`Kind`), so an SPI device named BME280 is still `UnknownPart`.

Nothing else was needed: no per-part conditionals remained (the old `if d.part == "BMP581"` second
capacitor is just a second `Decouple` entry).

## Line counts
- Before (76e8cafe): I2C arms 76 lines (L1420-1495) plus SPI arms and their line closure 83 lines
  (L1518-1600) = 159, plus a 14-line W25Q special case inside `device()`.
- After: the two call sites 8 + 21 lines; types 47; the six rows 131 (rustfmt expands each pin onto a
  line); `row_for` 13; `draw_device` 62. Table plus interpreter: 254 lines (as inserted).
- Net: `git diff --shortstat` of crates: +273 / -165, so the file grows by 108 lines. The table is
  longer than the arms it replaces; the gain is that the arms are data, not that the code is shorter.
  The interpreter itself is about 110 lines (types plus `row_for` plus `draw_device`).

## A new part (ICP-10111)
It has a parts row (parts.json L660) and no arm, so `validate` passes and `emit` refuses with
`UnknownPart`. With the table: a `Row` of the shape above is enough, no new code, provided its
features are among those nine. It would still need (outside this spike's scope, unchanged by the
table): its KiCad symbol vendored in `library.rs`; `KNOWN_PARTS`; and `ir::names::device_nets`
(`BME280`/`BMP581` `SDO`, W25Q `WP`/`HOLD`, ICM20948 `REGOUT`), which restates the strap and own-net
names the table now holds. That is the one real duplication the table would remove.

## Tests
- `cargo test -p aethelgard-eda --no-fail-fast`: 177 passed, 1 failed in the lib; adversarial_1300 (12),
  adversarial_1301 (2), own_parts (6), supply_libm (2): all pass. The one failure is
  `schematic::tests::the_arms_are_the_known_parts` (it scans for `match d.part.as_str() {` and finds
  none: "the I2C and the SPI device matches": 0 != 2). Expected, as the brief says. It would be replaced
  by a test that `ROWS` and `KNOWN_PARTS` agree (name, once each, kind).
- `cargo test -p aethelgard-cli --test parts_arms_tier1`: 2 passed, 0 failed. This is the test that
  actually draws every part, and it is the one that matters.
- `cargo clippy -p aethelgard-eda --all-targets -D warnings`: clean. `cargo fmt --check`: clean (after
  one import reorder).
- Not run: the rest of the workspace tests; no mutation testing.

## Recommendation
- Do it. The six arms are data plus the nine features above, and the sweep shows no part's bytes move.
- A row carries: part name; kind (I2C/SPI); the symbol lib_id; pin to role (supply, ground, a bus line,
  strap-to-supply with a name, address strap with its target list and whether it goes through a resistor,
  own net with its capacitor); the ordered passives; and the optional stand-in value and note.
- Where it lives: first in eda (as in this spike), with a table-agreement test replacing the source
  scan, so the symbol and the roles stay next to the library that vendors the symbol. It should move to
  the IR parts rows only if a second consumer needs it. The natural second consumer is
  `names::device_nets`, which today restates the strap and own-net names in the IR crate; the IR cannot
  depend on eda, so if the table is to be the single source for that, it must live in the IR (a
  `parts.json` section for the pin map, symbol id and passives), with the interpreter staying in eda. That
  is a schema change (spec/parts epoch) and a decision for the architect; this spike did not measure it.
- Order constraint: a row's pin list order is the draw order of straps and own nets, and so the R and C
  reference numbers. Re-ordering a row moves bytes. A reorder is a bytes change and needs the sweep.
- Cost not measured: whether `draw_device`'s roles are enough for the next part (a DRV8833-style driver is
  a different shape: `pin_devices` and bridges are not drawn by these arms).
