# AET-WO-1912 §1 step 1: from tools-shard logs fetched by fetch.sh, the per-shard table and the lagging rows.
# Usage: python3 -I table.py REPO REV RUNSDIR RUN...
#   Every measurement is set against the rows of REV (`git -C REPO show REV:ci/...`), the head being re-weighed.
# Per shard: job id and wall (jobs API), run step (its `##[group]Run ... shard.sh run` to its `shard=K expected=` line),
# the job's overhead (wall less run step), planned seconds (the plan step's line), and per binary the sum of its tests'
# rows (or its colocate row) against libtest's `finished in`.
# Per test, one of three measurements:
#   solo  the only test of its binary in that shard: the binary's `finished in` is the test's alone time;
#   lock  in a binary that serialises calls on one lock (LOCKED): its `ok` less the previous `ok` (or the binary's
#         `Running`). The lock is held per call, not per test, so this is a candidate, not a weight;
#   ub    otherwise its `ok` less its binary's `Running`: an upper bound, a candidate.
import re, sys, subprocess
from datetime import datetime

LOCKED = {'tests/board_tier2.rs', 'tests/against_fab_tier2.rs', 'tests/verify_2k2_tier2.rs', 'tests/twin_tier2.rs'}
TS = re.compile(r'^[^\t]*\t[^\t]*\t\ufeff?(\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d\.\d+)Z ?(.*)$')
DEFAULT = 0.05
def ts(s): return datetime.fromisoformat(s[:26])
def iso(s): return datetime.fromisoformat(s.replace('Z', ''))

def rows(repo, rev, f, cols):
    out = {}
    for l in subprocess.run(['git', '-C', repo, 'show', f'{rev}:ci/{f}'], capture_output=True, text=True, check=True).stdout.splitlines():
        if l.startswith('#') or not l.strip(): continue
        p = l.split('\t')
        out[tuple(p[:cols])] = float(p[cols])
    return out

def shard(path):
    lines = []
    for l in open(path, encoding='utf-8', errors='replace'):
        m = TS.match(l.rstrip('\n'))
        if m: lines.append((ts(m.group(1)), m.group(2)))
    start = next(t for t, s in lines if s.startswith('##[group]Run') and 'shard.sh run' in s)
    end = next((t for t, s in lines if re.match(r'shard=\d+ expected=', s)), lines[-1][0])  # a failed shard has no result line
    planned = {}
    for t, s in lines:
        m = re.match(r'shard (\d+): \d+ tests, planned (\d+) s', s)
        if m: planned[int(m.group(1))] = int(m.group(2))
    cpu = next((s for t, s in lines if 'Model name' in s or 'model name' in s), '')
    bins, cur = [], None  # bins: dict(binary, tests=[(name, s, how)], passed, fin)
    for t, s in lines:
        if t < start: continue
        if s.startswith('ALONE ') or 'model name' in s: break  # the throwaway's alone pass follows the shard's own tests
        m = re.match(r'\s*Running (.+?) \(', s)
        if m: cur = dict(binary=m.group(1), run=t, prev=t, tests=[]); continue
        if cur is None: continue
        m = re.match(r'test (\S+) \.\.\. ok', s)
        if m:
            if cur['binary'] in LOCKED: cur['tests'].append((m.group(1), (t - cur['prev']).total_seconds(), 'lock')); cur['prev'] = t
            else: cur['tests'].append((m.group(1), (t - cur['run']).total_seconds(), 'ub'))
            continue
        m = re.search(r'test result: ok\. (\d+) passed.*finished in ([\d.]+)s', s)
        if m:
            if int(m.group(1)) > 0:
                cur['passed'], cur['fin'] = int(m.group(1)), float(m.group(2))
                if cur['passed'] == 1 and len(cur['tests']) == 1:
                    cur['tests'] = [(cur['tests'][0][0], cur['fin'], 'solo')]
                bins.append(cur)
            cur = None
    return (end - start).total_seconds(), planned, bins, cpu

