# AET-WO-1322 — FEEDBACK: eda's reduction and reader hardening

**Branch:** `wo/aet-1322-eda-hardening` from `main` `7fbe5d11`. **Builder:** Claude (Opus 5.5 orchestrating), with two Sonnet subagents for read-only inventories (guards, echo sites) and one fresh Opus subagent for the targeted adversarial pass. Each report was spot-checked against the code before use. Written 2026-11-23.

**Status: READY** once CI is green on the last push (§7). **The answer:**
- **(1) Guards.** `validate` runs in `place()` (`emit`, `emit_moved`, `placement`), not in `design()`. So `Drawing::of`, `decoupling_picofarads` and the `extract`/`loops`/`supply` paths still reach every guard that `validate` shadows.
  - Those guards are **labelled** `belt-and-suspenders:`, each with a test that reaches it directly.
  - Only `crossing`'s two lookups were dead, by code order. They are folded into one `device_on`, shared with `behind`, or made `Internal` with the reason stated.
- **(2) `prepare_read`** holds each label once in a shared chain, and renders it only into a refusal.
  - On a 10 kB name over 100 000 pads, `main` grows the peak by 983 MB; the branch by under 2 MB.
  - `tests/prepare_read_memory.rs` holds it.
- **(3) The reader's echo** is bounded to 200 bytes of `{:?}`, cut on a character boundary, with the rest counted. That is `strict::echo` and `strict::shown`, in the strict reader, `board.rs` and `route::read`.
  - A 14 MiB reference went from 29.4 MB on stdout to 697 B.
  - After the adversarial pass, a chain of carriers is also bounded whole.
  - `MAX_ITEMS` counts courtyard edges and footprint keepouts. The largest committed board holds 4 151 of 20 000.
- **(4)** The IPROPI text names the `GND` tie that `validate` admits, and where.
- **(5) Already on `main`:** `4300i` (`ab9d4663`) made eda call `aethelgard_ir::names::references`, which is `pub`. The IR is untouched.
- **(6)** `1319`'s M2 is restored. It now lowers its `DECOUPLERS` count with the row, builds, and is **killed by bytes**. `check` refuses a part with two `DECOUPLERS` entries.
- **(7)** `PadEscapeExhausted`'s `kind`, `id` and message are tested, through the router-to-emitter mapping (`pcb::unroutable`).
- **Every committed specification's emitted bytes are identical** (sweep 280 files, `d4diff` 277 cases, `bulk.sh`). The one difference in `d1319` is the refusal text that (4) rewrites, recorded by its `d1-ipropi-undeclared` case. **19 of 19 mutants are killed.**

## 0. How it was made

- **Predictions first:** `spike/1322/PREDICTIONS.md` at `24ab9d5c`, before any run. Results are beside them in `spike/1322/RESULTS.md`, each prediction marked hit or missed.
- **Bus:** at the start I posted that (5) was already built and how (1) would go ("from 1322", architect inbox). No reply was needed.
- **Kit:**
  - `1319`'s `run.sh`/`compare.sh` (the null sweep and both harnesses), unchanged; `1321`'s `bulk.sh`;
  - `spike/1322/count.py` (what the item cap counts, on every board);
  - `spike/1322/mutants.sh` (19 mutants);
  - `spike/1319/harness/mutants.sh`, with M2 restored and the mutants taken as arguments.
  - Base and branch are each a `git archive` with its own target dir. Evidence is in `~/.cache/aet-1322/`.

## 1. What was built

- **(2), `board.rs`:** `Within { label, footprint, parent: Option<Rc<Within>> }`. `name()` gives an item's own label, and `render()` joins the chain only for a refusal.
  - `admitted_keepout` judges where the zone sits: the nearest named carrier is a footprint with nothing named above it. It no longer tests `starts_with("footprint ")` and `contains(" of ")` on the text.
  - **That changes one verdict, on purpose:** a keepout under a footprint whose reference contains " of " was refused by the old string test and is admitted now.
  - The adversary's differential (3 770 boards, depth ≤ 3) found that change and no other.
  - Test: `a_footprints_keepout_is_judged_by_where_it_sits`.
