# AET-WO-2310 §1.1 — results, measured after the predictions (`4264c5c3`)

Main is `15798f91`. Every run used main's CLI, built from a `git archive` of `15798f91` (`~/.cache/aet-2310/main`),
in the pinned tools image `aethelgard-tools@sha256:e5fc7ddb…` under `capped-run` (memory.max 8 GiB, swap 0,
pids 8192, read back each time; `~/.cache/aet-2310/caps.log`), nice 10, one container at a time.
**Nothing in `crates/` changed. No byte of `fab/` or `scenarios/` changed.** Labels: **measured**, **read at
source** (file:line on `15798f91`), **inferred**.

## Verdict against each prediction

| # | Held? | Measured |
|---|---|---|
| P1 | **yes** | Sweep A (`out/sweep-a.txt`): the four `spike/3305` archives are refused `UnknownVersion` "unknown format version 14 in header", exit 3. The `spike/2309` flood archive replays exit 0, its output byte-identical to 2309's `branch-full` line |
| P2 | **yes, but one clause was wrong** | Sweep B: 64 archives; **all 64 replay exit 0 with the same `verdict`, `expect`, `matched`, `findings` and `expect_findings` as their campaign lines** (`out/sweep-b.txt`). Every committed scenario file but `cases.json` is in a campaign line. **Wrong:** "every line is `matched: true`". 18 are false, all mutant runs, by construction (2309's recipe: a mutant runs scenarios that expect the unmutated outcome): `two-0x75`, `two-0x77`, and 16 lines over the 13 seeded mutants. The other 46 are true |
| P3 | **yes** | All 64 campaign lines say `"window":"MAX_VIRTUAL_NS"` |
| P4 | **yes** | Item 7's census (`out/census-b.txt`): 63 runs read and booted, 1 read and not booted (`seeded-no-boot-word`). **0** `ok` reads with an unread witness, on any run. Kind 12 and 21 statuses seen: `ok` and `timeout_txis` only. The flood archive's are `invalid` |
| P5 | **yes** | Eight committed scenarios plant one instant each (table below). Everything else in the format is a duration or a level |
| P6 | **half** | **Every droop is before its board's undisturbed boot word: yes.** Minimal 3,627,610 ns, bench 3,630,040, dshot 3,623,630, all in 3.6–3.7 ms as predicted. **Wrong:** sentinel-03's boot word is at **43,929,480 ns** (not about 4.8 ms), and sentinel-02-power's at **81,602,740 ns**. So **both sags are also before the boot word**: every one of the eight offsets is negative |
| P7 | **yes** | Both `simulate` runs of each base give the same `boot_word_ns`. For each of the four `droop-bor3`s it equals the archive's kind 32 `boot_word_ns`; `motors/dshot600-droop`'s extra pull-up override does not move it. For the no-reset runs, kind 29's first injection is that boot word plus `QUANTUM_NS` (100,000): 3,727,610 on minimal `droop-tolerated`, 44,029,480 on sentinel-03 `droop-tolerated` and `battery-sag-tolerated`. So those runs' own boot is the undisturbed one. `sentinel-02-power` has no serial port and no kind 29; its sag never reaches Renode (read at source: kind 31 only), so its boot is the base's (inferred) |
| P8 | **yes** | Every instant and every boot word is a whole number of 10 ns instructions, so `boot_word_ns + offset = start_ns` exactly, on the same instruction, for all eight. It needs a **signed** offset and the **undisturbed** boot's word (the disturbed boot's is after the release: 4,859,310 on minimal `droop-bor3`) |
| P9 | **wrong in its cost, on the campaign's Renode path only** | Read at source. On the campaign's Renode path, no committed scenario needs an extra boot. A sag's chain never feeds the boot (kind 31 only), so it can be solved after the run's own boot, from that boot's word. A droop that renders no reset needs the undisturbed boot *before* the rail solve, and that boot **is** the run. A droop that renders a reset already runs the undisturbed boot (`main.rs` L1997), now after the rail solve (L1949). **What changes is the order.** **But some checks run before any boot today, and they read the instant:** `droop_end_ns` (chaos `campaign.rs` L617), `check_battery_sag`'s start bound (`battery.rs` L108), `solve_battery_sag` (`cli/src/campaign.rs` L263, `BatterySagSolvedBelowFloor`), and `verify`'s tool-less coverage stage, which resolves a sag (`verify.rs` L1111). Under a boot-word anchor each of them can check only the offset before the boot. The check on the resolved instant then waits for a boot: a refusal that costs no Renode today would cost one |
| P10 | **yes** | Read at source: the 4241 §0a instants are firmware spin counts in `twin_tier2.rs` (L1682, L2341–2352, L2429, L2601) and pinned expectations in `campaign_tier2.rs` (L705–712, L949). None is a scenario field |
| P11 | **yes** | Below |
| P12 | **yes, two details off** | Below. `rustc` on `PATH` is rustup's proxy, but 21,347,088 B, not small (the same file as `cargo` and `rustup`). The real `rustc` is 648,536 B, and the compiler is `librustc_driver` (158 MB) plus `libLLVM` (208 MB). The tree is 718 MB; its digest takes **0.25–0.30 s**, not 2–10 s |
| P13 | **yes** | Read at source: `campaign` takes `--firmware` and builds nothing (`cli/src/campaign.rs` L995) |

