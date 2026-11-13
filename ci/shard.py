#!/usr/bin/env python3
"""Shard the `tools` job's tests (AET-WO-1907). Pure functions over files, stdlib only.

Entry: one line per listed test occurrence, `suite<TAB>binary<TAB>test`.
  suite   `ws` (the workspace) or `bridge` (the analog bridge, outside it).
  binary  what cargo printed after `Running`, e.g. `tests/board_tier2.rs`.
A test NAME is the unit of assignment: `--exact` selects by name across every
binary of a suite, so two binaries holding one name run it in the same shard,
and the shard must see as many passes as there are occurrences.

Subcommands (each exits nonzero, printing why, on any refusal; INV-06):
  parse SUITE LOG            a `cargo test -- --list` log -> entries on stdout
  assign N WEIGHTS ENTRIES OUTDIR [BASE0]
                             LPT assignment -> OUTDIR/shard-K.tsv
  guard N ENTRIES OUTDIR     the four lists are disjoint and their union is the list
  names SHARDFILE SUITE      the names a shard runs in a suite, one per line
  result K SHARDFILE WSLOG BRIDGELOG TIER2_RE
                             passes in the logs equal the assignment; prints the shard's result line
  aggregate DIR N TIER2_MIN  across the shards' artifacts: one plan, nothing missing, floor held
"""
import os
import re
import sys

DEFAULT_WEIGHT = 1.0  # seconds, for a test the weights file does not know (stated, not hidden)
SUITES = ("ws", "bridge")


def die(msg):
    print("shard: " + msg, file=sys.stderr)
    sys.exit(1)


def read_tsv(path, cols):
    rows = []
    try:
        fh = open(path, encoding="utf-8")
    except OSError as e:
        die("cannot read %s: %s" % (path, e))
    with fh:
        for n, line in enumerate(fh, 1):
            line = line.rstrip("\n")
            if not line or line.startswith("#"):
                continue
            parts = line.split("\t")
            if len(parts) != cols:
                die("%s:%d: want %d tab-separated fields, got %d" % (path, n, cols, len(parts)))
            rows.append(parts)
    return rows


# ---- parse -----------------------------------------------------------------

CARGO_NOISE = re.compile(r"^\s*(Finished|Running|Doc-tests|Compiling|Fresh|Blocking|Downloaded|Downloading|Updating|Locking|Adding)\b")
RUNNING = re.compile(r"^\s*Running (.+?) \(")
SUMMARY = re.compile(r"^\d+ tests?, \d+ benchmarks?$")
LISTED = re.compile(r"^(\S+): test$")


def parse(suite, path):
    if suite not in SUITES:
        die("unknown suite %r" % suite)
    try:
        lines = open(path, encoding="utf-8").read().split("\n")
    except OSError as e:
        die("cannot read %s: %s" % (path, e))
    binary = None
    out = []
    for n, line in enumerate(lines, 1):
        if not line.strip():
            continue
        m = RUNNING.match(line)
        if m:
            binary = m.group(1)
            continue
        if CARGO_NOISE.match(line) or SUMMARY.match(line):
            continue
        m = LISTED.match(line)
        if m:
            if binary is None:
                die("%s:%d: a test before any `Running` line" % (path, n))
            if "\t" in line:
                die("%s:%d: a tab in a test name" % (path, n))
            out.append((suite, binary, m.group(1)))
            continue
        die("%s:%d: unrecognised line in the test list: %r" % (path, n, line[:120]))
    if suite == "ws" and not out:
        die("the workspace listed no tests")
    return out


# ---- assign ----------------------------------------------------------------

def load_entries(path):
    rows = [tuple(r) for r in read_tsv(path, 3)]
    if not rows:
        die("%s lists no tests" % path)
    for s, _b, _t in rows:
        if s not in SUITES:
            die("%s: unknown suite %r" % (path, s))
    return rows


def keys_of(entries):
    keys = {}
    for s, _b, t in entries:
        keys[(s, t)] = keys.get((s, t), 0) + 1
    return keys


