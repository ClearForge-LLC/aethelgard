# AET-WO-1200 — The pin solver: a deterministic search from `Specification` and `PinTable` to `SolvedTopology`

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main` after `AET-WO-1101` has merged (named in the kickoff; verify live HEAD before branching).
**Branch:** `wo/aet-1200-solver`.
**Author:** Claude (architect) · **Builder:** a **strong** builder for the spike, then the architect rules whether the build half stays with it or moves to a cheap one (`docs/roadmap.md` §6). A fresh session on Furnace.
**Phase:** P1 · **Gate clauses owned:**
- **Clause 2:** the reference specification, with one pin deliberately assigned twice, makes the solver exit nonzero naming both claimants, and no board files are emitted;
- **Clause 4, solver half:** solving twice gives byte-identical output.

**Grounds:**
- `docs/architecture.md` §4:
  - `RUL-02`: a constraint solver, with a pure-Rust search in a fixed order as the prior, and Z3 only if the spike justifies it;
  - `RUL-16`: `SolvedTopology` is the solver's alone, `BTreeMap` only;
  - `RUL-14`.
- `docs/architecture.md` §2.8: the pin table's source.
- `docs/northstar.md` §4: `INV-01`, `INV-03` and `INV-06`.
- The seams:
  - `crates/aethelgard-ir`: `Specification`, `SolvedTopology`, `PinAssignment` and `PinTable`;
  - `crates/aethelgard-svd-hal::pin_table`;
  - `crates/aethelgard-cli/src/main.rs`, the command tree;
  - `examples/sentinel-01-minimal.json`.

> **What this is:** `solve(&Specification, &PinTable) -> Result<SolvedTopology, SolveError>`, a pure function in `aethelgard-ir`, plus `aethelgard solve --file <spec>` in the CLI. Because the MCP tool list is generated, the subcommand becomes an MCP tool with no other change.
>
> It is NOT:
> - clocks or timing beyond bus speed bounds;
> - voltage-domain routing;
> - any emitter;
> - a second part.

**Cadence:** spike first, then STOP for the architect's ruling, then build. One PR for both halves, left unmerged.

## 0. Spike — the questions it must answer

Record every answer in `docs/work-orders/AET-WO-1200.FEEDBACK.md` under "Spike", with the command or CI run that measured it. Then STOP. Probes go under `spike/`; the build deletes them.

- **S1 The search.**
  - **The model to build:**
    - every bus picks one peripheral **instance** of its kind, `I2C1` to `I2C4` or `SPI1` to `SPI6`, as the table offers. No two buses share an instance;
    - every signal (`sda`, `scl`, `sck`, `mosi`, `miso`) picks a pin whose table entry carries `<instance>_<SIGNAL>`, for example `I2C1_SDA`, with its alternate function;
    - every SPI device's `cs` picks any I/O pin, as a plain GPIO output (§1.2);
    - no pin is used twice;
    - a hand-set `peripheral` or `pin` is fixed.
    - **the debug port is reserved:** `PA13` (SWDIO) and `PA14` (SWCLK) are never assigned, because a board that loses them cannot be programmed or debugged. This addresses `AET-WO-1101` FEEDBACK finding 4, as ruled at review of PR #27. A hand pin on either refuses with `ReservedPin`. Debugging is SWD-only, so the JTAG-only pins `PA15`, `PB3` and `PB4` stay assignable. The README states that assumption.
  - **The order is fixed:**
    - buses in document order, I2C before SPI;
    - instances in ascending name order;
    - pins in the table's key order;
    - backtracking, with forward checking that drops pins already taken.
  - **What to measure:** solve time on two specifications.
    - The reference probe: `examples/sentinel-01-minimal.json`.
    - A stress specification: four I2C buses and four SPI buses, each SPI bus with two devices, written under `spike/`.
  - **Z3 is measured only if the search takes more than 1 s on the stress specification.** Otherwise record "Z3 not measured: the search met the bound". That is `RUL-02`'s inverted prior, applied.
- **S2 Determinism.** Two solves of the same specification give byte-identical JSON, on Furnace and in `gates`. Say what in the search could make the output depend on anything but its inputs.
- **S3 Refusals that name causes.** For each, record the exact message the search can produce:
  - **a hand-pinned pin claimed twice:** both claimants, by signal path, such as `i2c_buses[0].sda` and `spi_devices[0].cs`;
  - **a bus no instance can serve:** the bus, and why;
  - **a pin the table does not have, or a peripheral it does not offer:** the name;
  - **an infeasible combination after the search:** the smallest set of buses that cannot all be placed. If computing it is too costly, say so and name the bus that failed last.
- **S4 The `SolvedTopology` change.** `cs` is a GPIO output with no alternate function, and `PinAssignment` requires one. Propose the smallest change to the type. The architect's default is a separate `gpio: BTreeMap<String, String>` field, mapping net to pin. Say whether the default holds.

**STOP** when §0 is answered: FEEDBACK's Spike section written, the PR open, every answer cited.

## 1. Scope — the build half, after the ruling

1. **`solve`, in `aethelgard-ir`,** as S1 measured it, with the spike's ruled changes.
   - `specification_sha256` is the sha256 of the specification's input bytes as read (`INV-03`).
   - `solve` returns refusals as one `SolveError` enum with `kind()` and the claimant fields.
2. **The `SolvedTopology` change** S4's ruling picks, with `SolvedTopology::validate` updated and tested.
3. **`aethelgard solve --file <spec>`.**
   - It reads the file under the 16 MiB cap, validates the specification (exit 2 on refusal, as `validate`), loads the part's pin table (`UnknownPart` gives exit 2), solves, and prints the `SolvedTopology` as one JSON line, exit 0.
   - A solver refusal is exit 2, with the kind and a `claimants` array in the refusal envelope, because the specification cannot be realized.
4. **Tests:**
   - the gate's clause 2 on the reference example with one pin set twice;
   - two solves byte-identical (clause 4's solver half);
   - each S3 refusal;
   - the stress specification solved within the bound S1 measured;
   - the MCP tool list now includes `solve`, through `AET-WO-1500`'s test if it has merged; otherwise record "not yet merged".
5. **Documentation:**
   - `aethelgard-ir/README.md` gains the search order and the refusals;
   - the CLI README gains `solve`.

## 2. Invariants and fences

- **`INV-01`:** the solver's output depends only on the specification and the table.
- **`INV-06`:** nothing is guessed. An unplaceable bus refuses.
- **No `unsafe`.** No new dependency for the search. Z3, if the spike justifies it, is a ruling, not a build step.
- **Protected surfaces diff to empty:**
  - the steering documents, `README.md`, `LICENSE-*`, `docs/inputs/**`;
  - `container/`, `.github/workflows/`;
  - every crate but `aethelgard-ir` and `aethelgard-cli`;
  - in `aethelgard-cli`, everything but the command tree, `solve`, its tests, and the README.
- One PR, left unmerged. **Push and revoke by the FUR-WO-0301 F7 route:** a `grep -o` redirect from your transcript into `gh-write prep`'s file, never the Write tool.

## 3. Acceptance

1. Every §0 answer is in FEEDBACK with its measurement.
2. §1.4's tests pass in `gates` and on Furnace.
3. CI is green on the first push after the ruling.

## 4. Scope fence

Clocks, timing, voltage domains, emitters, other parts, and any change to `PinTable` or its source.

## 5. Adversarial pass — targeted

1. Find a specification on which two solves differ, or on which the result depends on anything but the input.
2. Find a valid specification the search refuses (a false unsat), or an invalid one it accepts: a pin used twice, or a signal on the wrong alternate function.
3. Find an input that makes the search exponential. Measure it, and say whether a bound is needed.

## 5a. Reduction pass — diff-sized

Run the `reduction-pass` skill on the finished diff. One line in FEEDBACK's gates.

## 6. Upward-feedback directive

`docs/work-orders/AET-WO-1200.FEEDBACK.md`, same schema. It carries:
- the spike's measurements, for `architecture.md` §2 by amendment;
- the S4 ruling as built;
- the gates line;
- a promote-or-needs-none pass over `AET-WO-1101`'s FEEDBACK.

## 7. Flag-and-stop conditions

Park the branch and say so, in FEEDBACK and in the session, if any of these appears:
- A search that is not byte-deterministic.
- A design that needs Z3 or any C++ dependency without a ruling.
- A refusal that cannot name its claimants.
- `unsafe`, or a test that can only go green by being skipped, ignored, or weakened.
- Any step that needs `sudo`, a system package, or a secret beyond the push route above.

## 8. Kickoff prompt

**Step 1, in its own turn:** send `/goal` alone, read what is in force, and dismiss it.

**Step 2, in its own turn, set the goal for the spike half:**

```
/goal AET-WO-1200 spike: every §0 question has a measured answer or a recorded reason it cannot be, cited in FEEDBACK, on an open unmerged PR from wo/aet-1200-solver, and the build half has not started.
```

Confirm that the "Goal set:" echo matches.

**Step 3, the kickoff message:** carried by the architect at kickoff, with the live base. After the ruling, the architect sets a build goal and gives the ruling in the same message, so the stop is never held against a goal it can no longer meet.
