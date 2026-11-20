# AET-WO-1103 FEEDBACK: no board's name in engine code, held by a test that can fail

**Branch** `wo/aet-1103-no-board-names` from `main` 52a2d62e. **Test:** `crates/aethelgard-cli/tests/board_names.rs` (tier 1, 4 tests, 0.16 s; std only, no new dependency, no weight row). **Fix:** `crates/aethelgard-cli/src/ledger.rs` `GATE_SPI` (the one line and its doc comment).

## 0. Outcome

- The test **fails on `main`'s tree at `ledger.rs:388` and nowhere else**, and passes on the branch (§3).
- All three §2 mutants are killed, and three more that guard the selection and the two adversarial fixes (§4).
- No §7 condition fired:
  - the scanner finds **one** string hit in non-test code, the known one;
  - the `#[cfg(test)]` skip needs no parser dependency (§2.3).
- **Decision needed (F1):** the fix changes the six SPI enable rows' citation text in every `verify` verdict. That moves the verdict line, so it moves `fab/sentinel-01-bench/package/MANIFEST.json`'s `verdict_sha256`, and CI's `bench-package` (`verify --against-fab`) is predicted to fail until the package is rewritten. The package is outside this order's fence (§6).

## 1. A board name, from the repository (§1.1)

Derived by `names()`; nothing is a hand list.

- **Source:** each directory under `boards/` and each file stem under `examples/`, lower-cased, with `_` read as `-`.
- **A stem names a board** when one of its `-` segments is all digits.
  - The **board** is the segments up to and including that number.
  - The **family** is the segments before it.
- **On `main`:**

  | Stem | Board | Note |
  |---|---|---|
  | `boards/sentinel-01`, `examples/sentinel-01` | `sentinel-01` | |
  | `boards/sentinel-01-bench`, `sentinel-01-ext-bmp581`, `sentinel-01-minimal` | `sentinel-01` | variants |
  | `sentinel-02-bench` | `sentinel-02` | Sentinel-02 has no stem of its own; the variant names it |
  | `invalid-zero-domain` | — | **excluded:** no number. It is `validate`'s refusal fixture and names no board (its `project_name` is `sentinel-01`, which the rule does not read) |

  So the families are `{sentinel}` and the boards are `{sentinel-01, sentinel-02}`.
