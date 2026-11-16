# AET-WO-3201 FEEDBACK — evolve, replay and the judge

**Branch:** `wo/aet-3201-evolve` · **Base:** `8030fa5` (`origin/main`) · **Builder:** Claude Code (Opus 5.5) as `forge` on Furnace · **Cadence:** build, with one measured stop (§1.1).

Labels: **measured** (run on this branch), **read** (read at source, file:line at `8030fa5`), **inferred**.

## §1.1 — where each item lives and what it costs (committed before any measured run)

**No item needs a trace-version bump** (read). So §7's first stop does not fire, and the build goes on.

- **Severity (B3).** It is archived in the CBOR, not just the verdict JSON: kind 14 carries `[kind, severity, count]` per finding, and `severity` is a text field (`crates/aethelgard-chaos/src/record.rs:368`, encode at `:1121`). Today's read checks it only as 1 to 64 printable bytes (`record.rs:1942`). The judge writes two values in a campaign verdict, `error` and `info` (`crates/aethelgard-cli/src/verdict.rs:699`, `:710`). KiCad's `warning` (`kicad.rs:182`) never reaches a campaign. A closed set `{error, info}`, checked in `Record::check` (which runs on write and on read), changes no byte and no encoding. **No bump.**
- **B1 and B2: committed replay hashes.** The state hash is the fold's (`replay.rs`). The joins are judged afterwards, in `campaign::check_run` → `check_joins` (`campaign.rs:735`, `:1911`), which reads the records and adds no byte. A new refusal there cannot move any hash (read). The golden stream (`chaos/tests/golden.rs`) holds no campaign record, so `check_run` never sees it (read).
- **B1 and B2: seeded answers.** The literal rules would refuse honest runs, so **two of the work order's wordings need an exception, which I build and name here**:
  - **B1, held low.** A held line has no kind 7 or 8 *by design*. The analog pass leaves it out of the decks (`cli/src/analog.rs:160-164`), and its fault is a kind 9 `from: [held_low]` naming that net (`:184-200`). "Each kind-15 line requires its kind 7 or 8" would refuse `held-low-sda` and `held-low-scl`, both E1 seeded cases: a §7 stop. **Built instead:** each kind-15 line has exactly one of a kind 7, a kind 8, or a held-low kind 9 naming that net. This is what the campaign writes (read), and no seeded case moves (predicted).
  - **B2, an unread boot.** A boot killed at a bound is honestly unread, and its verdict holds exactly one of `cpu_aborted`, `wall_bound_exceeded` or `memory_cap_exceeded`, **never** `boot_word_missing` (`cli/src/main.rs:1563-1567`, `renode.rs:263-290`). "No read and no `boot_word_missing` is refused" would refuse every honest killed boot. **Built instead:**
    - a boot that read nothing must name the bound it was killed at;
    - a boot that was read names `boot_word_missing` exactly once when its word is not the expected one, and never otherwise (the judge, `renode.rs:2142`).

    No seeded case is killed at a bound, and every seeded case's verdict already obeys this (predicted).
- **F-B10.** A finding's kind is text in kind 14, so a new name needs **no new trace kind**: only the judge, evolve's table and replay (read).
- **F15** changes only the verdict of an unbooted run with a timed-out record. No committed archive or hash holds one, and no seeded case is one: `no-boot-word` reads its device before skipping the word (`3100` F15). So no hash moves (predicted).

### Predictions (before measurement)

| # | Prediction | Basis |
|---|---|---|
| P1 | Every seeded case keeps its manifest's domain, rule, evidence and masked, index for index. | No new record in any run; F15 touches unbooted runs with timeouts only |
| P2 | `held-low-sda` and `held-low-scl` replay under the new B1 rule (their held line is bound by its kind 9) | read, as above |
| P3 | The golden trace and state hashes are unchanged | read |
| P4 | Some chaos and evolve test fixtures hold a kind 15 with no rise for some line, or a boot with no matching findings. They need the missing record added, and no assertion weakened | inferred |
| P5 | The 47 kΩ proposal is byte-identical: the E1 path never builds a `Field`, and `facts` reads the same 400 kHz from `i2c_speeds` | read |
| P6 | **Revised before any measurement** (the advisor's review): a held-low DShot run (`4300a` P12) would name **I3**, never the new rule. The held line faults its bus, the missing boot word then decides `I3` at board scope, and `I3` sorts below any appended rule. Only a read that is late on a *healthy* bus lets the new rule name the run: a firmware-side delay on the DShot board, as a mutant. Predicted for that case: findings `boot_word_missing`, `identity_read_outside_window`, and possibly `register_mismatch` (outputs not yet started); rule = the new one; evidence = the unset kind 12 | inferred |
| P9 | Renaming F-B10's data mismatch moves no seeded case. The reference board's parts (BME280, W25Q64JV, TMP117) have no script, so every mismatch is an identity step's (`ir/data/parts.json`). One tier-2 assertion names a *data* step's mismatch as `{bus}_identity_mismatch`: `twin_tier2.rs:3092-3093`, `4220`'s no-wait test. It moves to the new name | read |
| P10 | An unset record is archived as status `invalid`, with the judge's fill as its value. `status_name` has no arm for the fill (`renode.rs:2734`), and `rearm` fills both words (`:609-622`). So the archive cannot tell a record never written from one the firmware filled with garbage, and no new status word is needed: the rule selects `invalid` records in an unbooted run on a short (DShot) window, with an exact count, which fails closed | read |
| P7 | `cap-390p-sda-2k2` × `leave-firmware` is `I3` with evidence `[6,7] [7,9]`: the healthy SCL line's rise is not evidence. The one-bus M1 mutant (board-scope `I3` plus every rise) names `[5,7]` too, so only this case kills it | inferred from `2205`'s candidate table (`4:7 5:9`, shifted by two since `4300b`) |
| P8 | `TIER2_MIN` is unchanged: the new seeded cases ride the existing seeded tier-2 tests, with no new tier-2 test function | read |

## Fence: where the items live (read; flagged, not a scope change)

- **B1–B3 are not in `replay.rs`.**
  - B1 and B2 live in `chaos/src/campaign.rs`: `check_run` and `check_joins`, the rules `replay` and `attribute` apply after the fold.
  - B3 lives in `chaos/src/record.rs`, in `Record::check` for kind 14.
  - `replay.rs` is the fold and the hash, which this work order must not move. I edit `campaign.rs` and `record.rs` only where these items need it.
- **F-B10 changes `identity_finding` in `renode.rs`.** That is the booted path, outside "only the no-boot-word path", and §1.5 requires it. The change is one expression, the finding's name.
- **Overlaps with `4230` (PR #161), told by inbox:**
  - `check_run` after the self-loop check;
  - `PER_EVENT`'s length in evolve;
  - `propose::facts`: 4230 adds the domain maximum on the same lines where I take the speed from `i2c_speeds`;
  - `judge_boot`: 4230 inserts directly above "The identity reads". My F15 change touches only the `else` arm, through a helper.
