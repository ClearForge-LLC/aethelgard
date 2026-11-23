#!/usr/bin/env bash
# AET-WO-1907 §3: the guard fails each run it exists to fail. Fixtures only; no cargo, no image.
set -uo pipefail
export LC_ALL=C
# The run logs below are timed as on a runner's four test threads unless a fixture says otherwise (AET-WO-1914 P3).
export RUST_TEST_THREADS=4
here=$(cd "$(dirname "$0")" && pwd)
py=(bash "$here/shard.sh")
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
fail=0
# Which awk the fixtures run under (AET-WO-1914: gawk warns on a `\.` in a -v value, mawk does not).
echo "== awk: $(awk -W version 2>&1 | head -1)"

ok()   { echo "PASS $1"; }
bad()  { echo "FAIL $1"; fail=1; }
# refuses NAME REASON CMD...: the command must exit nonzero, say REASON, and print no awk warning (AET-WO-1914: a
# refusal that exits for another reason, or whose first line is gawk's `\.` warning, is not the refusal it names).
refuses() {
  local name=$1 reason=$2; shift 2
  if "$@" >"$tmp/out" 2>&1; then bad "$name: accepted"
  elif ! grep -qF -- "$reason" "$tmp/out"; then bad "$name: refused, but not because: $reason (it said: $(head -1 "$tmp/out" | cut -c1-160))"
  elif grep -q 'awk: .*warning' "$tmp/out"; then bad "$name: an awk warning: $(grep -m1 'awk: .*warning' "$tmp/out")"
  else ok "$name: refused ($reason)"; fi
}
accepts() { local name=$1; shift; if "$@" >"$tmp/out" 2>&1; then ok "$name"; else bad "$name: $(head -1 "$tmp/out")"; fi; }
# The harness itself (AET-WO-1914): a refusal for another reason, or one that prints an awk warning, is a FAIL.
(refuses "x" "the reason" bash -c 'echo another reason >&2; exit 1') | grep -q '^FAIL x: refused, but not because: the reason' \
  && ok "refuses: a refusal for another reason fails" || bad "refuses: a refusal for another reason passed"
(refuses "y" "the reason" bash -c 'echo "awk: warning: escape sequence"; echo the reason; exit 1') | grep -q '^FAIL y: an awk warning' \
  && ok "refuses: a refusal that prints an awk warning fails" || bad "refuses: an awk warning passed"

