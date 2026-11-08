# AET-WO-1700 — FEEDBACK

**Branch:** `wo/aet-1700-quickstart` · **Base:** `3d13f49` (PR #61) · **Builder:** Claude Code (Sonnet 5.5) as `forge` on Furnace · **Cadence:** build-only. **Result:** §1 built; no §7 stop. One deviation from §1.2, flagged as F1.

Entry schema: `finding · where · type · recommendation · decision-needed`.

## Pins

| Pin | Value |
|---|---|
| Engine image | `ghcr.io/clearforge-llc/aethelgard@sha256:4c440a2d9e8ebcfbfc5ac49c0d10c7ca3b705a232e0c8895d1e98e0c7764234f` |
| Published from | `51b248d2647e0d88527d8d07eed6ae92568c6709` (AET-WO-1304, PR #62), by `engine-image` run 42466218093 on `main`, 2026-11-08T09:20:05Z. The digest-to-commit link is the GitHub Packages tag (the tag is the commit sha) |
| Specification | `examples/sentinel-01-minimal.json` at `3d13f4922b0adddb2f761c3a814c53fcfec61ac8` (on `main`), sha256 `acd60081f2d111b1b1ed0d974078fff45ccf2a967ec5f42317417f8ce68149d1` |

## Findings

- **F1 · `.github/workflows/quickstart.yml` · deviation from §1.2 · the engine image carries no `org.opencontainers.image.revision` label** (measured: `podman inspect` of the published image lists authors, description, documentation, licenses, source, url, vendor only). The assertion "the digest's revision label is reachable from `origin/main`" cannot be made, and adding the label means editing `engine.Dockerfile` or the publish job, which is outside the fence. Substituted: the workflow asserts the digest form and that the image's `source` label is this repository; the specification's pinned commit is asserted reachable from `origin/main` as written. The digest-to-commit link is the Packages tag above. **Recommendation:** a small work order for `engine-image` to set `org.opencontainers.image.revision=$GITHUB_SHA` on the published image (`docker/build-push-action` `labels:`), then the workflow step can compare it with `git merge-base --is-ancestor`. **decision-needed:** yes (architect).
- **F2 · README block · the "nothing but Docker, `bash`, `curl`, `sha256sum`" line** · the block also uses `grep`, `id` and a here-string, as the work order's own example (`grep -q`) does. The JSON edit runs inside the image (`python3 -I`), because the image has no `jq`. **decision-needed:** no.
- **F3 · local runs used podman** with a `docker` shim adding `--userns=keep-id` (rootless podman maps `--user` differently). Real Docker is exercised by the CI run below only.

## Gates

`cargo fmt`, `clippy` and `test` are not affected: no Rust file changed. Diff: `README.md` (`## Quickstart` only), `.github/workflows/quickstart.yml` (new), one line of `crates/aethelgard-cli/README.md`, this file.

## Negative controls

Each is the workflow's own step scripts (extracted from the YAML) run by a local harness against a copy of the repository; podman stands in for Docker.

| Control | Result |
|---|---|
| The real block | **pass**, every step, ends `quickstart ok` |
| One check changed to expect `"verdict":"fail"` (the ERC check) | **fails**, step `run it, as written`, exit 1, after emit printed ok and ERC printed pass |
| Both markers removed | **fails**, `begin marker missing or repeated` |
| CRLF line endings throughout | **fails**, `begin marker missing or repeated` |
| A second fenced `bash` block after the end marker, inside the markers | **fails**, `not exactly one fenced block` |

## Adversarial pass (§5)

1. *Pass while ERC or DRC fails.* Every `ae` call is an assignment, so a nonzero exit stops the script under `-e`; each verdict is also grepped, and DRC's `"findings":[]` is checked. The refusal step asserts exit 2 exactly, then `PinClaimedTwice`, then the two claimants in order. The fail-expecting control above is the proof for ERC; DRC and `solve` are guarded by the same shape.
2. *Run something other than the README's bytes.* Markers match whole lines only (`grep -x`), so a marker inside a code span or after text is not a marker and the count check fails; a repeated marker fails; CR fails; more or fewer than two fence lines fails; text before the opening or after the closing fence fails. The run step executes the file the extract step wrote, not a re-extraction. Run, not just reasoned: no markers, CRLF, second block. Reasoned only: marker in a code span.
3. *Depend on more than Docker, `bash`, `curl`, `sha256sum`.* See F2. The run directory is a fresh `mktemp -d` with no checkout and no Rust; the pinned-commit and source-label steps are separate steps.

## Reduction pass (§5a)

Nothing to remove: the diff is one block, one workflow, one line. The source-label step is the F1 substitute, not an addition.

## Local run of the block (podman)

```
spec.json: OK
1290020c27ccdef9f72a82ee7ed0a2644c70982939c0898e3bf24bf0dea0d336
{"ok":true,"project_name":"sentinel-01","specification_sha256":"acd60081f2d111b1b1ed0d974078fff45ccf2a967ec5f42317417f8ce68149d1","schematic":{"path":"out/sentinel-01.kicad_sch","sha256":"349968e34c8f14a244db008d56c38239a5e27d8613d003727941e7c1bfe9b3f9"},"bill_of_materials":{"path":"out/sentinel-01.bom.csv","sha256":"5ae3521299618066d99bb66ec45d7014f260a43edcc0487076ae7008e26a8799"},"board":{"path":"out/sentinel-01.kicad_pcb","sha256":"7b9b0e3dee3ff0d5dfa2cf8d90bd99ba615ca43a573699c062875189707c2070"}}
{"ok":true,"verdict":"pass","check":"erc","input":{"path":"/work/out/sentinel-01.kicad_sch","sha256":"349968e34c8f14a244db008d56c38239a5e27d8613d003727941e7c1bfe9b3f9"},"tool":{"name":"kicad-cli","version":"10.0.7","exit_code":0},"sandbox":{"landlock_abi":8,"tcp_confined":true,"scoped":true,"sockets_denied":true},"included_severities":["error","warning"],"findings":[]}
{"ok":true,"verdict":"pass","check":"drc","input":{"path":"/work/out/sentinel-01.kicad_pcb","sha256":"7b9b0e3dee3ff0d5dfa2cf8d90bd99ba615ca43a573699c062875189707c2070"},"tool":{"name":"kicad-cli","version":"10.0.7","exit_code":0},"sandbox":{"landlock_abi":8,"tcp_confined":true,"scoped":true,"sockets_denied":true},"included_severities":["error","warning"],"rules":"specification","specification_sha256":"acd60081f2d111b1b1ed0d974078fff45ccf2a967ec5f42317417f8ce68149d1","schematic":{"path":"/work/out/sentinel-01.kicad_sch","sha256":"349968e34c8f14a244db008d56c38239a5e27d8613d003727941e7c1bfe9b3f9"},"findings":[]}
{"ok":false,"error":{"kind":"PinClaimedTwice","id":"PB9","claimants":["i2c_buses[0].sda","spi_devices[0].cs"],"message":"pin `PB9` is claimed by both `i2c_buses[0].sda` and `spi_devices[0].cs`"}}
quickstart ok
double.json
out
spec.json
```

## CI run

Run 42466864493 on PR #63 (head 435a12c), real Docker on `ubuntu-latest`: **green**. All four steps passed; the specification commit `3d13f4922b0adddb2f761c3a814c53fcfec61ac8` is on `main`; the image's source label is `https://github.com/ClearForge-LLC/aethelgard`. Output of `run it, as written` (lines cut at 700 bytes):

```
spec.json: OK
ghcr.io/clearforge-llc/aethelgard@sha256:4c440a2d9e8ebcfbfc5ac49c0d10c7ca3b705a232e0c8895d1e98e0c7764234f
{"ok":true,"project_name":"sentinel-01","specification_sha256":"acd60081f2d111b1b1ed0d974078fff45ccf2a967ec5f42317417f8ce68149d1","schematic":{"path":"out/sentinel-01.kicad_sch","sha256":"349968e34c8f14a244db008d56c38239a5e27d8613d003727941e7c1bfe9b3f9"},"bill_of_materials":{"path":"out/sentinel-01.bom.csv","sha256":"5ae3521299618066d99bb66ec45d7014f260a43edcc0487076ae7008e26a8799"},"board":{"path":"out/sentinel-01.kicad_pcb","sha256":"7b9b0e3dee3ff0d5dfa2cf8d90bd99ba615ca43a573699c062875189707c2070"}}
{"ok":true,"verdict":"pass","check":"erc","input":{"path":"/work/out/sentinel-01.kicad_sch","sha256":"349968e34c8f14a244db008d56c38239a5e27d8613d003727941e7c1bfe9b3f9"},"tool":{"name":"kicad-cli","version":"10.0.7","exit_code":0},"sandbox":{"landlock_abi":8,"tcp_confined":true,"scoped":true,"sockets_denied":true},"included_severities":["error","warning"],"findings":[]}
{"ok":true,"verdict":"pass","check":"drc","input":{"path":"/work/out/sentinel-01.kicad_pcb","sha256":"7b9b0e3dee3ff0d5dfa2cf8d90bd99ba615ca43a573699c062875189707c2070"},"tool":{"name":"kicad-cli","version":"10.0.7","exit_code":0},"sandbox":{"landlock_abi":8,"tcp_confined":true,"scoped":true,"sockets_denied":true},"included_severities":["error","warning"],"rules":"specification","specification_sha256":"acd60081f2d111b1b1ed0d974078fff45ccf2a967ec5f42317417f8ce68149d1","schematic":{"path":"/work/out/sentinel-01.kicad_sch","sha256":"349968e34c8f14a244db008d56c38239a5e27d8613d003727941e7c1bfe9b3f9"},"findings":[]}
{"ok":false,"error":{"kind":"PinClaimedTwice","id":"PB9","claimants":["i2c_buses[0].sda","spi_devices[0].cs"],"message":"pin `PB9` is claimed by both `i2c_buses[0].sda` and `spi_devices[0].cs`"}}
quickstart ok
double.json
out
spec.json
```
