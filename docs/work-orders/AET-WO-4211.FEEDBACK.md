# AET-WO-4211 FEEDBACK — The W25Q64JV is NRND: its lifecycle cited, and every board that uses it acknowledges it

**Builder:** Claude (strong tier, Opus), worktree `~/aethelgard-4211`, branch `wo/aet-4211-flash-nrnd` from `main` `1178ca8`.
**Cadence:** build only, with a targeted adversarial pass.

Labels:
- **[read]**: read at source, in this repository at `1178ca8`, or in Scotty's evidence (`~/.cache/aet-ds/scotty-2026-11-16/`, read-only).
- **[measured]**: run on Furnace (host cargo under a capped user scope; containers under `~/bin/capped-run`) or on GitHub's runner.
- **[inferred]**: neither; reasoning from the two.

## 1. Predictions (committed before any measured run)

All **[inferred]** unless marked.

**The evidence [read].**
- `winbond-W25Q64JV-catalog-export-2026-11-16.csv` hashes `f10526aeaa7efbf7da5f358f1d1ac933275dcf1628546f136c9e39445ab42ac4`. Read with Python's `csv` reader (UTF-8 with BOM, CRLF): a header and 98 data rows, every one a `W25Q64JV` code. Column 15 is `Status-Industrial`. All 16 `W25Q64JV??IQ[G]` codes (the row's `xIQ` wildcard) read `N`, and so do the IM, JQ and JM codes (50 industrial codes in all). The 48 automotive codes read `-` there and `P` under `Status-Automotive`. `W25Q64JVSSIQ` is SOP 208 mil.
- The status image (`73aa6c75…`) shows the same table, with Industrial Status N for `W25Q64JVSSIQ` and `…IQG`. The longevity image (`7091ccdc…`) shows Longevity Y, Date Start 2016/03 for both.
- The legend "N = Not Recommended for New Design", and the WPLP meaning of Longevity Y, are in neither image nor the CSV. They come from the architect's README note. The row cites the legend as read through that note.
- DigiKey's "Active" screenshots are not in the evidence directory, so they are not held. The row notes the disagreement as Scotty's report.

**§1.2, `nrnd` against `4210` D1 [read].** The ruling as relayed (`~/.cache/aet-bus/4210.md` line 25; `AET-WO-4210.FEEDBACK.md` line 487) says:
- an `eol` or `nrnd` part must be acknowledged as `{part, status, statement}`;
- `validate` refuses an unacknowledged one by name;
- every verdict, every generated file's header and the fabrication package carry the banner;
- "a fab-intended specification is no longer refused outright: it is refused unless acknowledged".

The code treats `nrnd` exactly as `eol`:
- `validate_lifecycle` (`lib.rs` 2449) and `lifecycle_banner` (`lib.rs` 218) branch only on `Active`;
- the banner's wording differs: "not recommended for new designs" instead of "end of life";
- no fab-path rule names `Eol` (`verify.rs`, `fab.rs`); the one `Eol` refusal in `power.rs` 1617 reads `power-parts.json` rows, not `parts.json`.

So this is **not** a §7 STOP. The work order's line "EOL is refused for fabrication" describes the spike's rule, which Scotty's ruling withdrew (`4210` A2). That is a finding (F1), not a code difference.

**The entry's shape [read].** `accept_lifecycle` holds `{part, status, statement}` objects (`LifecycleAcceptance`, `lib.rs` 135), not the work order's `["W25Q64JVxIQ"]`. Each fixture gains `{"part": "W25Q64JVxIQ", "status": "nrnd", "statement": …}`. `sentinel-02-power.json` appends it beside its ICM20948 entry.

**The files [read].** 19 specifications place the part: 2 examples, 16 CLI fixtures (3 under `solve/`) and `aethelgard-ir/tests/fixtures/sentinel-02-power.json`. 21 more Rust sources name it, some building specifications in code. The two `topology.json` files are solved topologies and take no entry.

**What the change does:**
- **P1.** With the row at `nrnd` and no entry, `validate` on the reference exits 2 with `LifecycleNotAccepted`, id `U4`, the message naming `W25Q64JVxIQ`, `` `nrnd` `` and the CSV. With the entry it exits 0. An entry with `status: "eol"` gives `LifecycleStatusMismatch`.
- **P2.** Every refusal fixture (`invalid-zero-domain.json`, `solve/pigeonhole-spi.json`, `solve/thrash-pins.json`, `solve/stress.json` as each one is used) refuses as before, because the lifecycle step is validation's last (step 15).
- **P3.** `emit` on the reference writes `.kicad_sch`, `.kicad_pcb` and the BOM CSV. Each is byte-identical to the baseline except lines that carry the specification's sha256. **The KiCad files carry no banner:** the eda emitter has no lifecycle code. That gap against D1's "every generated file's header" applies to `eol` as well; F-B3/D-B2 made it unreachable, and the flash now reaches it (finding F2).
- **P4.** Every generated board-crate file with a header gains exactly one line, `LIFECYCLE NRND: W25Q64JVxIQ (U4) is not recommended for new designs by its vendor (…), acknowledged in accept_lifecycle`. That covers `board.rs`, `lib.rs`, `proofs.rs`, `Cargo.toml`, `firmware/src/main.rs`, `firmware/Cargo.toml`, `firmware/.cargo/config.toml`, `link.x` and `sentinel-01.repl`. Their specification-hash lines change as well, and nothing else does. This is D1 as ruled, not a board change. `Cargo.lock` is unchanged.
- **P5.** The firmware ELF's sha256 is unchanged: a comment line moves no code, and the build strips debug info. The one risk is a panic `Location` line number in `main.rs` below the header; I expect `main.rs` to embed none.
- **P6.** `simulate --spec`, `check speeds`, `extract`, `generate` and `verify` verdicts on the reference gain `lifecycle: [<that line>]`. `verify --fab-out` writes a package whose `FAB-NOTES.md` gains a `## Lifecycle` section with the NRND line. The `W25Q64JVxIQ … not held` line that `lifecycle_unheld` gives today goes, because the citation is held. Every boot verdict stays pass.
- **P7.** Tests that pin the reference's specification sha256 (`c1072b71…`), or that assert no banner on a board placing the flash, fail until re-pinned. Those include `own_models_tier1.rs` 71, `twin_tier2.rs` 3061 and `parts.rs` 72/580/713. No other assertion moves.
- **P8.** The quickstart job is not triggered by this PR (its paths are `README.md` and its own workflow). Its "as written" step runs the pinned image on the pinned specification, neither changed. After merge, the main-image step runs the new image on the changed reference. Its four checks (emit `ok`, ERC pass, DRC pass with no findings, `PinClaimedTwice` with the same claimants) pass, because the entry survives the double-claim edit and the refusal comes from the solver.

**Adversarial predictions (§5):**
- **A1.** `"part": "w25q64jvxiq"` (another case) does not match. The device is then unacknowledged (`LifecycleNotAccepted`, U4), checked before the entry is found unused.
- **A2.** The entry named twice gives `LifecycleAcceptedTwice`.
- **A3.** The entry on a board without the flash gives `LifecycleAcceptanceUnused`.
- **A4.** `"status": "NRND"` (upper case) gives `LifecycleStatusMismatch`.
- **A5.** A part absent from the table (`"part": "W25Q64JV"`) gives `LifecycleAcceptanceUnused`, or `LifecycleNotAccepted` first if the board places the flash.
