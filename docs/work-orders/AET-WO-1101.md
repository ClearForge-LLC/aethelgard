# AET-WO-1101 — The pin table: ST's own data for the STM32H753ZI, pinned, vendored and parsed

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main` after `AET-WO-1100` has merged (named in the kickoff; verify live HEAD before branching).
**Branch:** `wo/aet-1101-pin-table`.
**Author:** Claude (architect) · **Builder:** a cheap builder (`docs/roadmap.md` §6), in a fresh session on Furnace.
**Phase:** P1. It owns no gate clause. It is the table `AET-WO-1200`'s solver searches, and the solver's refusals ("unknown pin", "pin assigned twice") are only as true as this table.

**Grounds:**
- `docs/architecture.md`:
  - §2.8: the source, commit and hashes;
  - §2.1: the SVD has no pin table;
  - `RUL-11`: BSD-3-Clause data keeps ST's notice;
  - `RUL-14`: the identifier set;
  - `RUL-16`: `BTreeMap`, no `HashMap`.
- `docs/northstar.md` §4: `INV-01` (the types live in `aethelgard-ir`) and `INV-06` (an unknown part halts).
- `docs/roadmap.md` §6: stub graduation is stated, not implied.
- The seams:
  - `crates/aethelgard-svd-hal` (a stub);
  - `crates/aethelgard-ir/src/lib.rs`;
  - `crates/aethelgard-cli/tests/workspace_shape.rs`.

> **What this is:** two of ST's XML files, vendored byte for byte and checked against their pinned hashes before anything reads them, parsed into a `PinTable` the solver will consume.
>
> It is NOT:
> - the solver;
> - a second part;
> - the SVD or register blocks, which `AET-WO-1400` covers;
> - a CLI subcommand.

**Cadence:** build-only. There is no STOP for a ruling, except §7. One PR, left unmerged.

## 1. Scope

### 1.1 Vendor the source

Under `crates/aethelgard-svd-hal/data/st-open-pin-data/`, add four files:

| File | Content |
|---|---|
| `STM32H753ZITx.xml` | `mcu/STM32H753ZITx.xml`, byte for byte, sha256 `d167c0884383dd22adc1f25b712c4b12ed1cdf35d898af7edba34ce0e7f98a05` |
| `GPIO-STM32H747_gpio_v1_0_Modes.xml` | `mcu/IP/GPIO-STM32H747_gpio_v1_0_Modes.xml`, byte for byte, sha256 `49f51ae2508a11df6b385b86ed21172ea43478c4d314314db465e0f8d278c246` |
| `LICENSE` | the repository's `LICENSE`, verbatim (BSD-3-Clause, © STMicroelectronics) |
| `SOURCE.md` | the repository URL, the commit, each file's original path and sha256, and one line saying the files are unmodified |

- **Where to fetch them:** `https://raw.githubusercontent.com/STMicroelectronics/STM32_open_pin_data/7d1f1514ed5583ec5007ad91236b4e1d377295b1/<path>`. Verify each sha256 before committing. A mismatch is a §7 stop.

### 1.2 The types, in `aethelgard-ir`

```rust
pub struct PinTable {
    pub part: String,                       // "STM32H753ZITx"
    pub package: String,                    // "LQFP144"
    pub source: PinSource,
    pub pins: BTreeMap<String, Pin>,        // keyed by canonical pin name, e.g. "PB8"
}
pub struct Pin {
    pub position: u16,                      // package pin number, e.g. 139
    pub alternate_functions: BTreeMap<String, u8>, // signal -> AF number, e.g. "I2C1_SCL" -> 4
}
pub struct PinSource {
    pub repository: String,
    pub commit: String,
    pub files: BTreeMap<String, String>,    // file name -> sha256
}
```

- All three are `Serialize`. None is `Deserialize`: a pin table is built from the vendored source and never read from a user.
- The table must pass `RUL-14`: every key and signal name is printable ASCII `0x21`–`0x7E`, 1 to 64 bytes.

### 1.3 The parse, in `aethelgard-svd-hal`

