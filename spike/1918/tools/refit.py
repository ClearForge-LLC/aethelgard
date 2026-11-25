#!/usr/bin/env python3
"""AET-WO-1918 (3), ruled (i) at 06:37Z: set every weight to its worst sample, from CI's own runs.

  refit.py ENTRIES BLOCKS ALONE RUN... --post RUN... > decisions.tsv
    ENTRIES  ci/shard.sh plan's entries.tsv for the head being weighed (suite, binary, test): rows are kept
             only for tests it lists (P1).
    BLOCKS   spike/1916/measure.py blocks over every run (run, shard, binary, n, wall, planned, delta).
    ALONE    spike/1912/alone_times.py over the throwaway run (shard, binary, name, seconds, cpu).
    RUN...   run directories stamped by spike/1916/stamp.sh (times.tsv: shard, binary, test, seconds), the
             in-shard samples of every test (P3, an upper bound: REFIT RULE 5).
    --post   the runs on or after this WO's changes: the only samples for a test or block it changed.

Rules:
  - a test of a lock-serialised binary (board_tier2, twin_tier2): its alone time (REFIT RULE 1, 2), never a
    shared-process sample;
  - a colocated block (ci/colocate.tsv): its worst `finished in` (REFIT RULE 3), rounded up;
  - any other test: its worst in-shard sample (REFIT RULE 5), 3 decimals, if 0.5 s or more or it had a row;
  - a test or block this WO changed: its post runs only.
Rewrites ci/test-weights.tsv's and ci/colocate.tsv's values in place (comments kept; a new row appended under
one comment), and prints each decision. Stdlib only.
"""
import math
import re
import sys

args = sys.argv[1:]
post_at = args.index("--post")
entries_f, blocks_f, alone_f, runs, post = args[0], args[1], args[2], args[3:post_at], args[post_at + 1 :]
CHANGED_TESTS = {
    ("tests/verify_2k2_tier2.rs", "verify_binds_the_reference_and_two_runs_print_the_same_bytes"),
    ("tests/verify_2k2_tier2.rs", "verify_fab_out_writes_the_package_its_verdict_names_at_any_path"),
    ("tests/boot_tier1.rs", "a_boot_that_never_returns_is_refused_at_the_hang_guard"),
    ("tests/boot_tier1.rs", "the_test_s_hang_guard_is_on_the_verdict_and_only_simulate_takes_it"),
    ("tests/campaign_tier1.rs", "a_renode_that_never_returns_is_refused_and_archives_nothing"),
}
CHANGED_BLOCKS = {"tests/against_fab_tier2.rs"}
LOCKED = set(re.search(r"^LOCKED='([^']*)'", open("ci/shard.sh").read(), re.M).group(1).split())


def table(path, cols):
    out = {}
    for line in open(path):
        if line.startswith("#") or not line.strip():
            continue
        p = line.rstrip("\n").split("\t")
        out[tuple(p[:cols])] = p[cols]
    return out


weights = table("ci/test-weights.tsv", 2)
colocate = table("ci/colocate.tsv", 1)
blocks = {k[0] for k in colocate}
listed = set()
for line in open(entries_f):
    s, b, t = line.rstrip("\n").split("\t")
    listed.add((b, t))

# In-shard samples.
samples = {}
for run in runs:
    for line in open(f"{run}/times.tsv"):
        _, b, t, sec = line.rstrip("\n").split("\t")
        if b in LOCKED or b in blocks:
            continue
        key = (b, t)
        if key in CHANGED_TESTS and run not in post:
            continue
        samples.setdefault(key, []).append(float(sec))
# Alone times.
alone = {}
for line in open(alone_f):
    if line.startswith("#") or not line.strip():
        continue
    _, b, t, sec, _cpu = line.rstrip("\n").split("\t")
    alone.setdefault((b, t), []).append(float(sec))
# Block walls.
bwall = {}
for line in open(blocks_f):
    p = line.rstrip("\n").split("\t")
    if p[0] == "run" or len(p) < 5:
        continue
    run, b, wall = p[0], p[2], float(p[4])
    if b not in blocks or (b in CHANGED_BLOCKS and not any(run == r.rstrip("/").rsplit("/", 1)[-1] for r in post)):
        continue
    bwall.setdefault(b, []).append(wall)

decisions = []
new_w = dict(weights)
for key in sorted(listed):
    b, t = key
    old = weights.get(key)
    if b in blocks:
        continue
    if b in LOCKED:
        if key in alone:
            v = max(alone[key])
            new_w[key] = f"{v:.3f}"
            decisions.append(("alone", b, t, old, new_w[key], len(alone[key])))
        else:
            decisions.append(("NO-ALONE-SAMPLE", b, t, old, old, 0))
        continue
    if key in samples:
        v = max(samples[key])
        if v >= 0.5 or old is not None:
            new_w[key] = f"{v:.3f}"
            decisions.append(("worst", b, t, old, new_w[key], len(samples[key])))
for key in list(new_w):
    if key not in listed:
        decisions.append(("DROPPED-UNLISTED", key[0], key[1], new_w[key], None, 0))
        del new_w[key]
new_c = dict(colocate)
for (b,) in colocate:
    if b in bwall:
        v = math.ceil(max(bwall[b]))
        new_c[(b,)] = str(v)
        decisions.append(("block", b, "", colocate[(b,)], str(v), len(bwall[b])))
    else:
        decisions.append(("NO-BLOCK-SAMPLE", b, "", colocate[(b,)], colocate[(b,)], 0))


def rewrite(path, cols, new, header):
    out, seen = [], set()
    for line in open(path):
        if line.startswith("#") or not line.strip():
            out.append(line)
            continue
        p = line.rstrip("\n").split("\t")
        key = tuple(p[:cols])
        if key not in new:
            continue
        seen.add(key)
        out.append("\t".join(list(key) + [new[key]]) + "\n")
    added = [k for k in sorted(new) if k not in seen]
    if added:
        out.append(header)
        out.extend("\t".join(list(k) + [new[k]]) + "\n" for k in added)
    open(path, "w").write("".join(out))


rewrite("ci/test-weights.tsv", 2, new_w, "# AET-WO-1918 (3) refit: rows new at the refit (each its worst sample; see the decisions above it in spike/1918/out/refit.tsv).\n")
rewrite("ci/colocate.tsv", 1, new_c, "# AET-WO-1918 (3) refit: blocks new at the refit.\n")
for d in decisions:
    print("\t".join("" if x is None else str(x) for x in d))
