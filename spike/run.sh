#!/usr/bin/env bash
# AET-WO-1200 spike: every measurement FEEDBACK cites, from the repo root.
#   spike/run.sh gates     fmt, clippy -D warnings, and the debug test
#   spike/run.sh measure   release timings and digests -> stdout
#   spike/run.sh fuzz SEED COUNT
set -euo pipefail
cd "$(dirname "$0")/.."
cargo_() { local sub=$1; shift; cargo "$sub" --manifest-path spike/solver-probe/Cargo.toml --target-dir target/spike-solver "$@"; }
case "${1:-}" in
  gates)
    cargo fmt --manifest-path spike/solver-probe/Cargo.toml -- --check
    cargo_ clippy --all-targets --locked -- -D warnings
    cargo_ test --locked
    ;;
  measure)
    cargo_ build --release --locked -q
    echo "# measure, release, $(hostname), HEAD $(git rev-parse --short HEAD) (plus any uncommitted change), $(rustc --version), NODE_LIMIT=10000000, runs=21"
    for f in examples/sentinel-01-minimal.json spike/specs/*.json; do
      NODE_LIMIT=10000000 PRUNING=fc,im,pm,memo,im+pm,im+pm+memo target/spike-solver/release/measure 21 "$f"
    done
    ;;
  fuzz)
    cargo_ build --release --locked -q
    target/spike-solver/release/fuzz "$2" "$3"
    ;;
  *) echo "usage: spike/run.sh gates|measure|fuzz SEED COUNT" >&2; exit 64 ;;
esac