## (3) The planted-event mapping

The instruction is ns/10, at 100 MIPS (`anchor::instant`). "Boot word" is the undisturbed boot's, from sweep M
(`simulate --for 100ms` of the base, twice), cross-checked against kind 32 where the run has one.

| Scenario | Base (firmware) | Planted | Instant ns (instr.) | Boot word ns (instr.) | Offset ns (instr.) | Cross-check | Re-expressed lands |
|---|---|---|---|---|---|---|---|
| `sentinel-01/droop-bor3` | sentinel-01-minimal | droop start | 1,000,000 (100,000) | 3,627,610 (362,761) | **−2,627,610** (−262,761) | kind 32 3,627,610 | same instr. |
| `sentinel-01/droop-tolerated` | sentinel-01-minimal | droop start | 1,000,000 (100,000) | 3,627,610 (362,761) | **−2,627,610** (−262,761) | kind 29 3,727,610 | same instr. |
| `sentinel-01-bench/droop-bor3` | sentinel-01 | droop start | 1,000,000 (100,000) | 3,630,040 (363,004) | **−2,630,040** (−263,004) | kind 32 3,630,040 | same instr. |
| `motors/dshot600-droop` | motors-dshot600 | droop start | 100,000 (10,000) | 3,623,630 (362,363) | **−3,523,630** (−352,363) | kind 32 3,623,630 | same instr. |
| `sentinel-03/droop-bor3` | sentinel-03 | droop start | 1,000,000 (100,000) | 43,929,480 (4,392,948) | **−42,929,480** (−4,292,948) | kind 32 43,929,480 | same instr. |
| `sentinel-03/droop-tolerated` | sentinel-03 | droop start | 1,000,000 (100,000) | 43,929,480 (4,392,948) | **−42,929,480** (−4,292,948) | kind 29 44,029,480 | same instr. |
| `sentinel-03/battery-sag-tolerated` | sentinel-03 | sag start | 10,000,000 (1,000,000) | 43,929,480 (4,392,948) | **−33,929,480** (−3,392,948) | kind 29 44,029,480 | same instr. |
| `sentinel-02-power/battery-sag-tolerated` | sentinel-02-power | sag start | 10,000,000 (1,000,000) | 81,602,740 (8,160,274) | **−71,602,740** (−7,160,274) | simulate ×2 only | same instr. |

What follows from the instants (each moves with its start, since ngspice solves the same deck): the resets
rendered (`reset@1018330A`/`1231690R` on both Sentinel-01 boards, `118330A`/`331690R` on dshot,
`1044580A`/`1644090R` on sentinel-03) and the sags' kind 31.

**What the table says that the WO did not expect.**
- **Every committed planted event is before the boot word, by 2.6 to 71.6 ms.** The droops were authored when
  the minimal board's boot word was at 26,620 ns (4241 §0a). A droop at 1 ms was then 973,380 ns *after* the boot
  word. 4241's identity reads moved the boot word past every one of them, and no verdict said so.
- **Relative to the boot word, a pre-boot event is brittle the other way.** A later boot word moves each
  re-expressed droop later from reset, by the same shift. A shorter boot can put sentinel-02-power's sag
  (−71.6 ms) before reset, which must then be refused by name. Relative to reset, the same instants never moved.
- **Re-expressing with the signed offsets above keeps every instant, so no committed verdict should move.**
  That is inferred from P8's identity; the build's replay proves it. The other reading, re-planting the droops
  where they were authored (on the minimal board, +973,380 ns after the boot word), would move every droop into the running firmware.
  Its verdicts are unmeasured, and it would be a §7 stop.

## (5) The tool hashes and where they come from

**libngspice** (measured in the image):
- The bridge's loader maps `/lib/x86_64-linux-gnu/libngspice.so.0` (`ldd`; `/lib` is `/usr/lib` merged). It is a
  symlink to **`libngspice.so.0.0.12`**, 8,858,472 B.
- sha256 **`8b24e526865425243ee57efa9dcbdde99e7220393eb77a1ab863e849a2e0d761`** (18 ms). Its md5 `ebc7c2c1…9834`
  equals the one `dpkg` recorded in `libngspice0:amd64.md5sums`.