suite() {
local N=$1
echo "== $N shards"
# A list: eight ws tests (one name in two binaries), two bridge tests, two tier-2 binaries.
mk_list() {
  {
    printf '     Running unittests src/lib.rs (target/debug/deps/a-1)\n'
    for t in u1 u2 u3 shared; do printf '%s: test\n' "$t"; done
    printf '\n4 tests, 0 benchmarks\n     Running tests/board_tier2.rs (target/debug/deps/b-2)\n'
    for t in v1 v2 shared2; do printf '%s: test\n' "$t"; done
    printf '\n     Running tests/other.rs (target/debug/deps/c-3)\n'
    for t in shared w1; do printf '%s: test\n' "$t"; done
    printf '\n   Doc-tests aethelgard_ir\n\n0 tests, 0 benchmarks\n'
  } >"$tmp/ws.log"
  printf '     Running unittests src/main.rs (target/debug/deps/d-4)\nbr1: test\nbr2: test\n' >"$tmp/br.log"
  { "${py[@]}" parse ws "$tmp/ws.log"; "${py[@]}" parse bridge "$tmp/br.log"; } >"$tmp/entries.tsv"
  printf 'tests/board_tier2.rs\tv1\t100\n' >"$tmp/weights.tsv"
}
mk_list
accepts "plan: $N shards from the list" "${py[@]}" assign $N "$tmp/weights.tsv" "$tmp/entries.tsv" "$tmp/plan"
accepts "guard: the plan passes" "${py[@]}" guard $N "$tmp/entries.tsv" "$tmp/plan"
[ "$(cat "$tmp/plan"/shard-*.tsv | wc -l)" -eq 10 ] && ok "plan: 10 distinct names (a name in two binaries is one unit)" || bad "plan: unit count"
grep -q "shared	2" "$tmp/plan"/shard-*.tsv && ok "plan: the shared name carries 2 occurrences" || bad "plan: occurrences"

# A co-located binary stays whole in one shard.
printf 'tests/board_tier2.rs\t30\n' >"$tmp/colocate.tsv"
accepts "plan: a co-located binary" "${py[@]}" assign $N "$tmp/weights.tsv" "$tmp/entries.tsv" "$tmp/plan2" 0 "$tmp/colocate.tsv"
n=$(grep -l "^ws	v1	" "$tmp/plan2"/shard-*.tsv | wc -l); m=$(cat $(grep -l "^ws	v1	" "$tmp/plan2"/shard-*.tsv) | grep -c "^ws	\(v1\|v2\|shared2\)	")
[ "$n" = 1 ] && [ "$m" = 3 ] && ok "plan: the co-located binary's three tests are in one shard" || bad "plan: co-location"
accepts "guard: the co-located plan passes" "${py[@]}" guard $N "$tmp/entries.tsv" "$tmp/plan2"

# Each failure, in a copy of the good plan.
break_plan() { rm -rf "$tmp/p"; cp -r "$tmp/plan" "$tmp/p"; }

break_plan; first=$(head -1 "$tmp/p/shard-1.tsv"); sed -i 1d "$tmp/p/shard-1.tsv"
[ -s "$tmp/p/shard-1.tsv" ] || printf 'ws\tfiller\t1\n' >"$tmp/p/shard-1.tsv"
refuses "guard: a test assigned to no shard" "1 test(s) in no shard, first: ws $(cut -f2 <<<"$first")" "${py[@]}" guard $N "$tmp/entries.tsv" "$tmp/p"

break_plan; head -1 "$tmp/p/shard-0.tsv" >>"$tmp/p/shard-2.tsv"
refuses "guard: a test in two shards" "ws $(head -1 "$tmp/p/shard-0.tsv" | cut -f2) is in shard 0 and shard 2" "${py[@]}" guard $N "$tmp/entries.tsv" "$tmp/p"

break_plan; : >"$tmp/p/shard-3.tsv"
refuses "guard: a shard that lists nothing" "shard 3 lists nothing" "${py[@]}" guard $N "$tmp/entries.tsv" "$tmp/p"

break_plan; sed -i 's/^\(ws\tshared\t\)2$/\11/' "$tmp/p"/shard-*.tsv
refuses "guard: a name run once though listed in two binaries" "says 1 occurrence(s), the list has 2" "${py[@]}" guard $N "$tmp/entries.tsv" "$tmp/p"

break_plan; printf 'ws\tnot_in_the_list\t1\n' >>"$tmp/p/shard-0.tsv"
refuses "guard: a shard test the list does not have" "1 assigned test(s) not in the list, first: ws not_in_the_list" "${py[@]}" guard $N "$tmp/entries.tsv" "$tmp/p"

: >"$tmp/empty.log"
refuses "parse: a workspace that lists no tests" "the workspace listed no tests" "${py[@]}" parse ws "$tmp/empty.log"
printf '     Running unittests src/lib.rs (x)\nt: test\nmystery output\n' >"$tmp/odd.log"
refuses "parse: an unrecognised line" "odd.log:3: unrecognised line in the test list: mystery output" "${py[@]}" parse ws "$tmp/odd.log"
printf '     Running unittests src/lib.rs (x)\nt: benchmark\n' >"$tmp/bench.log"
refuses "parse: a benchmark" "bench.log:2: unrecognised line in the test list: t: benchmark" "${py[@]}" parse ws "$tmp/bench.log"
refuses "names: no shard file" "cannot read $tmp/missing.tsv" "${py[@]}" names "$tmp/missing.tsv" ws

# A shard's run log against its assignment.
shard0="$tmp/plan/shard-0.tsv"
want=$(awk -F'\t' '$1=="ws" {n += $3} END {print n + 0}' "$shard0")
# st: a log as `run` writes it, each line stamped a second after the last.
st() { awk '{ printf "%d.000000\t%s\n", 1000 + NR, $0 }'; }
# bin BINARY PREFIX N [RESULT]: a binary's block of N results, `test PREFIX1 ... ok` and so on.
bin() {
  printf '     Running %s (target/debug/deps/x-1)\n\nrunning %d tests\n' "$1" "$3"
  for i in $(seq 1 "$3"); do printf 'test %s%d ... ok\n' "$2" "$i"; done
  printf '\ntest result: %s. %d passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.00s\n\n' "${4:-ok}" "$3"
}
mk_log() { # passed tier2-passed
  { bin 'unittests src/lib.rs' t "$1"; bin tests/board_tier2.rs v "$2"; } | st
}
RE='^tests/(board_tier2)[.]rs$'
mk_log "$((want - 0))" 0 >"$tmp/run.log"
accepts "result: passes equal the assignment" "${py[@]}" result 0 "$shard0" "$tmp/run.log" "$tmp/none.log" "$RE"
mk_log "$((want - 1))" 0 >"$tmp/run.log"
refuses "result: one test fewer ran than assigned" "ws: $((want - 1)) passed, the assignment is $want" "${py[@]}" result 0 "$shard0" "$tmp/run.log" "$tmp/none.log" "$RE"
mk_log 0 0 >"$tmp/run.log"
refuses "result: a shard that passes while running nothing" "ws: 0 passed, the assignment is $want" "${py[@]}" result 0 "$shard0" "$tmp/run.log" "$tmp/none.log" "$RE"
printf '     Running unittests src/lib.rs (x)\nrunning 2 tests\ntest a ... ok\ntest b ... FAILED\ntest result: FAILED. 1 passed; 1 failed; 0 ignored\n' | st >"$tmp/run.log"
refuses "result: a failed test" "ws: 1 failed, 0 ignored" "${py[@]}" result 0 "$shard0" "$tmp/run.log" "$tmp/none.log" "$RE"
: >"$tmp/emptyshard.tsv"
refuses "result: a shard file that lists nothing" "shard 0 lists nothing" "${py[@]}" result 0 "$tmp/emptyshard.tsv" "$tmp/run.log" "$tmp/none.log" "$RE"

printf '     Running tests/board_tier2.rs (x)\ntest result: ok. 2 passed; 0 failed; 0 ignored\n   Doc-tests aethelgard_cli\ntest result: ok. 5 passed; 0 failed; 0 ignored\n' >"$tmp/doc.log"
[ "$("${py[@]}" tally "$tmp/doc.log" "$RE" | cut -d" " -f4)" = 2 ] && ok "tally: a doc-test pass is not credited to tier 2" || bad "tally: doc-test attribution"
# The aggregate: $N shards' artifacts.
mk_art() { # dir tier2-per-shard
  rm -rf "$1"
  for i in $(seq 0 $((N - 1))); do
    d="$1/shard-$i"; mkdir -p "$d"
    cp "$tmp/entries.tsv" "$d/entries.tsv"; cp "$tmp/plan/shard-$i.tsv" "$d/shard.tsv"
    n=$(awk -F'\t' '{n += $3} END {print n + 0}' "$d/shard.tsv")
    printf 'shard=%d expected=%d passed=%d tier2=%d\n' "$i" "$n" "$n" "$2" >"$d/result.txt"
  done
}
mk_art "$tmp/art" 2
accepts "aggregate: $N shards, floor held (2 per shard)" "${py[@]}" aggregate "$tmp/art" $N $((2 * N))
refuses "aggregate: tier-2 tests silently stop running (one under the floor)" "only $((2 * N)) tier-2 tests passed across the shards, the floor is $((2 * N + 1))" "${py[@]}" aggregate "$tmp/art" $N $((2 * N + 1))
mk_art "$tmp/art" 0
refuses "aggregate: a tier-2 file runs nothing (0 < 1)" "only 0 tier-2 tests passed across the shards, the floor is 1" "${py[@]}" aggregate "$tmp/art" $N 1
mk_art "$tmp/art" 2; rm -rf "$tmp/art/shard-2"
refuses "aggregate: a shard left no result" "shard 2 left no result" "${py[@]}" aggregate "$tmp/art" $N $((2 * N))
mk_art "$tmp/art" 2; sed -i 's/passed=[0-9]*/passed=0/' "$tmp/art/shard-1/result.txt"
refuses "aggregate: a shard passed nothing" "shard 1: 0 passed of" "${py[@]}" aggregate "$tmp/art" $N $((2 * N))
mk_art "$tmp/art" 2; echo 'ws	extra	x	1' >>"$tmp/art/shard-3/entries.tsv"
refuses "aggregate: a shard listed different tests" "shard 3 listed different tests" "${py[@]}" aggregate "$tmp/art" $N $((2 * N))
mk_art "$tmp/art" 2; head -1 "$tmp/art/shard-0/shard.tsv" >>"$tmp/art/shard-1/shard.tsv"
# Its result counts the extra line too, so the refusal is the guard's, not the count's.
n=$(awk -F'\t' '{n += $3} END {print n + 0}' "$tmp/art/shard-1/shard.tsv"); printf 'shard=1 expected=%d passed=%d tier2=2\n' "$n" "$n" >"$tmp/art/shard-1/result.txt"
refuses "aggregate: a test in two shards" "guard: ws $(head -1 "$tmp/art/shard-0/shard.tsv" | cut -f2) is in shard 0 and shard 1" "${py[@]}" aggregate "$tmp/art" $N $((2 * N))

}
for n in 4 5 7; do suite $n; done

