#!/usr/bin/env python3
"""AET-WO-1918 (3) THROWAWAY (reverted before READY): add the opt-level=1 measurement to ci.yml, ruled (iii).

Copies three jobs with `-e CARGO_PROFILE_DEV_OPT_LEVEL=1` on every `$RUN` (the dev and test profiles at
opt-level 1; debug assertions and overflow checks stay on, the dev profile's defaults, and the three
`[profile.dev.package.*]` opt-level 2 stanzas stay as they are):
  - tools-build-o1:  the shared build, its archive `tools-build-o1`;
  - tools-shard-o1:  the same plan and shards on that build, the normal pass only (no fmt, clippy or upload);
  - bench-package-o1: Sentinel-01's verify --against-fab its committed package: passing is the package's
    bytes unmoved, the verdict line included.
None is in `tools`' needs, so none gates anything. Stdlib only.

usage: throwaway_o1.py .github/workflows/ci.yml
"""
import re
import sys

p = sys.argv[1]
s = open(p).read()


def job(name):
    m = re.search(rf"^  {re.escape(name)}:\n(?:(?:    .*|)\n)*", s, re.M)
    assert m, name
    return m.group(0)


O1 = "$RUN -e CARGO_PROFILE_DEV_OPT_LEVEL=1 "


def o1(text):
    return text.replace("$RUN ", O1)


build = o1(job("tools-build")).replace("  tools-build:\n", "  tools-build-o1:\n", 1)
build = build.replace("name: tools-build\n", "name: tools-build-o1\n")
build = build.replace("$RUNNER_TEMP/tools-build", "$RUNNER_TEMP/tools-build-o1")
build = build.replace("runner.temp }}/tools-build/", "runner.temp }}/tools-build-o1/")

shard = o1(job("tools-shard"))
shard = shard.replace("  tools-shard:\n    name: tools-shard (${{ matrix.shard }})", "  tools-shard-o1:\n    name: tools-shard-o1 (${{ matrix.shard }})", 1)
shard = shard.replace("needs: [route, tools-build]", "needs: [route, tools-build-o1]")
shard = shard.replace("name: tools-build\n", "name: tools-build-o1\n")
shard = shard.replace("runner.temp }}/tools-build\n", "runner.temp }}/tools-build-o1\n")
shard = shard.replace('"$RUNNER_TEMP/tools-build"', '"$RUNNER_TEMP/tools-build-o1"')
# Drop shard 0's fmt and clippy, and the upload (its artifact names are the real shards').
shard = re.sub(r"      - name: cargo fmt inside the image\n(?:        .*\n)*\n", "", shard)
shard = re.sub(r"      # Clippy's check build.*\n      - name: cargo clippy inside the image.*\n(?:        .*\n)*\n", "", shard)
shard = re.sub(r"      - name: keep what the aggregate checks\n(?:        .*\n)*", "", shard)

bench = o1(job("bench-package")).replace("  bench-package:\n", "  bench-package-o1:\n", 1)
# Only the comparison: the write step and its upload are a dispatch's, and `--rebind` is pinned to one job.
bench = re.sub(r"      - name: verify --fab-out, the package written for committing\n(?:        .*\n)*\n", "", bench)
bench = re.sub(r"      - name: keep the written package\n(?:        .*\n)*", "", bench)
assert "--rebind" not in bench and "--fab-out target" not in bench

for j, t in (("tools-build-o1", build), ("tools-shard-o1", shard), ("bench-package-o1", bench)):
    assert O1 in t, j
    assert "upload-artifact" not in t or j == "tools-build-o1", j
s = s.rstrip("\n") + "\n\n  # AET-WO-1918 (3) THROWAWAY (reverted before READY): opt-level=1, measured not adopted (ruled (iii)).\n" + build + "\n" + shard + "\n" + bench
open(p, "w").write(s)
print("added tools-build-o1, tools-shard-o1, bench-package-o1")
