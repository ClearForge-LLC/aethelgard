#!/usr/bin/env bash
# container/test-debian-src.sh: `debian-src.py check` against its fixtures
# (AET-WO-4403 D5). Each table under container/fixtures/debian-src/ is cut
# from two real packages of ARDUPILOT-DEBIAN-GPL2.tsv and named below with the
# one refusal it must draw, exactly, on stderr ("F" stands for its path), with
# exit 1 and nothing on stdout; `ok` must pass. Both committed tables must
# pass, and every fixture must be in the table.
#
#   bash container/test-debian-src.sh [path/to/debian-src.py]
set -euo pipefail
check=${1:-container/debian-src.py}
dir=container/fixtures/debian-src
fail=0
tmp=$(mktemp)
trap 'rm -f "$tmp"' EXIT

run() {
  local f=$dir/$1.tsv want=$2 out err rc
  set +e
  out=$(python3 -I "$check" check "$f" 2> "$tmp"); rc=$?
  set -e
  err=$(cat "$tmp")
  if [ "$want" = ok ]; then
    [ "$rc" -eq 0 ] && [ -z "$err" ] && [[ $out == "ok   $f: "* ]] && { echo "ok   accept $1"; return; }
  else
    want="debian-src: ${want//F/$f}"
    [ "$rc" -eq 1 ] && [ -z "$out" ] && [ "$err" = "$want" ] && { echo "ok   refuse $1"; return; }
  fi
  echo "FAIL $1: want [$want], got exit $rc, stdout [${out:0:100}], stderr [${err:0:200}]"
  fail=1
}

base=$(grep -P '^base-passwd\t' "$dir/ok.tsv" | head -1 | cut -f8)
# name                   the refusal ("F" is the fixture's path)
table="
ok                       ok
empty                    F is empty
no-final-newline         F does not end with a newline
not-ascii                F missing, unreadable or not ASCII
seven-columns            F:1 has 7 columns, not 8
unsorted                 F is not sorted by source, version, file, or repeats a file
repeated-file            F is not sorted by source, version, file, or repeats a file
source-uppercase         F:1 source is not a Debian source name
version-space            F:1 version is not a Debian version
file-with-slash          F:1 file is not a plain file name
sha1-short               F:1 sha1 or sha256 is malformed
sha256-uppercase         F:1 sha1 or sha256 is malformed
size-zero                F:1 size is not a positive integer
no-dsc                   F: base-passwd 3.6.7 does not have exactly one .dsc
two-dsc                  F: base-passwd 3.6.7 does not have exactly one .dsc
tag-wrong                F: base-passwd 3.6.7 base-passwd_3.6.7.dsc names tag debian-src.base-passwd__0, not debian-src.base-passwd__3.6.7
manifest-digest-wrong    F: base-passwd 3.6.7 base-passwd_3.6.7.dsc names manifest sha256:$(printf '0%.0s' {1..64}), but its rows derive sha256:$base
file-sha256-changed      DERIVED
"

listed=()
while read -r name want; do
  [ -n "$name" ] || continue
  listed+=("$name")
  if [ "$want" = DERIVED ]; then
    # The manifest digest the edited rows derive is computed here, not read from
    # the check: the edited sha256 changes it.
    d=$(python3 -I -c '
import hashlib, importlib.util, sys
spec = importlib.util.spec_from_file_location("d", sys.argv[1]); m = importlib.util.module_from_spec(spec); spec.loader.exec_module(m)
rs = [l.rstrip("\n").split("\t") for l in open(sys.argv[2]) if l.startswith("base-passwd\t")]
print(hashlib.sha256(m.manifest_bytes(rs[0][0], rs[0][1], [(r[2], r[4], int(r[5])) for r in rs])).hexdigest())' "$check" "$dir/$name.tsv")
    want="F: base-passwd 3.6.7 base-passwd_3.6.7.dsc names manifest sha256:$base, but its rows derive sha256:$d"
  fi
  run "$name" "$want"
done <<< "$table"

for f in "$dir"/*; do
  n=$(basename "$f" .tsv)
  printf '%s\n' "${listed[@]}" | grep -qxF -- "$n" || { echo "FAIL $f is not in the table"; fail=1; }
done
for t in container/ARDUPILOT-DEBIAN-GPL2.tsv container/TOOLS-DEBIAN-GPL2.tsv; do
  if python3 -I "$check" check "$t" > /dev/null; then echo "ok   $t passes"; else echo "FAIL $t is refused"; fail=1; fi
done
exit "$fail"
