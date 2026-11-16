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

## 2. Measured against the predictions

Host runs used release CLIs built from `1178ca8` (`~/.cache/aet-4211/aethelgard-base`) and from this branch (`aethelgard-row`). Each ran `validate`, `solve`, `check speeds`, `emit` and `generate` on all 19 specifications (`~/.cache/aet-4211/tools/sweep.sh`), into `base-sweep/` and `new-sweep/`. Containers ran under `~/bin/capped-run`, one at a time, with `--network none`. Every read-back gave `memory.max=8589934592 memory.swap.max=0 pids.max=8192`.

| # | Result |
|---|---|
| Evidence | **Held [read]**, as in §1. |
| §1.2 | **Held [read]:** not a STOP. `nrnd` and `eol` take the same paths; only the banner's words differ. |
| P1 | **Held [measured].** With the row flipped and no entry, `validate` on the reference exits 2 with `LifecycleNotAccepted`, id `U4`, naming `` `nrnd` `` and the CSV by file name and column. With the entry it exits 0. `the_nrnd_flash_is_refused_unless_acknowledged_and_bannered_when_it_is` (`aethelgard-ir/tests/validate.rs`) holds both, `LifecycleStatusMismatch` for `eol`, and §5. |
| P2 | **Held [measured].** All 19 specifications give the same exit code for each of the five commands, before and after: `invalid-zero-domain` 2 everywhere; `pigeonhole-spi` and `thrash-pins` 0 at `validate`, then 2; `sentinel-02-own` 2 at `emit`/`generate` (no KiCad symbol for its own parts); the rest 0. |
| P3 | **Held, and sharper than predicted [measured].** All 19 specifications' `emit/` and `generate`'s `kicad/` (`.kicad_sch`, `.kicad_pcb`, `.bom.csv`) are byte-identical: the KiCad files carry no specification hash. The `emit` verdict changes only in its `specification_sha256`, and carries no banner (F2). |
| P4 | **Held [measured].** For each of the 15 specifications that generate, 9 board files differ: `Cargo.toml`, `firmware/.cargo/config.toml`, `firmware/Cargo.toml`, `firmware/link.x`, `firmware/src/main.rs`, `src/board.rs`, `src/lib.rs`, `src/proofs.rs` and the `.repl`. `~/.cache/aet-4211/tools/classify.py` sorted every diff hunk: each is a specification-hash line or one inserted `LIFECYCLE NRND: W25Q64JVxIQ (U4) …` line, with 0 other hunks. `Cargo.lock` and `firmware/Cargo.lock` are unchanged. |
| P5 | **Held [measured].** `aethelgard build` in the tools image on the base and branch `boards/sentinel-01` gives the same ELF, sha256 `430e48ab7a92755b3759d8f9dbaeca60cd163f2b42a7573dc0bc80743bb58de8`. |
| P6 | **Held on the host [measured]:** `validate`, `check speeds` and `generate` verdicts gain `lifecycle` with the one NRND line. The `verify --fab-out` side (`FAB-NOTES.md`'s `## Lifecycle` section with the NRND line and no `not held` line) is asserted in `verify_2k2_tier2.rs` and measured by CI's tier-2 run (§4). |
| P7 | **Held, wider than predicted [measured].** 93 host tests failed on the first run, in 14 targets. Most were specifications built in test code (the power fixture's four `base()` copies, `valid()`, `solve.rs`'s `spec()`) that cleared or lacked the entry, and three that remove the flash and so must drop the entry. The rest: the parts table's pinned sha256, the pending list, the solved digests, `validate`'s verdict, the svd-hal digests and UI files, and `own_models_tier1`'s no-banner assertion. |
| P8 | **Held [read + measured].** Its triggers are `README.md` and its own workflow, so this PR does not run it. Its "as written" step runs the pinned image `59e9f0aa…` on the pinned specification `10806528…/examples/sentinel-01-minimal.json` (`03a82a04…`); neither moves. After merge, `engine-image.yml` calls it with the new image, and `test this commit's image and example` swaps in this commit's reference and its sha256. **Measured:** the block's four checks, run in the pinned tools image with this branch's binary on the changed reference, print `quickstart ok` (`~/.cache/aet-4211/qs/out.log`). `emit` is `ok`; ERC and DRC pass with `findings: []`; `solve` on the double claim exits 2 with `PinClaimedTwice` and the same claimants. The KiCad hashes are the baseline's. **The pin is not moved.** |

**Every re-pinned literal moves only by the specification hash or the banner [measured]:**
- `solve_cli.rs`: with the old specification sha256 put back, the reference's solved output hashes to the old `5d64214d…d4fb` and the stress fixture's to the old `15dd2336…10c8`, exactly.
- `order_2203.rs`: with each text's `LIFECYCLE NRND` line removed and the old sha256 put back, the three are the old `c5003f4f…`, `875c5440…` and `63b028f0…`, exactly; the boot image did not move.
- `serial_ports.rs` (hash zeroed by the test): 8 of 10 files moved, each by its banner line, as P4 classified; the two lockfiles did not.
- `ui/motors-pwm/topology.json`: only its `specification_sha256` differs (`diff` after substituting both hashes).
- `aethelgard-ir/tests/parts.rs`: `PARTS_SHA256` moves with the row (`6429c287…` to `052d2afe…`, after A-C1's wording).

## 3. Findings and questions for the architect

- **F1 · The work order's premise "EOL is refused for fabrication" is the spike's rule, withdrawn [read].** Scotty's ruling on D1 (`~/.cache/aet-bus/4210.md` line 25) says "a fab-intended specification is no longer refused outright: it is refused unless acknowledged". `4210` A2 records `EolPartForFabrication` as superseded, and no fab-path rule names `Eol` today. The roadmap's 2026-11-15 changelog row still says "EOL refused for fabrication" (protected; not touched). The code matches the ruling, and `nrnd` is treated exactly as `eol`, so §1.2's STOP did not fire.
- **F2 · D1 says every verdict and every generated file's header carries the banner. Some do not [measured, read].** `validate`, `check speeds`, `generate`, `extract`, `simulate`, `campaign`, `verify` (and `FAB-NOTES.md`) and the board crate's and platform's headers carry it. These do not:
  - the `emit` verdict and the `solve` topology [measured];
  - the KiCad files, `.kicad_sch`, `.kicad_pcb` and `.bom.csv`, whose U4 note names the stand-in symbol, not NRND [measured];
  - the `check drc --spec`, `check support --spec`, `prove --spec` and `check pinout` verdicts [read; adversary A-B1].

  The README quickstart runs `emit` and `check drc --spec` on the reference. The gap is the same for `eol`: `sentinel-02-power.json`, with its acknowledged EOL ICM-20948, emits KiCad files with no banner on `main` today. That also makes `4210` D-B2/F-B3's "no board with an `eol`/`nrnd` part can be drawn" stale. The work order requires byte-identical boards, and eda is outside this fence, so nothing was changed. **Question:** should the KiCad files and the `emit` verdict carry the banner (eda and CLI, a small work order)?
- **F3 · The entry's shape [read].** The work order writes `accept_lifecycle: ["W25Q64JVxIQ"]`. The type (`LifecycleAcceptance`, `4210` D1) is `{part, status, statement}`, so each specification carries `{"part": "W25Q64JVxIQ", "status": "nrnd", "statement": "The board's SPI flash, NRND by its maker (Winbond's catalog, Status-Industrial N, 2026-11-16) with longevity support listed; kept until AET-WO-4500 picks a flash in mass production (AET-WO-4211)"}`.
- **F4 · What the citation could not hold [read].**
  - The catalog legend "N = Not Recommended for New Design" and the WPLP meaning of Longevity Y are in neither the CSV nor the two images. The row states them as the architect's README relays them.
  - DigiKey's screenshots are not in the evidence directory, so the row says "(not held)".
  - The CSV has 98 rows: 50 industrial codes, all `N`, and 48 automotive codes, `-` there and `P` under Status-Automotive. The 16 `W25Q64JV??IQ[G]` codes the row's `xIQ` covers are all `N`.
- **F5 · Files outside the fence's letter, each forced by the change:**
  - `boards/sentinel-01/**`, regenerated with `generate`, never by hand; the regeneration test compares it byte for byte;
  - `crates/aethelgard-svd-hal/tests/ui/{motors-pwm,serial-four}/board.rs` and `motors-pwm/topology.json`, blessed with `AETHELGARD_BLESS=1`;
  - `crates/aethelgard-cli/README.md`, its example line's hash prefix, which `the_readme_example_line_names_the_real_version_and_hash` reads.

  No source file under `src/` changed. No version number of any kind was taken.
- **F6 · Grok row 10, the Q64 package and pinout against `1300` D2 [read].** W25Q64JV Rev J (`452d906c…`, text `~/.cache/aet-4300c/ds/w25q64jv_revj.txt`): §3.1 Figure 1a, p. 5, "8-pin SOIC 208-mil (Package Code SS)". §3.3's pin table gives 1 /CS, 2 DO (IO1), 3 /WP (IO2), 4 GND, 5 DI (IO0), 6 CLK, 7 /HOLD or /RESET (IO3), 8 VCC. That equals the emitter's map (`schematic.rs` 862-874) and KiCad's `Memory_Flash:W25Q32JVSS` symbol pin for pin (`Package_SO:SOIC-8_5.3x5.3mm_P1.27mm`). The CSV gives `W25Q64JVSSIQ` as SOP 208 mil. §10.1's dimension table (p. 66) is an image the text extraction does not hold, so the body size is **not** read. The BOM note's text is in eda (`schematic.rs` 980), and changing it moves the emitted BOM, so the citation goes here. **Proposal:** the note cites "W25Q64JV Rev J §3.1/§3.3 p. 5" when eda next changes.
- **F7 · Reduction finding, the two `Lifecycle` enums (Grok row 14) [read].** `parts::Lifecycle {Active, Nrnd, Eol}` and `power_parts::Lifecycle {Active, Eol, Unread}` differ in their variants and meaning:
  - `Unread` is a power part's "no page states it", noted and allowed;
  - a power part's `Eol` is refused outright, with no acknowledgement path (`power.rs` 1617);
  - `Nrnd` does not exist for power parts.

  One enum would make each parser accept a state it refuses today, unless new refusals were added. That is not a deletion, and both enums live under `src/`, outside this fence. **Proposal:** fold them when power parts gain `accept_lifecycle` (the same D1 rule), which would remove `Unread` or turn it into a held "pending" citation.
- **F8 · Reduction finding: one fixture transform in four copies [read].** "The power fixture without U5, its bus, its constraints and its acknowledgement" exists four times:
  - `aethelgard-ir/tests/power.rs`, `sense_pin.rs` and `validate.rs`'s `full_document()`;
  - `aethelgard-cli/tests/common/power.rs`.

  This work order had to edit all four. The ir tests could share one `tests/common` module; the CLI's copy is in another crate.
- **F9 · `lifecycle_unheld` now has no case [read].** No row in `parts.json` is `pending` after this change, so `verify`'s `lifecycle_unheld` is always empty on real data. It stays as the guard for the next pending citation (the parser still accepts `pending`, and `a_pending_citation_must_be_an_https_url_and_is_listed` covers it). Reported, not removed.

## 4. Adversarial pass (targeted; a fresh subagent, Opus: judgement over gate logic)

The adversary was given the work order, the diff `1178ca8..84c0b33`, the lifecycle code and the ruling's text, not the builder's reasoning. It worked read-only on this worktree, with its scratch in `~/.cache/aet-4211/adv/`. It used the base and branch release CLIs, and no container. The builder checked each finding it adopted at the code.

| id | severity | finding | disposition |
|---|---|---|---|
| A-B1 | medium (architect's call) | The banner is missing from `emit`, `solve`, `check drc --spec`, `check support --spec`, `prove --spec` and `check pinout` verdicts, and from the KiCad files. This work order makes them reachable with an `nrnd` part on the reference. | **F2.** The same holds for `eol`, so it is no `nrnd`-versus-`eol` difference and no §7 STOP. The fix is in eda and the CLI, outside this fence. |
| A-C1 | low | The citation claimed more than its evidence. The legend and WPLP are only in the architect's README. "Longevity Y" is shown only for SSIQ/SSIQG. The status image went uncited, and the longevity image was cited without saying what it shows. | **Fixed:** the section now reads "column 15 Status-Industrial = N for all 16 xIQ codes (legend N = Not Recommended for New Design, relayed: not in the export); SSIQ screenshots 73aa6c75 (N) and 7091ccdc (Longevity Y since 2016/03); DigiKey: Active (not held)" (224 of 256 bytes). The CSV's own sha256 is the citation's `sha256`; the banner prints `document`, `revision` and `section` only. Boards, UI files and digests were re-taken; §2's classification re-ran with 0 unexpected hunks. |
| A-R1 | low | Two roadmap rows route to 4211: the Q64 pinout citation and the two enums. | Answered in **F6** and **F7**. |
| A-D1 | low | The CLI README still said `lifecycle_unheld` names the W25Q64JV today. | **Fixed**, `crates/aethelgard-cli/README.md`. |
| A-D2 | low | `lifecycle_unheld` can no longer fire on real data. The golden `FAB-NOTES` unit test in `cli/src/fab.rs` (984-1000) passes `lifecycle: &[]` with the comment "no lifecycle line", but `verify` now writes one for those boards. | **F9**; the `fab.rs` comment is in `src/`, outside the fence. The containment is asserted by tier-2 `verify_2k2_tier2` (this branch adds the NRND line and "no `not held`"). |
| A-D3 | info | Three topology literals still carry the old reference sha256 `c1072b71…`: `cli/src/renode.rs` 3236, `twin/src/tests.rs` 9, `twin/tests/spi_platform.rs` 14. No test compares them with a file. | Left. They are solved topologies the twin reads, not the specification, and two are under `src/`. |
| A-X1 | info | The old binary refuses the new reference (`LifecycleAcceptanceUnused`); the new binary refuses the old reference (`LifecycleNotAccepted`, U4). | **For the phase close:** the quickstart's image pin and specification pin must move together. Today they are a pair (`59e9f0aa…`, `10806528…`), so "as written" is unaffected (P8). |

**Attacks that held**, by the adversary's measurement unless marked read:
- §5 on `validate`:
  - the part in another case (lower or upper), with a trailing space, a Cyrillic look-alike or a zero-width space → `LifecycleNotAccepted U4`;
  - the entry twice → `LifecycleAcceptedTwice`;
  - a part the board does not place (BME280, ICM20948, W25Q128JV, bare W25Q64JV) → `LifecycleAcceptanceUnused`.
- `status` as `NRND`, `Nrnd`, `eol`, `nrnd ` or a look-alike → `LifecycleStatusMismatch U4`.
- A statement of 512 bytes passes. 513 bytes, empty, spaces only, a tab or non-ASCII → `LifecycleStatementInvalid`.
- An extra key, a duplicate key or a missing field → `SchemaViolation`.
- The unacknowledged reference is refused before anything is written by `solve`, `emit`, `generate`, `check speeds`, `extract`, `check drc --spec`, `check support --spec` and `verify`. `prove`, `simulate`, `campaign` and `propose` go through the same `validate` [read].
- `nrnd` versus `eol`: the banner's wording is the only branch [read].
- All 55 tracked specification files on both binaries: every one placing the flash passes with exactly one banner line (two for `sentinel-02-power`); `invalid-zero-domain` still fails with `ZeroNominalVoltage`.
- CI steps that run the reference (`ci.yml` 47-52 and 116, `engine-smoke.sh`, `ardupilot-image.yml` 223) expect exit 0 or 2, not exact text.
- Regenerated files and the `solve_cli` re-pins confirmed independently. No assertion was weakened: each "no LIFECYCLE" became an exact count plus a prefix check.
