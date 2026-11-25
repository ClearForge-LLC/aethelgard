#!/usr/bin/env python3
"""AET-WO-1918 (3) THROWAWAY (reverted before READY): the alone pass, ruled (i), as AET-WO-1916's 9c7c6cb8.

After a shard's own tests, each name spike/1918/alone.tsv gives that shard runs alone, one invocation and one
thread each; its libtest `finished in` is its alone time (REFIT RULE 1; the locked binaries board_tier2 and
twin_tier2 are weighed from it, never from a shared-process run). Skipped in the opt-level=1 shards, which
measure the normal pass only. Stdlib only.

usage: throwaway_alone.py ci/shard.sh
"""
import sys

p = sys.argv[1]
s = open(p).read()
anchor = '''  [ "$status" -eq 0 ] || die "shard $k: a test run failed"
  result "$k" "$shard" "$out/test-ws.log"'''
assert s.count(anchor) == 1
patch = '''  # AET-WO-1918 THROWAWAY (reverted before READY; REFIT RULE 1, ruled (i)): after the shard's own tests, each name
  # spike/1918/alone.tsv gives this shard runs alone, one invocation and one thread each; its libtest `finished in` is
  # its alone time. Not in the opt-level=1 shards (ruled (iii)), which time the normal pass only.
  if [ -z "${CARGO_PROFILE_DEV_OPT_LEVEL:-}" ]; then
    grep -m1 'model name' /proc/cpuinfo || true
    local name alone_names
    mapfile -t alone_names < <(awk -F'\\t' -v k="$k" '!/^#/ && $1 == k { print $2 }' "$here/../spike/1918/alone.tsv")
    for name in "${alone_names[@]}"; do
      echo "ALONE $name"
      set +e
      cargo test --workspace "${FEATURES[@]}" -- --exact "$name" --test-threads=1 </dev/null 2>&1 | tee -a "$out/alone.log"
      [ "${PIPESTATUS[0]}" -eq 0 ] || status=1
      set -e
    done
  fi
'''
s = s.replace(anchor, patch + anchor)
open(p, "w").write(s)
print("alone pass added")