- **The match** is a family, then `-`, `_` or nothing, then one or more digits, case-insensitive (`family_hits`).
  - It covers `Sentinel-01`, `SENTINEL_02`, `sentinel01` and `sentinel_01_board`.
  - It also covers **Sentinel-03**, whose specification is a test fixture with no directory or example. It shares the family.
  - **A board of a new family** is caught only once it has a directory under `boards/` or a stem under `examples/`. A bare `sentinel` is not a name: `renode.rs` has 16 plain-sense uses (4600's "plain sense" class), and `the_names_come_from_boards_and_examples` asserts they do not match.
  - **Not matched:** a space as separator (`Sentinel 01`), a spelled-out number, an escape inside a string (`"sentin\u{65}l-01"`).

## 2. The test (§1.2, §1.3)

### 2.1 Selection

- **Arms:** every `.rs` file under `crates/<member>/src/**`, `bridge/src/**` and `models/**` (`select`, one function for the real scan, the floor and the plant).
- **`*` is one path component.** Git's `crates/*/src/**` also reached `crates/aethelgard-cli/tests/fixtures/forge-log/src/main.rs`; this selection does not. The planted tree pins both:
  - `crates/widget/tests/t.rs` is not selected;
  - `crates/widget/fixtures/x/src/y.rs` is not selected.
- **On `main`:** 98 + 8 + 1 = 107 files, of which 3 are test-only modules (`twin/src/tests.rs`, `svd-hal/src/tests.rs`, `bridge/src/header.rs`); 113,596 lines read.

### 2.2 What is a hit

| Class | Allowed | What |
|---|---|---|
| `Code` | **no** | anything not below: string, byte, raw and C string literals, identifiers, attribute values (`#[doc = "…"]` included) |
| `Comment` | yes | `//` and `/* */` (nested) |
| `Doc` | yes | `///`, `//!`, `/** */`, `/*! */` |
| `TestItem` | yes | inside an inline `#[cfg(test)]` item |
| `TestFile` | yes | a file that is a `#[cfg(test)]` module |

- **Why doc comments are allowed:** they carry the same citations as line comments (where a value was measured, which board a doc example came from). They are not compiled into anything a board's artifacts read; `firmware.rs:140` is a doc example of a name's spelling. An explicit `#[doc = "…"]` attribute is a hit, as the WO asks of attribute values.
- **The failure** names file, line and the matched name, e.g. `crates/aethelgard-cli/src/ledger.rs:388: \`sentinel-01\``. Matches repeat on one line are reported once.

### 2.3 How `#[cfg(test)]` is skipped, and what it does not handle

A lexer of its own (`lex`, std only) gives every token with its byte range: comments, all string forms (raw with any `#` count), char literals as against lifetimes, raw identifiers. It panics on an unterminated string or comment, so a misread fails loudly. `test_only` then reads the token stream without comments.

- **Which attribute:** exactly `cfg(test)`. Everything else is scanned, so the skip can only be narrower than what is really test-only:
  - `cfg(any(test, feature = …))` — the §5 case; the planted tree's line 15 is a code hit;
  - `cfg(all(test, …))`, `cfg(not(test))`, `cfg_attr(test, …)`, a bare `#[test]`.
- **Where an item ends:** at the first `;` or `,` at its own depth, after a `{…}` group at its own depth closes, or before the close of the group around it (all of `()[]{}` count). So a region never runs past its item:
  - the planted `#[cfg(test)] let v = …;` inside a function ends at its `;`, and the next statement is a code hit (line 19);
  - a `"}"` inside a test module's string does not end the module (strings are lexed).
- **Out of line:** `#[cfg(test)] mod name;` at a file's top level removes `name.rs`, `name/mod.rs` and everything under `name/`, by Rust's module-path rule (`lib.rs`, `main.rs`, `mod.rs` are mod-rs files), or the file a `#[path = "…"]` names.
- **Not handled** (each over-scans, so it fails loudly rather than hiding a name):
  - a `,` in generics or closure arguments at the item's depth ends the region early;
  - `#[cfg(test)] mod b;` inside an inline module is not resolved;
  - `#![cfg(test)]` is honoured only before a file's first item.
- **Under-scans found by the adversarial pass, both closed** (§5): a `#[cfg(test)]` generic parameter, and a file one root declares `#[cfg(test)]` and another as code.
- **Why not `syn`:** it is already a dev-dependency (`workspace_shape.rs`), but it drops `//` and `/* */` comments (§1.5 counts them by crate), and its spans carry no line numbers without proc-macro2's `span-locations`, which is a manifest change outside the fence. The adversarial pass used `syn` instead as a cross-check (§5).

## 3. Both runs (§2)

- **On `main`'s tree** (the test commit `bd43528e`, before the fix, and again with the final test and `main`'s `ledger.rs`, `~/.cache/aet-1103/run-final-on-main-ledger.txt`):
  ```
  test no_board_name_in_engine_code ... FAILED
  a board's name in engine code (roadmap §6: capability enters through the specification; move it into a comment or a test):
  crates/aethelgard-cli/src/ledger.rs:388: `sentinel-01`
  test result: FAILED. 3 passed; 1 failed
  ```
- **On the branch:** `test result: ok. 4 passed; 0 failed` (0.16 s).

## 4. Mutants (§2), on the final test

| # | Mutation | Fails |
|---|---|---|
| M1 | the crates arm read literally as `crates/*/src` (no expansion) | the floor test (`crates/*/src selects 0 files, below its floor 90`) and the planted test |
| M2 | the `#[cfg(test)]` skip removed (`is_cfg_test` false) | the main test, on 27 test-only lines (first `cli/src/campaign.rs:1722`), and the planted test |
| M3 | `pub const PLANTED_1103: &str = "Sentinel_02 only";` appended to `ir/src/lib.rs` | the main test: `crates/aethelgard-ir/src/lib.rs:4572: \`sentinel-02\`` |
| M4 | the walk not recursive | the floor test (`models selects 0 files`) and the planted test |
| M5 | the generic-parameter `>` stop removed | the planted test |
| M6 | a non-test declaration no longer keeps a file scanned | the planted test |

M4–M6 go beyond §2: each guards a stop or a selection rule that a weaker test would let pass. Logs: `~/.cache/aet-1103/mut/final/`.

**The floors:** measured on `main` 52a2d62e as 98 files under `crates/*/src`, 8 under `bridge/src` and 1 under `models`. The floors are 90, 7 and 1, a little below each count, so an ordinary deletion passes while an arm that stops matching fails.

## 5. Adversarial pass (targeted)

A fresh subagent was given the code, not my conclusions, and probed it in its own scratch crate (`~/.cache/aet-1103/adv/src/`). I spot-checked both under-scans by planting them; each failed the planted test before its fix (M5, M6).