def assign(n, weights_path, entries_path, outdir, base0=0.0):
    if n < 1:
        die("need at least one shard")
    entries = load_entries(entries_path)
    weights = {}
    for b, t, sec in read_tsv(weights_path, 3):
        try:
            weights[(b, t)] = float(sec)
        except ValueError:
            die("%s: %r is not seconds" % (weights_path, sec))
    weight = {}
    for s, b, t in entries:
        weight[(s, t)] = weight.get((s, t), 0.0) + weights.get((b, t), DEFAULT_WEIGHT)
    occ = keys_of(entries)
    load = [0.0] * n
    load[0] = float(base0)
    shards = [[] for _ in range(n)]
    for key in sorted(weight, key=lambda k: (-weight[k], k[0], k[1])):
        i = min(range(n), key=lambda j: (load[j], j))
        load[i] += weight[key]
        shards[i].append(key)
    os.makedirs(outdir, exist_ok=True)
    for i, keys in enumerate(shards):
        body = "".join("%s\t%s\t%d\n" % (s, t, occ[(s, t)]) for s, t in sorted(keys))
        with open(os.path.join(outdir, "shard-%d.tsv" % i), "w", encoding="utf-8") as fh:
            fh.write(body)
        print("shard %d: %d tests, planned %.0f s" % (i, len(keys), load[i]))


# ---- guard -----------------------------------------------------------------

def load_shards(outdir, n):
    shards = []
    for i in range(n):
        p = os.path.join(outdir, "shard-%d.tsv" % i)
        rows = read_tsv(p, 3)
        shard = {}
        for s, t, c in rows:
            if (s, t) in shard:
                die("%s lists %s %s twice" % (p, s, t))
            if not c.isdigit() or int(c) < 1:
                die("%s: bad occurrence count %r" % (p, c))
            shard[(s, t)] = int(c)
        shards.append(shard)
    return shards


def guard_data(entries, shards):
    occ = keys_of(entries)
    if not occ:
        die("guard: the full list is empty")
    seen = {}
    for i, shard in enumerate(shards):
        if not shard:
            die("guard: shard %d lists nothing" % i)
        for key, c in shard.items():
            if key in seen:
                die("guard: %s %s is in shard %d and shard %d" % (key[0], key[1], seen[key], i))
            seen[key] = i
            if occ.get(key) != c:
                die("guard: %s %s: shard %d says %d occurrence(s), the list has %s" % (key[0], key[1], i, c, occ.get(key)))
    missing = sorted(set(occ) - set(seen))
    if missing:
        die("guard: %d test(s) in no shard, first: %s %s" % (len(missing), missing[0][0], missing[0][1]))
    extra = sorted(set(seen) - set(occ))
    if extra:
        die("guard: %d assigned test(s) not in the list, first: %s %s" % (len(extra), extra[0][0], extra[0][1]))
    return len(occ), sum(occ.values())


def guard(n, entries_path, outdir):
    names, occurrences = guard_data(load_entries(entries_path), load_shards(outdir, n))
    print("guard ok: %d test names, %d listed occurrences, %d shards" % (names, occurrences, n))


def names(shardfile, suite):
    for s, t, _c in read_tsv(shardfile, 3):
        if s == suite:
            print(t)


# ---- result ----------------------------------------------------------------

RESULT = re.compile(r"^test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored")


def tally(path, tier2_re):
    """(passed, failed, ignored, tier2 passed) from a cargo test log."""
    passed = failed = ignored = tier2 = 0
    file = None
    tre = re.compile(tier2_re)
    try:
        lines = open(path, encoding="utf-8", errors="replace").read().split("\n")
    except OSError as e:
        die("cannot read %s: %s" % (path, e))
    for line in lines:
        m = re.match(r"^ +Running (\S+)", line)
        if m:
            file = m.group(1)
            continue
        if re.match(r"^ +Doc-tests ", line):
            file = None  # a doc-test result is never credited to the last tier-2 binary
            continue
        m = RESULT.match(line)
        if m:
            p = int(m.group(2))
            passed += p
            failed += int(m.group(3))
            ignored += int(m.group(4))
            if file and tre.match(file):
                tier2 += p
    return passed, failed, ignored, tier2


