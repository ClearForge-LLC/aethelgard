# AET-WO-1306 — FEEDBACK

**Branch:** `wo/aet-1306-strict-reader` · **Base:** `origin/main` 58cbd53 (`AET-WO-1305`, with `AET-WO-1601`) · **Builder:** Claude Code (Opus 5.5) as `forge` on Furnace · **Cadence:** build-only, strong tier, full adversarial pass by a fresh subagent.

The work order itself (`docs/work-orders/AET-WO-1306.md`) lands through docs PR #100 and is not in this branch.

Entry schema: `finding · where · type · recommendation · decision-needed`.

## Decision needed first (F1): §3's "every board in the corpus is refused by `check drc`" contradicts "It is NOT a change to any verdict on an input that KiCad and the gate already read alike"

**What is built:** every board the gates read differently from KiCad is now refused by every gate that reads it. That covers the repeated-child class; 1306's new classes (a field named twice, a misspelt mandatory field, a quoted or missing keyword, an escape KiCad decodes otherwise, and schematic's `private` flag); and the schematic counterparts. The rest of 1305's 31 boards are refused by `extract` and `verify`, as before. `check drc` gives them KiCad's verdict, because KiCad and the DRC gate read them alike.

- **Of 1305's 31 refused boards, `check drc` (and `--spec --schematic`) now refuses 11 with exit 3:** the ten repeated-child boards, and `zone_clearance` as before (`LocalClearance`). Of the rest:
  - 14 fail by KiCad's own judgement (exit 1);
  - 2 are `ToolError` (KiCad will not load a layer named `TOP`);
  - 4 pass, with and without parity: `gr_rect_fcu`, `gr_text_copper`, `stackup`, `zone_fill_close`;
  - 2 pass plain `check drc` and fail parity: `cap_refvalue_only`, `controller_renamed`.
- **Why the 4 + 2 are not refused:**
  - They are not reader disagreements. KiCad reads the copper rectangle, the text, the parts and the renamed reference exactly as the gate does. The gate removes the stackup with the setup block, and DRC refills the zone.
  - `extract` refuses them because its bound does not model that copper, or because only the specification's design says what R1 or U1 is. `check drc` is not given the design.
  - Refusing them in `check drc` would change its verdict on inputs that KiCad and the gate read alike, which the work order's "It is NOT" excludes. Refusing an unknown top-level item would also change existing `check drc` inputs: `board.rs`'s `gr_text` test, the root-level atoms of `sockets_tier1.rs`, and the committed `drc-clean.kicad_pcb`.
- **`verify` refuses all 31.** Its board step is `check drc --spec --schematic` followed by `extract`, the same subcommands this table runs.
- **Recommendation:** read §3's corpus line as "every board KiCad and a gate read differently". Under that reading all its acceptance holds. Alternatively, rule that `check drc` takes on `extract`'s copper model (a closed world), which is a different work order.
- **Not a §7 stop:** no existing fixture or reference is refused (§7.1), KiCad's reading of every construct refused is cited below (§7.2), and the fence needed no change outside it (§7.3).

### The table, measured in the pinned image (`c69ff9bc…`): main 58cbd53 → this branch

`exit verdict-or-kind`. **Bold** marks a changed outcome; every other cell is identical. `drc-parity` is `check drc --spec examples/sentinel-01-minimal.json --schematic <reference schematic>`, as `verify` runs it; a schematic is judged with the reference board.

