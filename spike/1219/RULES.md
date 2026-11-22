# AET-WO-1219 — the class rule

Written and committed 2026-11-22, before any file under `crates/aethelgard-ir` was opened (see `PREDICTIONS.md` for what had been seen). The list in `LIST.md` is classed by this rule and nothing else; any later change to it is an amendment with a reason, at the end of this file.

## 1. What an entry is

| Unit | Code | One entry is | Counted toward acceptance |
|---|---|---|---|
| Specification key | K | one serde field of one type reachable from the specification's root type, counted **once**, however many JSON paths reach it. Its paths are listed with it | yes |
| Refusal kind | R | one variant of a refusal-kind enum (the `"kind"` string a refusal carries), counted once | yes |
| Held table | H | one table the crate holds as data: a file under `crates/aethelgard-ir/data/`, or a `const`/`static` table in source. Its **columns** are listed and classed beside it (§3) | yes, by table; columns tallied separately |
| Other type | T | a public type or field of the solved topology or another output type, not reachable from the specification | **no**: tallied separately so the acceptance count stays clean |

- **Enum variants** of a key's value set are **listed** under the key, with whether any committed specification uses each one. They are not counted as entries. They decide the key's class (§2, rule 2).
- An **unnamed key** (a map whose keys are user data, such as a net name) is classed by its value type and listed once.

## 2. The classes, decided in order

The classes are judged **from ST's side alone**: what ST's documents define, never what another chip has. A class must not rest on a fact about the RP2350 or any other second part, because no such document is held. The RP2350 column (§4) is separate and answers only from a held document.

**Rule 1: D, data, not schema.** The entry is a row, or a value inside a row, of a held table, and a second vendor would add rows, not keys, columns or variants. This applies to H rows only. A K entry is never D.

**Rule 2: S, ST-only meaning.** The meaning, or the **valid value set**, can be stated only through ST's documents. Examples:
- an ST enumeration or numbering (alternate-function numbers, VOS scales, `OSPEEDR` speed grades);
- an ST structure letter (FT, TT, the pin-structure codes of a DS table);
- an ST register or bit field, or an ST clock-tree mux position.

A neutral name over an ST value set is S.

**Rule 3: N, neutral meaning, ST name.** The meaning and the valid value set can both be stated in units, a published standard, or generic electronics. The test question: "the frequency of the external high-speed crystal, in Hz". **Only the name** (key, variant label or kind string) is ST's vocabulary, such as RCC, HSE, LSE, MCO, VOS, AF, BOR, `PB9`-style port names or a DS/RM document number. A rename alone would make it V.

**Rule 4: V, vendor-neutral.** Neither name, meaning nor value set is ST's.

**Tie-breaks:**
- **a. Unsure:** an entry that could be in two classes takes the more coupled one (S over N over V) and is flagged `?` with the doubt in one line.
- **b. Pin identifiers:** a key whose values are pin names in ST's `P<port><n>` form is classed by what the code does with the name.
  - **N** if the name is only matched against a table.
  - **S** if the code parses the port letter or number for meaning, for example a port's register block or EXTI line.
- **c. Value sets from a held table:** a value set read from a held table (part strings, pin names) leaves the key's class to its name and meaning, and the table is classed separately. A value set hard-coded in source (an enum, match arms, literals) is schema, and is classed by rules 2 to 4.
- **d. Other part vendors:** coupling to another part vendor (TI, Bosch, TDK, Murata, Winbond, …) is not ST coupling. Such an entry is classed V for this audit and tagged `other-vendor`.
- **e. Core architecture and standards:** Arm Cortex-M architecture concepts (SWD, the vector table, NVIC) and published standards (I2C, SPI, UART, DShot, JEDEC, IPC) are not ST. They are classed V and tagged `arch` or `std`.
- **f. Citations:** a key or column whose value is a citation (a document and page) is V, even when the documents it cites are ST's. What is cited is data.
- **g. Contained keys:** each key is classed on its own. A neutral key inside an ST-named container is V, and the container is classed separately.

## 3. Held tables

- **A table** is classed by its schema, its columns, not its rows.
- **Columns:** each column is classed by rules 2 to 4.
  - If every column is V or N, the table is **D**.
  - If any column is S, the table is **S**, and the S column is named.
- **Citations:** every table entry carries its own citation to a document and page, copied from the table itself. Where the table cites no page, the entry says "no page cited", and that is a finding.

## 4. The RP2350 column

- **Who gets one:** every N and S entry.
- **What a cell holds:** one of
  - **an equivalent**, with document and page;
  - **"none"**, with document and page;
  - **"not held"**.
- **Source:** only a held copy of the RP2350 datasheet. Never memory, never a fetch, never a KiCad symbol or another chip's port.

## 5. The count

- **Per class:** for each of K, R, H (and H columns), and T separately.
- **Per version:** for each K entry, the version that introduced it, taken from `versions::CHANGES` where it reaches. Older keys are bucketed "at or before vN", or dated by `git log -S`; the method used is stated with the table.

## 6. Amendments

| Date | Change | Reason |
|---|---|---|
| 2026-11-22 | **1.** H in source is every `const` or `static` item outside `#[cfg(test)]`, a scalar as a one-row table. Each is classed by §3 as a table with one column, its value: D when a second vendor would add rows (a list of ST peripheral names), S when the item itself is an ST register, bit or enumeration (`CFG2_SSM`), V when it is an engine limit or plumbing | §1 said "a `const`/`static` table". The work order's own examples include register and bit names, which the crate holds as scalars (`silicon.rs`, `vos.rs`). Made after the reader first ran and before any entry was classed |
| 2026-11-22 | **2.** ST facts held in a function's `match` arms or literals (a lookup in code, not a table) are not an entry unit. They are listed from a grep in FEEDBACK, one line per function, and not classed | Not a key, a refusal kind or a table. Listing them keeps the count from reading as the whole coupling. Made before classing |
