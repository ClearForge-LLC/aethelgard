# AET-WO-2310 §1.2 — the proposed shapes, for the ruling

Nothing here is built. Each item is stated with the reading it rests on (file:line on `main` `15798f91`) and my
lean. The measurements behind (3) and (5) are in `results.md`.

## Trace 19 (one bump, `event.rs` `TRACE_VERSION` 18 → 19)

### Header: `[magic, 19, run_id, "ns", artifacts, tools]`, tools now triples

Today each tool is `[name, version]` (`event.rs` L308–317, L346–367; six items, so any change is a bump).

| | Today (18) | Proposed (19) |
|---|---|---|
| tool entry | `[name, version]` | `[name, version, sha256 \| null]` |
| `aethelgard` | `0.2.0` | `0.2.0`, null |
| `aethelgard-bridge` | its source digest | the same, null (its version already is a content hash) |
| `ngspice` | `44.2` | `44.2`, **sha256 of the libngspice file the bridge's loader mapped** |
| `renode` | its build string | the same, null (pinned by build string and the Dockerfile's `sha256sum -c`; a content hash is a later row if wanted) |
| a new `rust-toolchain` entry | absent | `rustc -vV`'s `release (commit)`, and a content digest: ruling question Q-T below |

- **Lean: a third item per tool**, not `sha256:` folded into the version text. It keeps `version` the tool's own
  words (replay reports them, `event.rs` L312).
- **The name rule changes with a `rust-toolchain` entry.** `check_archived` (`campaign.rs` L3803) requires the
  header's names to equal `tools_of(records)`, and no record names the toolchain. So `tools_of` gains
  `rust-toolchain` on every campaign run. Today `campaign` needs no toolchain on the host. On a host with none,
  lean: the entry is `[rust-toolchain, "absent", null]`, named and not refused, so a campaign of a foreign or
  prebuilt ELF still runs. The alternative is to refuse `ToolUnavailable` (exit 4), which adds a dependency
  `campaign` does not have today.
- **Where libngspice's hash is taken.** The bridge links `-l:libngspice.so.0` (`bridge/build.rs` L22) and runs
  sandboxed with an empty environment. Only the bridge knows which file its loader mapped. So the hash is best
  taken **in the bridge** (`/proc/self/maps`, the mapped path's bytes), and reported in its identity JSON beside
  the banner (`analog.rs` `identified`). **Fence:** `bridge/` and `analog.rs` are outside the WO's fence. Ruling
  needed (lean: admit `bridge/src/{main,solver}.rs` and `cli/src/analog.rs`'s `identified` for this item only).
  - The alternative, the CLI hashing `/usr/lib/x86_64-linux-gnu/libngspice.so.0` itself, hashes an assumed path,
    not what was loaded. It would pass the adversarial "a tool hash that does not change when the tool's bytes
    do" only by luck of `ld.so`'s search order.

### Q-T: what the toolchain's digest is of, and what it is bound to (measured, `results.md` (5))

Hashing `rustc` is the trap, under every choice. `PATH`'s `rustc` is rustup's 21 MB proxy (the same bytes as
`cargo`), and the real `bin/rustc` is a 648 KB stub. The compiler is `librustc_driver` (158 MB) and `libLLVM`
(208 MB).
- **T1 (lean):** the active sysroot (`rustc --print sysroot`, 173 files, 718 MB), digested as the sha256 of its
  sorted per-file sha256 list. That is `a362f61c…5d6c`, in 0.25–0.30 s. Every byte of compiler, std, target
  library and `rust-lld` is in it.
- **T2:** only what builds the firmware: `rustc`, the two libraries, `rust-lld` and
  `rustlib/thumbv7em-none-eabihf`. It is smaller, but a list of what counts is one more thing to get wrong.
- **T3:** rustup's provenance (`multirust-channel-manifest.toml`'s tarball hashes). It says where the tree came
  from, not what it is; at most an addition to T1.
- **Binding, under every choice:** `campaign` judges an ELF it did not build, so a campaign-time digest is the
  host's toolchain. **Lean:** archive it under that name, and say in the README that it is not proven to be the
  ELF's. Binding it would need `build` to record the digest and `campaign` to check it: `firmware.rs` and
  `verdict.rs`, outside the fence. That would be a backlog row.

### Refusal of 18 and older, on the 1215 pattern

There is no trace equivalent of `aethelgard-ir` `versions.rs` `CHANGES` (L56). An old archive is refused today as
`UnknownVersion` "unknown format version 14 in header" (`archive.rs` L378), with no list of changes.

- **Lean:** a `TRACE_CHANGES` table in `aethelgard-chaos` (one row per version from 2, built from the
  `TRACE_VERSION` doc comment's prose, which it replaces), and `UnknownVersion` for an older trace gains the rows
  after it, as 1215's `UnsupportedSchemaVersion { changes }` does. A newer version is refused as today (no rows).
- Exit status unchanged (3).

### Records that move

| Record | Today | Proposed |
|---|---|---|
| override 3 (`droop`, kind 5's scenario record) | `[3, domain, floor, start_ns, fall, hold, rise, source, load, bor_level]` | `[3, domain, floor, from_boot_word_ns (signed), fall, hold, rise, source, load]` |
| override 6 (`battery_sag`) | `[6, series_milliohms, start_ns]` | `[6, series_milliohms, from_boot_word_ns (signed)]` |
| kind 16 `RailDeclared` | carries the scenario's `Droop` | carries the **resolved** droop: absolute `start_ns`, and `bor_level` from `mcu.bor_level` |
| kind 31 `BatterySagDeclared` | carries `sag.start_ns` | carries the resolved absolute `start_ns` |
| kind 32 `RecoveryBound` | archived only when a reset is released | **the reference:** archived for every droop and every sag, `boot_word_ns` the undisturbed boot's |

Replay then checks, from the trace alone:
- kind 16's (kind 31's) `start_ns` = kind 32's `boot_word_ns` + the override's `from_boot_word_ns`, and inside the
  run (`DROOP_BOUNDS` / `battery::START_NS_MAX` move to the resolved instant);
- kind 16's `bor_level` is the level kind 10's brown-out reading declares (`brown_out.expected`, judged under
  `BOR_LEV`'s mask), so the BOR level has one source, the specification, and replay still has it.
  `check_droop_records` (`campaign.rs` L1630–1782) reads the level and the instant from kind 16 instead of the
  override.
- A droop or sag whose undisturbed boot wrote no boot word cannot be placed: the run is refused by name
  (`PlantedEventUnplaced`, lean) before the rail is solved, never planted at a default (`INV-06`).

### Verdict rule (7)

`check_run`: on a read boot, an `ok` kind 12 (21) whose own kind 13 (22) witness — same reference and step — is
unread requires `i2c_transaction_unwitnessed` (`spi_transaction_unwitnessed`) among the verdict's findings, as
`renode.rs` `witness_finding` (L4037–4044) names it. Today `check_read_with_the_boot` (L1489–1494) allows the
unread witness on a read boot and nothing ties the finding to it.

### A sleeping boot (4)

`judge_boot` (`renode.rs` L2815–2820) returns a tool error (exit 5) when the boot word is set and the core
executed fewer instructions than the run holds; `campaign.rs` L1048–1051 then aborts the campaign.
- **This overturns `2100`'s ruled A10** (`AET-WO-2100.FEEDBACK.md` L350: "`wfi` after the word | exit 5
  (ruled)"), as item (4) asks. Named so the ruling is explicit.
- **Lean:** that shape becomes the finding `cpu_halted` (the name the same sleep already gets without the boot
  word, `renode.rs` L2881–2896): one name for one fact, the core executed fewer than its time. **Replay binds
  `cpu_halted` nowhere today** (no reference in chaos `campaign.rs`). The new run rule: on a booted kind 10,
  `cpu_halted` is among the findings exactly when `instructions_observed < instructions_expected`. Kind 10
  already archives both counts.
- **Not moved:** the sibling at L2822–2828 (a line of ours lagging its stamp while the core executed every
  instruction) is two witnesses disagreeing, a tool fault, and stays exit 5. The adversarial pass will try to
  reach exit 5 from a sleep by that path.

## Scenario format 6

| Override | 5 and older | 6 |
|---|---|---|
| `droop` | `start_ns`, `bor_level` | `from_boot_word_ns` (signed ns, a whole number of 10 ns instructions), no `bor_level` |
| `battery_sag` | `start_ns` | `from_boot_word_ns` (signed) |
| everything else | unchanged | unchanged |

- **Lean: versions 1–5 still read** for every override that did not change. A `droop` or `battery_sag` in a
  version below 6 is refused by name (`PlantedEventNeedsVersion6`, lean), with what changed. Of the 41 committed
  scenarios (`cases.json` aside), 33 have neither and stay byte-identical.
- **Why signed** (results, sweep M): all eight committed planted events (six droops, two sags) are *before*
  their board's undisturbed boot word, by 2.6 to 71.6 ms. An unsigned "after the boot word" cannot express any of
  them without moving it.
- **The 8 that move:** the six droops and two sags, each by its measured offset (results' table). **No generator
  writes any committed scenario** (`INV-07`: authored apart from anything generated), so they are re-expressed by
  hand.
- **The anchor: a ruling question (Q-A).** A pre-boot event is brittle the other way relative to the boot word:
  a later boot word moves it later from reset, and a shorter boot can put sentinel-02-power's sag (−71.6 ms)
  before reset (refused by name, `PlantedEventBeforeReset`, lean).
  - (a) **Every planted event from the boot word, signed** (the WO's text). Today's instants are kept exactly, so
    no committed verdict should move (inferred, results P8). **My lean**, with the refusal above. Its cost
    (results P9): the bound checks on the instant that run before any boot today (`droop_end_ns`,
    `check_battery_sag`'s start bound, `solve_battery_sag`, and `verify`'s tool-less coverage stage) can check
    only the offset before the boot. The resolved instant is checked after it, so those refusals cost a Renode
    boot.
  - (b) The scenario names its anchor, `after_reset_ns` or `from_boot_word_ns`. Pre-boot events keep reset;
    post-boot ones use the boot word. More format for the same eight instants.
  - (c) Re-plant the droops where they were authored: 973,380 ns *after* the boot word, as 1 ms was when the
    minimal boot word was at 26,620 ns. That moves every droop into running firmware, with unmeasured verdicts:
    a §7 stop by construction. I would measure it first if you want it. (973,380 ns is the minimal board's
    figure; each board's would be measured.)
- **Q-G: format 6 removes none of the re-pins that ground item (3).** 4241 §0a's nine re-pins are firmware spin
  counts in `twin_tier2.rs` (outside the fence) and pinned expectations in `campaign_tier2.rs` (L705–712, L949, in
  the fence). None is a scenario field, and the eight scenario instants never needed re-pinning: 4241 moved the
  boot word past them without moving a verdict. Lean: build (a) for future post-boot plants. In
  `campaign_tier2.rs`, derive the pinned instants from kind 32's `boot_word_ns` plus each one's delta, in place of
  the literals (in the fence). Leave `twin_tier2.rs`'s firmware spins to a backlog row (spin until the boot word
  plus N), which needs a fence of its own.

## (2) `window`

Measured (sweep B): every campaign line's `window` is `"MAX_VIRTUAL_NS"`.
- The campaign line (`cli/src/campaign.rs` L938, L1234): in the fence. Removed.
- **The verdict's `boot.window`** (`verdict.rs` L296, set at `main.rs` L2378) is outside the fence, and `main.rs`
  is 1607's. And it is not always `MAX_VIRTUAL_NS`: a foreign boot's says `FOREIGN_MAX_VIRTUAL_NS`
  (`renode/foreign.rs` L970, asserted by `boot_tier1.rs` L2632). Ruling needed. Lean: remove the campaign line's
  only; the verdict's keeps telling a foreign boot's window from a generated one's.

## (6) Names

The five literals in `renode.rs` (L295 `cpu_aborted`, L304 `wall_bound_exceeded`, L311 `memory_cap_exceeded`,
L2447 `platform_load_error`, L2449 `firmware_load_error`; L178 and L188 also say `platform_load_error`) become
five constants in `aethelgard-chaos` `campaign.rs`, and `UNREAD_FINDINGS` is built from them, so the judge and
replay share one spelling. The foreign copies (`renode/foreign.rs` L1039, L1041) are outside the fence: lean,
leave them and list them in FEEDBACK.

## Q-F: fence additions this shape needs (read at source)

1. `cli/tests/twin_tier2.rs`: L2993 asserts the campaign line's `window`, and L2746 pins exit 5 for the sleeping
   boot (`m1_a_core_that_slept_and_wrote_its_boot_word_is_the_ruled_disagreement`). Items (2) and (4) cannot close
   without editing both. L2217 (the verdict's `boot.window`) moves only if (2)'s verdict half does.
2. `bridge/src` and `cli/src/analog.rs` `identified`, for libngspice's hash, taken where it is loaded.
3. `cli/src/analog.rs` L370/L484/L925 hand the instant to the bridge; it stays absolute (the resolved one), so
   no change is expected there. Named in case it needs to move.
4. Droop and sag scenarios authored outside the fence, which format 6 refuses unless they move:
   `cli/src/verify.rs` L2696, L2725, L2994 and L3137 (1607's file); `cli/tests/fab_tier1.rs`;
   `cli/tests/board_tier2.rs`.
5. `verdict.rs` / `main.rs`, only if the verdict's `window` goes too (see (2); my lean is no).

## (1) The droop's `bor_level`

Dropped from the scenario (format 6) and from override 3. `BorLevelContradictsSpecification`
(`cli/src/campaign.rs` L416–424) goes with it: there is nothing left to contradict. Kind 16 carries the resolved
level, checked against kind 10 (above).