- **The entry point** is `pub fn pin_table(part: &str) -> Result<PinTable, PinTableError>`. It knows exactly one part, `STM32H753ZITx`. Any other refuses with `UnknownPart` (`INV-06`).
- **The input** is the vendored files, embedded with `include_bytes!` (no build script). Before anything parses them, their sha256 is checked against the pinned constants; a mismatch refuses with `SourceHashMismatch`, naming the file.
- **Parse with `roxmltree`** (MIT OR Apache-2.0, `#![forbid(unsafe_code)]`, version `0.21`). Keep its defaults, which refuse a DTD.
- **The rules, which are where ST's data needs care:**
  1. **Which pins.** Only `<Pin>` elements of the MCU file whose `Type` is `"I/O"` are in the table. That is 114 for this part. Power, `Reset`, `Boot` and `MonoIO` pins are left out.
  2. **Canonical names.** Some pin names carry a suffix, such as `PC14-OSC32_IN (OSC32_IN)`. The canonical name is the leading run that matches `P[A-K][0-9]{1,2}`. For example, `PC14`. A pin whose name has no such run, or two I/O pins with the same canonical name, refuse with `MalformedSource`. The architect's prototype of these rules, run on the pinned files, found eleven suffixed I/O names: `PC14-OSC32_IN (OSC32_IN)`, `PC15-OSC32_OUT (OSC32_OUT)`, `PH0-OSC_IN (PH0)`, `PH1-OSC_OUT (PH1)`, `PC2_C`, `PC3_C`, `PA13 (JTMS/SWDIO)`, `PA14 (JTCK/SWCLK)`, `PA15 (JTDI)`, `PB3 (JTDO/TRACESWO)` and `PB4 (NJTRST)`. All of them reduce to unique canonical names, with no refusal. The same prototype found 114 pins and no `MalformedSource` case in the real data.
  3. **Which alternate functions.** For each table pin, find the GPIO file's `<GPIO_Pin>` whose `Name` equals the MCU file's full pin name. Then take each `<PinSignal>` with a `GPIO_AF` value that is also listed under that pin's `<Signal>` in the MCU file. The GPIO file covers every package, 173 pins in all, so this intersection is what keeps another package's functions out.
  4. **Alternate-function numbers.** The number comes from the value's form `GPIO_AF<n>_<anything>`, with `n` in 0..=15. Any other form refuses with `MalformedSource`, naming the pin and the signal.
  5. **Anything else.** A table pin missing from the GPIO file, or a signal with two different AF numbers on one pin, refuses with `MalformedSource`. Signals with no `GPIO_AF` value, the analog and additional functions, are left out. Leaving out is not a refusal.
- **Errors** are one `thiserror` enum, `PinTableError`, with `kind()`, as `IrValidationError` has.

### 1.4 Graduating `aethelgard-svd-hal`

In `crates/aethelgard-cli/tests/workspace_shape.rs`, as `AET-WO-0400` did for `aethelgard-chaos`:
- `aethelgard-svd-hal` leaves `library_crates_declare_nothing_and_only_re_export` and `library_crates_are_only_a_manifest_and_lib_rs`;
- its dependencies leave `LIBRARY_DEPENDENCIES`.

One new test pins it:
- **Files:** `Cargo.toml`, `README.md`, `src/**/*.rs`, `tests/**/*.rs`, and the four files of §1.1.
- **Manifest keys:** within `package`, `dependencies`, `dev-dependencies` and `lints`.
- **Dependencies:** exactly `aethelgard-ir`, `roxmltree`, `sha2` and `thiserror`, plus `serde_json` as a dev-dependency.

`roxmltree` goes into `[workspace.dependencies]`.

If `AET-WO-0400` has already merged, follow the shape of its graduation test. If it has not, write the same shape, and the second of the two to merge reconciles them.

### 1.5 Documentation

Add `crates/aethelgard-svd-hal/README.md` covering:
- the source and the licence notice;
- the §1.3 rules;
- the refusals;
- how to add a part: a new pinned commit and hashes, by work order, never by hand-editing a vendored file.

## 2. Invariants (`docs/northstar.md` §4), the slice this work order touches

