# AET-WO-1502 FEEDBACK

**Result:** built; no §7 stop. One PR, unmerged.

## What was built
- `crates/aethelgard-cli/src/said.rs`: `first_error(text, Style)` and `suffix`. `Style::Cargo` = first trimmed line starting `error:`; `Style::Renode` = first line containing `[ERROR]`. Output is printable ASCII (every other byte `?`), cut to 240 bytes plus `…`.
- `kani.rs`: the `disagree` arm and the exit-not-0-or-1 arm. `renode.rs`: the nonzero-exit arm and both unrecognised arms (CPU plus load error; neither). No verdict, exit code or recognised outcome changed.
- Tests: 8 helper unit tests; 7 tier-1 tests in `tools_tier1.rs` (one per call site, plus no-line and hostile-line); tier-2 `prove_of_an_unresolvable_dependency_names_the_package` in `with_tools.rs`.

## Gates
- `fmt`, `clippy -D warnings` (also with `--features with-tools`) and `cargo test --workspace` pass locally.
- **Tier 2 was not run here** (no Kani on Furnace; the architect said CI's `tools` job proves it). It builds and lints. Its message assertions are only `error:` plus the package name, since I could not see whether the sandboxed cargo fails on `no matching package` or on a registry-unreachable error. If the `tools` job fails on the package name, that is the finding.

## Decisions for the architect
- D1: "the unrecognised arms" of `renode.rs` read as both the "CPU and a load error" arm and the "no CPU, no error" arm. Both quote.
- D2: the helper scans stdout then stderr (the existing `Output::text()`), so a line from either stream can be quoted. That is what the tool said; nothing outside the captured output can be quoted.
- D3: Kani's `error[E0277]:` lines do not start with `error:` and are not quoted, as the WO says. `compile_error` is a recognised outcome and unchanged.
- D4: the tier-2 test builds its crate in `std::env::temp_dir()` rather than adding a fixture, because `container/fixtures` is outside the fence.

## Adversarial pass (targeted)
1. Envelope/terminal: a line with `"`, `\`, ESC, BEL, TAB, DEL, `é` and 1000 bytes: the quote is printable ASCII, 240 bytes plus `…`, and stdout stays one JSON line (`prove_quotes_a_hostile_line_as_short_printable_ascii`). A multibyte char straddling the cut, lone CR and CR LF are unit-tested. The `…` is the one non-ASCII character, and JSON carries it.
2. Wrong stream / unprinted line: `x error: y`, `error[E…]`, and `warning:` lines are not quoted; with no match nothing is appended (tier-1 `…says_nothing_extra` for both tools). Nothing the CLI itself writes is in the scanned text.
- Mutants, one each: cut at 241 (killed by the 240/241 unit test); the Renode nonzero arm dropping the suffix (killed by two tier-1 tests). Both reverted.

## Reduction pass
No removals: the diff adds one 40-line module and uses it in four places; nothing duplicated or unused.