- **(3), `strict.rs`:**
  - `MAX_ECHO_BYTES` = 200.
  - `echo(&dyn Debug)` writes through a capped writer that keeps a prefix and counts the rest, so nothing long is built. Its output is `head… (the first N of M bytes as escaped)`.
  - `shown(&str)` returns the text as written when it is at most 200 bytes and `char::escape_debug` changes nothing in it; otherwise it returns `echo`. Every existing id and message (`R7`, `pad 1 of footprint R7`, `F.Cu`) is unchanged.
  - Where they are applied:
    - in `Refusal`: `FieldTwice.name`, `Quoted.word`, `Number.text`, `Version.found` and `Layer.name`;
    - in `board.rs`: the reference, the pad number, the zone's net and the net class, which are also the ids;
    - in `route/read.rs`: a part's reference and pad numbers (`named`), the layer, shape and flag atoms, and the zone's name and layer;
    - also there, the lists `{copper:?}`, `{layers:?}` and `{zones:?}`, bounded whole.
  - `board::MAX_CHAIN_BYTES` (800) bounds a rendered chain, after which the remaining carriers are counted (adversarial B1).
- **(3), `route/read.rs`:** one count, `items + Σ(courtyard edges + keepouts) ≤ MAX_ITEMS`, after the parts are read. The cap's test moves its accepted case by the footprint's four edges and gains a keepout case.
- **(1), `schematic_power.rs`, `schematic.rs`:**
  - `device_on` (labelled) is called by `behind` and `crossing`.
  - In `crossing`, the bus lookup and the second translator check became `Internal`, each with its reason.
  - `lines` is labelled.
  - `strapped`'s `UnsupportedAddress` is labelled; its direct test, `a_strap_follows_the_parts_tables_addresses`, already existed.
  - `design`'s name re-checks are labelled:
    - `#` and the duplicate reference are reached through `Drawing::of` by `cli/tests/hot_plug_tier1.rs`;
    - **no test reaches the `GND` and battery-net claims** (F3).
  - New test: `a_crossing_is_refused_without_one_device_or_a_translator` (0 and 2 devices, a capacitor row).
- **(4), `schematic_arms.rs`:** the message is `current_output_undeclared(name)`, held by `an_undeclared_current_output_names_the_tie_validate_admits`.
- **(6):**
  - `check()`'s reverse loop refuses a part with two entries. Test: `a_part_with_two_decoupler_entries_is_refused` (another count, the same count, apart).
  - `spike/1319/harness/mutants.sh` M2 also replaces `("BMP581", 2)` with `1`.
- **(7), `pcb.rs`:** `unroutable(route::Unroutable) -> EmitError`, the closure as it was. Test: `a_run_with_no_escape_keeps_its_shape`. The JSON envelope (`kind`, `id`, `message`) is the CLI's. eda's contract is those three accessors, and they are what is tested.

## 2. Measurement and acceptance

- **Bytes, base `7fbe5d11` against the branch's emitting code at `29d137c0`:**
  - `identical: 280 files`;
  - `d4diff identical: 277 cases, 926 files`;
  - `bulk.sh`: both identical;
  - `d1319 DIFFERS: 1 entries, 1 cases`, which is `d1-ipropi-undeclared/summary.txt`, `Drawing::of`'s refusal text (4). No drawn file moved.
- **The final head** (`cf299fbc` and after) changes only refusal paths: the chain bound and the folded count. It is re-swept against `29d137c0`'s output in §7.
- **(2)'s memory** is in RESULTS §(2) and under Mutants. The test is sized so that a regression costs about 1.5 GB (§6, the 45.6 GB run).
- **(3)'s echo and count** are in RESULTS §(3).

## 3. Mutants (sized to the diff)

`spike/1322/mutants.sh`: **19 of 19 are killed, each by its named test failing.**
- B1 survived its first run: the echo bound had shrunk each label, so the flat test no longer saw a copy per child. The test now nests ten named footprints, and B1 is killed.
- `1319`'s restored M2 builds (run exit 0) and is killed by bytes, against base and against the branch.

## 4. Decisions for the architect

None stop the work order. Two choices, both inside the fence:
- **D1 (1):** I labelled the guards and did not move `validate` into `design()`. That move would make them truly unreachable for about 6 ms a call, but it would change what `Drawing::of` returns, and `cli/tests` (outside the fence) depend on that.
- **D2 (2):** the keepout's verdict for a reference containing " of " changed from refused to admitted (§1). It is the rule as written, not a new refusal.

## 5. Adversarial pass (targeted, a fresh Opus subagent, frozen at `02c2afd6`)