- **`INV-06`.** An unknown part, a vendored file that has changed, and malformed data each halt with a named error. No pin or function is guessed.
- **`INV-01`.** The types live in `aethelgard-ir`, which still depends on no other member.
- **`RUL-11`.** ST's notice travels with the data. The engine's licence is unchanged.
- **No `unsafe`, and no build script.**
- **Protected surfaces diff to empty:**
  - the steering documents, `README.md`, `LICENSE-*`, `docs/inputs/**`;
  - `container/`, `.github/workflows/`;
  - every crate except `aethelgard-ir` (the §1.2 types only), `aethelgard-svd-hal`, and `aethelgard-cli/tests/workspace_shape.rs`.

  `Cargo.toml` and `Cargo.lock` change only to add `roxmltree`.
- One PR, left unmerged. Push with `gh-write`, and batch the pushes.

## 3. Tests and acceptance, proven not asserted

1. **Golden facts:**
   - the table has exactly 114 pins;
   - `PB8` is at position 139 and has `I2C1_SCL` on AF4;
   - `PC14` exists, under its canonical name;
   - every key and signal passes `RUL-14`.
2. **Determinism:** the table serialized to JSON has a pinned sha256 constant, computed on Furnace and reproduced in `gates`. A mismatch between the two is a §7 stop, not a constant to update.
3. **Refusals:**
   - `pin_table("STM32F407VGTx")` gives `UnknownPart`;
   - the hash check refuses a one-byte change to either file. Exercise it through a test-only function that takes the bytes; the embedded files are not edited;
   - each §1.3 `MalformedSource` case refuses, on a small hand-written fixture inside the test.
4. **The workspace gates:** the new graduation test, and fmt, clippy and test, pass in `gates` and on Furnace. `tools` stays green. CI is green on the first push.

## 4. Scope fence — what is NOT in this work order

- The solver, any other part, packages other than LQFP144, register blocks, or the Nucleo `.ioc` board file.
- Any network access at build or test time. The files are vendored.
- Editing, reformatting or trimming ST's files.

## 5. Adversarial pass — targeted

Every finding goes in FEEDBACK with a severity.

1. **Find a pin or function the table gets wrong** for the reference probe's likely buses: I2C1, I2C2 and SPI4. Cross-check five entries against the STM32H753 datasheet's alternate-function table, and cite the datasheet's revision.
2. **Get a changed vendored file past the hash check.**
3. **Find an ST name the canonical rule mishandles,** such as a pin with a `_C` suffix, or a name with no port letter.

## 5a. Reduction pass — diff-sized

Run the `reduction-pass` skill on the finished diff. One line in FEEDBACK's gates.

## 6. Upward-feedback directive

`docs/work-orders/AET-WO-1101.FEEDBACK.md`, same schema as before. It carries:
- the gates line with CI run URLs;
- the five datasheet cross-checks;
- any rule in §1.3 the data contradicted, as decision-needed;
- a promote-or-needs-none pass over `AET-WO-1100`'s FEEDBACK.

## 7. Flag-and-stop conditions

Park the branch and say so, in FEEDBACK and in the session, if any of these appears:

- A fetched file whose sha256 differs from §1.1's.
- A §1.3 rule the data violates in a way that needs a new rule rather than a refusal.
- A determinism constant that differs between Furnace and `gates`.
- A dependency beyond §1.4's, `unsafe`, or a build script.
- A test that can only go green by being skipped, ignored, or weakened.
- Any step that needs `sudo`, a system package, or a secret.

## 8. Kickoff prompt

**Step 1, in its own turn:** send `/goal` alone, read what is in force, and dismiss it.

**Step 2, in its own turn:**

```
/goal AET-WO-1101: §1 is built and every §3 item passes in gates and on Furnace, or a §7 stop is recorded in FEEDBACK; the adversarial and reduction passes and FEEDBACK are written; one open unmerged PR from wo/aet-1101-pin-table.
```

Confirm that the "Goal set:" echo matches.

**Step 3, the kickoff message:** carried by the architect at kickoff, with the live base.
