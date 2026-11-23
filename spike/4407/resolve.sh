#!/usr/bin/env bash
# spike/4407/resolve.sh NAME BASE SOURCES|'' PKG=VER...: what apt would install
# on BASE for the explicit pins, as one capped container (Furnace rule 3). With
# SOURCES, that deb822 file replaces the base's debian.sources (the snapshot
# shape); with '', the base's own moving sources are used. Writes NAME.out
# (apt's output) and NAME.tsv (package<TAB>version, sorted, from the Inst lines).
set -euo pipefail
name=$1 base=$2 src=$3; shift 3
m=(); [ -z "$src" ] || m=(-v "$(realpath "$src"):/etc/apt/sources.list.d/debian.sources:ro")
rc=0
nice -n 10 ~/bin/capped-run --mem 2G -- --rm "${m[@]}" --user 0 --entrypoint /bin/sh "$base" -c \
  "apt-get update -q >/tmp/u 2>&1 || { tail -3 /tmp/u; exit 1; }; apt-get install -s -y --no-install-recommends $*" > "$name.out" 2>&1 || rc=$?
grep '^Inst ' "$name.out" | sed -E 's/^Inst ([^ ]+) (\[[^]]*\] )?\(([^ ]+) .*/\1\t\3/' | LC_ALL=C sort > "$name.tsv" || true
echo "$name exit=$rc $(wc -l < "$name.tsv") packages"; grep -E '^E: ' "$name.out" || true