| id | severity | finding | disposition |
|---|---|---|---|
| B1 | low–medium | The echo bound was per label: 250 nested footprints with 10 kB references wrote 130 153 B (base 5 007 153) | **fixed** at `cf299fbc` (`MAX_CHAIN_BYTES`). Test `a_deep_chain_of_carriers_is_bounded_whole`; mutant B4 killed |
| B2 | info, pre-existing | A strict reader `path` echoes up to `MAX_PATH_BYTES` (256 × 74) of file keywords, 34 kB on a 250-deep unknown-keyword file, base and branch alike | **named** (F4) |
| B3 | cosmetic | The cut can split an escape (`\…`, `\u{2…`). It is still a prefix of `{:?}` and one line | **named**: the marker says it was cut |
| B4 | cosmetic | `shown` passes spaces raw, so a name can imitate the cut marker or an ` of ` chain. Nothing parses these strings | **named** |
| A1 | low, pre-existing | Without `validate`, `Drawing::of` draws two translators on one bus, both onto the same B-side nets | **named** (F5): a library caller's input that `validate` refuses (`CrossingInvalid`) |
| A2 | low, pre-existing | `design`'s SPI loop refuses a device on an undeclared bus as `Internal`, not by name. The new comment in `crossing` relies on it | **named** (F5) |

**What held** (the adversary's measurements):
- No input reached the two new `Internal`s: 13 specifications and 8 topology cases, each identical to base.
- `echo` was checked over every scalar value, every prefix length from 0 to 208, and as `str`, `Option`, tuple and `Vec`:
  - no panic;
  - no raw control, line-separator or bidi character;
  - always a prefix of `{:?}`;
  - the count exact.
- The single-level CLI bounds were 326–658 B, against base's 5–20 kB.
- A 20 001-edge courtyard is refused.

## 6. Findings (rows for §7.1)

- **F1 · The judges still echo file text whole.**
  - `route::check`/`connectivity` (`{what}` is a reference and pad number), `parity`, `fab` (drill lines, CPL references), `support`'s netlist and `extract`'s messages are outside "the reader" this work order bounded.
  - Each takes text the strict reader already read, bounded only by the 16 MiB input cap.
  - **Proposed row:** "every eda refusal quotes file text through `strict::shown`/`echo`".
- **F2 · `MAX_ITEMS` bounds a sum; the cost is a product.**
  - `check` scans vias × courtyard edges and keepouts × items. Under the summed cap of 20 000, each product is at most 10⁸, below the pair scan's 2 × 10⁸, so the 0.9 s measurement still bounds it. That is inferred, not timed.
  - A separate product cap would be exact.
- **F3 · No test reaches `design`'s `GND` and battery-net reserved-name claims** directly (they are labelled). Reaching them needs a specification, which eda's tests cannot build: eda has no `serde_json` or pin-table dev-dependency.
  - **Proposed row:** "eda's tests build a specification (a dev-dependency), or the CLI's `hot_plug_tier1` gains the two cases".
- **F4 · Strict paths are bounded at 256 × 74 B**, not at the echo bound (adversary B2). This is pre-existing.
- **F5 · A library caller that skips `validate`** draws what `validate` refuses (adversary A1: two translators on one bus) and gets `Internal` for an undeclared bus (A2). Both are pre-existing; D1 is the general answer.
- **F6 · Gaps the guard inventory reported, not verified here:**
  - an emitter-numbered `Y` (crystal) colliding with a specification's `y1` (`schematic.rs` ~726, `pcb.rs` ~1770);
  - two own nets that fold equal (~749);
  - `unconnected-` on connector, test-point and pin-device nets (`pcb.rs` ~426).
- **F7 · A stale doc comment, outside the fence:** `cli/tests/pin_devices_drawn_tier1.rs`'s `an_undeclared_nfault_is_open_and_an_undeclared_ipropi_is_refused` still says IPROPI tied to GND "cannot be declared".
- **F8 · Process:** an uncapped run of the memory test's first nested form against `main`'s `board.rs` grew the peak by **45.6 GB** on Furnace before failing. It is my mistake. Old code now runs only under `systemd-run --user --scope -p MemoryMax=4G -p MemorySwapMax=0`, and the test is sized so a regression costs about 1.5 GB.

## 7. Gates, fence, reduction pass

- **Gates:** see below (filled at the last push).
- **Fence:**
  - `crates/aethelgard-eda` (`board.rs`, `strict.rs`, `route.rs` tests, `route/read.rs`, `schematic.rs`, `schematic_arms.rs`, `schematic_power.rs`, `pcb.rs`, `tests/prepare_read_memory.rs`);
  - `spike/1319/harness/mutants.sh`;
  - `spike/1322/` and this file.
  - No IR, CLI, generator, twin, chaos, workflow or `ci/` file changed.
- **Reduction pass (diff):** −5 net in the fold · 2 duplicates removed (`crossing`'s device lookup is `device_on`; the two item-cap checks are one) · 0 dead paths removed beyond those · 4 protected kept (the new `belt-and-suspenders:` labels) · findings F1, F2 above.
