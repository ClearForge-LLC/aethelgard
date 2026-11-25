#!/bin/bash
# AET-WO-1918 candidate, inside the image at /repo: `cargo kani` on a scratch copy of a
# generated crate, one harness, with extra Kani flags. Run under tools/ctl.sh's `cmd` mode, so
# the scope's memory.peak is read as the last step. Prints Kani's verdict lines (the ones
# `aethelgard_cli::kani::judge` reads) so the candidate's result can be compared with `prove`'s.
# usage: bash cand.sh <crate dir> <harness name> <label> [kani flag...]
set -u
crate=$1 h=$2 label=$3; shift 3
out=/repo/target/cand/$label
rm -rf "$out" && mkdir -p "$out"
cp -a "$crate" "$out/crate"
cd "$out/crate" || exit 2
grep -q '^\[workspace\]' Cargo.toml || printf '\n[workspace]\n' >> Cargo.toml
cargo kani --harness "proofs::$h" --exact "$@" > "$out/kani.log" 2>&1
rc=$?
grep -E '^ *(VERIFICATION:-|Complete - |\*\* [0-9]+ of [0-9]+ cover|Checking harness |\*\* [0-9]+ of [0-9]+ failed)' "$out/kani.log"
grep -c 'Status: UNREACHABLE' "$out/kani.log" | sed 's/^/unreachable_checks=/'
echo "cand rc=$rc"
exit $rc
