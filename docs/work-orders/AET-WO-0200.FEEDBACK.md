# AET-WO-0200 — FEEDBACK

**Branch:** `wo/aet-0200-tools-image` · **Base:** `9c6b4d7` (the merge of PR #6) · **Builder:** Claude Code (Opus 5.5) as `forge` on Furnace, tmux `aet` · **Half:** spike, stopped for the architect's ruling. The build half has not started.

Entry schema: `finding · where (file:line) · type · recommendation · decision-needed`.

## Spike

Furnace still has no container runtime and no Nix. On 2026-10-20, `command -v` for `docker`, `podman`, `buildah`, `nix`, `apptainer`, `skopeo` and `crane` found none of them. Every measurement below therefore comes from GitHub Actions.

The spike lives entirely under `spike/` and `.github/workflows/spike-tools-image.yml`, so nothing sits at the build half's paths (`container/`). The workflow builds two candidate images, one per job, from one `spike/Dockerfile`:

- **a** `FROM kicad/kicad:10.0.7`
- **b** `FROM ubuntu:26.04` plus the archive's `kicad`

Both add ngspice, Renode, Rust and Kani from checksummed sources.

Each candidate's cold build exports its layers to the Actions cache, and a second job rebuilds it on a fresh runner from that cache. `spike/probe.sh` runs inside each image twice, normally and under `--network none`, and prints `ANSWER` lines.

SPIKE_RESULTS_PENDING
