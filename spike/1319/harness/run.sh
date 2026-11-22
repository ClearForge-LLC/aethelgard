#!/usr/bin/env bash
# AET-WO-1319: one tree's whole measurement. Builds the CLI, sweeps every
# committed specification (spike/1211d/measure/sweep.sh), and runs both
# harnesses (d4diff.rs: the 1211d reviewer's cases; d1319.rs: this work
# order's pin-device and kind-crossing cases). Two trees measured this way
# are judged by compare.sh.
#
# The tree is a copy (git archive of the base commit, or an rsync of the
# branch), never the worktree: the harnesses are dropped into its
# crates/aethelgard-cli/tests/, and Drawing gets a `dump` shim (every drawn
# part, `{:#?}`), applied the same way to both trees and never committed.
#
# Usage: run.sh <tree> <cargo target dir> <out dir> <repo with the specifications>
set -euo pipefail
tree=$1 target=$2 out=$3 repo=$4
here=$(cd "$(dirname "$0")" && pwd)
cp "$here/d4diff.rs" "$here/d1319.rs" "$tree/crates/aethelgard-cli/tests/"
python3 -I - "$tree/crates/aethelgard-eda/src/schematic.rs" <<'PY'
import sys
p = sys.argv[1]
s = open(p).read()
if "pub fn dump(&self)" not in s:
    anchor = "    /// Each anchored part's reference, with the IC it is anchored to\n"
    assert s.count(anchor) == 1, "the shim's anchor moved"
    shim = ("    /// AET-WO-1319 harness shim (never committed): every drawn part, in draw order.\n"
            "    pub fn dump(&self) -> String {\n        format!(\"{:#?}\", self.parts)\n    }\n\n")
    open(p, "w").write(s.replace(anchor, shim + anchor))
PY
cd "$tree"
cargo build --release -p aethelgard-cli --target-dir "$target" -j 6
cargo test --release -p aethelgard-cli --test d4diff --test d1319 --target-dir "$target" -j 6 --no-run
mkdir -p "$out"
bash "$repo/spike/1211d/measure/sweep.sh" "$target/release/aethelgard" "$repo" "$out/sweep"
for t in d4diff d1319; do
  bin=$(ls -t "$target"/release/deps/$t-* | grep -v '\.d$' | head -1)
  (cd "$tree/crates/aethelgard-cli" && D4OUT="$out/$t" "$bin" --test-threads 1)
done