| kind | class | file | `check drc` (board) / `check erc` (schematic) | `drc-parity` | `extract` |
|---|---|---|---|---|---|
| board | connectivity | `island_same_name` | 1 fail | 1 fail | 3 NetLabelsDisagree |
| board | connectivity | `segment_empty_net` | 1 fail | 1 fail | 3 NetLabelsDisagree |
| board | connectivity | `segment_no_net` | 1 fail | 1 fail | 3 BoardUnreadable |
| board | control | `user_layer_table_only` | 0 pass | 0 pass | 0 pass |
| board | coordinate | `huge_coord` | 1 fail | 1 fail | 3 BoardTooLarge |
| board | escape | `fp_rect_layer_octal` | 1 fail → **3 MalformedBoard** | 1 fail → **3 MalformedBoard** | 0 pass → **3 BoardUnreadable** |
| board | escape | `pintype_hex_no_connect` | 0 pass → **3 MalformedBoard** | 0 pass → **3 MalformedBoard** | 3 UnmodelledCopper → **3 BoardUnreadable** |
| board | escape | `pintype_octal_no_connect` | 0 pass → **3 MalformedBoard** | 0 pass → **3 MalformedBoard** | 3 UnmodelledCopper → **3 BoardUnreadable** |
| board | escape | `reference_field_octal` | 0 pass → **3 MalformedBoard** | 1 fail → **3 MalformedBoard** | 0 pass → **3 BoardUnreadable** |
| board | escape | `shared_unconnected_octal` | 1 fail → **3 MalformedBoard** | 1 fail → **3 MalformedBoard** | 0 pass → **3 BoardUnreadable** |
| board | field-twice | `fp_text_reference` | 0 pass → **3 MalformedBoard** | 1 fail → **3 MalformedBoard** | 0 pass → **3 MalformedBoard** |
| board | field-twice | `fp_text_value` | 0 pass → **3 MalformedBoard** | 1 fail → **3 MalformedBoard** | 0 pass → **3 MalformedBoard** |
| board | keyword | `pad_quoted_size` | 3 MalformedBoard | 3 MalformedBoard | 0 pass → **3 UnmodelledCopper** |
| board | keyword | `pad_type_quoted` | 5 ToolError → **3 MalformedBoard** | 5 ToolError → **3 MalformedBoard** | 0 pass → **3 MalformedBoard** |
| board | parts | `cap_as_pullup` | 1 fail | 1 fail | 3 DeviceNotOnLine |
| board | parts | `cap_refvalue_only` | 0 pass | 1 fail | 3 DeviceNotOnLine |
| board | parts | `controller_renamed` | 0 pass | 1 fail | 3 DeviceNotOnLine |
| board | repeated-child | `big_plate` | 1 fail → **3 MalformedBoard** | 1 fail → **3 MalformedBoard** | 3 UnmodelledCopper → **3 MalformedBoard** |
| board | repeated-child | `dup_end` | 1 fail → **3 MalformedBoard** | 1 fail → **3 MalformedBoard** | 3 UnmodelledCopper → **3 MalformedBoard** |
| board | repeated-child | `dup_fp_at` | 1 fail → **3 MalformedBoard** | 1 fail → **3 MalformedBoard** | 3 UnmodelledCopper → **3 MalformedBoard** |
| board | repeated-child | `dup_net_seg` | 0 pass → **3 MalformedBoard** | 0 pass → **3 MalformedBoard** | 3 UnmodelledCopper → **3 MalformedBoard** |
| board | repeated-child | `dup_pad_at` | 1 fail → **3 MalformedBoard** | 1 fail → **3 MalformedBoard** | 3 UnmodelledCopper → **3 MalformedBoard** |
| board | repeated-child | `dup_pad_net` | 1 fail → **3 MalformedBoard** | 1 fail → **3 MalformedBoard** | 3 UnmodelledCopper → **3 MalformedBoard** |
| board | repeated-child | `dup_pad_size` | 1 fail → **3 MalformedBoard** | 1 fail → **3 MalformedBoard** | 3 UnmodelledCopper → **3 MalformedBoard** |
| board | repeated-child | `dup_thickness` | 0 pass → **3 MalformedBoard** | 0 pass → **3 MalformedBoard** | 3 UnmodelledCopper → **3 MalformedBoard** |
| board | repeated-child | `dup_via_size` | 1 fail → **3 MalformedBoard** | 1 fail → **3 MalformedBoard** | 3 UnmodelledCopper → **3 MalformedBoard** |
| board | repeated-child | `dup_width` | 1 fail → **3 MalformedBoard** | 1 fail → **3 MalformedBoard** | 3 UnmodelledCopper → **3 MalformedBoard** |
| board | repeated-child | `general_twice` | 0 pass → **3 MalformedBoard** | 0 pass → **3 MalformedBoard** | 3 UnmodelledCopper → **3 MalformedBoard** |
| board | repeated-child | `segment_layer_and_layers` | 1 fail → **3 MalformedBoard** | 1 fail → **3 MalformedBoard** | 3 UnmodelledCopper → **3 MalformedBoard** |
| board | unmodelled-item | `arc_on_copper` | 1 fail | 1 fail | 3 UnmodelledCopper |
| board | unmodelled-item | `gr_rect_fcu` | 0 pass | 0 pass | 3 UnmodelledCopper |
| board | unmodelled-item | `gr_rect_top` | 5 ToolError | 5 ToolError | 3 UnmodelledCopper |
| board | unmodelled-item | `gr_text_copper` | 0 pass | 0 pass | 3 UnmodelledCopper |
| board | unmodelled-item | `user_layer_name` | 5 ToolError | 5 ToolError | 3 UnmodelledCopper |
| board | unmodelled-shape | `pad_circle` | 1 fail | 1 fail | 3 UnmodelledCopper |
| board | unmodelled-shape | `pad_circle2` | 1 fail | 1 fail | 3 UnmodelledCopper |
| board | unmodelled-shape | `pad_starcu` | 1 fail | 1 fail | 3 BoardUnreadable |
| board | unread-child | `pad_nolayers` | 1 fail | 1 fail | 3 UnmodelledCopper |
| board | unread-child | `pad_offset` | 1 fail | 1 fail | 3 UnmodelledCopper |
| board | unread-child | `stackup` | 0 pass | 0 pass | 3 UnmodelledCopper |
| board | unread-child | `via_padstack` | 1 fail | 1 fail | 3 UnmodelledCopper |
| board | unread-child | `zone_clearance` | 3 LocalClearance | 3 LocalClearance | 3 UnmodelledCopper |
| board | unread-child | `zone_fill_close` | 0 pass | 0 pass | 3 UnmodelledCopper |
| schematic | control | `reference` | 0 pass | 0 pass | — |
| schematic | escape | `rail_octal_unconnected` | 0 pass → **3 MalformedSchematic** | 1 fail → **3 MalformedSchematic** | — |
| schematic | escape | `value_name_hex` | 1 fail → **3 MalformedSchematic** | 0 pass → **3 MalformedSchematic** | — |
| schematic | escape | `value_name_octal` | 1 fail → **3 MalformedSchematic** | 0 pass → **3 MalformedSchematic** | — |
| schematic | escape | `value_text_hex` | 1 fail → **3 MalformedSchematic** | 0 pass → **3 MalformedSchematic** | — |
| schematic | escape | `value_text_octal` | 1 fail → **3 MalformedSchematic** | 0 pass → **3 MalformedSchematic** | — |
| schematic | field-spelling | `value_upper_case` | 1 fail → **3 MalformedSchematic** | 0 pass → **3 MalformedSchematic** | — |
| schematic | field-twice | `value_twice` | 0 pass → **3 MalformedSchematic** | 0 pass → **3 MalformedSchematic** | — |
| schematic | private-flag | `value_private` | 1 fail | 0 pass → **3 ReservedNetName** | — |
| schematic | repeated-child | `paper_twice` | 0 pass → **3 MalformedSchematic** | 0 pass → **3 MalformedSchematic** | — |
| schematic | repeated-child | `symbol_at_twice` | 1 fail → **3 MalformedSchematic** | 0 pass → **3 MalformedSchematic** | — |
| schematic | repeated-child | `symbol_lib_id_twice` | 0 pass → **3 MalformedSchematic** | 0 pass → **3 MalformedSchematic** | — |
| board | (reference) | `aethelgard generate` | 0 pass | 0 pass | 0 pass |