- Owner: `libngspice0` 44.2+ds-1 amd64.
- **Where from (inferred from the apt and dpkg logs):** `kicad/kicad:10.0.7`'s own apt run, 2026-10-18 13:59 (`/var/log/dpkg.log` L1511–2099; the
  first `history.log` command line). The Dockerfile's pinned line (L160–161, `libngspice0=44.2+ds-1`, run
  2026-11-08) installed nothing new for it: `dpkg.log` holds that one install. The apt lists are cleaned, so the
  mirror kicad used is not on record in the image. The source tarball is pinned
  (`TOOLS-DEBIAN-GPL2.tsv` L187–190). The `.so`'s bytes are pinned only transitively, by the base image's digest
  (`FROM kicad/kicad:10.0.7@sha256:3224d1bf…`, Dockerfile L129); no pin names the file itself (read at source).
- The identity the header archives today is `ngspice` = `44.2`, and the banner check (`analog.rs` `identified`)
  is the library's self-report (1606 F4).

**The toolchain** (measured in the image):
- `rust-toolchain.toml` names 1.99.1; `rustc -vV`: `1.99.1 (58cf8f9c3 2026-10-13)`, LLVM 23.1.1. The image also
  holds `nightly-2026-08-21`, which is not active.
- `PATH`'s `rustc` is `/opt/cargo/bin/rustc`, rustup's proxy: 21,347,088 B, sha256 `b1c8cb14…6c55`, the same bytes
  as `cargo` and `rustup`. `rustup-init` 1.29.1 is pinned by sha256 in the Dockerfile (L178–185).
- The sysroot is `/opt/rustup/toolchains/1.99.1-x86_64-unknown-linux-gnu`: 173 files, 718,181,603 B.
  - `bin/rustc` 648,536 B (`b53b780f…6853`). It `NEEDED`s `librustc_driver-7ed0375ae352bd75.so`, 158,240,640 B
    (`5a2cd52e…02c7`), and that loads `libLLVM.so.23.1-rust-1.99.1-stable`, 208,167,760 B.
  - **A hash of `rustc` alone is the adversarial pass's "hash that does not change when the bytes do":** the
    compiler is in the two libraries.
  - The firmware's target library is `lib/rustlib/thumbv7em-none-eabihf`, 72,172,351 B. The linker is
    `lib/rustlib/x86_64-unknown-linux-gnu/bin/rust-lld`: the target's default, with only `-Tlink.x` added by the
    board's `.cargo/config.toml` (inferred; the target spec cannot be printed on stable).
- A whole-tree digest (sha256 of the sorted per-file `sha256sum` list): `a362f61c…5d6c`, 0.30 s and 0.25 s,
  identical over two runs.
- **Provenance:** `lib/rustlib/multirust-channel-manifest.toml` (dist 2026-10-15) holds the static.rust-lang.org
  tarballs' sha256, and `lib/rustlib/components` names the six installed components. That is where the tree came
  from, not what the tree is.
- **Not bound to the ELF.** `campaign` judges an ELF it did not build (P13). Today only `build`'s verdict records
  the toolchain (`verdict.rs` L887, `.rustc_info.json`), outside the fence. A digest taken at campaign time is the
  host's toolchain, which built the ELF only when the same host built it. That is the ruling question in
  `shapes.md`.

## Out-of-fence authors the scenario change would break (read at source)

Droop or sag scenarios authored with `start_ns`/`bor_level` outside the fence: `cli/src/verify.rs` (L2696, L2725,
L2994, L3137; 1607's file), `cli/tests/fab_tier1.rs`, `cli/tests/board_tier2.rs`; and `cli/src/analog.rs`
(L370, L484, L925), which hands the resolved instant to the bridge. In the fence: `chaos/tests/{battery,droop,
campaign,pin_devices}.rs`, `cli/tests/campaign_tier1.rs`, `cli/tests/campaign_tier2.rs`.

## Files

- `sh/img.sh` (2309's, re-pointed), `sh/measure.sh` (2309's groups less 3306's `bme-bmp`, plus sweep M),
  `sh/replay-b.sh`, `sh/compare.py`, `sh/tools.sh`, `sh/tools2.sh`; `census/` (`census2310`, through `replay_dir`
  alone). Sweep A ran `aethelgard replay --archive` over `git ls-files '*/trace.cbor'` in the image.
- `out/campaign/*.jsonl`: main's 64 campaign lines, the "before" for acceptance (the branch will refuse these
  trace-18 archives, so these lines are the only record of their verdicts).
- `out/sweep-a.txt`, `out/sweep-b.txt` (compare), `out/sweep-b-replay.txt`, `out/census-b.txt`,
  `out/sweep-m.txt`, `out/sweep-t.txt`.
