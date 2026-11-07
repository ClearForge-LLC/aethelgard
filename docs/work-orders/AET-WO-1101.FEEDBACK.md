# AET-WO-1101 — FEEDBACK

**Branch:** `wo/aet-1101-pin-table` · **Base:** `7b295f8` (PR #26) · **Builder:** Claude Code (Sonnet 5.5) as `forge` on Furnace · **Result:** §1 built (including §1.4a); every §3 item passes locally on Furnace; one item (§5.1's datasheet cross-check) could not be done, below.

Entry schema: `finding · where · type · recommendation · decision-needed`.

## What was built

- `aethelgard-ir`: `PinTable`, `Pin`, `PinSource` (`Serialize` only), plus `PinTable::validate` (`RUL-14` over keys and signal names; not in the WO, it reuses the private `identifier` rather than duplicating it). §1.4a: `SupplyMismatch` and step 5a, README row, two tests plus one ordering test.
- `aethelgard-svd-hal`: vendored files (all four sha256 verified at fetch: the two XML files match §1.1; `LICENSE` is `2e804790…`), `pin_table`, `PinTableError { UnknownPart, SourceHashMismatch, MalformedSource }` with `kind()`, README. `roxmltree 0.21.2` added to `[workspace.dependencies]`; `Cargo.lock` gains `roxmltree` (and its `memchr`, already present) and the svd-hal edges. The crate's re-export of `aethelgard_ir::*` is gone: it was the stub's only content.
- `workspace_shape.rs`: svd-hal graduated, one new test pinning its files, keys and dependencies.
- Data matches the architect's prototype exactly: 114 pins, no `MalformedSource` case, the eleven suffixed names reduce to unique canonical names. No §1.3 rule was contradicted.

## Gates

- Furnace: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`: pass. Determinism constant `80dcd2751a3a0036f7609c06929a33b6446885b5ac04a380f34afcf0f61d9e2c` (sha256 of `serde_json::to_string` of the table). Cross-checked by an independent Python implementation of §1.3 on the same files, which produced the same digest.
- CI (`gates`, `tools`, and the constant reproduced in `gates`): CI_PLACEHOLDER
- Tier 2 (`--features with-tools`) cannot run on Furnace, as in `AET-WO-1100`; this diff touches none of that code.
- Mutation (sized to the diff, one mutant per guard, against the svd-hal suite): hash check forced true, killed; signal-in-MCU-file filter off, killed; AF conflict check off, killed; `{1,2}` widened to `{1,3}`, killed; duplicate-position check off, killed. One survived, the "no third digit after the run" guard: `take_while` counts every digit, so it could never fire. **Removed** (reduction pass).
- Reduction pass (diff-sized): removed that dead guard and a redundant length test. Nothing else cut: each remaining extra refusal below has a test that fails without it.

## Additions beyond §1.3 (flagged, not silent)

All are refusals on malformed input, none touches valid data, each has a test. **Decision-needed: no**, unless the architect wants them out.

1. A pin name with a third digit (`PA123`) is refused, not read as `PA12`.
2. Two I/O pins with one `Position`, or a non-numeric one, are refused.
3. A `<GPIO_Pin>` name listed twice in the GPIO file is refused (else one would win silently).
4. A leading zero in the AF number (`GPIO_AF04_X`), an empty tail (`GPIO_AF4_`) and signs are refused as "any other form".
5. `<Mcu RefName>` must be `STM32H753ZITx` and `Package` must be present; the table's `package` comes from the file.
6. Several `PossibleValue` under one `GPIO_AF` are all read; equal numbers are fine, different ones conflict. The real data has none.

## Adversarial pass

| # | Finding | Severity | Type | Recommendation | Decision-needed |
|---|---|---|---|---|---|
| 1 | **Datasheet cross-check NOT DONE.** `st.com` timed out through WebFetch, and a curl retry was denied by the permission classifier (bot-check circumvention); I did not work around it. No datasheet revision is cited and no entry is claimed verified against one. Entries to check against the STM32H753 datasheet's alternate-function table: `PB8` I2C1_SCL AF4 (pos 139); `PB9` I2C1_SDA AF4; `PB6` I2C1_SCL AF4; `PF0` I2C2_SDA AF4; `PE2` SPI4_SCK AF5. Also in the table: `PB7` I2C1_SDA AF4, `PF1` I2C2_SCL AF4, `PB10` I2C2_SCL AF4, `PB11` I2C2_SDA AF4, `PE5` SPI4_MISO AF5, `PE6` SPI4_MOSI AF5. These come from ST's own database, so they are internally consistent with ST, not independently confirmed | medium (the one §5 item undone) | gap | A human or a session with datasheet access does the five checks and records the revision | yes |
| 2 | Hash check bypass: none found. Bytes hashed are the bytes parsed (one `include_bytes!` slice, no re-read). Flip, append and truncate of either file each refuse naming that file; the unmodified bytes still build. A collision is out of scope. The hash constants and the `SOURCE.md` table are separate copies of the same fact (the `SOURCE.md` agreement is not tested) | low | verified | none | no |
| 3 | Names: `PC2_C`/`PC3_C` reduce to `PC2`/`PC3`; the data holds no plain `PC2`/`PC3`, so no collision, but an `_C` pad is an analog-switch variant of the pad, not a different pin, so a later part with both would be refused as a duplicate, loudly. A name with no port letter (`BOOT0`, `PL1`, empty) is refused. Ports above `K` are refused (`PL1`), which would need a rule change for a larger package | low | by design | none now | no |
| 4 | The table has analog-only pins' AFs absent by rule (leaving out is not refusal); `PA13`/`PA14`/`PB3` carry their debug functions as AF0 if the data lists them, so a solver may assign a debug pin to I2C. Not this WO's to decide | low | note for `AET-WO-1200` | The solver should treat `PA13`/`PA14`/`PA15`/`PB3`/`PB4` as reserved unless told otherwise | yes (architect) |

## What was not built

The solver, other parts or packages, register blocks, the `.ioc` file, any CLI subcommand (§4).

## Rules in §1.3 the data contradicted

None.

## Promote-or-needs-none pass over `AET-WO-1100` FEEDBACK

- A4 (supply mismatch): ruled, built here as §1.4a.
- A7 (envelope `id` null for `InvalidIdentifier`): ruled at review of PR #25; needs none here.
- A2, A3, A5, A6: unchanged by this work; A5 still binds `AET-WO-1200` (compare pins against the table, not by string), and this table's canonical keys are what it compares against.
- The §7 push-route note (mint to file): this session used the documented `gh-write prep` + redirect route; no gap.
