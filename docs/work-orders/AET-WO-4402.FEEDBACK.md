# AET-WO-4402 FEEDBACK — Housekeeping: the ArduPilot pin from its own publish, the pin's regex tested, the merged spikes deleted

**Builder:** Claude (cheap tier by the work order's label; this session runs on Opus), worktree `~/aethelgard-4402`, branch `wo/aet-4402-housekeeping` from `main` `80f5dac`.
**Cadence:** build only, with a targeted adversarial pass on the pin check. The work order is read from `origin/claude/argon-repo-docs-synthesis-rjrb0d` (docs PR #170); it is not committed here.

Labels:
- **[read]**: read at source, in this repository at `80f5dac`, or in a GitHub run log.
- **[measured]**: run on Furnace or on GitHub's runner.
- **[inferred]**: neither; reasoning from the two.

## 1. Predictions (committed before any pull, and before any run of the new check)

All **[inferred]**.

**The digest.** `ghcr.io/clearforge-llc/aethelgard-ardupilot@sha256:a4818a04…49d18` pulls anonymously with `podman pull` (the package is public: `4401` pulled `a6d8a4e5…` the same way), and its `org.opencontainers.image.revision` label is `4419c590e63e46e106113d5ce9e82c231d176e8e` (the `publish` job sets the label to `github.sha`).

**Today's check (`[ -r ]`, `pin=$(cat …)`, `[[ =~ ^…[0-9a-f]{64}$ ]]`) on the forms the work order lists:**
- The committed pin with one trailing newline: accepted, because `$(…)` strips trailing newlines. With two or more trailing newlines: also accepted, for the same reason.
- Uppercase hex, 63 or 65 digits, `:latest`, a digest and a tag together, another registry or repository, `sha512:`, two lines, a leading or trailing space, an empty file, CRLF: each refused by the regex message. Two lines are refused because bash's `=~` (POSIX ERE without `REG_NEWLINE`) matches `$` only at the end of the string, not before an embedded newline.
- A missing file: refused by the `missing or unreadable` message.

**The adversarial vectors:**
- **An embedded NUL: accepted by today's check.** Bash drops NUL bytes in a command substitution (with a warning on stderr), so a file holding a valid pin split by a NUL becomes a valid pin. This is the one I expect to get through, and the fix is to refuse any NUL in the file before the regex.
- Unicode look-alikes (Cyrillic `с` in `clearforge`, a fullwidth digit in the hex): refused. Under `en_US.UTF-8` the bracket `[0-9a-f]` might be read through the locale's collation; I expect glibc's rational ranges to make no difference, and the script pins `LC_ALL=C` either way.
- A very long line: refused by the regex.
- A symlink in place of the file: followed by both `[ -r ]` and `cat`; a symlink to a valid pin is accepted (it still names exactly one digest), a dangling one is refused as missing.
- A valid first line followed by a second: refused (as "two lines" above).
- A directory in place of the file: `[ -r ]` is true for a directory, so today's check prints `cat`'s own error and then the regex message with an empty pin. Still exit 1, but not by the right name.

**The spikes and the comments.** Nothing outside `docs/work-orders/`, `docs/reviews/` and the spikes themselves reads `spike/4210`, `4220`, `4300d` or `4300f`. The one other mention is `docs/roadmap.md`'s §7.1 backlog row that asks for this deletion; it is protected, so it stays. No test, digest or golden reads the `hwdef.dat` header comment bytes: ArduPilot's parser drops comment lines, and `ardupilot-pinout.sh` judges the ELF's decoded pins, not a hash of the board or its image.

## 2. Measured against the predictions

| Prediction | Result |
|---|---|
| The digest pulls; its revision label is `4419c590…` | **Held [measured]**, 2026-11-16 12:56 UTC, `podman pull` then `podman image inspect` (output below). |
| `container/` unchanged between `4419c59` and the base | **Held [measured]**: `git diff --stat 4419c59 80f5dac -- container/` prints nothing. |
| One or several trailing newlines accepted | **Held [measured]**; kept, and fixtured (`pin-newline`, `pin-blank-lines`). |
| The eleven named forms refused by the regex message; a missing file by `missing or unreadable` | **Held [measured]** under `en_US.UTF-8` and `C`. |
| An embedded NUL gets through today's check | **Held [measured]**: `nul-embedded` (a NUL inside the digest) and `nul-trailing` (a NUL after the newline) both exit 0 and print a valid pin; bash only warns `command substitution: ignored null byte in input`. |
| Unicode look-alikes refused | **Wrong in part [measured].** The Cyrillic `с` is refused. But under `en_US.UTF-8` (Furnace's locale), glibc's `[0-9]` matches `４` (U+FF14) and `٤` (U+0664), and `[a-f]` matches `á` (U+00E1), so `lookalike-fullwidth` and `lookalike-accent` **pass today's check**. Under `C`, `C.UTF-8` and `POSIX` they are refused. GitHub's runner sets a `C.UTF-8`-type locale **[inferred]**, so CI was not exposed, but the check's verdict depended on the host. |
| A very long line refused | **Held [measured]** (`long-line`, the pin plus 65 536 hex digits). |
| A symlink followed; a valid target accepted, a dangling one refused as missing | **Held [measured]**. |
| A valid first line then a second refused | **Held [measured]** (`second-line`, `two-lines`). |
| A directory exits 1 but not by name | **Held [measured]**: `cat: …: Is a directory`, then the regex message with an empty pin. |
| Nothing reads the four spikes | **Held [read]**, below. |
| Nothing reads the `hwdef.dat` header bytes | **Held [read]**, below. |

**The pull** (`~/.cache/aet-4402/pull.log`):
```
2026-11-16T12:56:37Z
Copying config sha256:f47ebf5238cc8c1cf6639dfc1a5b05c71768793dcbca8e546c8570b7ec1e5c4b
Writing manifest to image destination
f47ebf5238cc8c1cf6639dfc1a5b05c71768793dcbca8e546c8570b7ec1e5c4b
pull rc=0
$ podman image inspect --format '{{index .Labels "org.opencontainers.image.revision"}} {{.Digest}}' <pin>
4419c590e63e46e106113d5ce9e82c231d176e8e sha256:a4818a0472b8c1ed675b7a39e35a558fb4001fe2511b8abb6e02501214749d18
```

## 3. What was built

1. **The pin.** `container/ARDUPILOT-IMAGE` names `…@sha256:a4818a0472b8c1ed675b7a39e35a558fb4001fe2511b8abb6e02501214749d18`, in the file's one-line format with one trailing newline, as before.
2. **`container/check-ardupilot-pin.sh FILE`.** The `read the pin` step's check, moved; the step is now `pin=$(bash container/check-ardupilot-pin.sh container/ARDUPILOT-IMAGE)` on its own line, so under the runner's `bash -eo pipefail` a refusal stops the step before anything reaches `GITHUB_ENV` (**[measured]** by running the step's two lines under `bash --noprofile --norc -eo pipefail` on the committed pin and on `nul-embedded`). The messages are today's, with the path from `$1`, so the step prints them byte for byte. Three guards are new, each from §2:
   - `LC_ALL=C`, so the brackets are ASCII on every host;
   - a NUL anywhere is refused with the regex message;
   - only a regular file (or a symlink to one) is read; anything else is `missing or unreadable`.
   Trailing newlines are still stripped, as `$(cat …)` did.
3. **`container/test-check-ardupilot-pin.sh [CHECK]`.** A table names each fixture in `container/fixtures/ardupilot-pin/` with its verdict: `accept` (exit 0, stdout exactly the pin, stderr empty), `missing` (exit 1, stderr exactly `FILE missing or unreadable`), or `form` (exit 1, stderr exactly `FILE is not <form>: <pin>`). So a fixture cannot pass on the other refusal, and a fixture not in the table fails the test. The committed pin must pass. The check runs under `en_US.UTF-8` when the host has it (else `C.UTF-8`), so a check that drops its own `LC_ALL=C` fails there. It runs in the `image` job, after `check-pins against its fixtures`.
   - **Accepted (4):** `pin`, `pin-newline`, `pin-blank-lines`, `symlink`.
   - **Refused by form (20):** `uppercase-hex`, `hex-63`, `hex-65`, `tag-latest`, `digest-and-tag`, `other-registry`, `other-repository`, `sha512`, `two-lines`, `second-line`, `leading-space`, `trailing-space`, `empty`, `crlf`, `nul-embedded`, `nul-trailing`, `lookalike-cyrillic`, `lookalike-fullwidth`, `lookalike-accent`, `long-line`.
   - **Refused as missing (3):** `directory`, `symlink-dangling`, `does-not-exist` (no file; the table names it).
   Every fixture was written with `printf` and checked with `od -c`; `git ls-files --eol` shows `crlf` stored as `i/crlf`, the two NUL files as `-text`, and the two symlinks as mode `120000`. There is no `.gitattributes`.
4. **The spikes.** `spike/4210`, `spike/4220`, `spike/4300d`, `spike/4300f` deleted (54 files). `spike/1303` and `.dockerignore`'s `spike/` stay.
5. **The comments.** `crates/aethelgard-twin/src/tests.rs`: the `.repl` block now cites `AET-WO-2100.FEEDBACK.md` S1 (which quotes the whole file, measured at `459005f`), and the cold-RCC image cites S3, F10 (the measured enables `0x3`, `0x00200000`, `0x00001000` from the cold boot). The four `pincheck` headers now name `the topology solved from specification sha256 a277c933439524885deb931b2b930555b74aa94aefbee7b34c83ddef73aa1196`, the full value read from `spike/4400/sentinel-01.topology.json` at `47a883b^`. No file on `main` holds that specification now (`git ls-files | xargs sha256sum`: no match), so the hash stands alone. Each header changes one line for one line; the line counts are unchanged (32, 33, 33, 34).

## 4. The proofs the work order asks for

**Nothing reads the four spikes** **[read]**, run before deleting:
```
$ git grep -nE "spike/(4210|4220|4300d|4300f)" -- . ":!docs/work-orders" ":!docs/reviews" ":!spike/4210" ":!spike/4220" ":!spike/4300d" ":!spike/4300f"
docs/roadmap.md:655:| Delete `spike/4210`, `4220`, `4300d`, `4300f` and the stale comments naming `spike/2100` (`twin/src/tests.rs`) and `spike/4400` (four `pincheck` fixture headers) | Grok review 2026-11-16 | a cleanup work order (cheap tier) |
exit 0
$ git grep -nE "(^|[^0-9])(4210|4220|4300d|4300f)/" -- .github ci container scripts crates models examples boards Cargo.toml .dockerignore .gitignore
crates/aethelgard-ir/src/connectors.rs:38:/// `~/.cache/aet-4300f/ds/` on Furnace, and the section.
crates/aethelgard-ir/tests/parts.rs:73:    // Every datasheet is held in ~/.cache/aet-4200/ds/ or ~/.cache/aet-4210/ds/,
exit 0
```
The first hit is §7.1's backlog row asking for this deletion. The two in the second are `~/.cache` datasheet paths, not the spikes. Plain `grep` on Furnace is a wrapper that honours ignore files, so the record uses `git grep`. Some code cites a spike by work-order name (`AET-WO-4300d spike S2` in `ir/src/power.rs:873`, the `sentinel-02-power.json` sources). Those cite the record in the FEEDBACK, not the directory, and stay.

**Nothing reads the `hwdef.dat` header bytes** **[read]**. `aethelgard-pincheck`'s tests read `tests/common/topology.json` and `fixtures/step-g.allow.json`, never a `.hwdef.dat`. `header.rs` counts lines of the generated `hwdef.h`, not of the `.dat`. `ardupilot-build.sh` copies each `.dat` into the untracked board, and `ardupilot-pinout.sh` judges the ELF's decoded pins; no digest, size or line count of a `.dat` or of a built image is pinned anywhere. No dependent value moved.

**After the change** **[read]**: `git grep -nE "spike/(2100|4400|4210|4220|4300d|4300f)" -- . ':!docs/work-orders' ':!docs/reviews'` finds only `docs/roadmap.md:655`. That row is protected, so acceptance bullet 3 holds everywhere except there; see Q1.

**The mutation check** **[measured]**: each mutant is a scratch copy of the script, run through the test with `$1`; the source was never edited. `~/.cache/aet-4402/mutants.log`:

| Mutant | Fixtures whose verdict turned | Test |
|---|---|---|
| drop the `$` anchor | `hex-65`, `two-lines`, `second-line`, `trailing-space`, `crlf`, `long-line` | fails |
| widen `[0-9a-f]` to `[0-9a-fA-F]` | `uppercase-hex` | fails |
| drop `export LC_ALL=C` | `lookalike-fullwidth`, `lookalike-accent` | fails |
| drop the NUL guard | `nul-embedded`, `nul-trailing` | fails |
| drop the regular-file guard | `directory` | fails |
| drop the `^` anchor | `two-lines`, `leading-space` | fails |

The test against today's inline check (a copy that prints the pin to stdout) fails on exactly five fixtures: `nul-embedded`, `nul-trailing`, `lookalike-fullwidth`, `lookalike-accent` (under `en_US.utf8`), and `directory` (named by `cat`, not by the check).

## 5. Gates, adversarial pass, reduction pass

**CLAUDE.md gates** **[measured]**, on Furnace at `379d303`, in a `systemd-run --user --scope -p MemoryMax=32G -p MemorySwapMax=0` scope (`memory.swap.max` read back as `0`): `cargo fmt --all -- --check` clean; `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo test --workspace` 113 test binaries, **1826 passed, 0 failed, 0 ignored**, 329 s. No test was added or removed in Rust, so `TIER2_MIN` and the shard plan's weights are untouched. No container was run: the work order's only container step is the pull.

**Adversarial pass, targeted on the pin check** (a fresh subagent, Sonnet, read-only on the repository; I spot-checked its findings). It tried about 60 file contents under `en_US.UTF-8`, `C.UTF-8` and an unset `LANG`: vertical tab, form feed, tab, lone CR, ANSI escape, BOM, zero-width and no-break spaces, combining accent, an invalid UTF-8 byte, Cyrillic, Greek, fullwidth, Arabic-Indic and subscript digits, a leading blank line, a NUL first or last, and 50 MB on one line (refused in 0.9 s). It also tried file types: symlinks to `/dev/stdin`, `/proc/self/fd/0`, a FIFO, `.`, itself, `/dev/null`, `/dev/zero`, `/proc/self/{environ,cmdline,mem,maps}`, a mode-000 file, and no argument. And it tried environments: `GLOBIGNORE`, `IFS`, `SHELLOPTS=posix`, `POSIXLY_CORRECT`, `BASH_COMPAT`, `BASH_ENV`, exported functions, and a file swapped between the two reads 400 times. **No file content and no file type got a non-conforming pin to exit 0** **[measured]**. Under the swap, every exit 0 printed exactly the valid pin, because the printed value and the regex both come from the first read. The fixtures already cover every vector the work order lists that reaches the regex: look-alikes (three), NUL (two), a long line, a symlink, a second line.
- **A1, not a bypass, unchanged [measured]:** the refusal echoes the whole file to stderr, as today's inline message did. So a committed pin file holding `::warning::x` on its second line prints that line on its own to the step's log, where the Actions runner may read it as a workflow command (not confirmed on a runner). The job still fails with exit 1. The work order fixes the message as today's, so I left it; see Q4.
- **A2, environment only [measured]:** `BASH_ENV` pointing at a script that echoes adds a line to stdout, and so to `PIN=`. That needs a workflow `env:` or an earlier step writing `GITHUB_ENV`, which is already code execution in the job; not reachable from the pin file. Not fixed.
- **A3, residual [measured]:** a symlink to `/dev/stdin` is accepted when stdin is a regular file holding a valid pin. That is harmless: it still names one digest.

**Reduction pass (diff): −2 net in the workflow · 0 duplicates removed · 0 dead paths removed · 0 protected kept · 1 finding below.** I cut the clause in the `read the pin` comment that restated the command line. The finding is Q3: `publish` still holds a second copy of the regex, outside the fence.

## 6. Questions for the architect

- **Q1, acceptance bullet 3 and the roadmap.** `docs/roadmap.md:655` (§7.1's row for this work order) names `spike/4210`…`4300f`, `spike/2100` and `spike/4400`. It is protected, so it stays until the docs PR that retires the row. Everywhere else outside `docs/work-orders/` and `docs/reviews/`, `git grep` finds none of the six.
- **Q2, the workflow's `paths:` filters.** `ardupilot-image.yml` triggers on `container/ardupilot*`, `container/ARDUPILOT-IMAGE`, `check-pins.sh` and its fixtures, but not on `container/check-ardupilot-pin.sh`, `container/test-check-ardupilot-pin.sh` or `container/fixtures/ardupilot-pin/**`. This PR runs the test because it also changes `ARDUPILOT-IMAGE`. A later PR that touches only the check would not run its test. The fix is three lines in each of the two `paths:` lists, outside this fence.
- **Q3, a second copy of the regex.** The `publish` job's `push and record the digest` step checks the pushed digest with the same regex inline. The work order says the regex lives only in the new script; that step is outside the fence. It could call `check-ardupilot-pin.sh` on a temporary file, or keep its own copy labelled as deliberate.
- **Q4, the refusal echoes the file (A1).** Printing the pin bounded and escaped (for example `printf '%q' "${pin:0:200}"`) would close the workflow-command echo and the 50 MB log. That changes today's message, so it is yours to rule.
- **Q5, the `hwdef.dat` header's claim.** The headers say the boards were written from the topology of specification `a277c933…`. Since `4300b` (`33f4fd4`) they also carry `PA9`/`PA10` `USART1` and `SERIAL_ORDER`, which that specification did not solve. The new header is as accurate as the old one, no more. A follow-up could say "and the `4300b` serial lines".
- **Q6, tools-image.** `tools-image.yml` triggers on `container/**`, so this PR runs it, and the merge to `main` publishes a tools image from an unchanged Dockerfile. That was already true for `4401`. Nothing here changes `container/IMAGE`.
