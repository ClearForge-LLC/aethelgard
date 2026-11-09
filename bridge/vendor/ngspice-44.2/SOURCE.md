# Vendored from ngspice 44.2

`AET-WO-2200` §1.3 and ruling L3 on PR #66. The bridge declares libngspice's
interface by hand against this header, because the tools image ships
`libngspice0` 44.2 (`44.2+ds-1`) without `libngspice-dev` (`AET-WO-2101` F1).
`bridge/src/header.rs` checks every declaration of `bridge/src/ffi.rs` against
it, and `crates/aethelgard-cli/tests/workspace_shape.rs` checks the hash below.

- Repository: `https://sourceforge.net/p/ngspice/ngspice/`
- Tag: `ngspice-44.2` (commit `80d22fc8`, 2025-01-11)
- Fetched: 2026-11-09, from the tag's raw tree (`…/ci/ngspice-44.2/tree/<path>?format=raw`)

| File | Path at the tag | sha256 |
|---|---|---|
| `sharedspice.h` | `src/include/ngspice/sharedspice.h` | `4041c4e257e45baf4e6983e2fd981b1a2e975f082734a53b502d5bb6ea877c43` |
| `COPYING` | `COPYING` | `df9e98a686fad462feeff23d780d733bce3d615e351938ac86ef8119d707da20` |

The header's own licence line is "Modified BSD license"; `COPYING` is ngspice's
licence statement, which adopts the Modified BSD licence for its source. Neither
file is modified.
