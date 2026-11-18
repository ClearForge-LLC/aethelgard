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
