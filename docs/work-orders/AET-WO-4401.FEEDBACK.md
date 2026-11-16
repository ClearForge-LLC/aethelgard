# AET-WO-4401 FEEDBACK — The ArduPilot image: its pin, and the static check's blind spots

**Builder:** Claude (strong tier), worktree `~/aethelgard-4401`, branch `wo/aet-4401-image-pins` from `main` `8030fa5`.
**Cadence:** build only, with a targeted adversarial pass. The work order is read from `origin/claude/argon-repo-docs-synthesis-rjrb0d` (docs PR #162); it is not committed here.

Labels:
- **[read]**: read at source, in this repository at `8030fa5`, in a GitHub run log, or inside the published image.
- **[measured]**: run on Furnace (rootless podman through `~/bin/capped-run`, caps read back) or on GitHub's runner.
- **[inferred]**: neither; reasoning from the two.

## 1. Predictions (committed before any pull, smoke, or run of `check-pins.sh` on a new input)

All **[inferred]**.

**The published digest.**
- `ghcr.io/clearforge-llc/aethelgard-ardupilot@sha256:a6d8a4e5…1383a` pulls anonymously on Furnace (the package was made public after its first publish, as the tools package was; `architecture.md:104`). I hold this at about 60 %: if it is still private, the pull is refused with an authorization error, which is a §7 stop, not something to log in around.
- Its `org.opencontainers.image.revision` label is `8030fa5…` (the publish job sets it to `github.sha`).
- The smoke (`ardupilot-build.sh smoke`, MatekH743, `-j20`) passes under `--network none` on Furnace in 60–150 s, with a peak under 16 GiB; at 8 GiB twenty parallel `arm-none-eabi-gcc` jobs might be killed, which would be the cap, not "does not smoke".

**pip's provenance (A7's second half was [inferred] in 4400a).**
- The venv's pip is Debian's wheel from `python3-pip-whl`, pulled in by `python3.13-venv` through `python3-venv`. In trixie its version is `25.1.1+dfsg-1`, and `/opt/venv/bin/pip --version` says `pip 25.1.1`.
- Pinning `python3-pip-whl=<that version>` in the apt block pins the pip that runs, with no fetch beyond the apt install the build already makes. A hashed `pip` line in the requirements alone would still be installed once by the unpinned Debian pip, so it is the weaker route.

**Today's `check-pins.sh` on the four forms** (the "before" half of each fixture):
- `git submodule update --remote`: passes (no rule names `submodule`).
- `git fetch` and `git pull`: pass (the deny-list has only `git … clone`).
- `pipx install x`: passes (`pip3?` must be followed by a non-letter).
- `pip install x` without `--require-hashes`: already refused, but only by the generic "fetches outside the checked curl, clone and pip shapes" message, not by name.

**The adversarial vectors against a line-by-line check:**
- A line continuation (`git \` then `fetch`) gets past any rule that matches one physical line.
- `ARG` is already refused after `FROM`; `ENV G=fetch` then `git $G`, or `ENV P=pipx` then `$P install`, gets past name-matching rules.
- A `RUN <<EOF` heredoc: its body lines are physical lines, so a name rule sees them, but its continuation and quoting differ; none of the three Dockerfiles uses one.
- A script copied in and run: the check never reads it, so anything inside passes.
- One more found while reading: a `# escape=` parser directive changes the continuation character, which would defeat any joining of continued lines.
