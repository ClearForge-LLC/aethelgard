# AET-WO-1605 FEEDBACK — the tools image's Renode with upstream's NVIC fixes

## §1.1, before any measured run: what was found, and the predictions

### What was found (read at source; not yet measured on a run)

1. **No Renode release carries the fixes.**
   - `v1.17.0` (2026-09-07) is Renode's latest release (`gh api repos/renode/renode/releases`).
   - Both fixes are in `renode-infrastructure` after it.
2. **The fixes are three upstream commits, not two (a correction to the WO's grounds).**
   - `7b687540` (cache maintenance) rewrites a block that `c8b2075c` ("NVIC: Add DCCMVAC and DCIMVAC stubs", 2026-11-09) added.
   - Applied in order (`e7a8584c`, `c8b2075c`, `7b687540`), all three apply unmodified to 1.17.0's infrastructure, as one change to `src/Emulator/Cores/Arm-M/NVIC.cs` (+17 −3).
3. **The official asset was not built from the `v1.17.0` tag.**
   - Its build string `1.17.0+20260907gitf1dd1b4af` names renode `f1dd1b4af` ("Add Changelog for v1.17.0"), two commits before the tag `ab721d88`.
   - At `f1dd1b4af`, `src/Infrastructure` is `cd4b002a`, the commit `4400e3`'s hook cites. At the tag it is `066a7f13`, two MappedMemory commits later.
   - `TOOLS-SOURCES.md` cites the tag and `066a7f13`. So the image's written source offer names source two commits off the shipped object code. This is a finding for the stop.
4. **NVIC lives in one assembly, `Infrastructure.dll`.** The asset is a self-contained .NET 8 publish (runtime `8.0.12`) of separate assemblies, so the smallest route rebuilds only `Infrastructure.dll` from source.
5. **The control rebuild reproduces Antmicro's assembly.**
   - Inputs:
     - renode `f1dd1b4af` with its submodules, infrastructure `cd4b002a`;
     - `renode-resources` `14b80cde`, the `master` the build cloned on 2026-09-07. Its five referenced assemblies are byte-identical to the asset's.
     - .NET SDK `8.0.405` (`mcr.microsoft.com/dotnet/sdk@sha256:da1f981a…`);
     - 86 NuGet packages, recorded with their sha512.
   - Built at Antmicro's path `/ci/src/renode`, the result is the official `Infrastructure.dll` byte for byte, outside the deterministic identity fields (72 bytes):
     - the COFF and debug-directory timestamps;
     - the MVID;
     - the RSDS GUID;
     - the PDB checksum.
   - The IL and metadata are identical. The build takes 11 s.
6. **`--version` cannot tell the patched image from the stock one.** The build string comes from `Renode.dll`'s `InformationalVersion` (`Renode.csproj` L94), which this route leaves alone. That is a ruling item.

### Predictions (written before the measured runs)

Images: `scratch-1605-control` and `scratch-1605-patched`. Each is today's pinned tools image with only `Infrastructure.dll` replaced: the control rebuild, and the rebuild with the three patches.

1. **`RETTOBASE` (`4400e3`'s fixture, nested over an external interrupt):** control with no hook reads 1; patched with no hook reads 0; both read 0 with the hook. The hook changes nothing on the patched image.
2. **The cache-maintenance lines:** on the patched image, ArduPilot's run logs no NVIC `Unhandled write` kind. Every other kind and count is unchanged.
3. **Pinned virtual-time figures: none moves.**
   - The patch changes two things: an `ICSR` bit, read only in a handler nested over an external interrupt, which no generated firmware arms; and how cache-register writes log.
   - Logging takes no virtual time.
   - `4400e2`'s moves came from a development build carrying 517 other commits, not from these.
4. **Generated-firmware verdicts:** byte-identical between the stock image and the patched one (outside the memory sample), with and without the hook.
5. **ArduPilot under the foreign judge:** it passes on the patched image without the hook, every witness time identical to today's. `log_kept_bytes` falls by about the NVIC kinds' first lines.
6. **Wall time** falls on the patched image without the hook, with no Python hook on `ICSR` reads. Peak RSS is within 5 %.
7. **Our C# models** load unchanged; only `NVIC.cs` changed.
8. **On the patched image,** the pin test `renode_1_17_0_reads_rettobase_1_in_a_handler_nested_over_an_external_interrupt` fails, by design: the bug is gone. Every other tier-2 test passes.

## §1.1, measured (2026-11-18 03:49–07:45 UTC): STOPPED for the ruling

**Where:** Furnace, rootless podman, every container through `~/bin/capped-run` (8 GiB, swap 0, read back), one at a time.

**Images.** Each is today's pinned image (`container/IMAGE`, `…598501ab…`) with only the named assemblies copied over. All are tagged `localhost/aethelgard-tools:scratch-1605-*` and none was published.

| Image | `Infrastructure.dll` | `Renode.dll` | `renode --version` build line |
|---|---|---|---|
| stock | official (`02b3510b…`) | official (`b2087934…`) | `1.17.0+20260907gitf1dd1b4af` |
| control | rebuilt, no patch (`bdee9b85…`) | official | the same |
| patched | rebuilt with the three commits (`9b42991c…`) | official | the same |
| repro | the staged script's (`d878080b…`) | official | the same |
| ver2 | `d878080b…` | republished with build string `…gitf1dd1b4af.aet1605` (`c37d923b…`) | `1.17.0+20260907gitf1dd1b4af.aet1605` |

**CLIs.** Two builds of this branch's CLI were used:
- **main:** this branch's CLI as is, hook included.
- **no-hook:** a scratch copy of the tree with both `RETTOBASE_HOOK.to_owned(),` lines deleted (`renode.rs` L823, `foreign.rs` L553). It was never committed.

**Which DLL each run used:**
- Tier 2, E3 and E4 ran on `9b42991c`.
- The staged script builds `d878080b`.
- The two differ only in the 72 bytes of deterministic identity; their IL and metadata are byte-identical. The cause is SourceLink:
  - `9b42991c` was built in a full clone with remotes, so its PDB carries a SourceLink map. That map points at upstream `cd4b002a` for a file that had been patched.
  - The staged script's shallow fetch has no remote, so its PDB carries none.
  - No date or environment input feeds the build: no `UtcNow`, `BuildDate` or `PathMap` property anywhere in the Infrastructure project chain. `GUI_DISABLED` is the only environment condition, and no build here sets it.
- On the `d878080b` image (repro and ver2), I re-ran the witness and ArduPilot (below). Both match the `9b42991c` runs.

### The choice: a patched build (no release carries the fixes)

1. **No release.** `v1.17.0` is still Renode's latest (finding 1).
2. **The route measured:**
   - Keep Antmicro's portable asset (unchanged URL and sha256).
   - Rebuild one or two assemblies from the source that asset was built from: renode `f1dd1b4af`, `src/Infrastructure` `cd4b002a`, with the three commits applied.
   - Copy them over the asset.
3. **Reproducible, measured:**
   - `~/.cache/aet-1605/stage/build-infrastructure.sh`, with its `patches/` and `nuget.manifest` (staged for §1.2, not committed yet):
     - fetches every input by commit, shallow, and checks `HEAD`;
     - applies the three patch files;
     - restores and checks every NuGet package against a committed sha512 manifest (86 packages);
     - builds at Antmicro's path `/ci/src/renode` in `mcr.microsoft.com/dotnet/sdk:8.0.405@sha256:da1f981a…`.
   - From an empty container it gave `d878080b…` three times out of three. Each run took 29 s of wall time, fetch included.
   - The same script without the patches reproduces Antmicro's `Infrastructure.dll` except for the 72 identity bytes (finding 5).
4. **The version string (finding 6) is the decision.** It is measured both ways:
   - **(i) `Infrastructure.dll` only.**
     - `renode --version` stays `1.17.0+20260907gitf1dd1b4af`.
     - Every archive header's `tools` entry (`aethelgard-chaos` `Header.tools`, `AET-WO-2205`) would then name the stock build for a Renode that is not stock.
     - `smoke.sh` L47 checks only `Renode v1.17.0`, so with no new version the WO's acceptance line "its smoke test reads the new version" cannot hold.
   - **(ii) Republish `Renode.dll` too** (`-p:BuildDate=20260907 -p:CommitString=gitf1dd1b4af.aet1605`).
     - It is published exactly as `build.sh` L714 does (`dotnet publish -r linux-x64 -f net8.0`, `PORTABLE=true`, `Platform=AnyCPU`).
     - With the stock `CommitString`, the result is Antmicro's `Renode.dll` except for the same 72 identity bytes (offsets 136, 311104, 578724, 578808, 578894).
     - The stamped one reproduces (`c37d923b…`). It needs one more submodule (`lib/InpliTftpServer` `2a4877dc`, BSD-style) and 19 more NuGet packages (105 in all).
     - The publish took 5–7 s.
     - `renode --version` reads `build: 1.17.0+20260907gitf1dd1b4af.aet1605`.
   - **Recommended: (ii).** It is the only route on which the image says what it is: in `--version`, in every replay archive, and in the smoke test. Its cost is the archive bytes listed below.
5. **Cost:**
   - **Image size:** the overlay as a `COPY` layer adds 9.73 MB for (i) and 10.42 MB for (ii), measured on the scratch images. Extracting the asset in the builder stage and copying `/opt/renode` whole would make the net change about 0 [inferred].
   - **Build:** about 40 s in CI plus one pull of the 846 MB SDK image, used by the builder stage only.
6. **Licence obligations (`TOOLS-SOURCES.md`):**
   - The rebuilt code is MIT. `NVIC.cs` and the rest of `renode-infrastructure` carry `licenses/MIT.txt`, and `Renode.dll` is MIT too. The notice already ships in `/opt/renode/licenses`.
   - No LGPL part is modified: tlib, Packet.Net, GTK# and NetMQ are byte-for-byte the asset's.
   - The written offer must change, because today's is wrong as well as stale (finding 3). It must cite:
     - renode `f1dd1b4af`;
     - `src/Infrastructure` `cd4b002a` (its tlib stays `167decf9`, measured);
     - the three patch files, committed under `container/` and cited by upstream commit.
   - For (ii), it also cites `InpliTftpServer` `2a4877dc`.
7. **TIM5:** not the same change. The three commits touch only `NVIC.cs`.

### Measured, old against new

**E1, the `4400e3` witness** (`tests/fixtures/rettobase`, twin tier 2):

| Image | `renode_1_17_0_reads_rettobase_1_…` (pins the bug) | `the_cli_s_correction_…` | Nested handler's `ICSR`, no hook |
|---|---|---|---|
| stock / control | pass | pass | `0x842` (`RETTOBASE` 1) |
| patched / repro / ver2 | **fails**: `the bug: 0x42`, left 0 | **fails**: `only bit 11`, left 0 right 2048 | `0x42` (**`RETTOBASE` 0**) |

- On the patched image the hook reads `0x42` as well. It no longer changes anything.
- The build-string pin `the_pinned_image_s_renode_is_the_build_…` passes on stock, control, patched and repro, and fails only on ver2. That is route (ii)'s point.

**E4, unmodified ArduPilot, `simulate --foreign 4000ms`** (CI's `sentinel-02` ELF `a799dfb6`):

| Image / CLI | Verdict | Last witness (U5 FIFO data) | Instructions | `log_kept_bytes` | Warning kinds | Peak RSS (KiB) | Wall (s) |
|---|---|---|---|---|---|---|---|
| stock / main | pass | 3,610,548,200 ns | 81,608,436 | 1,211,414 | 68 | 849,988 | 11.77 |
| control / main | pass | the same | the same | 1,211,404 | 68 | 840,372 | 11.45 |
| patched / main | pass | the same | the same | 1,210,821 | 63 | 841,364 | 11.47 |
| patched / no-hook | pass | the same | the same | 1,210,474 | 63 | 830,420 | 11.04 |
| ver2 / no-hook | pass | the same | the same | 1,210,472 | 63 | 791,684 | 10.77 |

- Every witness time is identical to the nanosecond across all five runs, and so are the 1,798 D4 exemptions.
- **The cache-maintenance lines are gone.**
  - The five NVIC kinds that disappear:
    - `0xF50` ×2;
    - `0xF5C` ×18,942;
    - `0xF60` ×1;
    - `0xF6C` ×1;
    - `0xF70` ×5,117.
  - Every one of the other 63 kinds keeps the same count and the same first and last time.
  - Two NVIC kinds remain: `0xF90` and `0xF94`, one line each. They are `ITCMCR`/`DTCMCR`, the TCM controls, not cache maintenance.
- **CI's script.** I ran `ci/foreign-judge.sh` on the patched image with the no-hook CLI, and on stock with main's: exit 0 both, 46 s against 50 s.
  - All seven answers are identical (verdicts, refusals, kinds, witness times) except `log_kept`.
  - `log_kept` is −940 bytes in each of the four runs that boot.

**E3, the generated firmware's verdicts** (one firmware build, shared): both `simulate` verdicts and every campaign group.

| Against stock / main | Verdict flips | Campaign archive files (54) | `simulate` verdicts | Campaign lines |
|---|---|---|---|---|
| patched / main, patched / no-hook | 0 | byte-identical | identical outside the memory sample | only `run` (the archive path) differs |
| ver (stamped `Renode.dll`) / no-hook | 0 | **54 of 54 differ** | identical outside the memory sample | 27 `state_sequence_hash`, 32 `trace_bytes` (+8, e.g. 4,442 → 4,450), 27 `run` |

- The cause of the ver row is route (ii)'s build line. It is in the trace header's `tools` (8 more bytes). The header's sha256 is `h0`, which every state hash chains from (`aethelgard-chaos` `replay.rs` L93–96).
- No test or committed golden pins an archive byte or a state-hash literal. `campaign_tier2.rs` and `verify_2k2_tier2.rs` compare runs with each other.

**E2, the whole workspace with the tools feature** (stock / main against patched / no-hook):
- **Run times:** stock 6,293 s, patched 6,079 s. Twin tier 2 alone: 700.9 s against 657.6 s.
- **Pass counts:** stock 2,376 pass and 9 fail; patched 2,293 pass and 92 fail.
- **The 9 failures on both are this harness's, not the image's:**
  - `channels_tier1` ×7 and `sockets_tier1` ×1: `sandbox_probe is missing`.
  - `fab_tier1` ×1: `BaseOutsideRoot`.
  - Both come from building into `/cache/target*` outside the tree. So E2 is a comparison, not a gates run.
- **The 83 failures only on patched / no-hook are all the hook's own tests. That is the retirement work for §1.2:**
  - **80 tier-1 tests** (`boot_tier1` ×44, `campaign_tier1` ×36):
    - Their stand-in Renode accepts only a command line containing `rettobase_in_stub()`, so without the hook it exits 99.
    - One of them, `the_expected_value_is_the_tables_…` (`boot_tier1.rs` L1270), reads `Null` from the refused verdict.
    - None runs the image.
  - **1 unit test:** `the_rettobase_correction_follows_the_platform_and_precedes_the_firmware`.
  - **2 twin tests:** the two pin tests of E1.
- **The moved-figure count is 0.** Twin tier 2 holds every virtual-time figure the repository pins. 86 of its 88 tests pass on the patched image without the hook; the other 2 are the RETTOBASE pins. Every other tier-2 binary passes on both.
- **The `ANSWER 2100` lines:**
  - 90 keys were compared field by field, leaving out wall times, the memory sample and paths.
  - 77 keys are identical. The other 13 are explained:
    - **`4400e3` (RETTOBASE):** `inner` 0x842 → 0x42, the fix.
    - **Six serial finding messages:** they differ only in the 8 frame bytes. `serial::frame` hashes the firmware ELF's sha256, and each test builds its ELF under a per-PID `/tmp` path. The counts of bytes back are equal.
    - **Five campaign lines:** they differ only in `run`, `run_id` and `state_sequence_hash`. `run_id` hashes that ELF too (`campaign.rs` L249), and `h0` chains from it.
    - **`own`:** its `run_id` and `state_sequence_hash`, for the same cause.
  - **The log's caveat:**
    - `answer()` appends to `/dev/stderr`, while libtest writes stdout into the same file at its own offset. So some lines were overwritten.
    - Three tests' lines survive in one log only (`A3 flash offset`, `rewritten loop`, `spi forged record`). All three tests pass on both images.
    - A line lost from both logs would not show. So the ANSWER diff supports the count; it is not a census.

**Our C# models** load and run unchanged on every image: twin tier 2 and the foreign run use all of them. No model broke.

### Predictions against outcomes

1. **Held.** Control reads 1 without the hook; patched reads 0 with and without it.
2. **Missed.** Two NVIC kinds remain (`0xF90`/`0xF94`, the TCM controls, one line each). Every cache-maintenance kind is gone, and every non-NVIC kind is unchanged.
3. **Held.** No pinned figure moves.
4. **Held for route (i) only.** On (ii) every archive file and state hash moves, by the build line in the header. No verdict flips.
5. **Held.** `log_kept_bytes` falls by 940 bytes in the foreign run (and in CI's script runs).
6. **Not shown.** The wall times are one sample each (11.77 against 11.04 s, and 10.77 s on ver2), inside the 10.6–12.0 s spread `4400e` measured. RSS fell 2–7 %.
7. **Held.**
8. **Missed as written.**
   - The pin test fails as predicted, and so does its sibling `the_cli_s_correction_…`.
   - Removing the hook also fails the unit test on its placement and 80 tier-1 stub tests.
   - All are the hook's own tests, retired with it.

### For the ruling

- **R1, the route:**
  - (ii), recommended: rebuild `Infrastructure.dll` and republish `Renode.dll` with a build string naming this change, both copied over the official asset.
  - (i): `Infrastructure.dll` only, which needs the smoke acceptance line reworded.
- **R2, the build string under (ii):** `1.17.0+20260907gitf1dd1b4af.aet1605` is what I measured. Any SemVer build-metadata suffix works the same.
- **R3, the corrected source offer:**
  - `TOOLS-SOURCES.md` L15–16 cite tag `ab721d88` and `066a7f13`, but the asset was built from `f1dd1b4af` and `cd4b002a`.
  - It is corrected in §1.2 whichever route is ruled. I flag it because the offer has been wrong since `AET-WO-4403`.
- **R4, the goldens and verdicts that move:**
  - **Under (i):** none, outside `log_kept_bytes` in the foreign verdict.
  - **Under (ii):** every Renode-judged archive's bytes, `trace_bytes` (+8) and `state_sequence_hash`. None of them is pinned.
- **§1.2 after the ruling:**
  - the builder stage in `container/Dockerfile` (the staged script, the patches and the manifest under `container/`);
  - `smoke.sh` gains the build line;
  - the hook goes, through its pin tests: `RETTOBASE_BUGGY_BUILD` and `RETTOBASE_BUILD` move to the new build, and the witness test asserts 0 without the hook;
  - the stub tests lose `rettobase_in_stub`;
  - the `Unhandled write` summary is kept;
  - `TOOLS-SOURCES.md`;
  - the §7.1 rows.
- **The figures to re-pin:** none, on the measured evidence.

## §1.2, built to the ruling (#223, comment 6973081453)

### What was built

- **R1 (ii), R2.** `container/Dockerfile` gains a builder stage, `renode-nvic`, FROM the .NET SDK 8.0.405 by digest.
  - It fetches renode `f1dd1b4af` and the 14 trees it records as GitHub commit tarballs, each sha256-checked. These are `src/Infrastructure` `cd4b002a`, `lib/resources` `14b80cde`, the eleven `lib/` libraries, and InpliTftpServer `2a4877dc`.
  - It applies the three upstream commits from `container/renode-nvic/`, in upstream's `format-patch` form: `e7a8584c`, then `c8b2075c`, then `7b687540`.
  - It restores with `build.sh`'s publish properties, then refuses unless the set of restored `.nupkg` files equals `renode-nvic/nuget.sha256` (134 packages, compared in C-locale order) and each file matches its hash.
  - It builds `Infrastructure.dll` with `SourceRevisionId=cd4b002a…` and publishes `Renode.dll` as `build.sh` does, with `CommitString=gitf1dd1b4af.aet1605`. Both outputs are checked against the measured sha256: `ddb19d7b…` and `5734eb0c…`.
  - The final stage copies the two DLLs over the official asset: one `COPY --from` layer (the overlay R1 kept).
- **Why `SourceRevisionId`:** the .NET SDK takes `InformationalVersion` (`1.0.0+<commit>`) from `.git`, and a tarball has none. Without it, `Infrastructure.dll` differs from the `d878080b` measured in §1.1 by 123 kB of IL and metadata. With it, the bytes equal what §1.1 measured, and `Infrastructure.dll` names the source revision Antmicro's asset names.
- **smoke.sh** checks the full build line, `  build: 1.17.0+20260907gitf1dd1b4af.aet1605` (R2).
- **R3, the offer.** The Renode row of `TOOLS-SOURCES.md` now cites renode `f1dd1b4af`, `src/Infrastructure` `cd4b002a` (tlib `167decf9`), InpliTftpServer `2a4877dc`, and the three patch files, each with its upstream commit. The tlib and Packet.Net rows name the same source. The row for the offer having been wrong is under **Findings for the record** below.
- **The hook retires through its pin tests.**
  - These go: `RETTOBASE_HOOK`, `data/rettobase-hook.monitor`, its emission in `renode.rs` and `renode/foreign.rs`, the two unit tests that tied it to the Dockerfile's Renode, and the stubs' `rettobase_in_stub` in `boot_tier1.rs` and `campaign_tier1.rs`.
  - In `twin_tier2.rs`, the build pin moves to `PINNED_RENODE_BUILD = "1.17.0+20260907gitf1dd1b4af.aet1605"`. The witness becomes `the_pinned_renode_reads_rettobase_0_in_a_handler_nested_over_an_external_interrupt`: no hook, nested `RETTOBASE` 0, outer 1, `VECTACTIVE` 66 and 73, and both IABR bits.
  - The test that pinned the bug and the hook's own test are retired with the hook: 3 tier-2 tests become 2.
- **The `Unhandled write` summary** works unchanged on what remains: 63 kinds in ArduPilot's run, two of them NVIC (`0xF90`, `0xF94`, one line each).
- **Debian drift, which I read wrong (corrected in §1.3):** the strict `check-sources inside tools` refused my local scratch build's offer. That build had no cache, so apt resolved fresh and `linux-libc-dev`, an unpinned dependency of `libc6-dev`, came in at `6.12.117-1`. I regenerated the `linux` rows of `TOOLS-DEBIAN-GPL2.tsv` from it. CI's builds reuse the cached apt layer, which holds `6.12.115-2`. My regeneration therefore broke main's publish after #224 (§1.3), and 1605b (#225) reverted it.

### Measured on the built image (scratch `localhost/aethelgard-tools:scratch-1605-full`, never published; this branch's CLI)

Tier 2, ArduPilot, `foreign-judge.sh` and E3 ran on build `161fcde3`. Smoke and the strict source check ran on its rebuild `9628c8fe`, after the GPL-2 table and `LC_ALL` commits. Both carry the same two DLL hashes.

| Check | Result | Run |
|---|---|---|
| Image build, twice (the second after `LC_ALL=C`, which re-ran the whole `dotnet` step) | both DLLs `OK` against the pinned sha256 both times | `full-build2.log`, `full-build3.log` |
| `smoke.sh`, new image | pass, build line included | `smoke-new.log` |
| `smoke.sh`, today's pinned image (`container/IMAGE`) | **fails** by name: `no line is exactly '  build: 1.17.0+20260907gitf1dd1b4af.aet1605'` | `smoke-old.log` |
| `check-sources.sh inside tools` (strict, as the publish job runs it) | pass | `sources-new.log` |
| Twin tier 2 + every suite, in-tree target as CI (`cargo test --workspace --all-targets --features aethelgard-cli/with-tools`) | 2,374 pass, 8 fail: all 8 are `sandbox_probe is missing`. Here `--all-targets` built the example only as the test-mode `sandbox_probe-<hash>` (seen in `target/debug/examples`). Stock/main fails the same 8, plus one `BaseOutsideRoot` from its out-of-tree target. Counted by CI's `TIER2_RE`: tier 2 is 190 passed, 0 failed (stock/main: 191, 0) | `t2-new-branch.log` against `t2-stock-main.log` |
| The witness, new image | `inner=0x42 outer=0x849 iabr1=0x2040000` → `RETTOBASE` 0 nested, 1 alone; both RETTOBASE tests pass | `t2-new-branch.log` |
| The witness and the pin, **today's image**, this branch | **both fail**: nested `0x842` (left 1, right 0), and the build `1.17.0+20260907gitf1dd1b4af` is not the pinned build | twin_tier2 `rettobase` on `container/IMAGE` |
| ArduPilot, `simulate --foreign 4000ms` | pass. Witness times identical to stock to the ns (last 3,610,548,200), instructions 81,608,436 both, warning kinds 68 → 63 (the five cache-maintenance kinds), `log_kept_bytes` 1,211,414 → 1,210,477, peak RSS 849,988 → 764,300 KiB, wall 11.77 → 10.43 s (one sample each) | `ap/new-branch.json` |
| `ci/foreign-judge.sh` (CI's script) | exit 0, 47 s. All seven answers identical to stock/main's (verdicts, refusals, kinds, witness times) except `log_kept` (−927 to −932) and RSS | `fj-new-branch.log` against `fj-stock-main.log` |
| Generated firmware (E3, the shared-firmware method of §1.1) | 0 verdict flips. 54 of 54 archive files differ, only in `trace_bytes` (32 fields, +8, e.g. 4,329 → 4,337), `state_sequence_hash` (27) and `run` (27): the header's `tools` entry names the new build (R4). The two `simulate` verdicts are identical outside `memory` | `gv/` |

### The moved figures

| Figure | Where it is pinned | Old | New | Cause | Run |
|---|---|---|---|---|---|
| The image's Renode build | `twin_tier2.rs` `PINNED_RENODE_BUILD`; `smoke.sh` | `1.17.0+20260907gitf1dd1b4af` (in `RETTOBASE_BUILD`) | `1.17.0+20260907gitf1dd1b4af.aet1605` | R1 (ii), R2 | `smoke-new.log`; tier 2 |
| The nested handler's `RETTOBASE`, no hook | `twin_tier2.rs` witness | 1 (`0x842`, pinned as the bug) | 0 (`0x42`) | `e7a8584c` | tier 2, new image and old |
| `TIER2_MIN` | `ci.yml` | 191 | 190 | the hook's tests retired: 3 RETTOBASE tier-2 tests become 2 | `TIER2_RE` count: 191 passed in `t2-stock-main.log`, 190 in `t2-new-branch.log`; all suites 2,385 → 2,382 (−1 tier 2, −2 unit) |

- **`TIER2_MIN` is lowered,** against `ci.yml`'s "never lower it to pass". It is lowered because a test was retired with the code it tested (the ruling's "the hook retires through its pin tests"), not to pass, and it stays exact. I flag it for the architect.
- **Not pinned, and moved (R4, accepted):** every Renode-judged archive's `trace_bytes` (+8) and `state_sequence_hash`, as in the E3 row. No test or golden pins them.
- **Pinned, and unmoved:** every virtual-time figure in tests and goldens. Tier 2 passes on the new image with no figure edited.

### Mutations (the guards this WO adds; each against the image build, `--target renode-nvic`)

| Mutant | Killed by |
|---|---|
| m1, `e7a8584c` not applied (2 and 3 still apply) | `/out/Infrastructure.dll: FAILED` |
| m2, a line removed from `nuget.sha256` | `cmp`: `/tmp/restored /tmp/pinned differ: line 2` |
| m3, one package hash altered | `cairosharp…nupkg: FAILED` |
| m4, the stock `CommitString` | `/out/Renode.dll: FAILED` |
| m0, unmutated | builds |
| The image without the fix (today's `container/IMAGE`), against the tier-2 witness and pin | both fail (table above); `smoke.sh` fails |

5 of 5 mutants killed, each by the guard written for it, and the control builds.

### Adversarial pass (§5)

- **A figure re-pinned without a cause:** the three moved figures are in the table, each with its cause and run. Nothing else was re-pinned; tier 2 passes unedited.
- **The hook retired while the bug remains:** the witness asserts 0 without the hook. On today's image it fails (`0x842`), and so do the pin and the smoke test. A stock Renode in the image cannot pass CI.
  - Outside the image, a stock Renode now boots with the bug and nothing refuses it. Nothing refused it before either: main emitted the hook for every build. See D1.
- **A non-reproducible image:** every input is a digest, a commit tarball with its sha256, or a package in the hash manifest (m2 and m3), and both outputs are hash-checked (m1 and m4). Two independent local builds gave identical bytes. `check-pins.sh` passes.
- The stage needs the network for `dotnet restore`, from nuget.org, like every other tool download in the file. The manifest pins what may arrive.

### Findings for the record

| # | Finding | Since | Fixed here |
|---|---|---|---|
| F1 | `TOOLS-SOURCES.md` cited Renode tag `ab721d88` with `renode-infrastructure` `066a7f13`. The asset was built from `f1dd1b4af` with `cd4b002a`. The written offer for the LGPL tlib named the right files: the tlib commit is identical at both. The MIT C# source was mis-cited. | `AET-WO-4403` (`938b7f45`, #176, 2026-11-16) | yes (R3) |
| F2 | The tools image's apt layer reproduces only through the gha cache. `linux-libc-dev` is an unpinned dependency of `libc6-dev`, and Debian's index now serves `6.12.117-1`. A build with no cache resolves it fresh and drifts from `TOOLS-DEBIAN-GPL2.tsv` (`6.12.115-2`), so main's publish would refuse loudly. I first misread this as drift on main and regenerated the table; that broke main's publish (§1.3). | `4403` (#181) | no: §7.1 row ruled (§1.3) |
| F3 | `AET-WO-4400e3` FEEDBACK says "any other build is refused by name before the boot". No such refusal was built: main emitted the hook for every build, and `campaign_tier1`'s `the_header_names_the_renode_build_…` (`2205`) accepts any build. | `4400e3` | no, see D1 |

### For the architect

- **D1:** should `simulate` refuse a Renode build other than the pinned one?
  - This is the `INV-06` question F3 raises, and it conflicts with `2205`'s any-build test. Outside the image, a stock 1.17.0 now judges nested interrupts wrongly with no warning.
  - Not built: it is outside this WO's fence.
- **D2:** stale steering text, protected, for the architect to close:
  - `docs/architecture.md` L464 ("its correction");
  - `docs/roadmap.md` L395 and L402, and §7.1 L798 at `82925510` (the cache-maintenance row, closed by `7b687540` here). The WO's "§7.1 rows closed" is theirs.
- **D3, AWAITING-IMAGE:** `tools-image.yml` has no `workflow_dispatch`. Its `publish` job runs only on a push to main that touches `container/`, as it did for `2100`'s PR A (#57) and `4403`'s #176/#181 before their re-pins.
  - Until `container/IMAGE` names an image built from this branch's `container/`, CI's tier 2 runs on today's image, where the two RETTOBASE tests fail (measured above). So does `ardupilot-image.yml`'s `foreign` job, if this PR triggers it: it runs this CLI, which no longer has the hook, on `container/IMAGE`.
  - A main that carries only this branch's `container/` keeps every check green, by reading. `tools-image.yml`'s own job runs tier 1 only (`cargo test --workspace`, no `with-tools`), and main's unit tie to the Dockerfile reads the asset's URL and sha256, which do not change. `ci.yml` keeps using `container/IMAGE`.
  - The route is the architect's: a `container/`-only merge first, then `IMAGE` re-pinned here, or another dispatch.

### Gates

- `cargo fmt --check` and `cargo clippy --workspace --all-targets --features aethelgard-cli/with-tools -D warnings` are clean.
- Every suite passes in the image, except the 8 harness failures that stock shares.
- The CI package step (`pkgstep`) passes locally: 11 crates, and the CLI is built from the unpacked packages.
- `check-pins.sh` ok. `ci/test-weights.tsv` and `ci/colocate.tsv` name none of the retired tests.
- Reduction pass (diff): −9 net · 1 duplicate removed (the Dockerfile header's commit list, which `TOOLS-SOURCES.md` holds) · 0 dead paths removed (a fresh-agent sweep found no remnant of the hook) · 2 protected kept (the build string in both `smoke.sh` and `twin_tier2.rs`; the pin test and the witness test separately) · 0 findings below. The tier-1 stubs keep main's build string: they accept any build, so the edit was churn.

## §1.3, after #224: main's publish refused, and the fix (1605b, #225)

- **What happened.** Main's publish of `efb880c2` (run 44299148057, job `publish`) refused at `check-sources inside tools`: "does not mirror … linux=6.12.115-2". Nothing was pushed.
- **Cause, at source.** Every CI build of `efb880c2` held `linux-libc-dev` `6.12.115-2`: the #223 and #224 PR runs, main's image job and main's publish job. In each, the apt layer (`stage-1 2/10`) was `CACHED` from gha scope `tools-image`, because its RUN line has not changed since the published image.
  - The PR runs and the image job run `--drift warn`. Each printed `ANSWER debian_drift kind=tools missing=1 packages=linux=6.12.115-2`, and I missed it on #223 and on #224.
  - #224 carried my table regeneration from an uncached local build (§1.2, "Debian drift"). Debian's trixie-security index serves `6.12.117-1`; its pool still holds the `115-2` files.
- **Fix, ruled (architect, 13:20).** 1605b, #225: the table's three `linux` rows go back to `6.12.115-2`, which makes the file byte-identical to `c4a6cc6f`. That version is already mirrored (#181). The published image's apt layer is the cached one, so the image moves from the previous pin only in Renode's two DLLs.
- **For §7.1 (ruled; written by the architect at roadmap L778, `82925510`; owner: the next tools-image WO).** Resolve the tools image's apt from a pinned `snapshot.debian.org` date, so a build with no cache reproduces the layer CI publishes, and so `TOOLS-DEBIAN-GPL2.tsv` can be regenerated from any build.
- **Lesson.** I will regenerate a Debian table only from what CI's build holds, and read CI's `debian_drift` answer before READY.

## §1.4, the re-pin (architect, 11:20 and 14:20)

- **Published.** Main's tools-image run 44322816137 on `cf66dd35` (#225 merged), with jobs image, publish and source all green. The publish job pushed `aethelgard-tools:cf66dd35` as `sha256:e5fc7ddbe36c050e78763ad1d203ec8d176408a3e534bed5563c424d8d9a1b58`. `container/IMAGE` now names it, and main (`cf66dd35`, with 1303b) is merged in.
- **Measured on the published image** (pulled by digest; this branch's CLI; in-tree target as CI):

| Check | Result | Run |
|---|---|---|
| The two DLLs | `ddb19d7b…` and `5734eb0c…`, the pinned sha256 | `sha256sum` in the image |
| `renode --version` | `build: 1.17.0+20260907gitf1dd1b4af.aet1605` | same |
| `linux-libc-dev` | `6.12.115-2`, the table's (§1.3) | `dpkg-query` |
| `smoke.sh`, `--network none` | pass | same |
| The witness (`twin_tier2` `rettobase`) | 2 passed: `inner=0x42 outer=0x849 iabr1=0x2040000` | `witness-published.log` |
| ArduPilot, `simulate --foreign 4000ms` | pass. Every witness time and the instruction count (81,608,436) equal the scratch image's; last witness 3,610,548,200 ns. Only `log_kept_bytes` (+5) and the temp path differ | `ap/published.json` against `ap/new-branch.json` |
| `ci/foreign-judge.sh` | exit 0. All seven answers equal the scratch run's up to `log_kept` | `fj-published.log` |
| Gates in the image | `cargo fmt --check`, `clippy --all-targets --features aethelgard-cli/with-tools -D warnings` and `check-pins.sh` pass | |

- D3 is closed by this route: CI's tier 2 and `ardupilot-image.yml`'s `foreign` job now run on an image carrying the fixes.
- **CI on `a5b1b9ed`:** ci 44330802221, tools-image 44330802155, ardupilot-image 44330802184 and engine-image 44330802165 are all green. Tier 2: `ANSWER tier2_tests_passed=190 floor=190`, exact. `ardupilot-image`'s `foreign` job passes on the re-pinned image, the job that was red on `69283800`.
- Reduction pass (diff, §1.3–§1.4): nothing to remove. Since `69283800` this branch's own change is one line of `container/IMAGE` plus this record; 1605b's fix is on main.
