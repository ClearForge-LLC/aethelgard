# spike/1317 — the placer, monotone in parts (AET-WO-1317 §1.1)

Every figure here is measured by the harness on a scratch copy of `main` ff7fd53d: `git archive` into `~/.cache/aet-1317/trace`, with the hooks applied. The findings are in `docs/work-orders/AET-WO-1317.FEEDBACK.md`; this directory holds the predictions, the harness and the raw outputs.

- `PREDICTIONS.md`: committed before the first run (e5a061bf).
- `harness/hooks.patch`: trace lines and knock-out knobs in `pcb.rs`. With the hooks applied and nothing knocked out, `outline_tier1` passes 9/9, digest included, so the hooks move no placement.
- `harness/spike1317.rs`: the eda-side runner (`crates/aethelgard-eda/src/spike1317.rs` in the copy):
  - parts-level removal (`removals`, with or without the orphan closure);
  - a name-free identity per part;
  - the refusal-only fallback runner.
- `harness/driver.rs`: the example (`crates/aethelgard-cli/examples/spike1317.rs` in the copy). The variants are built with `outline_tier1`'s own edits and `common::rename::sized`. With U2 is 03d89554's fixture hunk, reversed. The modes are `outcome`, `trace`, `sweep`, `specsweep` and `fbsweep`.
- `analyse.py`: counts hard and soft non-monotone cases in a sweep TSV, as `PREDICTIONS.md` defines them.
- `out/`: `calibration.tsv`, `ablation.txt`, the traces, and the sweeps. The sweeps are R (part plus owned), R+ (plus orphans), S (spec level), and each with (k), (m) or (s) knocked out (`-nok`, `-nom`, `-nos`). `fb-*` are the salt fallback. `adv-*` are the adversarial runs (`harness/adversarial.rs`, public API only); the pairs files keep the header, each variant's full-board row and every placed row not `fit` (fit rows and rows `validate` refused are dropped; the driver regenerates them byte for byte).

Reproduce: apply `hooks.patch` to a copy of ff7fd53d. Copy `spike1317.rs` to `crates/aethelgard-eda/src/` and add `pub mod spike1317;`. Copy `driver.rs` to `crates/aethelgard-cli/examples/spike1317.rs`. Then `cargo build --release -p aethelgard-cli --example spike1317` and run from the copy's root, for example `spike1317 sweep "" base turn-forward` or `CLOSURE=1 FALLBACK=salt:1-8 spike1317 fbsweep base`.
