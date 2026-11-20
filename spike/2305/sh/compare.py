"""AET-WO-2305 §1.5: each run's records, main against the branch, by scenario: identical, or every
record that differs (python3 -I compare.py <runs-main> <runs-branch> <board>...)."""
import json, os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from trace import records
def runs(d):
    out = {}
    for l in open(os.path.join(d, 'campaign.jsonl')):
        if '"scenario"' in l:
            j = json.loads(l); out[j['scenario']] = (j, records(os.path.join(d, 'archive', j['run_id'], 'trace.cbor')))
    return out
a_root, b_root = sys.argv[1], sys.argv[2]
for board in sys.argv[3:]:
    a, b = runs(os.path.join(a_root, board)), runs(os.path.join(b_root, board))
    for name in a:
        if name not in b: print(board, name, 'only on main'); continue
        (ja, ra), (jb, rb) = a[name], b[name]
        same = ja['state_sequence_hash'] == jb['state_sequence_hash']
        diff = [('-', x) for x in ra if x not in rb] + [('+', x) for x in rb if x not in ra]
        print(f"{board} {name}: verdict {ja['verdict']}->{jb['verdict']} matched {ja['matched']}->{jb['matched']} state {'same' if same else 'moved'}; {len(diff)} records differ")
        for s, x in diff: print('   ', s, str(x)[:260])
    for name in b:
        if name not in a: print(board, name, 'only on the branch')