- **The lexer:** compared with `proc_macro2`'s tokenizer on 6,613 `.rs` files (this repo plus `~/.cargo/registry`). Every literal range and every delimiter outside comments matched, and nothing panicked. The comment/doc boundary was not cross-checked: `proc_macro2` has no comment tokens.
- **The skip, compared with `syn`:** `test_only`'s ranges against `syn`'s `cfg(test)` item spans over the 107 real files. They differ only where a preceding doc comment sits before the attribute (`eda/src/stack.rs:514`, a doc of a `#[cfg(test)]` fn, is therefore classed `Doc`; allowed either way). 74 hand-made shapes (let-else, match arms with and without commas, fields, variants, `use {}`, closures, macros, labels) swallowed no following code.
- **Closed, under-scans:**
  - **A1:** `fn f<U, #[cfg(test)] T>() -> &str { "sentinel-01" }`. `>` was not a stop, so the region ran to the body's `}`. Now a `>` (not `->` or `=>`) that closes more `<` than the region opened ends it. Planted, x.rs line 23.
  - **A2:** `lib.rs` has `mod shared;` while `main.rs` has `#[cfg(test)] mod shared;`. `shared.rs` was exempt, and `aethelgard-cli` has both roots. Now a file any non-test top-level `mod` declares stays scanned. Planted, `crates/gadget`. `#[path]` before `#[cfg(test)]` is now read too.
