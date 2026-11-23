# AET-WO-2310 §1.1 — predictions, written before any build or run

Committed before anything is built or run. Main is `15798f91`. Nothing in `crates/` changes for this measurement.
Every run is main's CLI, built from a `git archive` of `15798f91`, in the pinned tools image (`container/IMAGE`)
under `capped-run` (8 GiB, swap 0, read back), nice 10, one container at a time (`sh/img.sh`, from 2309's).

## What counts as a verdict (so §7's condition is decidable)

A campaign line's or a replay's **verdict fields**: `verdict`, `expect`, `matched`, `findings`, `expect_findings`,
`lifecycle`, `conditions`, `uid_boundary` (and `uid_boundary_unmet`), `observation_witness`, `battery_sag` (but
its `start_ns`), and the per-kind counts replay judges (`expect_counts`' kinds).

**May move without a stop:** `window` (removed, item 2); the header's tool entries (item 5); a planted instant's
own field (item 3); the droop's `bor_level` in the scenario (item 1); and what those move by construction:
`run_id`, `trace_bytes`, `event_count` only if a record is added or removed, `state_sequence_hash` only through a
record that changed, and `artifacts.scenario` (the scenario's bytes change).

## Sweeps

| Sweep | What | By |
|---|---|---|
| A | The committed archives: `git ls-files '*/trace.cbor'` (five) | `aethelgard replay`, main's CLI |
| B | Current-version archives made once by main's CLI over every committed scenario group and the seeded set's mutants (`sh/measure.sh`, 2309's groups, the 3306 `bme-bmp` groups dropped: they no longer generate) | `aethelgard campaign`, then `replay` of each archive |
| M | For (3): each planted scenario's undisturbed boot word, from `aethelgard simulate --for 100ms` of the scenario's own base specification (and firmware), run twice | `simulate`'s `boot.boot_word_ns` |
| T | For (5): what libngspice and the toolchain are on disk in the image, and where they come from | `ldd`, `realpath`, `dpkg -S`, `dpkg` md5sums, `sha256sum`, `rustup`, `du`, `time` |

## Predictions

| # | Prediction |
|---|---|
| P1 | **Sweep A.** The four `spike/3305` archives are trace 14: refused `UnknownVersion` ("unknown format version 14 in header"), exit 3. The `spike/2309` flood archive is trace 18: replays exit 0, its output byte-identical to 2309's recorded `branch-full` line (`verdict` fail, `matched` false, 11 findings, `event_count` 39, `trace_bytes` 3034, `state_sequence_hash` `eeedf797…`) |
| P2 | **Sweep B.** 60–70 archives. Every committed scenario file but `seeded/cases.json` appears in at least one campaign line. Every line is `matched: true`; every archive replays exit 0 with the same verdict fields as its campaign line |
| P3 | Sweep B: every campaign line's `window` is `"MAX_VIRTUAL_NS"` (item 2's ground) |
| P4 | Sweep B, item 7's census: on a read boot, no `ok` kind 12 or 21 record has its kind 13 or 22 witness unread without `{bus}_transaction_unwitnessed` among the findings: **0**. (The flood archive, sweep A, holds unread witnesses and carries `i2c_record_invalid` and `spi_record_invalid`, not `*_transaction_unwitnessed`: its records are not `ok`.) |
| P5 | **The planted instants (3).** Eight committed scenarios plant one each: six droops' `start_ns` (five at 1,000,000 ns, `motors/dshot600-droop` at 100,000 ns) and two battery sags' `start_ns` (10,000,000 ns). No other scenario field is an absolute instant: `fall_ns`, `hold_ns` and `rise_ns` are durations |
| P6 | **Every droop is planted before its board's undisturbed boot word.** Boot words (sweep M): `sentinel-01-minimal` 3,627,610 ns (4241 §0a: 362,761 instructions); `sentinel-01` (bench) and `motors-dshot600` within 3.6–3.7 ms; `sentinel-03` about 4.8 ms. So every droop offset is **negative** (about −2.6 to −3.8 ms), and both sag offsets are **positive** (about +5 to +6.4 ms). Since `4241`, the droops no longer fall after the boot word as they did at 26,620 ns: the boot shift moved them into the boot, silently (no verdict moved, so no test said so) |
| P7 | Sweep M is deterministic: both runs give the same `boot_word_ns`, and for each `droop-bor3` it equals that archive's kind 32 `boot_word_ns`. For `droop-tolerated` and the sags (no reset), there is no kind 32; the run's boot is the undisturbed one |
| P8 | **Re-expression lands on the same instruction for all eight**, because the arithmetic is an identity on `main`: `boot_word_ns + (start_ns − boot_word_ns) = start_ns`, and every instant is a whole number of 10 ns instructions. It holds only if the offset is **signed** and the boot word is the **undisturbed** boot's (the disturbed boot's is after the release). An unsigned "after the boot word" offset cannot express any committed droop |
| P9 | The cost of the reference: today the undisturbed boot runs only when a reset is released (`main.rs` L1997), after the rail is solved (L1949). Planting from it needs it before the rail solve, for every droop and every sag: one more Renode boot for each `droop-tolerated` (3) and sag (2) scenario. The sag's chain never reaches Renode, so for a sag the boot runs only to place the instant |
| P10 | **Test-planted instants (4241 §0a) are not scenario fields.** `twin_tier2.rs` L1682 (pair 111,061), L2341–2352 (spins 2,529,967 / 2,779,967 / 3,029,967), L2429 (settle 1,879,973), L2601 (spin 3,109,973) are spin counts inside test firmware; `campaign_tier2.rs` L705–712 and L949 are pinned expectations (3,627,610 / 3,727,610 / 4,959,310 / 3,700,990), not plants. None is re-expressible by the scenario format; `twin_tier2.rs` is outside the fence |
| P11 | **libngspice:** the bridge maps `/usr/lib/x86_64-linux-gnu/libngspice.so.0`, a symlink to `libngspice.so.0.0.x`, owned by `libngspice0:amd64` 44.2+ds-1; its dpkg md5 matches the file. It comes from the `kicad/kicad:10.0.7` base layer (Dockerfile L125 says the base already carries 44.2+ds-1), so the snapshot pin at L160–161 resolves to what is installed and fetches nothing |
| P12 | **The toolchain:** `rustc` on `PATH` is rustup's proxy (`/opt/cargo/bin` or alike); the real `rustc` under `/opt/rustup/toolchains/1.99.1-x86_64-unknown-linux-gnu/bin` is under 1 MiB and the compiler is `lib/librustc_driver-*.so` (over 100 MiB). **A hash of `rustc` alone is the adversarial pass's "hash that does not change when the bytes do".** The toolchain tree is 0.5–1.5 GiB; a sha256 of it takes 2–10 s. The firmware is linked by `rust-lld` from that tree |
| P13 | `campaign` takes an ELF and builds nothing (`campaign.rs` L995), so a toolchain hash taken at campaign time is not bound to the ELF it judges. That is a ruling question, posed with the candidates, not picked here |
