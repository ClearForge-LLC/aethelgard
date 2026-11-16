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

## 2. Measured against the predictions

| Prediction | Result |
|---|---|
| The digest pulls anonymously | **Held.** `podman pull ghcr.io/clearforge-llc/aethelgard-ardupilot@sha256:a6d8a4e5…1383a` took 16 s with no login [measured]. The digest is in run 43805854193, job `publish`, as `8030fa5…: digest: sha256:a6d8a4e5…1383a size: 2208` [read]. |
| Revision label `8030fa5…` | **Held.** `org.opencontainers.image.revision=8030fa5c61567868f6a223ed334011cb685b773d`, image 2,672,836,018 bytes [measured]. |
| Smoke passes, 60–150 s, under 16 GiB | **Held, faster.** `ardupilot-build.sh smoke` under `--network none`: `ANSWER smoke board=MatekH743 jobs=20 wall_ms=34561 flash_bytes=1550452`. It ran in `capped-run --mem 16G`, which read back `memory.max=17179869184 memory.swap.max=0 pids.max=8192` [measured]. On CI, the `pinned` job of run 43838686771 (first push, `c8d94f6`) printed `ANSWER pinned revision=8030fa5c…` and `ANSWER smoke board=MatekH743 jobs=4 wall_ms=158440 flash_bytes=1550452`. The flash size matches Furnace byte for byte [measured]. |
| pip is `python3-pip-whl 25.1.1+dfsg-1`, pulled in by `python3.13-venv` | **Held.** `dpkg-query`: `python3-pip-whl 25.1.1+dfsg-1`. `apt-cache depends python3.13-venv` lists `python3-pip-whl` and `python3-setuptools-whl 78.1.1-0.1`. `/opt/venv/bin/pip --version` reports `pip 25.1.1`. The wheel's sha256 is `20568e5d…cd9c` [measured]. |
| Today's check passes `submodule --remote`, `fetch`, `pull`, `pipx`; refuses unhashed pip only generically | **Held** [measured]. `origin/main`'s script, run on each fixture, exits 0 on every one except `pip-unhashed`, which it refuses with "fetches outside the checked curl, clone and pip shapes". The full log is in §5. |
| The adversarial vectors get past a line-by-line check | **Held, and worse than predicted.** On main, a `git clone` split over a continuation also gets past the clone *shape* check (`continuation-clone`), and `python3 -mpip install` gets past the deny-list (`pip-module`). |

## 3. What was built

**The pin.** `container/ARDUPILOT-IMAGE` holds the digest above, in `container/IMAGE`'s format: one line, a trailing newline, 116 bytes.

**Its consumers** [read, by grep over the repository]:
- The new `pinned` job in `ardupilot-image.yml` is the only consumer. It refuses a pin that is missing, unreadable, or anything but `ghcr.io/clearforge-llc/aethelgard-ardupilot@sha256:<64 hex>` on one line. It then pulls the digest, prints the image's revision label, and runs the smoke under `--network none`.
- `image`, the four `ardupilot` board jobs, and `publish` still build this PR's Dockerfile. `pinout` and `cli` never run the image. Nothing in `ci.yml` uses it.
- `README.md` does not name the image. `ARDUPILOT-LICENSE.md` names it but gives no digest, so it was left unchanged.

**The drift refusal.** `check-pins.sh`'s refusal for `container/IMAGE` is a format check, and it runs only when a Dockerfile builds `FROM ${TOOLS_IMAGE}`. `engine-image.yml` adds a `[ -r ]` check. No Dockerfile builds from the ArduPilot image, so the equivalent sits in the consumer's own step. The pull by digest is itself content-addressed. I tested that step outside CI by extracting it from the workflow text [measured]: the good pin exits 0. A missing pin, an empty file, a tag, another image name, upper-case hex and a second line each exit 1 with the message naming the file.

**`check-pins.sh`.** Physical lines are also joined into whole instructions, the way Docker joins them: a trailing `\` is dropped, and spaces or tabs may follow it. The next line is appended as it stands, with no space added, and comment and empty lines in between are skipped. Each instruction is then split into words. Separators become `;`. Quotes and backslashes are deleted, because the shell splices `gi''t` into `git`. Brackets, backquotes and `$` become spaces. A word's name drops an `X=` prefix, a `-m` prefix and its directory. On that:
- `git` is allowed only as `git clone` (followed exactly by the checked shape's arguments), `git -C DIR rev-parse HEAD` and `git -C DIR status --porcelain`.
- These are refused by name: `git submodule … --remote`, `git fetch`, `git pull`, `pipx`, and a `pip*` whose command includes `install` but not `--require-hashes`.
- A `-m venv` without `--without-pip` requires `python3-pip-whl=<version>` in the apt block.
- A heredoc, a `# escape=` directive, a carriage return, and a `COPY` source that is missing or looks like a script (`#!`, the exec bit, `.sh/.bash/.py/.pl`) are refused.
- A line that a by-name rule refused is skipped by the old generic deny-list, so each fixture draws exactly its own refusal.
- The header now gives what is left of A7.

**pip.** `python3-pip-whl=25.1.1+dfsg-1` is in the ArduPilot Dockerfile's apt block. It is the version measured in the published image, and the install needs no fetch beyond the apt install the build already makes. The pip that runs `pip install --require-hashes` is therefore pinned. A hashed `pip` line in the requirements would still have been installed once by Debian's unpinned pip (§1), which is why it was not chosen.

