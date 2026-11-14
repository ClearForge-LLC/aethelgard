#!/usr/bin/env bash
# AET-WO-1907: shard the `tools` job's tests. Bash and awk only (RUL-12); no Python.
#
# Entry: one line per listed test occurrence, `suite<TAB>binary<TAB>test`.
#   suite   `ws` (the workspace) or `bridge` (the analog bridge, outside it).
#   binary  what cargo printed after `Running`, e.g. `tests/board_tier2.rs`.
# A test NAME is the unit of assignment: `--exact` selects by name across every binary of a
# suite, so two binaries holding one name run it in the same shard, and the shard must see as
# many passes as there are occurrences. A shard file is `suite<TAB>name<TAB>occurrences`.
#
# Job steps (run from the repository root, inside the image; the cargo commands are the job's own):
#   shard.sh plan OUT       list every test, assign them to the shards (ci/SHARDS), run the guard
#   shard.sh run OUT K      run shard K's tests, then check the passes against the assignment
# Pure functions over files, each refusing with a message and a nonzero exit (INV-06):
#   parse SUITE LOG                a `cargo test -- --list` log -> entries on stdout
#   assign N WEIGHTS ENTRIES OUTDIR [BASE0 [COLOCATE]]
#                                  LPT assignment -> OUTDIR/shard-K.tsv
#   guard N ENTRIES OUTDIR         the shard lists are disjoint and their union is the list
#   names SHARDFILE SUITE          the names a shard runs in a suite, one per line
#   result K SHARDFILE WSLOG BRIDGELOG TIER2_RE
#                                  passes in the logs equal the assignment; prints the result line
#   aggregate DIR N TIER2_MIN      across the shards' artifacts: nothing missing, floor held
set -euo pipefail
export LC_ALL=C
here=$(cd "$(dirname "$0")" && pwd)
TAB=$(printf '\t')

# ci/test-weights.tsv holds CI seconds directly (AET-WO-1908): each test run alone on a GitHub
# runner in the pinned image. A test it does not list gets DEFAULT_WEIGHT, the measured mean of
# the 927 tests under 0.5 s (0.042 s), rounded up: stated, not hidden.
DEFAULT_WEIGHT=0.05
#
# REFIT RULE (AET-WO-1908 §8): refresh the weights when any tools-shard job passes 20 minutes, start to end,
# or when the spread (slowest shard minus fastest) passes 6 minutes on two runs in a row.
#  1. In a throwaway commit on the PR, change `run` so each name runs in its own invocation:
#       RUSTC_BOOTSTRAP=1 cargo test ... -- -Z unstable-options --report-time --test-threads=1 --exact NAME
#     (RUSTC_BOOTSTRAP=1 lets the pinned stable toolchain take libtest's nightly-only flag; it makes
#     cargo rebuild once, a minute). Print each test's `ok <seconds>` with its name. A test that spawns
#     a tool can interleave its output with the result line: take the time from a later `ok <t>` line
#     and the name from the last `test NAME ...` line, never from one line.
#  2. Read the times from the job logs (`gh run view --job ID --log`), write `binary<TAB>test<TAB>seconds`
#     for every test of 0.5 s or more into ci/test-weights.tsv, and revert the commit. Never weigh a
#     test from a shared-process run: tests that take a lock (board_tier2) read as their queue time.
#  3. Set a binary whose tests overlap in ONE process (attribute_seeded) in ci/colocate.tsv to the
#     worst in-process wall time of the block over recent runs, not the sum of its alone times.
#  4. ci/SHARDS stays 8 (Scotty, 2026-11-14). Rebalance with the block whole (ci/colocate.tsv), and
#     check the plan with `bash ci/shard.sh plan DIR`, which prints each shard's planned seconds.

die() { echo "shard: $*" >&2; exit 1; }
readable() { [ -r "$1" ] || die "cannot read $1"; }

# ---- parse -----------------------------------------------------------------

