"""The null sweep's judge (python3 -I null-compare.py <sweep-a> <sweep-b> [--versions]): for each
specification, the generated tree file by file and the simulate verdict; for each scenario, the
campaign's lines and every archived record. With --versions, a difference only in the trace version
byte, the run id, the state hashes and the trace's byte count is reported as a version line, not a
difference (the bump's own commit is the baseline for every later one)."""
import json, os, re, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from trace import records
a_root, b_root = sys.argv[1], sys.argv[2]
VERSIONED = {"run_id", "state_sequence_hash", "trace_bytes"}
# Run-to-run noise, never a verdict's: the sweep's own directory, and the
# memory a run measured (`memory.peak_rss_kib`, `memory.share_permille`).
def norm(text, root):
    text = text.replace(root.rstrip("/").replace(os.path.expanduser("~/.cache/aet-1211c"), "/cache") + "/", "<sweep>/")
    try:
        lines = [json.loads(l) for l in text.splitlines() if l.strip()]
    except ValueError:
        return text
    for v in lines:
        if isinstance(v, dict):
            v.pop("memory", None)
    return lines
def kind24(records):
    """Kind 24 without the two fields trace 18 appends (its idle and tick): (records, how many had them)."""
    out, n = [], 0
    for r in records:
        if isinstance(r, list) and r and r[0] == 24 and len(r) == 2 + 12:
            out.append(r[:-2]); n += 1
        else:
            out.append(r)
    return out, n
bad = 0
def lines(p):
    return [json.loads(l) for l in open(p) if l.strip()] if os.path.exists(p) else None
for kind in ["spec", "scenario"]:
    for n in sorted(os.listdir(os.path.join(a_root, kind))):
        a, b = os.path.join(a_root, kind, n), os.path.join(b_root, kind, n)
        out, notes = [], []
        if kind == "spec":
            # solve's bytes, with the solved version line normalised (D7: the
            # bump moves it on every board; the new fields only on a pin
            # device's).
            sa, sb = (os.path.join(r, "solve.json") for r in (a, b))
            if os.path.exists(sa) and os.path.exists(sb):
                import re as _re
                ver = lambda t: _re.sub(r'"schema_version":\d+', '"schema_version":N', t)
                if ver(open(sa).read()) != ver(open(sb).read()): out.append("solve.json differs beyond its version line")
            for f in ["tree.sha256", "simulate.json", "generate.json"]:
                x = norm(open(os.path.join(a, f)).read(), a_root) if os.path.exists(os.path.join(a, f)) else None
                y = norm(open(os.path.join(b, f)).read(), b_root) if os.path.exists(os.path.join(b, f)) else None
                if x != y: out.append(f"{f} differs")
        else:
            la, lb = lines(os.path.join(a, "campaign.jsonl")), lines(os.path.join(b, "campaign.jsonl"))
            if la is None or lb is None or len(la) != len(lb):
                out.append("campaign lines differ in number")
            else:
                for x, y in zip(la, lb):
                    x = norm(json.dumps(x), a_root)[0]; y = norm(json.dumps(y), b_root)[0]
                    keys = {k for k in set(x) | set(y) if x.get(k) != y.get(k)}
                    if "--versions" in sys.argv: keys -= VERSIONED
                    if keys: out.append(f"line {x.get('scenario', 'summary')}: {sorted(keys)}")
                    if "run_id" in x and "run_id" in y:
                        ra = records(os.path.join(a, "archive", x["run_id"], "trace.cbor"))
                        rb = records(os.path.join(b, "archive", y["run_id"], "trace.cbor"))
                        if "--versions" in sys.argv:
                            (ra, na), (rb, nb) = kind24(ra), kind24(rb)
                            if nb and not na: notes.append(f"kind 24 gains idle and tick on {nb}")
                        if ra != rb:
                            out.append(f"records differ: {len(ra)} vs {len(rb)}")
                            for k, (p, q) in enumerate(zip(ra, rb)):
                                if p != q: out.append(f"  first at {k}: {str(p)[:200]} != {str(q)[:200]}"); break
        bad += bool(out)
        print(f"{kind} {n}: {'identical' if not out else '; '.join(out)}{' (' + '; '.join(notes) + ')' if notes else ''}")
print(f"differences: {bad}")
sys.exit(1 if bad else 0)
