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
