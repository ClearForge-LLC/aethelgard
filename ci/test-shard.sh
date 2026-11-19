#!/usr/bin/env bash
# AET-WO-1907 §3: the guard fails each run it exists to fail. Fixtures only; no cargo, no image.
set -uo pipefail
export LC_ALL=C
here=$(cd "$(dirname "$0")" && pwd)
py=(bash "$here/shard.sh")
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
fail=0

ok()   { echo "PASS $1"; }
bad()  { echo "FAIL $1"; fail=1; }
# expect NAME CMD...: the command must exit nonzero
refuses() { local name=$1; shift; if "$@" >"$tmp/out" 2>&1; then bad "$name: accepted"; else ok "$name: refused ($(head -1 "$tmp/out" | cut -c1-90))"; fi; }
accepts() { local name=$1; shift; if "$@" >"$tmp/out" 2>&1; then ok "$name"; else bad "$name: $(head -1 "$tmp/out")"; fi; }

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
refuses "guard: a test assigned to no shard" "${py[@]}" guard $N "$tmp/entries.tsv" "$tmp/p"

break_plan; head -1 "$tmp/p/shard-0.tsv" >>"$tmp/p/shard-2.tsv"
refuses "guard: a test in two shards" "${py[@]}" guard $N "$tmp/entries.tsv" "$tmp/p"

break_plan; : >"$tmp/p/shard-3.tsv"
refuses "guard: a shard that lists nothing" "${py[@]}" guard $N "$tmp/entries.tsv" "$tmp/p"

break_plan; sed -i 's/^\(ws\tshared\t\)2$/\11/' "$tmp/p"/shard-*.tsv
refuses "guard: a name run once though listed in two binaries" "${py[@]}" guard $N "$tmp/entries.tsv" "$tmp/p"

break_plan; printf 'ws\tnot_in_the_list\t1\n' >>"$tmp/p/shard-0.tsv"
refuses "guard: a shard test the list does not have" "${py[@]}" guard $N "$tmp/entries.tsv" "$tmp/p"

: >"$tmp/empty.log"
refuses "parse: a workspace that lists no tests" "${py[@]}" parse ws "$tmp/empty.log"
printf '     Running unittests src/lib.rs (x)\nt: test\nmystery output\n' >"$tmp/odd.log"
refuses "parse: an unrecognised line" "${py[@]}" parse ws "$tmp/odd.log"
printf '     Running unittests src/lib.rs (x)\nt: benchmark\n' >"$tmp/bench.log"
refuses "parse: a benchmark" "${py[@]}" parse ws "$tmp/bench.log"
refuses "names: no shard file" "${py[@]}" names "$tmp/missing.tsv" ws

# A shard's run log against its assignment.
shard0="$tmp/plan/shard-0.tsv"
want=$(awk -F'\t' '$1=="ws" {n += $3} END {print n + 0}' "$shard0")
mk_log() { # passed tier2-passed
  printf '     Running unittests src/lib.rs (target/debug/deps/a-1)\ntest result: ok. %d passed; 0 failed; 0 ignored; 0 measured; 0 filtered out\n     Running tests/board_tier2.rs (target/debug/deps/b-2)\ntest result: ok. %d passed; 0 failed; 0 ignored; 0 measured; 0 filtered out\n' "$1" "$2"
}
RE='^tests/(board_tier2)\.rs$'
mk_log "$((want - 0))" 0 >"$tmp/run.log"
accepts "result: passes equal the assignment" "${py[@]}" result 0 "$shard0" "$tmp/run.log" "$tmp/none.log" "$RE"
mk_log "$((want - 1))" 0 >"$tmp/run.log"
refuses "result: one test fewer ran than assigned" "${py[@]}" result 0 "$shard0" "$tmp/run.log" "$tmp/none.log" "$RE"
mk_log 0 0 >"$tmp/run.log"
refuses "result: a shard that passes while running nothing" "${py[@]}" result 0 "$shard0" "$tmp/run.log" "$tmp/none.log" "$RE"
printf 'test result: FAILED. 1 passed; 1 failed; 0 ignored\n' >"$tmp/run.log"
refuses "result: a failed test" "${py[@]}" result 0 "$shard0" "$tmp/run.log" "$tmp/none.log" "$RE"
: >"$tmp/emptyshard.tsv"
refuses "result: a shard file that lists nothing" "${py[@]}" result 0 "$tmp/emptyshard.tsv" "$tmp/run.log" "$tmp/none.log" "$RE"

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
refuses "aggregate: tier-2 tests silently stop running (one under the floor)" "${py[@]}" aggregate "$tmp/art" $N $((2 * N + 1))
mk_art "$tmp/art" 0
refuses "aggregate: a tier-2 file runs nothing (0 < 1)" "${py[@]}" aggregate "$tmp/art" $N 1
mk_art "$tmp/art" 2; rm -rf "$tmp/art/shard-2"
refuses "aggregate: a shard left no result" "${py[@]}" aggregate "$tmp/art" $N $((2 * N))
mk_art "$tmp/art" 2; sed -i 's/passed=[0-9]*/passed=0/' "$tmp/art/shard-1/result.txt"
refuses "aggregate: a shard passed nothing" "${py[@]}" aggregate "$tmp/art" $N $((2 * N))
mk_art "$tmp/art" 2; echo 'ws	extra	x	1' >>"$tmp/art/shard-3/entries.tsv"
refuses "aggregate: a shard listed different tests" "${py[@]}" aggregate "$tmp/art" $N $((2 * N))
mk_art "$tmp/art" 2; head -1 "$tmp/art/shard-0/shard.tsv" >>"$tmp/art/shard-1/shard.tsv"
refuses "aggregate: a test in two shards" "${py[@]}" aggregate "$tmp/art" $N $((2 * N))

}
for n in 4 5 7; do suite $n; done