def result(k, shardfile, wslog, bridgelog, tier2_re):
    rows = read_tsv(shardfile, 3)
    if not rows:
        die("shard %s lists nothing; a shard that runs nothing fails" % k)
    expect = {s: sum(int(c) for ss, _t, c in rows if ss == s) for s in SUITES}
    total = tier2 = 0
    for suite, log in (("ws", wslog), ("bridge", bridgelog)):
        if expect[suite] == 0:
            if os.path.exists(log):
                die("%s ran though shard %s has no %s tests" % (log, k, suite))
            continue
        p, f, ig, t2 = tally(log, tier2_re)
        if f or ig:
            die("%s: %d failed, %d ignored" % (suite, f, ig))
        if p != expect[suite]:
            die("%s: %d passed, the assignment is %d" % (suite, p, expect[suite]))
        total += p
        tier2 += t2
    print("shard=%s expected=%d passed=%d tier2=%d" % (k, sum(expect.values()), total, tier2))


# ---- aggregate -------------------------------------------------------------

def aggregate(d, n, floor):
    floor = int(floor)
    expected = passed = tier2 = 0
    entries0 = None
    for i in range(n):
        base = os.path.join(d, "shard-%d" % i)
        res = os.path.join(base, "result.txt")
        if not os.path.exists(res):
            die("shard %d left no result (it did not finish)" % i)
        fields = dict(f.split("=", 1) for f in open(res).read().split())
        if fields.get("shard") != str(i):
            die("shard %d: result names shard %r" % (i, fields.get("shard")))
        e, p, t = int(fields["expected"]), int(fields["passed"]), int(fields["tier2"])
        if p != e:
            die("shard %d: %d passed of %d assigned" % (i, p, e))
        ent = open(os.path.join(base, "entries.tsv"), "rb").read()
        if entries0 is None:
            entries0 = ent
        elif ent != entries0:
            die("shard %d listed different tests" % i)
        expected += e
        passed += p
        tier2 += t
    # Re-run the guard over what the shards themselves ran, from their own files.
    shards = []
    for i in range(n):
        rows = read_tsv(os.path.join(d, "shard-%d" % i, "shard.tsv"), 3)
        shards.append({(s, t): int(c) for s, t, c in rows})
    names_, occurrences = guard_data(load_entries(os.path.join(d, "shard-0", "entries.tsv")), shards)
    if expected != occurrences or passed != occurrences:
        die("%d passed and %d assigned across the shards; the list has %d" % (passed, expected, occurrences))
    print("ANSWER tools_total_passed=%d names=%d" % (passed, names_))
    print("ANSWER tier2_tests_passed=%d floor=%d" % (tier2, floor))
    if tier2 < floor:
        die("only %d tier-2 tests passed across the shards, the floor is %d" % (tier2, floor))


def main(argv):
    if len(argv) < 2:
        die(__doc__)
    cmd, a = argv[1], argv[2:]
    try:
        if cmd == "parse":
            for row in parse(a[0], a[1]):
                print("\t".join(row))
        elif cmd == "assign":
            assign(int(a[0]), a[1], a[2], a[3], float(a[4]) if len(a) > 4 else 0.0)
        elif cmd == "guard":
            guard(int(a[0]), a[1], a[2])
        elif cmd == "names":
            names(a[0], a[1])
        elif cmd == "result":
            result(a[0], a[1], a[2], a[3], a[4])
        elif cmd == "aggregate":
            aggregate(a[0], int(a[1]), a[2])
        else:
            die("unknown subcommand %r" % cmd)
    except IndexError:
        die("missing argument for %s" % cmd)


if __name__ == "__main__":
    main(sys.argv)
