"""The null sweep's judge (python3 -I null-compare.py <sweep-a> <sweep-b> [--versions]): for each
specification, the generated tree file by file and the simulate verdict; for each scenario, the
campaign's lines and every archived record. With --versions, a difference only in the trace version
byte, the run id, the state hashes and the trace's byte count is reported as a version line, not a
difference (the bump's own commit is the baseline for every later one)."""
import json, os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from trace import records
a_root, b_root = sys.argv[1], sys.argv[2]
VERSIONED = {"run_id", "state_sequence_hash", "trace_bytes"}
bad = 0
def lines(p):
    return [json.loads(l) for l in open(p) if l.strip()] if os.path.exists(p) else None
for kind in ["spec", "scenario"]:
    for n in sorted(os.listdir(os.path.join(a_root, kind))):
        a, b = os.path.join(a_root, kind, n), os.path.join(b_root, kind, n)
        out = []
        if kind == "spec":
            for f in ["tree.sha256", "simulate.json", "generate.json"]:
                x = open(os.path.join(a, f)).read() if os.path.exists(os.path.join(a, f)) else None
                y = open(os.path.join(b, f)).read() if os.path.exists(os.path.join(b, f)) else None
                if x != y: out.append(f"{f} differs")
        else:
            la, lb = lines(os.path.join(a, "campaign.jsonl")), lines(os.path.join(b, "campaign.jsonl"))
            if la is None or lb is None or len(la) != len(lb):
                out.append("campaign lines differ in number")
            else:
                for x, y in zip(la, lb):
                    keys = {k for k in set(x) | set(y) if x.get(k) != y.get(k)}
                    if "--versions" in sys.argv: keys -= VERSIONED
                    if keys: out.append(f"line {x.get('scenario', 'summary')}: {sorted(keys)}")
                    if "run_id" in x and "run_id" in y:
                        ra = records(os.path.join(a, "archive", x["run_id"], "trace.cbor"))
                        rb = records(os.path.join(b, "archive", y["run_id"], "trace.cbor"))
                        if ra != rb: out.append(f"records differ: {len(ra)} vs {len(rb)}")
        bad += bool(out)
        print(f"{kind} {n}: {'identical' if not out else '; '.join(out)}")
print(f"differences: {bad}")
sys.exit(1 if bad else 0)