- **§5's three spellings:**

  | Spelling | Caught? |
  |---|---|
  | a constant built by concatenation (`concat!("sentinel", "-01")`, `"sentinel-".to_owned() + "01"`, `format!("sentinel-{n:02}")`, `stringify!(sentinel - 01)`, a `\`-continued string) | **no**; the test reads one token's text |
  | `include_str!` of a board path (`include_str!("../../boards/sentinel-01/x.repl")`) | **yes**, the path is a string literal. The included file's **content** is not scanned, and a path built with `concat!` is not caught |
  | a name in a `build.rs` | `bridge/build.rs` **no** (outside the arms; it has no board name today). A member's `build.rs` cannot exist (`workspace_shape`'s `no_member_has_a_build_script`). `models/**/build.rs` would be selected |

- **Not scanned (F2):**
  - `crates/*/examples/*.rs` (`sandbox_probe.rs`);
  - the C# models `models/renode/*.cs`, which are engine code (no board name in them today);
  - data files code includes (`crates/*/data/*.json` and others; none names a board today).
- **`cfg(any(test, feature = …))`** is scanned, not skipped (planted, x.rs line 15).
- **Over-scan, accepted:** `cfg(all(test, …))`, `cfg(test,)` and a bare `#[test]` fn are reported as code. They fail loudly; none is in the tree.

## 6. The fix, and what moves (§1.4)

- **`ledger.rs:388`.** `GATE_SPI`'s text ended `(aethelgard-twin platform, boards/sentinel-01/sentinel-01.repl)`. It now ends `(aethelgard-twin platform, each solved SPI's block in the emitted .repl)`. The measurement is kept in its doc comment: `Measured on \`boards/sentinel-01/sentinel-01.repl\` (L15, \`SPI1\`), the platform \`aethelgard-twin\` emits`.
- **The ledger table, before and after:** a temporary test dumped `table()`, and that test was not committed. The diff is 12 lines, the `source` and `note` of the six SPI enable rows (`SPI1EN`–`SPI6EN`). With the two citations normalised they are identical, and nothing else in the table moved (`~/.cache/aet-1103/table.diff`). The ledger's own tests pass, and `cited()` still holds: the row's `source` keeps `STM32H7_RCC.cs L<n>`.
- **What else moves (F1):** a `verify` verdict carries each ledger row's `source` and `note`. The last measured bench verdict, `spike/3303b/out/after/sentinel-01-bench/verify.json`, holds the old text. So:
  - every verdict's bytes change in those rows;
  - `fab/sentinel-01-bench/package/MANIFEST.json`'s `verdict_sha256` (`20dfbb36…`) is predicted to move, and CI's `bench-package` (`verify --against-fab`) to fail until the package is written again.
  - Nothing else committed holds the text: a grep of the tree outside `spike/` finds it only in `ledger.rs`.
  - `verify_2k2_tier2`'s manifest check is self-consistent (a fresh run against its own package), and `bench_board_tier1` passes.

  **The package is outside my fence.** The order's literal "nothing else moves" cannot hold for a citation that is emitted. Proposed: the usual CI-written package (`ci.yml` `write=true` on this branch), committed byte for byte, with the fence widened to `fab/sentinel-01-bench/package/`. **2304b (#254) also rewrites that MANIFEST**, so whichever merges second takes a fresh package.

## 7. The count, for roadmap §6 (§1.5)

Roadmap §6 says "Measured 2026-11-15: outside tests and comments, no crate names Sentinel." Measured from this test, with "a name" being `sentinel` + number:

| | before the fix | after |
|---|---|---|
| files selected | 107 (98 + 8 + 1), 3 of them test-only modules | 107 |
| lines read | 113,594 | 113,596 (+2 doc lines) |
| hits in code (forbidden) | **1** (`cli/src/ledger.rs:388`) | **0** |
| hits in comments | 1 | 1 |
| hits in doc comments | 6 | 7 |
| hits in `#[cfg(test)]` items | 33 | 33 |
| hits in `#[cfg(test)]` module files | 1 | 1 |

**Allowed comment hits, by crate (after):**
- `aethelgard-cli`: doc `ledger.rs:388` (`sentinel-01`, the moved citation).
- `aethelgard-eda`:
  - comment `pcb.rs:672` (`sentinel-02`);
  - docs `pcb.rs:897` and `route.rs:137` (`sentinel-03`), `stack.rs:514` (`sentinel-01`, doc of a test fn).
- `aethelgard-ir`: docs `connectors.rs:66` and `ds009.rs:12` (`sentinel-02`).
- `aethelgard-svd-hal`: doc `board/firmware.rs:140` (`sentinel-01`, the doc example).

Test-only hits by crate: cli 27, ir 3, svd-hal 2, pincheck 1, twin 1 (`tests.rs`). bridge and models: none.

**This agrees with 4600's 62.** 4600's "sentinel" grep less its 18 plain-sense uses and 2 manifests is 42, the same as this test's 1 + 1 + 6 + 33 + 1. The full list, with each hit's class, prints with `cargo test -p aethelgard-cli --test board_names -- --nocapture`.

So after this order the §6 sentence is true as measured by a test, with two qualifiers worth wording in:
- "comments" includes doc comments;
- the test covers `.rs` files only (F2).

## 8. Findings

| # | Finding | Where | Kind | Proposed | Needs a ruling |
|---|---|---|---|---|---|
| F1 | The fix moves every verdict's SPI enable rows' text, so the committed bench package's `verdict_sha256` and CI's `bench-package` comparison move (predicted; CI on this PR measures it) | `fab/sentinel-01-bench/package/MANIFEST.json`, `.github/workflows/ci.yml` `bench-package` | decision | widen the fence to the package; dispatch `write=true`; commit the CI-written package | **yes** |
| F2 | The selection is `.rs` under three arms. Not scanned: `crates/*/examples/*.rs`, `bridge/build.rs`, the C# models `models/renode/*.cs`, and data files code includes. None names a board today | `board_names.rs` `ARMS` | note | a later order widens the arms to examples and build scripts, and reads `.cs` and included data as text with no comment rule | no |
| F3 | A name built at run time or by `concat!` (§5) passes. Review is the only check | — | note | none; a token test cannot see it | no |
| F4 | A new board family is caught only once it has a directory under `boards/` or a stem under `examples/`. Sentinel-03 is caught only because it shares Sentinel's family | `names()` | note | none | no |

## 9. Gates

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean on the final head.
  - My first final-gate script piped clippy through `tail`, so a lint failure from the adversarial commit did not set the exit code (two collapsible `if`s).
  - Re-run with its exit status read directly, then fixed in the reduction commit.
- `cargo test --workspace`: 138 binaries, 2,306 passed, 0 failed, on `ba211de8` (capped at 32G). After the reduction (test file only), `board_names` passed and clippy was clean; the full run is repeated on the head that is pushed (§10).
- **No tier-2 test added:** `TIER2_MIN` does not move. No weight row: the new binary runs in 0.16 s.
- **Reduction pass (diff):** −13 net (the lexer's `b"…"`, `c"…"` and `b'…'` branches duplicated the string and char branches; a byte char before a comment and a byte string are planted) · 1 duplicate removed · 0 dead paths removed · 0 protected kept · 0 findings below.

## 10. State

- Commits on `52a2d62e`:
  - `bd43528e` the test;
  - `d7157542` the fix;
  - `ba211de8` adversarial;
  - `a1d4ae24` reduction;
  - then this FEEDBACK.
- `main` had not moved at the last fetch.
- Evidence: `~/.cache/aet-1103/`, including the runs, the table dumps and diff, `mut/final/`, `adv/` and `gates-*.txt`.
