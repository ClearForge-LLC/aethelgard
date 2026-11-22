#!/usr/bin/env bash
# AET-WO-1219: build the serde dump in scratch, dump every committed
# specification, then run the reader. Scratch: ~/.cache/aet-1219.
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/../.." && pwd)
scratch=${AET_1219_SCRATCH:-$HOME/.cache/aet-1219}
export PATH=$HOME/.cargo/bin:$PATH
mkdir -p "$scratch/dump/src"
cp "$here/dump.rs" "$scratch/dump/src/main.rs"
cat > "$scratch/dump/Cargo.toml" <<TOML
[package]
name = "aet-1219-dump"
version = "0.0.0"
edition = "2024"
publish = false

[workspace]

[dependencies]
aethelgard-ir = { path = "$repo/crates/aethelgard-ir" }
serde_json = "1"
TOML
cp "$repo/Cargo.lock" "$scratch/dump/Cargo.lock"
nice -n 10 cargo build -q --release --offline --manifest-path "$scratch/dump/Cargo.toml" \
  --target-dir "$scratch/target"
# Every JSON file in the three directories versions.rs reads (`current_specifications`).
files=()
for d in examples crates/aethelgard-cli/tests/fixtures crates/aethelgard-ir/tests/fixtures; do
  for f in "$repo/$d"/*.json; do files+=("$f"); done
done
"$scratch/target/release/aet-1219-dump" "${files[@]}" > "$scratch/dump.jsonl"
python3 -I "$here/reader.py" --repo "$repo" --dump "$scratch/dump.jsonl" --out "$here/out" "$@"