## What was built (§1)

1. **`aethelgard_eda::strict`** (new, `crates/aethelgard-eda/src/strict.rs`). It is grown from 1305's walk, but holds only the universal rules; `extract`'s copper model stays in `extract`.
   - **`RULES`:** each node a gate reads (`kicad_pcb`, `general`, `segment`, `via`, `gr_rect` and its `stroke`, `footprint`/`module`, `pad`, `zone` and its `connect_pads` and `fill`; `kicad_sch`, a placed `symbol`, a library `symbol`), with the groups of children that set one value KiCad holds once. A group can name several spellings: `uuid`/`tstamp`, a track's `layer`/`layers`, a footprint's `solder_paste_margin_ratio`/`solder_paste_ratio`. A second member of a group is `Repeated`, refused and never resolved first-or-last.
   - **Escapes (`sexpr.rs`):** a string escape other than `\" \\ \n \r \t` is `Escape`, refused for every file and every gate (kind `MalformedBoard`/`MalformedSchematic`, or extract's `BoardUnreadable`, with no path). KiCad's `DSNLEXER::NextTok` (`dsnlexer.cpp` 655–712) decodes `\a \b \f \v`, `\xHH` and `\ooo`, and keeps the backslash before any other character; the old tokenizer dropped it. No vendored library file, generated file or committed fixture holds any other escape (searched).
   - **Keywords:** a pad's type and shape (positions 2 and 3) must be bare atoms (`Quoted`); `parsePAD` reads keyword tokens there.
   - **Fields:** a footprint field named twice, counting a legacy `fp_text reference`/`fp_text value` as the `Reference`/`Value` it sets, is `FieldTwice`; so is a symbol field named twice. A schematic `property` spelling a mandatory name non-canonically is `FieldSpelling`. A schematic field's name is read past a bare `private` (`Node::named`), as `parseSchField` reads it.
   - **Typed refusals:** exit 3, `MalformedBoard` or `MalformedSchematic`, with `id` the node's path, such as `kicad_pcb/footprint[7]/pad[2]` or `kicad_sch/symbol[1]/property[2]`. Unreadable bytes and the wrong root keep their old kinds and messages, with no path.
   - **What it hands out:** a `Tree`, which only `strict::board`, `strict::schematic` and `strict::footprint` make. It is read through `Node`, whose methods are `head`, `arg`, `last`, `len`, `is_empty`, `texts`, `path`, `lists`, `all`, `one`, `named` and `field`. `one` and `field` refuse a second match (`Ambiguous`, `FieldTwice`), wherever the rules are silent.
   - An unknown child is not refused here; whether one matters is the gate's (§1.1's qualifier). `extract`'s closed allowlists refuse them, as before.
