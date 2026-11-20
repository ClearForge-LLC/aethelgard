# AET-WO-1912 §1 step 2 (AET-WO-1913: and an `ALONE @tests/X.rs` marker, that binary whole: its `finished in` with every test passed): read the throwaway run's alone pass from its tools-shard logs (fetch.sh).
# Usage: python3 -I alone_times.py RUNDIR > alone-times.tsv
# For each `ALONE NAME` marker, every binary that then reports `1 passed` gives NAME's alone time, its libtest
# `finished in` (one test, one thread, one invocation). Prints shard<TAB>binary<TAB>name<TAB>seconds<TAB>cpu,
# and each shard's CPU model and colocated blocks' `finished in` from the normal pass on stderr.
import glob, importlib.util, os, re, sys
spec = importlib.util.spec_from_file_location('table', os.path.join(os.path.dirname(os.path.abspath(__file__)), 'table.py'))
table = importlib.util.module_from_spec(spec); spec.loader.exec_module(table)
d = sys.argv[1]
for f in sorted(glob.glob(f'{d}/shard-*.log'), key=lambda p: int(p.rsplit('-', 1)[1][:-4])):
    k = int(f.rsplit('-', 1)[1][:-4])
    cpu, name, binary, seen = '', None, None, set()
    blocks = []
    for l in open(f, encoding='utf-8', errors='replace'):
        m = table.TS.match(l.rstrip('\n'))
        if not m: continue
        s = m.group(2)
        if 'model name' in s: cpu = s.split(':', 1)[1].strip()
        if s.startswith('ALONE '): name = s[6:].strip(); continue
        mb = re.match(r'\s*Running (.+?) \(', s)
        if mb: binary = mb.group(1); continue
        mr = re.search(r'test result: ok\. (\d+) passed.*finished in ([\d.]+)s', s)
        if mr and binary:
            if name is None and int(mr.group(1)) > 0: blocks.append(f'{binary} {mr.group(1)} tests {mr.group(2)} s')
            if name is not None and name.startswith('@'):
                if binary == name[1:] and int(mr.group(1)) > 0: print(f'{k}\t{binary}\t{name}\t{mr.group(2)}\t{cpu}'); seen.add(name)
            elif name is not None and int(mr.group(1)) == 1:
                print(f'{k}\t{binary}\t{name}\t{mr.group(2)}\t{cpu}'); seen.add(name)
        if re.search(r'test result: FAILED|error: test failed', s): print(f'shard {k}: FAILED near {name}', file=sys.stderr)
    print(f'shard {k}: cpu "{cpu}"; {len(seen)} alone; normal-pass binaries: {"; ".join(b for b in blocks if float(b.rsplit(" ", 2)[-2]) >= 60)}', file=sys.stderr)