# AET-WO-1910 (ruled on PR #233, comment 6999515740): the planner's two refusals (D6) and the gates split (D5).
echo "== the planner's refusals and the gates split"
fx="$here/fixtures"
# P1: a row that names no listed test, or a block that names no listed binary, is refused by name.
printf 'tests/board_tier2.rs\tv1\t100\ntests/board_tier2.rs\trenamed_away\t12\n' >"$tmp/stale.tsv"
refuses "P1: a weight row naming no listed test" "tests/board_tier2.rs renamed_away names no listed test" "${py[@]}" assign 4 "$tmp/stale.tsv" "$tmp/entries.tsv" "$tmp/p1"
printf 'tests/gone.rs\t30\n' >"$tmp/gone.tsv"
refuses "P1: a block naming no listed binary" "tests/gone.rs names no listed binary" "${py[@]}" assign 4 "$tmp/weights.tsv" "$tmp/entries.tsv" "$tmp/p1b" 0 "$tmp/gone.tsv"
# P2 on CI's own logs: a9b79f43's shards (AET-WO-1314, run 44372757248) against that commit's weights name the eleven
# tests behind its overrun; main 565b32c5's (run 44441889769), against its weights with the rows this build adds, name none.
"${py[@]}" heavy_unweighted "$fx/p2-a9b79f43.log" "$fx/p2-a9b79f43-weights.tsv" "$fx/p2-colocate.tsv" >"$tmp/a9b" 2>&1
[ "$(wc -l <"$tmp/a9b")" -eq 11 ] && [ "$(grep -c '^tests/supply_tier1.rs	' "$tmp/a9b")" -eq 8 ] \
  && [ "$(grep -c '^tests/via_ring_tier1.rs	' "$tmp/a9b")" -eq 2 ] \
  && grep -q '^tests/outline_tier1.rs	the_fixture_fits_its_bound_with_connectors_on_the_side_edges$' "$tmp/a9b" \
  && ok "P2: a9b79f43 names its eleven unweighted heavy tests" || bad "P2: a9b79f43: $(tr '\n' ';' <"$tmp/a9b")"
