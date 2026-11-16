# spike/3400: Sentinel-01 as a bench board, on two layers

AET-WO-3400's probes. Nothing here is engine code; a build deletes this directory (roadmap §6). The record is `docs/work-orders/AET-WO-3400.FEEDBACK.md`.

From the repository root, on `main`:

1. `cargo build --release -p aethelgard-cli`
2. `python3 -I spike/3400/make_variants.py` writes `variants/v0.json` … `v7.json` and the four-layer `v0-4l.json` … `v7-4l.json` (the same specification with `board` taken from `crates/aethelgard-cli/tests/fixtures/four-layer.json`, the amendment's S2). V0 is `examples/sentinel-01-minimal.json` byte for byte; each later variant only adds to it (`relaxations.py` checks that).
3. `spike/3400/run.sh <variant>` (e.g. `v5`, or the probe `p-nrst`) writes `results/<variant>/` with one `<step>.json`, `.exit` and `.stderr` per step:
   - `1-validate`, `2-generate`, `3-emit`, `4-erc` and `5-drc` (`--spec`) are the work order's five commands;
   - `0-solve`, `6-drc-parity`, `7-support` and `8-extract` are extras.

   KiCad steps run in the pinned tools image through `~/bin/capped-run` (memory capped, swap 0, read back). `gen/` and `emit/` are regenerated, not committed.
4. The renaming check (AET-WO-1307's 20 seeds) on a variant's emitted board:
   `CARGO_TARGET_DIR=~/.cache/aet-3400/target cargo run --release --manifest-path spike/3400/rename/Cargo.toml -- spike/3400/variants/v5.json spike/3400/results/v5/emit/s01-v5.kicad_pcb`
   It is saved as `results/<variant>/9-rename.txt`. `AET_3400_SKIP_CLASSES=1` is the negative control.
5. `python3 -I spike/3400/relaxations.py` diffs every variant against V0 (`results/relaxations.txt`).

`sources/` holds the research behind S1 and S4 (dated, with URLs), the KiCad listings from the pinned image, and the gates. `probes/t8-boot0-on-swd.*` holds the one refusal measured outside a variant: BOOT0 on a connector pin.