parse() {
  local suite=$1 log=$2
  case $suite in ws | bridge) ;; *) die "unknown suite '$suite'" ;; esac
  readable "$log"
  awk -v suite="$suite" -v lf="$log" '
    function bad(m) { print "shard: " lf ":" NR ": " m > "/dev/stderr"; bad_seen = 1; exit 1 }
    /^[ \t]*$/ { next }
    /^[ \t]*Running .* \(/ { b = $0; sub(/^[ \t]*Running /, "", b); sub(/ \(.*$/, "", b); binary = b; next }
    /^[ \t]*(Finished|Running|Doc-tests|Compiling|Fresh|Blocking|Downloaded|Downloading|Updating|Locking|Adding)([ \t]|$)/ { next }
    /^[0-9]+ tests?, [0-9]+ benchmarks?$/ { next }
    /^[^ \t]+: test$/ {
      if (binary == "") bad("a test before any `Running` line")
      name = $0; sub(/: test$/, "", name)
      print suite "\t" binary "\t" name; n++; next
    }
    { bad("unrecognised line in the test list: " substr($0, 1, 120)) }
    END { if (!bad_seen && suite == "ws" && n == 0) { print "shard: the workspace listed no tests" > "/dev/stderr"; exit 1 } }
  ' "$log"
}

# ---- assign ----------------------------------------------------------------

assign() {
  local n=$1 weights=$2 entries=$3 out=$4 base0=${5:-0} colocate=${6:-/dev/null}
  case $n in '' | *[!0-9]* | 0) die "need at least one shard" ;; esac
  readable "$weights"; readable "$entries"; readable "$colocate"
  [ -s "$entries" ] || die "$entries lists no tests"
  mkdir -p "$out"
  local units="$out/.units"
  # Pass 1: one row per member of a unit: weight, the unit's first key (suite, name), the member (suite, name),
  # its occurrences. A binary whose tests overlap when run together (they share the runner's cores) is
  # kept whole: its tests are one block of the stated CI seconds, so splitting it does not lose the overlap.
  awk -F'\t' -v wf="$weights" -v cf="$colocate" -v dflt="$DEFAULT_WEIGHT" '
    function bad(m) { print "shard: " m > "/dev/stderr"; exit 1 }
    FILENAME == wf { if ($0 ~ /^#/ || $0 == "") next
      if (NF != 3) bad(wf ":" FNR ": want 3 tab-separated fields, got " NF)
      if ($3 !~ /^[0-9]+([.][0-9]+)?$/) bad(wf ": " $3 " is not seconds")
      w[$1 "\t" $2] = $3; next }
    FILENAME == cf { if ($0 ~ /^#/ || $0 == "") next
      if (NF != 2) bad(cf ":" FNR ": want 2 tab-separated fields, got " NF)
      if ($2 !~ /^[0-9]+([.][0-9]+)?$/) bad(cf ": " $2 " is not seconds")
      blk[$1] = $2; next }
    { if (NF != 3) bad("entries:" FNR ": want 3 tab-separated fields, got " NF)
      if ($1 != "ws" && $1 != "bridge") bad("entries: unknown suite " $1)
      ne++; es[ne] = $1; eb[ne] = $2; et[ne] = $3 }
    END {
      for (i = 1; i <= ne; i++) { k = es[i] "\t" et[i]; occ[k]++
        if ((eb[i] in blk) && !(k in bl)) bl[k] = eb[i] }
      for (i = 1; i <= ne; i++) { k = es[i] "\t" et[i]
        if (k in bl) continue
        wt[k] += ((eb[i] "\t" et[i]) in w) ? w[eb[i] "\t" et[i]] : dflt }
      for (k in bl) { b = bl[k]; if (!(b in rep) || k < rep[b]) rep[b] = k }
      for (k in wt) printf "%.6f\t%s\t%s\t%d\n", wt[k], k, k, occ[k]
      for (k in bl) printf "%.6f\t%s\t%s\t%d\n", blk[bl[k]], rep[bl[k]], k, occ[k]
    }' "$weights" "$colocate" "$entries" >"$units" || exit 1
  # A total order: heaviest first, then by the unit's first key, then the member.
  LC_ALL=C sort -t "$TAB" -k1,1gr -k2,2 -k3,3 -k4,4 -k5,5 "$units" >"$units.sorted"
  # Pass 2: heaviest first onto the lightest shard, ties to the lowest index.
  awk -F'\t' -v n="$n" -v base0="$base0" -v out="$out" '
    function flush(   j, best, i) {
      if (gid == "") return
      best = 0
      for (j = 1; j < n; j++) if (load[j] < load[best]) best = j
      load[best] += gw
      for (i = 1; i <= gm; i++) print mem[i] > (out "/.raw-" best)
      cnt[best] += gm
      gm = 0
    }
    BEGIN { for (j = 0; j < n; j++) { load[j] = (j == 0) ? base0 : 0; printf "" > (out "/.raw-" j) } }
    { id = $2 "\t" $3
      if (id != gid) { flush(); gid = id; gw = $1 }
      mem[++gm] = $4 "\t" $5 "\t" $6 }
    END { flush()
      for (j = 0; j < n; j++) { close(out "/.raw-" j); printf "shard %d: %d tests, planned %.0f s\n", j, cnt[j], load[j] } }
  ' "$units.sorted" || exit 1
  local j
  for ((j = 0; j < n; j++)); do
    LC_ALL=C sort -t "$TAB" -k1,1 -k2,2 "$out/.raw-$j" >"$out/shard-$j.tsv"
    rm -f "$out/.raw-$j"
  done
  rm -f "$units" "$units.sorted"
}

# ---- guard -----------------------------------------------------------------

guard() {
  local n=$1 entries=$2 out=$3 files=() j
  for ((j = 0; j < n; j++)); do readable "$out/shard-$j.tsv"; files+=("$out/shard-$j.tsv"); done
  readable "$entries"
  awk -F'\t' -v n="$n" -v ef="$entries" '
    function bad(m) { print "shard: guard: " m > "/dev/stderr"; done = 1; exit 1 }
    BEGIN { for (k = 2; k < ARGC; k++) idx[ARGV[k]] = k - 2 }
    FILENAME == ef { if (NF != 3) bad(ef ":" FNR ": want 3 fields"); occ[$1 "\t" $3]++; ne++; next }
    { if (NF != 3) bad(FILENAME ":" FNR ": want 3 fields")
      i = idx[FILENAME]; key = $1 "\t" $2; cnt[i]++
      if (key in seen) { if (seen[key] == i) bad(FILENAME " lists " $1 " " $2 " twice"); bad($1 " " $2 " is in shard " seen[key] " and shard " i) }
      seen[key] = i
      if ($3 !~ /^[0-9]+$/ || $3 < 1) bad(FILENAME ": bad occurrence count " $3)
      if (!(key in occ)) { extra++; if (extra == 1) firstextra = $1 " " $2; next }
      if (occ[key] != $3) bad($1 " " $2 ": shard " i " says " $3 " occurrence(s), the list has " occ[key]) }
    END {
      if (done) exit 1
      if (ne == 0) bad("the full list is empty")
      for (i = 0; i < n; i++) if (cnt[i] == 0) bad("shard " i " lists nothing")
      for (k in occ) if (!(k in seen)) { miss++; if (miss == 1 || k < firstmiss) firstmiss = k }
      if (miss) { gsub(/\t/, " ", firstmiss); bad(miss " test(s) in no shard, first: " firstmiss) }
      if (extra) bad(extra " assigned test(s) not in the list, first: " firstextra)
      names = 0; for (k in occ) { names++; total += occ[k] }
      printf "guard ok: %d test names, %d listed occurrences, %d shards\n", names, total, n }
  ' "$entries" "${files[@]}"
}

names() {
  local shardfile=$1 suite=$2
  readable "$shardfile"
  awk -F'\t' -v s="$suite" '$1 == s { print $2 }' "$shardfile"
}

# ---- result ----------------------------------------------------------------

# passed failed ignored tier2 of a cargo test log; a doc-test result is never credited to the
# last tier-2 binary.
tally() {
  local log=$1 re=$2
  readable "$log"
  awk -v re="$re" '
    /^ +Running / { file = $2; next }
    /^ +Doc-tests / { file = ""; next }
    /^test result: [A-Za-z]+\. [0-9]+ passed; [0-9]+ failed; [0-9]+ ignored/ {
      p += $4; f += $6; i += $8; if (file != "" && file ~ re) t += $4 }
    END { printf "%d %d %d %d\n", p, f, i, t }
  ' "$log"
}

result() {
  local k=$1 shardfile=$2 wslog=$3 bridgelog=$4 re=$5
  readable "$shardfile"
  [ -s "$shardfile" ] || die "shard $k lists nothing; a shard that runs nothing fails"
  local suite log expect total=0 tier2=0 expected=0 p f ig t2
  for suite in ws bridge; do
    if [ "$suite" = ws ]; then log=$wslog; else log=$bridgelog; fi
    expect=$(awk -F'\t' -v s="$suite" '$1 == s { n += $3 } END { print n + 0 }' "$shardfile")
    expected=$((expected + expect))
    if [ "$expect" -eq 0 ]; then
      [ ! -e "$log" ] || die "$log ran though shard $k has no $suite tests"
      continue
    fi
    read -r p f ig t2 < <(tally "$log" "$re")
    { [ "$f" -eq 0 ] && [ "$ig" -eq 0 ]; } || die "$suite: $f failed, $ig ignored"
    [ "$p" -eq "$expect" ] || die "$suite: $p passed, the assignment is $expect"
    total=$((total + p))
    tier2=$((tier2 + t2))
  done
  echo "shard=$k expected=$expected passed=$total tier2=$tier2"
}

# ---- aggregate -------------------------------------------------------------

# field NAME FILE: the value of NAME in a `a=1 b=2` result line
field() { awk -v k="$1" '{ for (i = 1; i <= NF; i++) { split($i, kv, "="); if (kv[1] == k) print kv[2] } }' "$2"; }

aggregate() {
  local d=$1 n=$2 floor=$3 i base e p t expected=0 passed=0 tier2=0 tmp names occurrences g
  case $n in '' | *[!0-9]* | 0) die "need at least one shard" ;; esac
  case $floor in '' | *[!0-9]*) die "the floor is not a number" ;; esac
  tmp=$(mktemp -d)
  trap 'rm -rf "$tmp"' RETURN
  for ((i = 0; i < n; i++)); do
    base=$d/shard-$i
    [ -f "$base/result.txt" ] || die "shard $i left no result (it did not finish)"
    [ "$(field shard "$base/result.txt")" = "$i" ] || die "shard $i: the result names shard '$(field shard "$base/result.txt")'"
    e=$(field expected "$base/result.txt"); p=$(field passed "$base/result.txt"); t=$(field tier2 "$base/result.txt")
    case $e$p$t in '' | *[!0-9]*) die "shard $i: the result is malformed" ;; esac
    [ "$p" -eq "$e" ] || die "shard $i: $p passed of $e assigned"
    { [ -f "$base/entries.tsv" ] && [ -f "$base/shard.tsv" ]; } || die "shard $i left no list"
    cmp -s "$base/entries.tsv" "$d/shard-0/entries.tsv" || die "shard $i listed different tests"
    [ "$(awk -F'\t' '{ n += $3 } END { print n + 0 }' "$base/shard.tsv")" -eq "$e" ] || die "shard $i: $e assigned in its result, its own list says otherwise"
    cp "$base/shard.tsv" "$tmp/shard-$i.tsv"
    expected=$((expected + e)); passed=$((passed + p)); tier2=$((tier2 + t))
  done
  # Re-run the guard over what the shards themselves ran, from their own files.
  g=$(guard "$n" "$d/shard-0/entries.tsv" "$tmp") || exit 1
  names=$(sed -n 's/^guard ok: \([0-9]*\) test names, \([0-9]*\) listed.*/\1/p' <<<"$g")
  occurrences=$(sed -n 's/^guard ok: \([0-9]*\) test names, \([0-9]*\) listed.*/\2/p' <<<"$g")
  { [ "$expected" -eq "$occurrences" ] && [ "$passed" -eq "$occurrences" ]; } \
    || die "$passed passed and $expected assigned across the shards; the list has $occurrences"
  echo "ANSWER tools_total_passed=$passed names=$names"
  echo "ANSWER tier2_tests_passed=$tier2 floor=$floor"
  [ "$tier2" -ge "$floor" ] || die "only $tier2 tier-2 tests passed across the shards, the floor is $floor"
}

