#!/bin/bash
# AET-WO-4400j: each new or changed test alone in the tools image (inside, at /repo), libtest's `finished in`; the
# weights rule (AET-WO-1310): Furnace time times 2.8, rows only for 0.5 s or more.
set -u
t() { local bin=$1 name=$2; local s; s=$(cargo test --offline --locked -p aethelgard-cli $bin -- --exact "$name" 2>&1 | grep -o 'finished in [0-9.]*s' | tail -1); echo "$bin	$name	$s"; }
t --bin=aethelgard renode::tests::every_run_for_runs_exactly_its_nanoseconds_in_renode
t --bin=aethelgard renode::foreign::tests::the_summarised_log_reads_as_the_whole_one
t --bin=aethelgard renode::foreign::tests::a_planted_difference_in_any_source_reads_as_in_the_whole_log
t --bin=aethelgard renode::foreign::tests::a_devices_warnings_are_never_summarised
t --bin=aethelgard renode::foreign::tests::the_end_is_read_four_times_ending_at_the_window_and_never_past_it
t --bin=aethelgard renode::foreign::tests::only_an_rc_ports_search_fields_are_left_unjudged_at_the_end
for n in renodes_stdin_is_held_open_and_never_ends another_tools_stdin_is_dev_null_and_ends; do t --test=tools_tier1 $n; done
for n in the_bridges_stdin_is_held_open_as_renodes_is a_foreign_end_read_fails_only_if_it_fails_at_all_four a_foreign_run_without_exactly_four_end_reads_is_untrusted an_input_run_may_run_twelve_seconds_and_a_plain_one_five repeated_exempt_cycles_are_summarised_and_counted_and_a_bad_one_is_not; do t --test=boot_tier1 $n; done