def main():
    cand_out = None
    if sys.argv[1] == '--candidates': cand_out = open(sys.argv[2], 'w'); del sys.argv[1:3]
    repo, rev, d, runs = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4:]
    coloc = rows(repo, rev, 'colocate.tsv', 1); weights = rows(repo, rev, 'test-weights.tsv', 2)
    blocks, samples = {}, {}
    print(f'Rows: ci/test-weights.tsv and ci/colocate.tsv at {rev}. Runs: {", ".join(runs)}.\n')
    for run in runs:
        sha, concl = open(f'{d}/{run}/run.tsv').read().split('\t')[:2]
        jobs = {}
        for l in open(f'{d}/{run}/jobs.tsv'):
            jid, name, _, a, b = l.rstrip('\n').split('\t')
            m = re.match(r'tools-shard \((\d+)\)', name)
            if m: jobs[int(m.group(1))] = (jid, (iso(b) - iso(a)).total_seconds())
        print(f'### run {run} (head {sha[:8]}, {concl.strip()})\n')
        print('| shard | job | job min | run step s | overhead s | planned s (its head) | rows at REV, s | binaries over their rows by 60 s or more: finished in / rows |')
        print('|---|---|---|---|---|---|---|---|')
        for k in sorted(jobs):
            jid, wall = jobs[k]
            step, planned, bins, cpu = shard(f'{d}/{run}/shard-{k}.log')
            total, over = 0.0, []
            for b in bins:
                key = b['binary']
                if (key,) in coloc:
                    r = coloc[(key,)]
                    blocks.setdefault(key, []).append((b['fin'], run, k, jid))
                else:
                    r = sum(weights.get((key, n), DEFAULT) for n, _, _ in b['tests'])
                    for n, s, how in b['tests']:
                        samples.setdefault((key, n), []).append((s, how, run, k, jid))
                total += r
                if b['fin'] - r >= 60: over.append(f'{key.split("/")[-1]} {b["fin"]:.0f}/{r:.0f}')
            print(f'| {k} | {jid} | {wall/60:.1f} | {step:.0f} | {wall-step:.0f} | {planned.get(k)} | {total:.0f} | {"; ".join(over)} |')
        print()
    print('### colocated blocks: every `finished in` against the row\n')
    print('| block | row | samples (s, run/shard) | worst | row / worst |')
    print('|---|---|---|---|---|')
    for b in sorted(blocks):
        r = coloc[(b,)]; ss = blocks[b]; w = max(ss)
        print(f'| {b} | {r:.0f} | {"; ".join(f"{s:.2f} ({run}/{k})" for s, run, k, _ in ss)} | {w[0]:.2f} ({w[1]} job {w[3]}) | {r/w[0]:.2f}{" **lags**" if r < 0.75*w[0] else ""} |')
    print()
    print('### tests: any sample of 30 s or more, or a row more than 25 % under its worst sample (worst of 1 s or more)\n')
    print('`solo` is an alone time; `lock` and `ub` are candidates only (see the header).\n')
    print('| binary | test | row | worst solo | worst lock/ub | samples | verdict |')
    print('|---|---|---|---|---|---|---|')
    out = []
    for key, ss in samples.items():
        row = weights.get(key)
        r = row if row is not None else DEFAULT
        solo = max((s for s, how, *_ in ss if how == 'solo'), default=None)
        cand = max((s for s, how, *_ in ss if how != 'solo'), default=None)
        worst = max(s for s, *_ in ss)
        if worst < 1 or not (worst >= 30 or r < 0.75 * worst): continue
        if solo is not None and r < 0.75 * solo: v = 'lags (solo)'
        elif solo is not None and cand is not None and r < 0.75 * cand and cand > solo: v = 'candidate (solo holds)'
        elif solo is None and r < 0.75 * worst: v = 'candidate: alone run'
        else: v = 'holds'
        if cand_out and v != 'holds': cand_out.write(f'{key[0]}\t{key[1]}\t{worst:.2f}\n')
        out.append((worst, f'| {key[0]} | {key[1]} | {row if row is not None else "-"} | {f"{solo:.2f}" if solo is not None else "-"} | {f"{cand:.2f}" if cand is not None else "-"} | {len(ss)} | {v} |'))
    for _, l in sorted(out, reverse=True): print(l)

if __name__ == '__main__': main()