**Fixtures.** `container/fixtures/check-pins/` holds 17 Dockerfiles and `copy-script.sh`. `container/test-check-pins.sh` requires each Dockerfile to exit 1 with exactly its `# expect:` refusals, and the three real Dockerfiles to pass. It runs in the `image` job, and the workflow's path filters now include the pin, the harness and the fixtures.

## 4. Forks taken (architect may overrule)

- **F1, a consumer job.** No job pulled the image before this work order, and the board jobs must keep building this PR's Dockerfile, so I added `pinned` as the one consumer. It is also the CI witness for §7's "pulls and smokes on CI". Cost: about 2 minutes and a 2.7 GB pull on each `ardupilot-image` run.
- **F2, git is allow-listed, not deny-listed.** Refusing only `fetch`, `pull` and `submodule --remote` by name would let `git $G`, `git -c alias.f=fetch f` and `git $(echo fetch)` through. Any other git subcommand is refused by a generic message. The by-name messages are kept for the four forms.
- **F3, the venv rule.** It is not asked for in §1.2, but without it, dropping the `python3-pip-whl` line would silently unpin pip again.
- **F4, the heredoc, escape, carriage-return and COPY-script refusals** close §5's vectors outright rather than parse them. No Dockerfile in the repository uses any of the four [read].

## 5. Adversarial pass, targeted (§5)

The "before" column is `origin/main`'s `check-pins.sh` (`8030fa5`); the "after" column is this branch. All rows are [measured].

| Vector | Attempt | Before | After |
|---|---|---|---|
| Line continuation | `gi\` (then a space and a tab) / `t fetch` (`continuation`) | passes | refused, `git fetch` |
| Line continuation | `git \` / `clone … --remote-submodules …` (`continuation-clone`) | passes | refused, clone outside shape |
| Line continuation | `# escape=` then a backquote continuation (`escape-directive`) | passes | refused |
| Line continuation | CRLF line ends (`crlf`) | passes | refused |
| ARG/ENV-built | `ARG` after FROM | refused already | refused |
| ARG/ENV-built | `ENV G=fetch` / `git -C … $G` (`env-built`) | passes | refused, generic git |
| ARG/ENV-built | `ENV P=pipx` / `$P install` (`env-pipx`) | passes | refused, `pipx` |
| ARG/ENV-built | `ENV P=pip` / `$P install x` | refused already, generic | refused, generic |
| RUN heredoc | `RUN <<EOF` / `git … fetch` (`heredoc`) | passes | refused, heredoc and `git fetch` |
| Copied script | `COPY copy-script.sh …` / `RUN sh …` (`copy-script`) | passes | refused |
| Quoting | `python3 -c "…system('git … pull')"` (`quoted`) | passes | refused |
| Quoting | `gi''t`, `"gi"t`, `g\it`, `eval "g""it fetch"` (`quote-splice` holds one) | pass | refused |
| Other spellings | `-mpip install` (`pip-module`); `/opt/venv/bin/pip` (`pip-unhashed`) | pass / generic | refused by name |
| Other spellings | `--git-dir=…`, `-C/opt/src`, `env git`, a tab, `GIT_DIR=… git`, `ONBUILD RUN`, `SHELL [...]`, `find -exec git fetch`, `uv pip install`, `pip3.13` | (not run on main) | refused |
| **Left, declared in the header** | `sh -c "$(printf '\x67it fetch')"`, `${G}t fetch`: the name is assembled at run time | — | passes |
| **Left, declared in the header** | `uvx empy`: an installer the list does not name | — | passes |
| **Left, declared in the header** | `COPY x.txt` / `RUN sh /x.txt`: a plain file run by an interpreter | — | passes |

**Mutation** (one mutant per guard this work order adds, run against `test-check-pins.sh`): 20 of 20 killed [measured]. The guards are the fetch/pull rule, the submodule `--remote` rule, pipx, unhashed pip, the no-space join, the trailing-blank join, the generic git rule, the clone shape, the escape directive, the heredoc, the COPY script, the venv rule, quote deletion, bracket normalization, the by-name skip of the generic list, rev-parse's allowance, the `X=` prefix, the `-m` prefix, the basename, and the carriage return. The first run left the `-m` and basename mutants alive. That is how the `pip-module` fixture was added and `pip-unhashed` came to use a full path. Harness: `~/.cache/aet-4401/mut/run.py`.

## 6. Findings and questions for the architect

- **Q1, the pin is stale once this merges.** This PR changes `ardupilot.Dockerfile` (the pip line), so `main` publishes a new image and `ARDUPILOT-IMAGE` then names the older one. That matches `RUL-15` and `container/IMAGE`. A follow-up PR, or 4400b, sets the new digest from that publish's summary. A §7.1 row would replace the D-4 row.
- **Q2, `# syntax=`.** A `# syntax=docker/dockerfile:1` directive makes BuildKit pull a frontend image by tag, which is an unpinned fetch the check does not refuse. No Dockerfile here has one [read]. It belongs in the next container work order as a backlog row; I did not build it, because it is outside A7.
- **Q3, other unpinned apt dependencies.** Only packages named in the block are pinned. `python3.13-venv` (`3.13.5-2+deb13u5`) and `python3-setuptools-whl` (`78.1.1-0.1`, installed but never put in the venv) come in as dependencies at whatever version trixie serves, as does every other dependency in both images. The work order asked only for pip.
- **Q4, CRLF in Docker.** I did not measure whether BuildKit joins a line ending in `\` plus a carriage return. The refusal does not depend on the answer [inferred].
- **The roadmap §7.1 row** for 4400a A7 (`roadmap.md:534`) can be closed. Its remainder is what the header now declares. The steering documents are protected, so this is the architect's edit.
