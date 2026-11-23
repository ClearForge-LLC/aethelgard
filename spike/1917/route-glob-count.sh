#!/usr/bin/env bash
# AET-WO-1917 §1.1 (4): over today's tree, the literals that carry a format placeholder `{...}` or end in `/`, and which of
# them could name a docs-only path once each placeholder is read as "any run of characters" and a trailing slash dropped.
# Two readings are counted:
#   anchored  the literal holds a docs-only anchor outside its placeholders: `docs/` or `docs` as a segment, `fab/`, a
#             root name stem (README, LICENSE) or `.md`; the unresolvable `{}/{}` forms are not counted
#   any       every literal whose wildcard reading matches some docs-only path, directory or name (fully fail-closed)
# Usage: bash spike/1917/route-glob-count.sh LITS.tsv   (LITS: route.sh's `lits` over every tracked .rs, file<TAB>kind<TAB>lit)
set -euo pipefail
export LC_ALL=C
root=$(cd "$(dirname "$0")/../.." && pwd)
lits=$1
docs=$(mktemp); trap 'rm -f "$docs"' EXIT
# The positive list, as ci/route.sh is_docs reads it.
git -C "$root" ls-files | awk '
  /^fab\/package\// || /^fab\/[^\/]*\/package\// { next }
  /^fab\/.*\/BENCH\.md$/ { next }
  /^docs\// { print; next }
  /^fab\/.*\.md$/ { print; next }
  /\// { next }
  /\.md$/ || /^README/ || /^LICENSE/ { print }' >"$docs"
awk -F'\t' -v df="$docs" '
  function norm(p) { while (sub(/^(\.\.\/|\.\/|\/)/, "", p)) ; return p }
  function rx(l,   out, i, c) { out = ""
    for (i = 1; i <= length(l); i++) { c = substr(l, i, 1)
      if (c == "{") { j = index(substr(l, i), "}"); if (j == 0) { out = out "\\{"; continue } out = out ".*"; i += j - 1; continue }
      if (c ~ /[][\\.^$*+?()|]/) out = out "\\" c; else out = out c }
    return "^" out "$" }
  FILENAME == df { full[$0] = 1; n = split($0, seg, "/"); b[seg[n]] = 1
    d = $0; while (sub(/\/[^\/]*$/, "", d)) dir[d] = 1; next }
  {
    L = $3
    ph = (L ~ /\{[^}]*\}/); ts = (L ~ /\/$/)
    if (!ph && !ts) next
    m = norm(L); sub(/\/+$/, "", m); if (m == "") next
    r = rx(m); hit = ""
    for (p in full) if (p ~ r) { hit = p; break }
    if (hit == "") for (p in dir) if (p ~ r) { hit = p "/"; break }
    if (hit == "") for (p in b) if (p ~ r) { hit = "name " p; break }
    if (hit == "") next
    stripped = L; gsub(/\{[^}]*\}/, "", stripped)
    anchored = (stripped ~ /(^|\/)docs(\/|$)/ || stripped ~ /(^|\/)fab\// || stripped ~ /README|LICENSE/ || stripped ~ /\.md/)
    print (anchored ? "anchored" : "any") "\t" $1 "\t" L "\t" hit
  }' "$docs" "$lits" | sort | uniq -c | sort -k2,2 -k1,1rn