2. **Every gate reads the strict tree first (§1.2):**
   - `check drc`: `board::read` → `board::prepare_read`, the rule walk and the setup splice;
   - with `--schematic`: `parity::schematic` and `parity::board` on the same `Tree`. Parity no longer re-parses the prepared copy;
   - `check erc`: `parity::flat`, the strict read plus the sheet refusal;
   - `extract`: `strict::board`, then its closed walk, `route::read(&Tree)`, `connectivity` and `check`;
   - `verify`: runs the same subcommands.
   - `route::part_of(&Sx)` keeps its signature for `pcb.rs`, which is outside the fence, and judges the generator's footprint with `strict::footprint` before reading it.
3. **The fence (§1.3): a type plus a guard test.** Justified in F2.
4. **The corpus (§1.4):** `crates/aethelgard-cli/tests/fixtures/strict/<board|schematic>/<class>/<name>.edit`. There are 55 files: 1305's 32 boards, 11 new boards and 12 schematics, 12 of them from the adversarial pass. Each is an exact find-and-replace on the generated reference. Its header names its class and records main's verdict and the verdict each gate must now give. A test fails if an edit stops applying (`tests/common/corpus.rs`).
   - **Tier 1** (`strict_tier1.rs`): every refusal, by kind and path, with an empty `PATH`. Every accepted file reaches the tool lookup (`ToolMissing`), and `extract`'s verdicts are checked exactly.
   - **Tier 2** (`board_tier2.rs`): KiCad's verdict on every file the reader accepts.
   - `TIER2_MIN` 74 → 75.

## KiCad 10.0.7's reading, established from its source (kickoff item 2)

Source: gitlab `kicad/code/kicad`, tag `10.0.7`. Files read: `pcbnew/pcb_io/kicad_sexpr/pcb_io_kicad_sexpr_parser.cpp`, `eeschema/sch_io/kicad_sexpr/sch_io_kicad_sexpr_parser.cpp`, `pcbnew/footprint.cpp`, `common/template_fieldnames.cpp`, `common/dsnlexer.cpp`.