# ---- the job's steps -------------------------------------------------------

FEATURES=(--locked --offline --features aethelgard-cli/with-tools)
BRIDGE=(--manifest-path bridge/Cargo.toml --locked --offline)
# Canary files (AET-WO-1900): the tier-2 test binaries, as cargo prints them.
TIER2_RE='^tests/(with_tools|twin_tier2|board_tier2|campaign_tier2|attribute_seeded|adversarial_1301_tools|adversarial_1302_tools)[.]rs$'
# Fixed work shard 0 does besides tests (bridge fmt + clippy), in seconds, measured on CI: the two steps took 25 s.
BASE0=${SHARD0_FIXED_SECONDS:-25}

plan() {
  local out=$1 n
  n=$(tr -d '[:space:]' <"$here/SHARDS")
  case $n in '' | *[!0-9]* | 0) die "ci/SHARDS must hold a positive integer" ;; esac
  # stderr carries the `Running` lines the parser needs; both streams, in order.
  cargo test --workspace "${FEATURES[@]}" -- --list >"$out/list-ws.log" 2>&1 \
    || { cat "$out/list-ws.log"; die "the workspace test list failed"; }
  cargo test "${BRIDGE[@]}" -- --list >"$out/list-bridge.log" 2>&1 \
    || { cat "$out/list-bridge.log"; die "the bridge test list failed"; }
  { parse ws "$out/list-ws.log"; parse bridge "$out/list-bridge.log"; } >"$out/entries.tsv"
  assign "$n" "$here/test-weights.tsv" "$out/entries.tsv" "$out" "$BASE0" "$here/colocate.tsv"
  guard "$n" "$out/entries.tsv" "$out"
}

