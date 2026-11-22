#!/bin/bash
# AET-WO-2308: each new tier-1 test alone, in the image (ci-img.sh, CI's layout), three times; prints
# the worst wall time in seconds, for ci/test-weights.tsv (rows are tests of 0.5 s or more).
set -u
cd /repo
cargo test --offline -p aethelgard-cli --no-run > /tmp/weigh-build.log 2>&1 || { tail -5 /tmp/weigh-build.log; exit 1; }
cargo test --offline -p aethelgard-eda --lib --no-run >> /tmp/weigh-build.log 2>&1 || exit 1
bin_of() { ls -t target/img/debug/deps/$1-* | grep -v '\.d$' | head -1; }
weigh() {
  local bin=/repo/$1 test=$2 dir=$3 worst=0 s e ms
  for _ in 1 2 3; do
    s=$(date +%s%N)
    (cd "$dir" && "$bin" --exact "$test" --test-threads 1 2>&1 | grep -q "1 passed") || echo "FAILED $test" >&2
    e=$(date +%s%N); ms=$(( (e-s)/1000000 ))
    [ "$ms" -gt "$worst" ] && worst=$ms
  done
  printf '%s\t%s\t%d.%03d\n' "$4" "$test" $((worst/1000)) $((worst%1000))
}
cli=crates/aethelgard-cli
weigh "$(bin_of fab_tier1)" a_drawn_twin_stand_in_is_refused_by_each_fab_flag_by_name "$cli" tests/fab_tier1.rs
weigh "$(bin_of fab_tier1)" a_tree_that_differs_writes_no_package_and_says_so "$cli" tests/fab_tier1.rs
main=$(for f in $(ls target/img/debug/deps/aethelgard-* | grep -v '\.d$'); do "$f" --list 2>/dev/null | grep -q "verify::tests" && echo "$f"; done | head -1)
for t in verify::tests::the_exit_code_is_the_verdicts_whatever_the_fab_mode \
         verify::tests::the_campaign_names_the_renode_build_every_run_archive_names \
         fab::tests::a_swapped_elf_or_board_is_refused_by_its_own_binding \
         fab::tests::the_notes_list_every_part_the_bom_notes_and_only_those \
         tree::tests::against_reports_an_empty_directory_and_diff_does_not; do
  weigh "$main" "$t" "$cli" "unittests src/main.rs"
done