- **A scalar child is a setter, so a repeat keeps the last.** For example, `parseGeneralSection` `T_thickness` → `SetBoardThickness`; `parsePCB_TRACK` `T_width` → `SetWidth`, and `T_layer` and `T_layers` both set the track's layers; `parsePCB_VIA`; `parsePAD`; `parseZONE`; `parseFOOTPRINT_unchecked`; `parseSchematicSymbol`. Each child in a `RULES` group was read as such a setter.
- **What is appended, and so may repeat:** a footprint's `pad`, `fp_*`, `model`, `zone` and `group`; `attr` (its flags are OR-ed into one value, `attributes |= …`, so a second `attr` adds rather than replaces, and parity already reads them all); a zone's `polygon` (holes after the outline) and `filled_polygon`; a pad's `primitives`; the board's `net`, `footprint`, `segment`, `via` and `zone`.
- **Footprint fields:** `property` sets `GetField(name)->SetText`, matching names with `==` (`FOOTPRINT::GetField`). `fp_text reference`/`fp_text value` overwrite `Reference()`/`Value()` (`T_fp_text` → `PCB_FIELD( *text, FIELD_T::REFERENCE )`). The last one wins.
- **Schematic fields:** `parseSchField` maps a name to a mandatory field with `CmpNoCase(GetCanonicalFieldName(id))`, so `"VALUE"` is the `Value` field; a mandatory field is replaced, not added. Measured in the image: main's parity passed `(property "VALUE" "unconnected-GND")` on a power symbol, while KiCad names that net `unconnected-GND`.
- **Escapes:** `DSNLEXER::NextTok` (lines 655–712) decodes `\"` `\\` `\a` `\b` `\f` `\n` `\r` `\t` `\v`, `\x` with 1–2 hex digits and `\` with 1–3 octal digits; before any other character it yields a literal `\`. Measured by the adversarial pass in the image: `kicad-cli sch export netlist` names a rail `"unconnected\055V"` `unconnected-V`, and `pcbnew` reads a pintype `"bidirectional+no\137connect"` as `bidirectional+no_connect`.
- **`private`:** `parseSchField` reads an optional `T_private` token before a field's name (lines 2290–2296); a quoted `"private"` is a string, not the token.
- **`general`** skips unknown children (`default: // Skip everything else`). No gate reads them.

## Findings and gaps

