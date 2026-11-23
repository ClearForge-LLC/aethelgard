# AET-WO-3308 §1.2: the stop, with the list's source and the row table

Stopped at the `b9d5f4b6` base. The measurement is in this directory:
- `PREDICTIONS.md`, committed before the runs;
- `ordered-hashes.txt`;
- `ROWS.md`, the row table, with `rows-candidates.txt` the mechanical line list;
- `test3-draft.patch` and `test3-draft.out`;
- `results.md`, the predictions against the runs.

## The list's source

**The ten files are byte-identical at `ac5f8c82` (their creation, 2026-11-17 09:00Z), `ee835bde` (the last commit before the order ended, 2026-11-19 19:43Z), `cd79024c` and `b9d5f4b6`.** No commit after `ac5f8c82` touched any of them (`git log ac5f8c82~1..b9d5f4b6` on the ten paths lists only `ac5f8c82`). They equal `MANIFEST.json`'s hashes. A9's premise holds, and §7's "hashes differ" stop does not fire.

- **Q-ZIP (asked on the bus, 11:44Z):** does Scotty hold the uploaded zip? If he does, its members' hashes are the source, and a member that differs from the range is a stop.
- **Lean, if he does not:** the header names the range `ac5f8c82..ee835bde` and the order date 2026-11-19, says the bytes held across it, and says no upload was hashed.

## Decisions (each with a lean)

- **D1. `ORDERED.sha256`'s form.**
  - **Lean:** `sha256sum` lines with paths relative to the list's directory (`package/sentinel-01-bench-F_Cu.gtl`), so `cd fab/sentinel-01-bench && sha256sum -c ORDERED.sha256` checks it by hand. The header is `#` lines above the entries, as the WO words it, and the ruling block is appended below.
  - Whether GNU `sha256sum -c` (the image's) ignores the `#` lines is recorded in `results.md`.
  - The test parses the list itself, so it refuses CRLF, a duplicate, a malformed entry, and a `#` line among the entries other than the header and ruling blocks.
- **D2. (2)'s "files with a fabrication extension".**
  - **Lean: a closed name set.** `package/`'s files are exactly the ten listed plus a named set of the rest (`MANIFEST.json`, `FAB-NOTES.md`, the four CSVs, the ELF). Any other file, whatever its extension, fails, naming it.
  - That catches a renamed layer, a new `.gbr`, a `.drl` split in two (`-PTH`/`-NPTH`), and a layer under an extension a pattern did not foresee.
  - The cost: a new non-fab file in `package/` (a new CSV) edits the test's named set, never `ORDERED.sha256`.
  - **The alternative** is an extension pattern (`.g??`, `.gm?`, `.gbr`, `.drl`, `.xln`, `.exc`, `.nc`). A name it does not foresee passes.
- **D3. Test (3)'s token rule.** `BENCH.md` holds a hex token of 8 or more characters, not `0x`-prefixed, only as follows:
  - exactly one 64-character token, MANIFEST's ELF;
  - and any abbreviated token (`…`) one of a named set of non-ELF digests: today `78212e91…b789` (SNOSD82D, L97) and `452d906c…` (the `parts.json` row, L259), each with its name in the test.
  - **It therefore fails on:** a one-character-off hash; a prefix only, which leaves no full token; a second full hash anywhere, table rows included; and any other abbreviated hash, which is how the sheet names ELFs today. Each failure names the line.
  - **Measured on today's `BENCH.md`:** the draft (`test3-draft.patch`) fails with `full hashes found: []`, listing every abbreviated token: `008a2a59…` (L5), `f3e871bb…` (L92), `78212e91…` (L97), `e4e24372…` (L224) and `452d906c…` (L259) (`test3-draft.out`).
    - The draft does not yet set the two non-ELF digests aside; the named set is this decision's lean.
    - The WO predicted "fails, naming `008a2a59`". It fails, but because no 64-hex hash exists to mismatch.
- **D4. The S rows** (`ROWS.md`, class S). Each is text scoped to `008a2a59` or `e4e24372`, and **no predicted value fails** against `f3e871bb`.
  - **Lean:** the sheet names one ELF, and the S rows become `f3e871bb`'s:
    - L5 the full hash;
    - L79, L105, L111–112 eight words;
    - L92–97's table joins the records table, and its "not the ordered one" aside goes;
    - L125–126: the U3 read follows; I2C1 is addressed again at `0x48`;
    - L224 drops `e4e24372` and "Flash that ELF";
    - L260: after record 2;
    - L387 drops the ordered-ELF clause;
    - L401's VOS3 bullet goes.
  - **"The ordered ELF"** (the phrase, and the "U2 empty" heading) becomes "the package ELF".
  - **The one that is close to "a changed prediction":** B9 part 1's "exactly these lines, once" (L364–371) is the ordered ELF's. For `f3e871bb`, L99 already states the second transaction (`Address write: 48`, `NACK`, `Stop` with U3 empty; the `ACK` sequence with U3 fitted).
    - **Lean:** lift L99's sentence into the pass line unchanged. That restates a prediction the sheet already makes for this ELF, and creates none. **The architect rules whether that counts as a change.**
- **D5. The ELF-independent rows** (class I) are listed, not re-run.
- **F1 (reported, not a stop).** A local `verify --against-fab` re-derives 16 of the 17 package files byte for byte, the ELF `f3e871bb` among them.
  - `MANIFEST.json` differs only in `verdict_sha256`: the verdict line differs from CI's in a field I could not identify (`results.md`).
  - No `fab/` byte would move. For FEEDBACK's backlog: a machine-dependent verdict line means no one off CI can check the manifest.

**Stop conditions not met:**
- No predicted RAM word or register value fails to hold (class D: none).
- No `fab/` byte moved: this stop commits only `spike/3308/`.
