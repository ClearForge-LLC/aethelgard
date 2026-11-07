# AET-WO-1100 — FEEDBACK

**Branch:** `wo/aet-1100-specification` · **Base:** `e4fa8a7` (PR #18) · **Builder:** Claude Code (Sonnet 5.5) as `forge` on Furnace · **Result:** §1 built; every §3 item passes locally; no §7 stop.

Entry schema: `finding · where · type · recommendation · decision-needed`.

## What was built

- `aethelgard-ir`: `Specification` (renames `TopologyGraph`), `Mcu`, `Signal`, `I2cBus`, `SpiBus`, `I2cDevice`, `SpiDevice`, `SolvedTopology`, `PinAssignment`, all through `object_only!`; every `Option` key is required. `validate` follows §1.2's twelve steps and its step-2 identifier order. `SolvedTopology::validate` follows §1.3. `assignments` refuses a repeated key at parse time. The only collections used for lookup are `BTreeSet`/`BTreeMap`, and none is iterated to choose a refusal. No new dependency.
- `IrValidationError`: `EmptyProjectName`, `EmptyDomainId`, `EmptyNetName` are replaced by `InvalidIdentifier { path }`; `DuplicateNetName` now means a signal net declared twice, and `DuplicateNetConstraint` is the old net_constraints case. Added accessor `path()` (not in the WO): `id()` is `None` for `InvalidIdentifier`, so the path needed a getter.
- CLI: `validate` takes a `Specification`; report fields unchanged (`net_constraints` is now 1). Examples are the §1.1 document and its zero-voltage twin; `ci.yml` untouched. READMEs updated (`aethelgard-ir/README.md` is new).
- Tests: `aethelgard-ir/tests/validate.rs` rewritten (40 tests: one per refusal incl. all 31 identifier paths, step order, serde edge cases per struct, round trips, sorted output). Three CLI tests added (solved document, smuggled field, invalid identifier path). The three `refuses_empty_*` and two `…whitespace_variants` tests were rewritten as `InvalidIdentifier` tests, not deleted.

## Gates

- Local (Furnace): `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`: pass. `gates` example commands run by hand: minimal exits 0; zeroed exits 2 naming `VDD_3V3`.
- CI: see the run URLs below.
- Reduction pass (diff-sized): removed nothing further. `unique_keys` is the only new helper that is not a direct §1 requirement and it is one §1.3 demands. Old `trim` handling is gone with the three `Empty*` variants.

## Adversarial pass

| # | Finding | Severity | Type | Recommendation | Decision-needed |
|---|---|---|---|---|---|
| A1 | Smuggling a solved field into a `Specification` (`assignments`, `alternate_function` in a `Signal`) and a specification field into a `SolvedTopology`: all refused by `deny_unknown_fields`, tested at the library and the CLI. | none | verified | none | no |
| A2 | The identifier rule accepts names two humans read as the same: `O`/`0`, `l`/`1`/`I`, and case variants (`VDD_3V3` vs `vdd_3v3`) are distinct nets and distinct domains. | low | gap | Accept (RUL-14 is ASCII-only by design). A case-insensitive uniqueness check would need a ruling | yes |
| A3 | The rule accepts characters KiCad's s-expression files or Rust would refuse as names: `"`, `\`, `(`, `)`, `;`, `/`, `-`, `.`, a leading digit. Safe for JSON, unsafe once an emitter writes them out. | medium for P1 emitters | gap | `AET-WO-1500`-era emitters must quote or refuse; nothing here is wrong against RUL-14 | yes (whether to narrow RUL-14) |
| A4 | A device's `supply` is not compared with its bus's `domain` (or `mcu.supply`): a 1.8 V part on a 3.3 V bus validates. Step 5 checks only that each names *a* declared domain. | medium | missing rule | Add a step (decision: equality, or compatible tolerances) | yes |
| A5 | Two signals pinned by hand to one pin are not caught (as §1.2 says, `AET-WO-1200` does it); a pin written `PB8` and `pb8` would later look like two pins. | low | by design | `AET-WO-1200` must compare against its table, not by string | no |
| A6 | Order dependence: none found. No set is iterated; refusals come from `Vec` order, `SolvedTopology` order from `BTreeMap`. Serialization is byte-identical across input key orders (tested for `assignments`; `Specification` keeps document order). | none | verified | none | no |
| A7 | The CLI envelope's `id` is `null` for `InvalidIdentifier`; the JSON path travels in `message`, which the README says is not contract. | low | design tension | If callers need the path, add a `path` field to the envelope (an amendment) | yes |

## What was not built

Pin-table checks, solver, emitters, passives, other buses and a version migration, per §4. Nothing constructs a `SolvedTopology` outside tests.

## Rules in §1.2 that the build found wrong or missing

None wrong. A4 is a missing rule (decision-needed above).

## Promote-or-needs-none pass

- `AET-WO-0300b.FEEDBACK.md` (merged on base as #18): needs none. Its one open item (no real Renode `[ERROR]` case) is a fixture matter outside this work order.
- `AET-WO-0400.FEEDBACK.md`: not yet merged.