# AET-WO-1910 (ruled on PR #233, comment 6999515740): the planner's two refusals (D6) and the gates split (D5).
echo "== the planner's refusals and the gates split"
fx="$here/fixtures"
# P1: a row that names no listed test, or a block that names no listed binary, is refused by name.
printf 'tests/board_tier2.rs\tv1\t100\ntests/board_tier2.rs\trenamed_away\t12\n' >"$tmp/stale.tsv"
refuses "P1: a weight row naming no listed test" "${py[@]}" assign 4 "$tmp/stale.tsv" "$tmp/entries.tsv" "$tmp/p1"
grep -q 'renamed_away names no listed test' "$tmp/out" && ok "P1: the row is named" || bad "P1: the row is not named"
printf 'tests/gone.rs\t30\n' >"$tmp/gone.tsv"
refuses "P1: a block naming no listed binary" "${py[@]}" assign 4 "$tmp/weights.tsv" "$tmp/entries.tsv" "$tmp/p1b" 0 "$tmp/gone.tsv"
grep -q 'tests/gone.rs names no listed binary' "$tmp/out" && ok "P1: the block is named" || bad "P1: the block is not named"
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
  printf '     Running unittests src/lib.rs (target/debug/deps/a-1)\ntest u1 has been running for over 60 seconds\ntest result: ok. %d passed; 0 failed; 0 ignored; 0 measured; 0 filtered out\n' "$1"
}
shard=$(ls "$tmp"/plan/shard-*.tsv | head -1); k=${shard##*-}; k=${k%.tsv}
want=$(awk -F'\t' '$1=="ws" {n += $3} END {print n + 0}' "$shard")
mk_heavy "$want" >"$tmp/heavy.log"
: >"$tmp/w0.tsv"
refuses "P2: result refuses an unweighted test past 60 s" "${py[@]}" result "$k" "$shard" "$tmp/heavy.log" "$tmp/none.log" "$RE" "$tmp/w0.tsv"
grep -q 'unittests src/lib.rs u1' "$tmp/out" && ok "P2: result names it" || bad "P2: result does not name it: $(head -1 "$tmp/out")"
printf 'unittests src/lib.rs\tu1\t70\n' >"$tmp/w1.tsv"
accepts "P2: the same test, weighed, passes" "${py[@]}" result "$k" "$shard" "$tmp/heavy.log" "$tmp/none.log" "$RE" "$tmp/w1.tsv"
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
refuses "gates: a tools-only row naming no test target" "${gs[@]}" targets "$tmp/meta.json" "$tmp/only.tsv"
grep -q 'names no test target: a gone' "$tmp/out" && ok "gates: the row is named" || bad "gates: the row is not named"
printf 'a\tt1\n' >"$tmp/only.tsv"
refuses "gates: a tools-only name two packages share" "${gs[@]}" targets "$tmp/meta.json" "$tmp/only.tsv"
meta ',{"kind":["bench"],"name":"speed"}'; printf 'a\tt2\n' >"$tmp/only.tsv"
refuses "gates: a target of a kind neither job runs by name" "${gs[@]}" targets "$tmp/meta.json" "$tmp/only.tsv"
grep -q 'a speed (bench)' "$tmp/out" && ok "gates: the bench is named" || bad "gates: the bench is not named"
meta ',{"kind":["example"],"name":"demo","test":true}'
refuses "gates: an example with test = true" "${gs[@]}" targets "$tmp/meta.json" "$tmp/only.tsv"
meta ',{"kind":["example"],"name":"demo","test":false}'
accepts "gates: an example built, not tested" "${gs[@]}" targets "$tmp/meta.json" "$tmp/only.tsv"
# Every test of a lock-serialised binary carries a row.
refuses "plan: a lock-serialised binary's test with no row" "${py[@]}" assign 4 "$tmp/weights.tsv" "$tmp/entries.tsv" "$tmp/lk" 0 /dev/null "tests/board_tier2.rs"
grep -q 'tests/board_tier2.rs v2 has no row' "$tmp/out" && ok "plan: the test is named" || bad "plan: the test is not named"
printf 'tests/board_tier2.rs\tv1\t100\ntests/board_tier2.rs\tv2\t1\ntests/board_tier2.rs\tshared2\t1\n' >"$tmp/wl.tsv"
accepts "plan: every lock-serialised test weighed" "${py[@]}" assign 4 "$tmp/wl.tsv" "$tmp/entries.tsv" "$tmp/lk2" 0 /dev/null "tests/board_tier2.rs"

echo "== the shared build (AET-WO-1911 §1.2)"
# compiled_in: cargo compiled before a test binary ran; a tool's later `Compiling` line is the test's own output.
printf '    Finished `test` profile\n     Running tests/a.rs (target/debug/deps/a-1)\n   Compiling inside-a-test v0\n' >"$tmp/fresh.log"
accepts "compiled_in: a fresh build, a test's own Compiling line after Running" "${py[@]}" compiled_in "$tmp/fresh.log"
printf '   Compiling aethelgard-cli v0.2.0 (/repo/crates/aethelgard-cli)\n    Finished `test` profile\n     Running tests/a.rs (x)\n' >"$tmp/stale.log"
refuses "compiled_in: a shard that compiles" "${py[@]}" compiled_in "$tmp/stale.log"
printf '\033[1m\033[32m   Compiling\033[0m aethelgard-cli v0.2.0\n     Running tests/a.rs (x)\n' >"$tmp/color.log"
refuses "compiled_in: a coloured Compiling line" "${py[@]}" compiled_in "$tmp/color.log"
grep -q 'this shard compiled' "$tmp/out" && ok "compiled_in: says so" || bad "compiled_in: the reason"
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
refuses "unpack: an archive from another tree (a stale binary)" bash -c "cd '$tmp/s' && bash ci/shard.sh unpack '$tmp/art'"
grep -q 'built from another tree or image' "$tmp/out" && ok "unpack: says another tree" || bad "unpack: tree reason"
clone; (cd "$tmp/s" && echo img@sha256:2 >container/IMAGE && g commit -qam image)
refuses "unpack: an archive built in another image" bash -c "cd '$tmp/s' && bash ci/shard.sh unpack '$tmp/art'"
clone; cp -r "$tmp/art" "$tmp/art2"; printf 'x' >>"$tmp/art2/target.tar.zst"
refuses "unpack: an archive that is not its stamp's" bash -c "cd '$tmp/s' && bash ci/shard.sh unpack '$tmp/art2'"
grep -q "is not its stamp's" "$tmp/out" && ok "unpack: says the sha256" || bad "unpack: sha reason"
(cd "$tmp/b" && echo dirty >src/a.rs && rm -rf "$tmp/art3")
refuses "pack: from a dirty tree" bash -c "cd '$tmp/b' && bash ci/shard.sh pack '$tmp/art3'"
(cd "$tmp/b" && git checkout -q src/a.rs)
clone; echo edited >"$tmp/s/src/a.rs"
refuses "unpack: over an edited checkout" bash -c "cd '$tmp/s' && bash ci/shard.sh unpack '$tmp/art'"
clone; mkdir -p "$tmp/s/target"
refuses "unpack: over an existing target/" bash -c "cd '$tmp/s' && bash ci/shard.sh unpack '$tmp/art'"
exit "$fail"
