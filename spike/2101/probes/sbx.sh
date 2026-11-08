#!/bin/bash
# Run a command under the CLI's own sandbox, unchanged: the reaper and the
# trampoline of the built `aethelgard` binary (run.rs reaper_main/confine_main),
# with the same read-only system dirs Invocation adds (SYSTEM_DIRS) plus the
# ones simulate adds for Renode (/proc, /dev/urandom, the tool's dirs).
# The reaper checks parent_id() == --cli-pid, so this shell is the "CLI".
# Usage: sbx.sh <workdir> <status-file> [extra --ro/--rw args...] -- <tool> [args...]
# Environment is cleared as Invocation does: HOME/TMPDIR=workdir, DOTNET_EnableDiagnostics=0.
# stdin is inherited (fd 0) - the production Invocation sets it to /dev/null.
set -u
AETH=${AETH:-/cache/target/release/aethelgard}
wd=$1; st=$2; shift 2
extra=()
while [ "$1" != "--" ]; do extra+=("$1"); shift; done; shift
ro=(); for d in /usr /lib /lib64 /bin /sbin /etc /proc /dev/urandom /opt/renode /usr/local/bin; do ro+=(--ro "$d"); done
env -i HOME="$wd" TMPDIR="$wd" DOTNET_EnableDiagnostics=0 PATH=/usr/local/bin:/usr/bin:/bin \
  "$AETH" --aethelgard-internal-reaper --cli-pid $$ --status "$st" "${ro[@]}" --rw "$wd" "${extra[@]}" -- "$@"
rc=$?
echo "SBX reaper-exit=$rc status: $(tr '\n' '|' < "$st")" >&2
exit $rc
