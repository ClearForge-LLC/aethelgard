# AET-WO-3308 §1.1: the predictions against the runs

The runs were made in the pinned image `ghcr.io/clearforge-llc/aethelgard-tools@sha256:1e0ab105…` under `capped-run`: 8 GiB, and 16 GiB for verify, with swap 0. The CLI was built at `b9d5f4b6` (`da6c4ad7` adds only this directory).

| | Predicted | Ran | Held? |
|---|---|---|---|
| P1 | the ten hashes equal at `ac5f8c82`, `ee835bde`, `cd79024c` | equal at those three and `b9d5f4b6`; only `ac5f8c82` ever touched the ten paths; all equal MANIFEST's (`ordered-hashes.txt`) | yes (run before the prediction file; the WO carried it) |
| P2 | `verify --against-fab` passes: HEAD re-derives `f3e871bb` | **16 of 17 files re-derive byte for byte, the ELF `f3e871bb…52c4` among them; `MANIFEST.json` differs** (`against_fab: differs`) | **in part**: see below |
| P3 | test (3) fails with no full hash, not "naming `008a2a59`" | `full hashes found: []`; it lists `L5 008a2a59…`, `L92 f3e871bb…`, `L97 78212e91…`, `L224 e4e24372…`, `L259 452d906c…` (`test3-draft.out`) | yes. The draft does not yet set the two non-ELF digests aside: that is D3's rule |
| P4 | the twin: boot word, records 0–2, console pattern, the 25 stored registers | `simulate-f3e871bb.json`, verdict `pass`: `0x2100b007`; U2 ok `0x60`; U4 ok `0xef4017`; U3 `timeout_txis` `identity_not_witnessable`; the 12 bytes; 25/25 registers equal `BENCH.md`'s words under their masks; no PWR or FLASH row stored | yes |
| P5 | every value holds; citations move only; the S rows are text | `ROWS.md`: 0 class D. B2's 44 rows regenerate identically from the twin and the ledger (`b2-regenerated.md`). The supply rows hold against the ledger. The SCK and SCL figures hold against `silicon` | yes |

## P2's `MANIFEST.json` difference

**What was shown.** The derived manifest equals the committed one in every byte except `verdict_sha256`.
- Replacing the committed `b33470720d8c…` with the local verdict line's hash gives exactly the derived file's sha256, `d108981a…`.
- The local line's hash is `4dcb0bea…`, the line before the fab comparison, with `ok: true`.
- Every package file, the ELF included, is the committed one. So HEAD's generated crate builds `f3e871bb`, and generator rows are judged against HEAD's source.

**What was not found:** which field of the verdict line differs from CI's.
- The local line carries `uid_boundary: false`, `uid_boundary_unmet: user_namespace` on 17 gates (rootless podman). Substituting `toolchain`, `tool_uid`, or `uid_boundary: true` does not give `b33470…`, so something else differs too.
- CI's bench-package log does not print the verdict, so the two lines cannot be diffed.
- `ci.yml` (bench-package) already says the committed package is the one CI writes, "never one written on another machine". `main`'s own `--against-fab` passed at `7dec1d7b` (run 45153863481).

**So this is a reported limit, not a stop:** no `fab/` byte would move, and no prediction rests on `verdict_sha256`. Making the verdict line machine-independent is a backlog item for FEEDBACK.

## GNU `sha256sum` and a `#` header (D1)

The image's `sha256sum (GNU coreutils) 9.7` reads a list with two `#` lines above the entries: `OK` for each, exit 0, also with `--strict`. So `cd fab/sentinel-01-bench && sha256sum -c ORDERED.sha256` would check the list by hand, header and all.