| # | Finding | Where | Type | Recommendation | Decision-needed |
|---|---|---|---|---|---|
| **F1** | §3's corpus line against the work order's "It is NOT" (above) | §3 | contradiction | read "the corpus" as the boards KiCad and a gate read differently | **yes** |
| F2 | **The fence is a type plus a guard test, not a `clippy.toml`.** `disallowed-methods` is crate-wide. The vendored readers (`library.rs`, `schematic.rs`, `pcb.rs`) share `aethelgard-eda` with the gates, so each would need an `#[allow(clippy::disallowed_methods)]`. Those files are outside the fence (§7.3), and an `allow` is 1204's own A1 hole. A line in `aethelgard-cli/clippy.toml` would also break 1204's guard test, which requires that file to list exactly two paths. What holds instead: a gate reads only `Node`, so `.child(…)` on what it reads does not compile (mutant `fence-child-on-node`: E0599). `strict_fence.rs` gives every module of `eda/src` and `cli/src` a role, and the role decides whether the module may look into or parse a raw tree. It pins `Sx`'s and `Node`'s method lists, and names the entry points and `Sx::child`/`children`/`property`/`items` by path. **What the guard catches is a test failure, not a build failure:** a gate that re-parses its text and calls `child` on the result compiles, and `strict_fence.rs` fails it (mutant `fence-reparse`). The adversarial pass found two mutants the first guard missed (F13), both now caught: picking one of a `Node`'s children with `.next()`/`.last()`/`.find(…)` on `all`/`lists`, and `sexpr` reached through an alias (`use crate::sexpr as raw`) or a multi-line grouped import | `strict.rs`, `strict_fence.rs` | fence | accept; or rule that the vendored readers move under one module, which would let `pub(in …)` make `child` uncallable from the gates at compile time (outside this fence) | yes |
| F3 | **Seven `extract_tier1` rows change kind, `UnmodelledCopper` → `MalformedBoard`.** They are the repeated-child rows: width, via size, footprint `at`, `Value` twice, thickness twice, outline layer twice and `general` twice. Each is still refused with exit 3. §1.1 types a strict refusal `MalformedBoard` with a path, and §1.2 runs the strict reader first. No verdict changed; the label did | `extract_tier1.rs` | kind change | accept | no |
| F4 | **Three reader disagreements main's `extract` passed** (exit 0), and 1305's adversarial pass did not find: `fp_text reference "J1"` after U1's `Reference` property (KiCad reads J1), `fp_text value "BMP280"` after U2's `Value` (KiCad reads BMP280), and `("size" 0.54 4)` in a pad, whose head 1305's walk skipped. All three are refused now | `extract` | fixed defect | none | no |
| F5 | **`check erc` refuses a file whose root is not `kicad_sch`** (`MalformedSchematic`, exit 3); main gave `ToolError` (exit 5), since KiCad wrote no report. KiCad cannot load it, so it is not an input both read alike. The same holds for parity's schematic, as before | `parity::flat` | verdict change | accept | no |
| F6 | **Two outlines.** `route::read` took the first `Edge.Cuts` `gr_rect` and now refuses a second (`2 outlines`). `extract` already refused a second top-level `gr_rect`, and the router reads only generated boards, so no verdict moves | `route.rs` | first-match removed | none | no |
| F7 | **Parity reads KiCad 5's `module` as a footprint**, as KiCad 10 does (`T_module`); main read only `footprint`. No fixture had a `module` with `board_only`; a unit test now does | `parity.rs` | gap closed | none | no |
| F8 | **A second `(setup …)` stays `board.rs`'s own `SetupTwice`.** `setup` is not in `RULES`: the gate replaces the block whole, and the existing variant and its test keep their meaning | `board.rs` | scope | none | no |
| F9 | **What `check drc` passes that the fabricator may read otherwise: the file's `stackup` and its saved zone fill.** The gate replaces `setup` and refills zones in its scratch copy only. The caller's file keeps a 105 µm, εr 10 stackup (`stackup`) or a fill 10 µm from SDA (`zone_fill_close`), and a fabricator that reads the file rather than refilling sees it. This is not a reader disagreement but the gate's procedure (`RUL-03`). `extract` refuses both | `check drc` | gap | a backlog item: refuse a saved fill and a stackup in `check drc`, or put "refill before fabrication" in the fabrication package (`AET-WO-3300`) | yes |
| F10 | **The corpus is edit recipes, not whole files.** 43 boards and schematics in full come to about 4.7 MB; the recipes are 164 KB. Each recipe asserts that its edit applies exactly once to the reference generated at test time. A generator change that moves an anchor fails loudly | fixtures | form | none | no |
| F11 | **`TMPDIR` in the local tier-2 run is 0711, not the kickoff's 0700.** The container runs as root, as CI's does, and `AET-WO-1601`'s scratch-base rule needs others' search bit on a root-owned base so the tool uid can reach its directory. A 0700 base is `ScratchUnsafe` as root | local runner | process | none | no |
| **F13** | **The adversarial pass broke the build (B1–B6, F1–F2; all closed, each with a fixture or a guard).** **B1/B2 (high):** an escape the old tokenizer decoded differently from KiCad, `"unconnected\055V"` or `"bidirectional+no\137connect"`, passed `check drc --spec --schematic`, `extract` and `check erc` with exit 0 on both main and 795c520, while KiCad read `unconnected-V` and `no_connect`. That reopened 1301's A6 and A7. **B3:** `"\126alue"`, `"\x2d"` and `(property private "Value" …)` on a power symbol passed parity. **B4:** an `fp_rect` on `"F\056Cu"` passed `extract` with SDA's bound unchanged. **B5:** a second field named by escape. **B6:** a quoted pad type passed `extract`; KiCad will not load it. **Fence F1/F2:** see F2. B1–B5 are older than this work order (the tokenizer has decoded this way since 1300), but they sit under the strict reader, and §5.1 names escapes | `sexpr.rs`, `strict.rs`, `parity.rs`, `strict_fence.rs` | fixed defect | none | no |
| F14 | **Edits beyond the files §2 names, each the minimum the named ones needed:** `eda/src/lib.rs` (`pub mod strict;`); the CLI's `main.rs` `check_kicad` (it hands `prepare`'s strict `Tree` to parity, 13 lines) and `src/extract.rs` (the refusal's `id`, 7 lines), both part of the gates' error mapping; `ci.yml` `TIER2_MIN` (kickoff item 3); and the two crate READMEs' refusal lists. No emitter, no vendored file, nothing protected, and no `clippy.toml` | as listed | fence | accept | no |
| F12 | **`Node::field` and `Node::one` refuse a second match even where `RULES` already did.** For a node the rules cover, that is the type's contract rather than a second check: a footprint under a node no rule reaches is named by `board.rs` through `field`, and is refused there (test `one_refuses_a_second_match_wherever_the_rules_are_silent`). Their mutants are killed | `strict.rs` | design | none | no |

## Mutation (sized to the diff; one mutant each; scratch clone `~/.cache/aet-1306/mut`; capped scope)

First pass at 795c520 (`~/.cache/aet-1306/mut.py`, 17 mutants), then targeted re-runs after each change (`mut2.sh`, `mut3.py`):

| Mutant | Suite | Result |
|---|---|---|
| a repeated child no longer refused (`judge`) | eda lib, `strict_tier1` | killed |
| a field named twice no longer refused | eda lib, `strict_tier1` | killed |
| case-folding of symbol field names removed | eda lib, `strict_tier1` | **survived**: dead, because `spelling` refuses every non-canonical mandatory name first. Removed by the reduction, and the unit test now asserts `FieldSpelling` |
| `spelling` never refuses | eda lib, `strict_tier1` | killed |
| `Node::one` takes the first of two | eda lib | killed |
| `Node::field` takes the first of two | eda lib | **survived**, then killed by a new test: a footprint under a node no rule reaches (F12) |
| `general`'s `thickness` dropped from `RULES` | `strict_tier1` (`dup_thickness`) | killed |
| a track's `layer`/`layers` split into two groups | `strict_tier1` (`segment_layer_and_layers`) | killed |
| a zone inside a footprint not judged | eda lib (new test) | killed |
| a placed symbol's `lib_id` dropped from `RULES` | `strict_tier1` (`symbol_lib_id_twice`) | killed |
| `extract` maps a headless list to `at` | `extract_tier1` | survived as a mutant, not as a gap: `at` collides with the pad's own `at`, which is refused anyway. The faithful mutant, skipping a headless list as 1305 did, is killed by `pad_quoted_size` |
| `route::read` takes the first of two outlines | eda lib (new test) | killed |
| parity reads `footprint` and not `module` | eda lib (new test) | killed |
| `flat` no longer refuses a sheet | `uid_boundary_tier1`, `placement_tier1` | killed |
| escapes decoded as before (`other => other`) | `sexpr` unit test | killed |
| the `private` flag not skipped | parity and strict unit tests | killed |
| a quoted pad type allowed | strict unit test, `strict_tier1` (`pad_type_quoted`) | killed |

**The fence, mutated:**

| Mutant | Result |
|---|---|
| `fp.child("at")` on a `Node` in `route.rs` | **does not compile** (E0599) |
| `sexpr::parse(text)` and `.child(…)` in `board.rs` | `strict_fence` fails |
| `strict::board` renamed, callers included, tests untouched | the guard test does not compile |
| a new module `sneaky.rs` using `.child` | `strict_fence` fails ("a module was added") |
| `Sx::child` renamed `first_child`, callers included | the guard test does not compile |
| adversary F1: `pad.all("net").next()` in `parity.rs` | `strict_fence` fails ("takes one of a node's children") |
| adversary F2: `use crate::sexpr as raw; raw::parse(text)` handed to `schematic::pins` | `strict_fence` fails |
| F2 with a multi-line `use crate::sexpr::{ SexprError, parse, };` | `strict_fence` fails |

The clone was rebuilt unmutated after each round and passed.

## Adversarial pass (full, fresh subagent on Opus, frozen tree 795c520)

Brief: the work order and the frozen code, with no conclusions of mine. KiCad's source was fetched from tag 10.0.7, and the subagent ran in its own copy and capped containers. Its report: `~/.cache/aet-1306/adv/out/FINDINGS.md`, `evidence.txt`, and probes generated by `out/scripts/mk.py`.

**§5.1, a file read two ways that still passes:** six broke (F13), each with how KiCad's reading was established (source lines, `pcbnew.LoadBoard`, `kicad-cli sch export netlist`). All six are closed. Every one of its 42 probes was re-run on the fixed binary: each break is refused by every gate that reads its kind. What still passes is its own controls, or what it marked held (below).

**Held, by its report:** the builder's own spellings; repeated `RULES` children; `attr` (OR-ed); footprint-in-footprint; a nested or respelled `sheet`; unknown children; quoted keywords elsewhere; quoted numbers (`NeedNUMBER`); number forms; a bare atom against a string; `()`; `|`; deep nesting; a 1.1 MB atom (read alike); a dotted `İ` (KiCad in `C.UTF-8` did not fold it); `(net N "name")`; version-dependent conversions; a power-symbol pin name; an escaped shared unconnected net (KiCad's parity caught it).

