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
