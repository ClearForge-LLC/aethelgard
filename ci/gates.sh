#!/usr/bin/env bash
# AET-WO-1910 D5 (a), ruled on PR #233 (comment 6999515740): the `gates` job's tests, on the host, with every test target
# but those ci/tools-only.tsv names, which run in the `tools` job alone. Bash and jq only (RUL-12).
#
#   gates.sh targets METADATA LIST   the `--test` names gates runs, one per line, from `cargo metadata` JSON; refuses by
#                                    name a LIST row that names no test target, one whose name is the test target of a
#                                    second package (a `--test NAME` would run both), and any target of a kind neither
#                                    job runs by these flags
#   gates.sh run                     build the examples (tier 1's stand-in tools are examples), then test the libraries,
#                                    binaries and every other test target by name, then the doctests
set -euo pipefail
export LC_ALL=C
here=$(cd "$(dirname "$0")" && pwd)

die() { echo "gates: $*" >&2; exit 1; }
readable() { [ -r "$1" ] || die "cannot read $1"; }

targets() {
  local meta=$1 list=$2 all moved bad
  readable "$meta"; readable "$list"
  # package<TAB>kind<TAB>name for every target of every workspace package.
  all=$(jq -r '.packages[] | .name as $p | .targets[] | "\($p)\t\(.kind | join(","))\t\(.name)"' "$meta") \
    || die "$meta is not cargo metadata"
  [ -n "$all" ] || die "$meta lists no targets"
  # A kind these flags do not run (a bench, say) would run in neither job by name.
  bad=$(awk -F'\t' '$2 !~ /^(lib|rlib|proc-macro|bin|test|example|custom-build)$/ { print $1 " " $3 " (" $2 ")" }' <<<"$all")
  [ -z "$bad" ] || die "a target of a kind gates.sh does not run: $bad"
  moved=$(awk -F'\t' '/^#/ || NF == 0 { next } NF != 2 { print "BAD " FNR; next } { print $1 "\t" $2 }' "$list")
  if grep -q '^BAD ' <<<"$moved"; then die "$list: want package<TAB>target, line $(grep '^BAD ' <<<"$moved" | head -1 | cut -d' ' -f2)"; fi
  awk -F'\t' -v moved="$moved" '
    BEGIN { n = split(moved, m, "\n"); for (i = 1; i <= n; i++) if (m[i] != "") want[m[i]] = 1 }
    $2 == "test" { tests++; have[$1 "\t" $3] = 1; owners[$3]++; name[$1 "\t" $3] = $3 }
    END {
      for (k in want) {
        if (!(k in have)) { split(k, kk, "\t"); print "gates: tools-only names no test target: " kk[1] " " kk[2] > "/dev/stderr"; failed = 1; continue }
        if (owners[name[k]] > 1) { print "gates: tools-only " name[k] " is a test target of " owners[name[k]] " packages; --test cannot tell them apart" > "/dev/stderr"; failed = 1 }
      }
      if (failed) exit 1
      for (k in have) if (!(k in want)) { split(k, kk, "\t"); run[kk[2]] = 1 }
      ran = 0; for (r in run) ran++
      for (r in run) print r
      # The union is the list: every test target either runs here by name or is named tools-only.
      moved = 0; for (k in want) moved++
      covered = 0; for (k in have) if ((k in want) || (name[k] in run)) covered++
      if (covered != tests) { print "gates: " tests - covered " test target(s) in neither job" > "/dev/stderr"; exit 1 }
    }' <<<"$all" | sort
}

run() {
  local meta args=() t
  meta=$(mktemp); trap 'rm -f "$meta"' RETURN
  cargo metadata --no-deps --format-version 1 --locked >"$meta"
  mapfile -t names < <(targets "$meta" "$here/tools-only.tsv")
  [ "${#names[@]}" -gt 0 ] || die "no test target to run"
  for t in "${names[@]}"; do args+=(--test "$t"); done
  echo "gates: ${#names[@]} test targets by name; tools only: $(grep -cv '^#' "$here/tools-only.tsv")"
  cargo build --workspace --locked --examples
  cargo test --workspace --locked --lib --bins "${args[@]}"
  cargo test --workspace --locked --doc
}

cmd=${1:?usage: gates.sh targets METADATA LIST | run}
shift
case $cmd in
  targets) targets "$@" ;;
  run) run ;;
  *) echo "usage: gates.sh targets METADATA LIST | run" >&2; exit 2 ;;
esac