"${py[@]}" heavy_unweighted "$fx/p2-565b32c5.log" "$fx/p2-565b32c5-weights.tsv" "$fx/p2-colocate.tsv" >"$tmp/base" 2>&1 \
  && [ ! -s "$tmp/base" ] && ok "P2: silent on main 565b32c5" || bad "P2: main 565b32c5: $(tr '\n' ';' <"$tmp/base")"
# P2 in a shard's result: a test with no row that libtest reports past 60 s is refused, by name; weighed, accepted.
mk_heavy() { # passed
  { printf '     Running unittests src/lib.rs (target/debug/deps/a-1)\n\nrunning %d tests\ntest u1 has been running for over 60 seconds\n' "$1"
    for i in $(seq 1 "$1"); do printf 'test t%d ... ok\n' "$i"; done
    printf 'test result: ok. %d passed; 0 failed; 0 ignored; 0 measured; 0 filtered out\n' "$1"; } | st
}
shard=$(ls "$tmp"/plan/shard-*.tsv | head -1); k=${shard##*-}; k=${k%.tsv}
want=$(awk -F'\t' '$1=="ws" {n += $3} END {print n + 0}' "$shard")
mk_heavy "$want" >"$tmp/heavy.log"
: >"$tmp/w0.tsv"
refuses "P2: result refuses an unweighted test past 60 s" "ran over 60 s with no row in w0.tsv (the plan gave each 0.05 s; weigh it by the REFIT RULE): unittests src/lib.rs u1" "${py[@]}" result "$k" "$shard" "$tmp/heavy.log" "$tmp/none.log" "$RE" "$tmp/w0.tsv"
printf 'unittests src/lib.rs\tu1\t70\n' >"$tmp/w1.tsv"
accepts "P2: the same test, weighed, passes" "${py[@]}" result "$k" "$shard" "$tmp/heavy.log" "$tmp/none.log" "$RE" "$tmp/w1.tsv"
# AET-WO-1914 P3: a test that took 10 s or more by its stamps, with no row, is refused by name. One binary, three tests
# on two threads: a and b start at 100, c (third by name) when a finishes at 101.
printf 'ws\ta\t1\nws\tb\t1\nws\tc\t1\n' >"$tmp/p3.tsv"
p3log() { # c's-end b's-end [extra-line-after-a]
  printf '100.000000\t     Running unittests src/lib.rs (target/debug/deps/a-1)\n100.000000\t\n100.000000\trunning 3 tests\n'
  printf '101.000000\ttest a ... ok\n'; [ -z "${3:-}" ] || printf '%s\n' "$3"
  printf '%s\ttest c ... ok\n%s\ttest b ... ok\n%s\t\n%s\ttest result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 9s\n' "$1" "$2" "$2" "$2"
}
p3() { env RUST_TEST_THREADS=2 "${py[@]}" result 0 "$tmp/p3.tsv" "$tmp/p3.log" "$tmp/none.log" "$RE" "$@"; }
printf 'unittests src/lib.rs\tb\t20\n' >"$tmp/wb.tsv"
p3log 105.000000 112.000000 >"$tmp/p3.log"
refuses "P3: an unweighted test of 12 s" "took 10 s or more with no row in w0.tsv (the plan gave each 0.05 s; weigh it by the REFIT RULE): unittests src/lib.rs b 12.000" p3 "$tmp/w0.tsv"
accepts "P3: the same test, weighed, passes" p3 "$tmp/wb.tsv"
p3log 111.000000 112.000000 >"$tmp/p3.log"
refuses "P3: 10 s exactly is refused" "unittests src/lib.rs c 10.000" p3 "$tmp/wb.tsv"
p3log 110.500000 112.000000 >"$tmp/p3.log"
accepts "P3: a queued test is timed from its start, not its binary's (c: 9.5 s, not 10.5)" p3 "$tmp/wb.tsv"
refuses "P3: on three threads the same c started with its binary (10.5 s)" "unittests src/lib.rs c 10.500" env RUST_TEST_THREADS=3 "${py[@]}" result 0 "$tmp/p3.tsv" "$tmp/p3.log" "$tmp/none.log" "$RE" "$tmp/wb.tsv"
printf 'unittests src/lib.rs\t30\n' >"$tmp/co.tsv"
p3log 105.000000 112.000000 >"$tmp/p3.log"
accepts "P3: a co-located binary's test is weighed as its block (b, 12 s, no row)" p3 "$tmp/w0.tsv" "$tmp/co.tsv"
p3log 170.000000 171.000000 >"$tmp/p3.log"
refuses "P3: no upper bound (69 s with no libtest 60 s line)" "unittests src/lib.rs c 69.000" p3 "$tmp/wb.tsv"
p3log 105.000000 106.000000 "102.000000	tool output then the result: test c-is-not-a-name ... ok" >"$tmp/p3.log"
accepts "P3: a tool's line that looks like a result but names no Rust path is not timed" p3 "$tmp/wb.tsv"
p3log 105.000000 106.000000 | sed 's/\ttest c \.\.\. ok/\ta tool line with no newline, then test c ... ok/' >"$tmp/p3.log"
accepts "P3: a result after a tool's unfinished line is timed" p3 "$tmp/wb.tsv"
# Fail closed: a line P3 cannot time is refused, never skipped (adversarial, §5).
p3log 105.000000 106.000000 "a line with no stamp" >"$tmp/p3.log"
refuses "P3: a line with no time stamp" "p3.log:5: a line with no time stamp: a line with no stamp" p3 "$tmp/wb.tsv"
p3log 105.000000 106.000000 "90.000000	a stamp from before" >"$tmp/p3.log"
refuses "P3: a time stamp that goes back" "p3.log:5: the time stamp goes back" p3 "$tmp/wb.tsv"
p3log 105.000000 106.000000 | sed 's/test c \.\.\. ok/test c ...ok/' >"$tmp/p3.log"
refuses "P3: a result line P3 cannot read" "unittests src/lib.rs: timed 2 of its 3 tests" p3 "$tmp/wb.tsv"
p3log 105.000000 106.000000 | sed '$d' >"$tmp/p3.log"
refuses "P3: a log that ends inside a binary" "the log ends inside unittests src/lib.rs" p3 "$tmp/wb.tsv"
p3log 105.000000 106.000000 >"$tmp/p3.log"
refuses "P3: a thread count that is not one" "the test thread count 'x' is not a positive integer" env RUST_TEST_THREADS=x "${py[@]}" result 0 "$tmp/p3.tsv" "$tmp/p3.log" "$tmp/none.log" "$RE"
# P3 on CI's own log: shard 4 of main d09a5187 (run 45099268629; held.sh's stamps from GitHub's, cargo's and libtest's
# own lines kept), against that commit's weights and blocks, frozen beside it. It names the one test; with its row, none.
awk -F'\t' '{ print "ws\t" $2 "\t1" }' <("${py[@]}" times "$fx/p3-d09a5187-shard-4.log" 4) >"$tmp/p3ci.tsv"
refuses "P3: main d09a5187's shard 4 against its own weights" "the REFIT RULE): tests/pin_device_refusals.rs a_power_sense_input_is_not_generated 20.971;" "${py[@]}" result 4 "$tmp/p3ci.tsv" "$fx/p3-d09a5187-shard-4.log" "$tmp/none.log" "$RE" "$fx/p3-d09a5187-weights.tsv" "$fx/p3-d09a5187-colocate.tsv"
[ "$(grep -c 'a_power_sense_input_is_not_generated 20.971;$' "$tmp/out")" -eq 1 ] && ok "P3: it names that test alone" || bad "P3: d09a5187: $(tail -1 "$tmp/out")"
{ cat "$fx/p3-d09a5187-weights.tsv"; printf 'tests/pin_device_refusals.rs\ta_power_sense_input_is_not_generated\t20.971\n'; } >"$tmp/w-d09.tsv"
accepts "P3: the same shard with the row AET-WO-1914 adds" "${py[@]}" result 4 "$tmp/p3ci.tsv" "$fx/p3-d09a5187-shard-4.log" "$tmp/none.log" "$RE" "$tmp/w-d09.tsv" "$fx/p3-d09a5187-colocate.tsv"
# The gates split (ci/gates.sh): the complement of tools-only by name; refused by name otherwise.
gs=(bash "$here/gates.sh")
meta() { # extra-target-json
  printf '{"packages":[{"name":"a","targets":[{"kind":["lib"],"name":"a"},{"kind":["test"],"name":"t1"},{"kind":["test"],"name":"t2"}%s]},{"name":"b","targets":[{"kind":["lib"],"name":"b"},{"kind":["test"],"name":"t1"},{"kind":["test"],"name":"u"}]}]}\n' "$1" >"$tmp/meta.json"
}
meta ""
printf '# comment\na\tt2\n' >"$tmp/only.tsv"
accepts "gates: every other test target runs by name" "${py[@]/shard.sh/gates.sh}" targets "$tmp/meta.json" "$tmp/only.tsv"
[ "$("${gs[@]}" targets "$tmp/meta.json" "$tmp/only.tsv" | tr '\n' ' ')" = "t1 u " ] && ok "gates: t1 (two packages) and u run, t2 does not" || bad "gates: the complement"
printf 'a\tgone\n' >"$tmp/only.tsv"
refuses "gates: a tools-only row naming no test target" "names no test target: a gone" "${gs[@]}" targets "$tmp/meta.json" "$tmp/only.tsv"
printf 'a\tt1\n' >"$tmp/only.tsv"
refuses "gates: a tools-only name two packages share" "tools-only t1 is a test target of 2 packages" "${gs[@]}" targets "$tmp/meta.json" "$tmp/only.tsv"
meta ',{"kind":["bench"],"name":"speed"}'; printf 'a\tt2\n' >"$tmp/only.tsv"
refuses "gates: a target of a kind neither job runs by name" "does not run: a speed (bench)" "${gs[@]}" targets "$tmp/meta.json" "$tmp/only.tsv"
meta ',{"kind":["example"],"name":"demo","test":true}'
refuses "gates: an example with test = true" "does not run: a demo (example, test = true)" "${gs[@]}" targets "$tmp/meta.json" "$tmp/only.tsv"
meta ',{"kind":["example"],"name":"demo","test":false}'
accepts "gates: an example built, not tested" "${gs[@]}" targets "$tmp/meta.json" "$tmp/only.tsv"
# Every test of a lock-serialised binary carries a row.
refuses "plan: a lock-serialised binary's test with no row" "tests/board_tier2.rs v2 has no row" "${py[@]}" assign 4 "$tmp/weights.tsv" "$tmp/entries.tsv" "$tmp/lk" 0 /dev/null "tests/board_tier2.rs"
printf 'tests/board_tier2.rs\tv1\t100\ntests/board_tier2.rs\tv2\t1\ntests/board_tier2.rs\tshared2\t1\n' >"$tmp/wl.tsv"
accepts "plan: every lock-serialised test weighed" "${py[@]}" assign 4 "$tmp/wl.tsv" "$tmp/entries.tsv" "$tmp/lk2" 0 /dev/null "tests/board_tier2.rs"

echo "== the shared build (AET-WO-1911 §1.2)"
# compiled_in: cargo compiled before a test binary ran; a tool's later `Compiling` line is the test's own output.
printf '    Finished `test` profile\n     Running tests/a.rs (target/debug/deps/a-1)\n   Compiling inside-a-test v0\n' >"$tmp/fresh.log"
accepts "compiled_in: a fresh build, a test's own Compiling line after Running" "${py[@]}" compiled_in "$tmp/fresh.log"
printf '   Compiling aethelgard-cli v0.2.0 (/repo/crates/aethelgard-cli)\n    Finished `test` profile\n     Running tests/a.rs (x)\n' >"$tmp/stale.log"
refuses "compiled_in: a shard that compiles" "stale.log: this shard compiled" "${py[@]}" compiled_in "$tmp/stale.log"
printf '\033[1m\033[32m   Compiling\033[0m aethelgard-cli v0.2.0\n     Running tests/a.rs (x)\n' >"$tmp/color.log"
refuses "compiled_in: a coloured Compiling line" "color.log: this shard compiled" "${py[@]}" compiled_in "$tmp/color.log"
printf '   Compiling aethelgard-cli v0.2.0\n     Running tests/a.rs (x)\n' | st >"$tmp/stamped.log"
refuses "compiled_in: a Compiling line in run's stamped log" "stamped.log: this shard compiled" "${py[@]}" compiled_in "$tmp/stamped.log"
# pack and unpack, on a scratch repository with its own copy of ci/ and container/IMAGE.
g() { git -c user.name=t -c user.email=t@t -c init.defaultBranch=main -c commit.gpgsign=false "$@"; }
mkrepo() { rm -rf "$1"; mkdir -p "$1/ci" "$1/container" "$1/src"; cp "$here/shard.sh" "$here/SHARDS" "$1/ci/"; echo img@sha256:1 >"$1/container/IMAGE"; echo a >"$1/src/a.rs"; echo /target/ >"$1/.gitignore"
  (cd "$1" && g init -q . && g add -A && g commit -qm a); }
mkrepo "$tmp/b"
(cd "$tmp/b" && mkdir -p target/debug/deps target/debug/incremental && echo bin >target/debug/deps/t-1 && echo inc >target/debug/incremental/x && bash ci/shard.sh pack "$tmp/art" >/dev/null) \
  && ok "pack: target/ packed with its stamp" || bad "pack"
grep -q '^tree=' "$tmp/art/stamp" && grep -q '^image=img@sha256:1$' "$tmp/art/stamp" && grep -q '^sha256=' "$tmp/art/stamp" && ok "pack: the stamp names the tree, the image and the sha256" || bad "pack: stamp"
clone() { rm -rf "$tmp/s"; g clone -q "$tmp/b" "$tmp/s"; }
clone; accepts "unpack: the same tree and image" bash -c "cd '$tmp/s' && bash ci/shard.sh unpack '$tmp/art'"
[ -f "$tmp/s/target/debug/deps/t-1" ] && [ ! -e "$tmp/s/target/debug/incremental" ] && ok "unpack: the binaries, not incremental/" || bad "unpack: contents"
[ "$(stat -c %Y "$tmp/s/src/a.rs")" = 946684800 ] && [ "$(stat -c %Y "$tmp/s/src")" = 946684800 ] && ok "unpack: sources and directories dated the epoch" || bad "unpack: mtimes"
[ "$(stat -c %Y "$tmp/s/target/debug/deps/t-1")" != 946684800 ] && ok "unpack: the build keeps its own mtimes" || bad "unpack: build mtimes"
clone; (cd "$tmp/s" && echo b >src/a.rs && g commit -qam other)
refuses "unpack: an archive from another tree (a stale binary)" "built from another tree or image" bash -c "cd '$tmp/s' && bash ci/shard.sh unpack '$tmp/art'"
clone; (cd "$tmp/s" && echo img@sha256:2 >container/IMAGE && g commit -qam image)
refuses "unpack: an archive built in another image" "image=img@sha256:2" bash -c "cd '$tmp/s' && bash ci/shard.sh unpack '$tmp/art'"
clone; cp -r "$tmp/art" "$tmp/art2"; printf 'x' >>"$tmp/art2/target.tar.zst"
refuses "unpack: an archive that is not its stamp's" "is not its stamp's" bash -c "cd '$tmp/s' && bash ci/shard.sh unpack '$tmp/art2'"
(cd "$tmp/b" && echo dirty >src/a.rs && rm -rf "$tmp/art3")
refuses "pack: from a dirty tree" "the checkout is not clean:  M src/a.rs" bash -c "cd '$tmp/b' && bash ci/shard.sh pack '$tmp/art3'"
(cd "$tmp/b" && git checkout -q src/a.rs)
clone; echo edited >"$tmp/s/src/a.rs"
refuses "unpack: over an edited checkout" "the checkout is not clean:  M src/a.rs" bash -c "cd '$tmp/s' && bash ci/shard.sh unpack '$tmp/art'"
clone; mkdir -p "$tmp/s/target"
refuses "unpack: over an existing target/" "target/ exists before the archive is unpacked" bash -c "cd '$tmp/s' && bash ci/shard.sh unpack '$tmp/art'"
exit "$fail"
