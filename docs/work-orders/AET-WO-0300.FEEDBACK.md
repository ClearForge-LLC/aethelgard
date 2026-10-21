# AET-WO-0300 — FEEDBACK

**Branch:** `wo/aet-0300-cli-tools` · **Base:** `7a238ef` (PR #11) · **Builder:** Claude Code (Opus 5.5) as `forge` on Furnace · **Half:** spike, stopped for the architect's ruling. The build half has not started.

Entry schema: `finding · where (file:line) · type · recommendation · decision-needed`.

## Spike

Furnace still has no container runtime and none of the four tools, so every tool measurement below comes from GitHub Actions, inside `ghcr.io/clearforge-llc/aethelgard-tools@sha256:74f69bd3…` pulled by the digest in `container/IMAGE`. Everything spike lives under `spike/` and `.github/workflows/spike-cli-tools.yml`.

The Rust probes in `spike/probes` (`json-summary`, `confine`, `spawn-tree`) were built and unit-run on Furnace first. Landlock and the process-tree behaviour were also measured on Furnace's own kernel, as a cross-check on the runner's.

SPIKE_RESULTS_PENDING