run() {
  local out=$1 k=$2 shard="$1/shard-$2.tsv" ws br status=0
  readable "$shard"
  cp "$shard" "$out/shard.tsv"
  rm -f "$out/test-ws.log" "$out/test-bridge.log" "$out/result.txt"
  # An empty name list would make --exact run every test: refuse it per suite.
  mapfile -t ws < <(names "$shard" ws)
  mapfile -t br < <(names "$shard" bridge)
  if [ "${#ws[@]}" -gt 0 ]; then
    set +e
    cargo test --workspace "${FEATURES[@]}" -- --exact "${ws[@]}" 2>&1 | tee "$out/test-ws.log"
    [ "${PIPESTATUS[0]}" -eq 0 ] || status=1
    set -e
  fi
  if [ "${#br[@]}" -gt 0 ]; then
    set +e
    cargo test "${BRIDGE[@]}" -- --exact "${br[@]}" 2>&1 | tee "$out/test-bridge.log"
    [ "${PIPESTATUS[0]}" -eq 0 ] || status=1
    set -e
  fi
  [ "$status" -eq 0 ] || die "shard $k: a test run failed"
  result "$k" "$shard" "$out/test-ws.log" "$out/test-bridge.log" "$TIER2_RE" | tee "$out/result.txt"
}

cmd=${1:?usage: shard.sh parse|assign|guard|names|result|aggregate|plan|run ...}
shift
case $cmd in
  parse | assign | guard | names | result | aggregate | tally) "$cmd" "$@" ;;
  plan) mkdir -p "$1"; plan "$@" ;;
  run) run "$@" ;;
  *) echo "usage: shard.sh parse|assign|guard|names|result|aggregate|plan|run" >&2; exit 2 ;;
esac
