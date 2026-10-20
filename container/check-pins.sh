#!/usr/bin/env bash
# container/check-pins.sh: refuses a Dockerfile that can drift (INV-06). Every
# FROM is pinned by sha256 digest, every curl download is checked by
# `sha256sum -c` against a digest written in the file, every apt package has an
# exact version, and nothing is piped into a shell. Needs no container runtime.
#
#   bash container/check-pins.sh [container/Dockerfile]
set -euo pipefail
set -f  # no globbing when words are split below

file=${1:-container/Dockerfile}
[ -f "$file" ] || { echo "check-pins: $file not found" >&2; exit 1; }
bad=0
refuse() {
  echo "check-pins: $1" >&2
  bad=1
}

# Bases pinned by digest.
while read -r line; do
  [[ "$line" =~ @sha256:[0-9a-f]{64}([[:space:]]|$) ]] || refuse "unpinned base: $line"
done < <(grep -E '^FROM[[:space:]]' "$file")
grep -qE '^FROM[[:space:]]' "$file" || refuse "no FROM line"

# Every curl output file has a sha256sum check naming it.
while read -r out; do
  grep -qE "echo \"[0-9a-f]{64}  $out\" \| sha256sum -c -" "$file" || refuse "download without sha256 check: $out"
done < <(grep -oE 'curl [^|&]*-[a-zA-Z]*o +[^ ]+' "$file" | awk '{ print $NF }')

# No pipe into a shell.
if grep -qE '\|[[:space:]]*(ba|z|da)?sh\b' "$file"; then refuse "pipes a download into a shell"; fi

# Every package on an `apt-get install` line or its continuations is name=version.
in_install=0
while IFS= read -r line; do
  if [[ "$line" == *"apt-get install"* ]]; then
    in_install=1
    line=${line#*apt-get install}
  fi
  [ "$in_install" -eq 1 ] || continue
  for word in ${line%\\}; do
    case "$word" in
      \&\&*) in_install=0; break ;;
      -*) ;;
      *=*) ;;
      *) refuse "apt package without an exact version: $word" ;;
    esac
  done
  [[ "$line" == *\\ ]] || in_install=0
done < "$file"

[ "$bad" -eq 0 ] || exit 1
echo "check-pins: $file ok"