**Unconfirmed, by its report:** whether `\0` truncates a name in KiCad. It is moot now: `\0` is an octal escape, and the tokenizer refuses it.

**§5.2, a caller-input path still reaching a first-match lookup:** none in the frozen code. The only `.next()` on a `Node` iterator was `route::part_of` on the engine's own footprint; it is now `one("footprint")`. The fence missed two mutants (F2, F13), both now caught.

**§5.3, an existing fixture whose verdict changed:** it put a wrapper in place of the CLI and ran main and this branch side by side on every `check` and `extract` call the suites make: 266 tier-1 and 59 tier-2. Seven differ, all `extract` on `extract_tier1.rs`'s repeated-child mutants: exit 3 both, with the kind `UnmodelledCopper` → `MalformedBoard` (F3). `container/fixtures/*` are unchanged, and no input that KiCad and the gate read alike changed. Not covered by it: `verify`'s internal gates, `board_tier2` (stopped for Kani's time) and `campaign`. The verdict table above covers `verify`'s board step, and the image run (Gates) passes every tier-2 test.

## Reduction pass (diff)

1. **Removed, proved by a test:** the case-folding of schematic field names (`strict::fold`). Its mutant survived because `spelling` refuses every non-canonical spelling of a mandatory name first, so two names that fold together never both reach the comparison. The unit test now asserts `FieldSpelling` for `"Value"` + `"VALUE"`, and `FieldTwice` for `"Value"` twice.
2. **Removed:** `extract::footprint`'s own duplicate-`property` check. The strict reader's `FieldTwice` refuses it first (row `property_twice`).
3. **Removed:** the CLI's second schematic reader, `refuse_sheets`' own parse and walk. Both ERC and parity now call `parity::flat`, through one `read_schematic` helper.
4. **Removed:** `route::part_of`'s `.lists().next()`, replaced by `one("footprint")`, so no gate module picks a child by position.
5. **Kept, and why:** `extract`'s closed tables keep their repeat flags. They refuse what KiCad may repeat but the bound reads once (`model`, a zone's `polygon`, a second top-level `gr_rect`). `board.rs`'s `SetupTwice` keeps its variant (F8). `Node::one` and `Node::field` keep their own refusals (F12).

