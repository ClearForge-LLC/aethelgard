#!/bin/bash
# AET-WO-1918 P4, inside the image at /repo: what CBMC hands kani-driver for one harness.
# 1. `cargo kani --keep-temps --verbose` on a scratch copy of the generated crate prints
#    the CBMC command Kani runs and keeps its goto binary.
# 2. That command is run again with its output to a file, and the file is measured
#    (spike/1918/tools/cbmc_json.py): its size, how many results and traces it holds, and
#    how much of it is traces.
# usage: bash cbmc-out.sh <crate dir> <harness name> <out dir>
set -u
crate=$1 h=$2 out=$3
rm -rf "$out" && mkdir -p "$out"
cp -a "$crate" "$out/crate"
cd "$out/crate" || exit 2
# The copy sits under /repo, so it gets its own empty workspace (the CLI copies to scratch instead).
grep -q '^\[workspace\]' Cargo.toml || printf '\n[workspace]\n' >> Cargo.toml
cargo kani --harness "proofs::$h" --exact --keep-temps --verbose > "$out/kani.log" 2>&1
echo "kani rc=$?" >&2
# Kani prints the CBMC command line it runs (`[Kani] Running: \`cbmc ...\``, or a bare `cbmc ...` line).
cmd=$(grep -oE '(^|`)cbmc --[^`]*' "$out/kani.log" | head -1 | sed 's/^`//')
[ -n "$cmd" ] || { echo "no cbmc command in kani.log" >&2; exit 3; }
echo "$cmd" > "$out/cbmc.cmd"
t0=$(date +%s)
# shellcheck disable=SC2086
$cmd > "$out/cbmc.out" 2> "$out/cbmc.err"
echo "cbmc rc=$? seconds=$(( $(date +%s) - t0 )) bytes=$(stat -c %s "$out/cbmc.out")" >&2
python3 -I /repo/spike/1918/tools/cbmc_json.py "$out/cbmc.out" >&2
echo "CTL1918 memory.peak=$(cat /sys/fs/cgroup/memory.peak)" >&2
