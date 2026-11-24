#!/usr/bin/env python3
"""AET-WO-1610 §1.1, F1: every one-key deletion over the committed specifications
(spike/1608/corpus.py's deletions, the 1608 rows not having been kept), keeping the
mutants `validate` refuses as `InvalidJson`, and every deletion that leaves `{}`.

Usage: python3 -I spike/1610/f1.py --bin target/debug/aethelgard SPEC...
"""

import argparse
import concurrent.futures
import importlib.util
import json
import os

here = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location(
    "corpus", os.path.join(here, "..", "1608", "corpus.py")
)
corpus = importlib.util.module_from_spec(spec)
spec.loader.exec_module(corpus)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--bin", required=True)
    ap.add_argument("--jobs", type=int, default=6)
    ap.add_argument("specs", nargs="+")
    a = ap.parse_args()
    jobs, skipped = [], []
    for s in a.specs:
        text = open(s).read()
        if corpus.run(a.bin, text)["kind"] != "OK":
            skipped.append(s)
            continue
        doc = json.loads(text)
        for path, how in corpus.paths(doc):
            if how != "delete":
                continue
            m = corpus.mutate(doc, path, how)
            parent = m
            for p in path[:-1]:
                parent = parent[p]
            jobs.append((s, path, parent == {}, json.dumps(m, indent=2)))
    rows = []
    with concurrent.futures.ThreadPoolExecutor(a.jobs) as ex:
        futs = {ex.submit(corpus.run, a.bin, t): (s, p, e) for s, p, e, t in jobs}
        for f in concurrent.futures.as_completed(futs):
            s, p, e = futs[f]
            r = f.result()
            r.update({"spec": s, "at": corpus.show(p), "leaves_empty": e})
            rows.append(r)
    rows.sort(key=lambda r: (r["spec"], r["at"]))
    print(f"specifications: {len(a.specs) - len(skipped)}, deletions: {len(rows)}")
    for s in skipped:
        print(f"skipped (refused at base): {s}")
    print("\n## refused InvalidJson")
    for r in rows:
        if r["kind"] == "InvalidJson":
            print(json.dumps({k: r.get(k) for k in ("spec", "at", "exit", "message", "leaves_empty")}))
    print("\n## deletions leaving {} (any kind)")
    for r in rows:
        if r["leaves_empty"]:
            print(json.dumps({k: r.get(k) for k in ("spec", "at", "exit", "kind")}))


if __name__ == "__main__":
    main()