Reduction pass (diff): 4 removals (one a whole function, proved dead by its surviving mutant and then by a test) · 0 load-bearing removals · 3 kept with reasons · no `belt-and-suspenders:` code touched

## Gates

- **Host, at 7555726** (the final code): `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets -- -D warnings`, with and without `--features aethelgard-cli/with-tools`; `cargo test --workspace`: 936 passed, 0 failed. Main gave 921; this branch adds 15 (6 `strict` units, 1 `sexpr` escape unit, 4 `strict_tier1`, 3 `strict_fence`, and 1 `extract_tier1` test retyped, F3).
- **The pinned image `c69ff9bc…`, at 7555726, CI's `tools` job step by step** (`~/.cache/aet-1306/tools-job.sh`) under rootless podman 5.7:
  - steps: fetch and build with the network; the bridge's build and tests; `cargo fmt --all -- --check`; clippy with `with-tools`, `-D warnings`; `cargo test --workspace --locked --offline --features aethelgard-cli/with-tools`, with `--network none` after the build;
  - **1,019 tests passed, 0 failed; the tier-2 canary counted 75 = `TIER2_MIN`** (74 + 1: `every_corpus_file_the_reader_accepts_gets_its_stated_kicad_verdict`);
  - caps: `--memory 16g --memory-swap 16g --pids-limit -1`, then 0 written into the container's `memory.swap.max`. Read back on the test container: `memory.max` 17179869184, `memory.swap.max` 0, `pids.max` max (`caps.log`).
- **`TMPDIR` (kickoff item 3, F11):** the first image run, at 795c520, set `TMPDIR` to a 0711 directory of mine. It failed one test, 1601's `as_root_a_third_uid_cannot_change_what_build_reads` ("0 of 0 attempts refused"), and cargo stopped there, so the canary read 32. That test looks for the CLI's scratch only under `/var/lib/aethelgard`, the root default when `TMPDIR` is unset, so with `TMPDIR` set it found no scratch to race. CI's job leaves `TMPDIR` unset, and the runs at cb891e3 and 7555726 were made in that shape; both pass that test. A test that cannot run under a set `TMPDIR` is 1601's to decide (backlog), not this work order's.
- **§3, `verify` on the reference, byte-identical to main's:** `verify --spec examples/sentinel-01-minimal.json --scenarios scenarios/sentinel-01/pullup-2k2.json`, main's binary (58cbd53, its bridge beside it) and this branch's, in the image. Both exit 0 with verdict pass, and the stdout is byte-identical: sha256 `52a4f5d1b5b9bc4c0b04e50605b3d27b084cf9cf6f1d0b145fb483b772982082`.
- **Committed generated files** are unchanged: no file under `boards/`, `examples/` or any emitter is in the diff, and the regeneration guards pass. The reference board's sha256 stays `7b9b0e3d…2070`.
- **CI on the PR** is the record for `gates`, `tools` and `image`.
