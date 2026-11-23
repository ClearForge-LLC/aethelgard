#!/usr/bin/env python3
"""AET-WO-1608 §1.1: the refusal kinds a hand-written specification meets.

For each committed specification that `validate` accepts as committed, delete
each object key (every depth) and empty each string value, one at a time, and
run `aethelgard validate` on each mutant. Writes one JSON line per mutant to
--rows and prints the kind counts.

Each number is also set to 0, one at a time (mutant `zero`): a number a
template keeps is a value a person may leave unread.

Usage: python3 -I corpus.py --bin target/debug/aethelgard --rows OUT.jsonl SPEC...
"""

import argparse
import collections
import concurrent.futures
import json
import os
import subprocess
import sys
import tempfile


def paths(node, prefix=()):
    """Yield (path, kind) for every key and every string value."""
    if isinstance(node, dict):
        for k, v in node.items():
            yield prefix + (k,), "delete"
            yield from paths(v, prefix + (k,))
    elif isinstance(node, list):
        for i, v in enumerate(node):
            yield from paths(v, prefix + (i,))
    elif isinstance(node, str):
        yield prefix, "empty"
    elif isinstance(node, (int, float)) and not isinstance(node, bool):
        yield prefix, "zero"


def mutate(doc, path, how):
    doc = json.loads(json.dumps(doc))
    parent = doc
    for p in path[:-1]:
        parent = parent[p]
    if how == "delete":
        del parent[path[-1]]
    elif how == "zero":
        parent[path[-1]] = 0
    else:
        parent[path[-1]] = ""
    return doc


def show(path):
    out = ""
    for p in path:
        out += f"[{p}]" if isinstance(p, int) else (("." if out else "") + p)
    return out


def run(binary, text):
    r = subprocess.run(
        [binary, "validate"], input=text.encode(), capture_output=True, timeout=120
    )
    line = r.stdout.decode().strip()
    try:
        body = json.loads(line)
    except json.JSONDecodeError:
        return {"exit": r.returncode, "kind": "UNPARSED", "raw": line[:200]}
    if body.get("ok"):
        return {"exit": r.returncode, "kind": "OK"}
    e = body["error"]
    return {
        "exit": r.returncode,
        "kind": e["kind"],
        "path": e.get("path"),
        "message": e["message"],
    }


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--bin", required=True)
    ap.add_argument("--rows", required=True)
    ap.add_argument("--jobs", type=int, default=6)
    ap.add_argument("specs", nargs="+")
    a = ap.parse_args()

    jobs = []
    skipped = []
    for spec in a.specs:
        text = open(spec).read()
        base = run(a.bin, text)
        if base["kind"] != "OK":
            skipped.append((spec, base["kind"]))
            continue
        doc = json.loads(text)
        for path, how in paths(doc):
            jobs.append((spec, path, how, json.dumps(mutate(doc, path, how), indent=2)))

    rows = []
    with concurrent.futures.ThreadPoolExecutor(a.jobs) as ex:
        futs = {ex.submit(run, a.bin, t): (s, p, h) for s, p, h, t in jobs}
        for f in concurrent.futures.as_completed(futs):
            s, p, h = futs[f]
            r = f.result()
            r.update({"spec": s, "mutant": h, "at": show(p)})
            rows.append(r)
    rows.sort(key=lambda r: (r["spec"], r["mutant"], r["at"]))
    with open(a.rows, "w") as out:
        for r in rows:
            out.write(json.dumps(r, sort_keys=True) + "\n")

    for spec, kind in skipped:
        print(f"skipped (base refused as {kind}): {spec}", file=sys.stderr)
    print(f"specifications: {len(a.specs) - len(skipped)}, mutants: {len(rows)}")
    for how in ("delete", "empty", "zero", None):
        sel = [r for r in rows if how is None or r["mutant"] == how]
        c = collections.Counter(r["kind"] for r in sel)
        print(f"\n## {how or 'both'}: {len(sel)}")
        for k, n in c.most_common():
            print(f"{n:6d} {100 * n / len(sel):5.1f}%  {k}")


if __name__ == "__main__":
    main()
