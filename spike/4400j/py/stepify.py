"""AET-WO-4400j (1): rewrite a captured foreign run so virtual time advances in whole seconds, with a probe at each.
The probe prints one `AET-STAT` line: the second, /proc/self/status (VmRSS, RssAnon, RssFile, VmHWM, in kB), and the
GC's figures (GetTotalMemory(false), and from GetGCMemoryInfo: the heap after the last GC, committed, fragmented;
gen-2 collections). FULLGC=1 adds GetTotalMemory(true), a forced full collection (it perturbs the run).
PAUSE="6 9": at those seconds, after the probe, a compacting full GC, then OUT/pause-T is written and the monitor
waits until OUT/go-T exists (the dump is taken meanwhile; sh/replay.sh).
Splitting is at quantum multiples (whole seconds), so the machine runs the same instants; the pause/resume per second
is the perturbation (an input's timeline already pauses at every byte).
READS=FILE STEP_MS=10 FROM_MS=4000: also, every STEP_MS from FROM_MS, `echo AET-READ t=<ms>` and the commands in FILE
(the CLI's own end-of-window reads, copied from between its BEGIN and END markers): the end read at every step (F4).
Usage: stepify.py run   E_ARG OUT    -> the -e argument with its one RunFor stepped (stdout)
       stepify.py file  SCRIPT OUT   -> the timeline script rewritten in place"""
import os, re, sys
RUN = re.compile(r'emulation RunFor "(\d+)\.(\d{9})5"')
mode, src, out = sys.argv[1:4]
full = os.environ.get("FULLGC") == "1"
pauses = {int(x) for x in os.environ.get("PAUSE", "").split()}
reads = open(os.environ["READS"]).read().split("; ") if os.environ.get("READS") else []
g = int(os.environ.get("STEP_MS", "1000")) * 10**6 if reads else 10**9
from_ns = int(os.environ.get("FROM_MS", "0")) * 10**6

def run_for(ns):
    return 'emulation RunFor "%d.%09d5"' % (ns // 10**9, ns % 10**9)

def probe(t):
    status = "' '.join(l.split(':')[0] + '=' + l.split()[1] for l in open('/proc/self/status') if l.split(':')[0] in ('VmRSS', 'RssAnon', 'RssFile', 'VmHWM'))"
    gi = "__import__('System').GC.GetGCMemoryInfo()"
    fields = ["%s" % status, "__import__('System').GC.GetTotalMemory(False)", gi + ".HeapSizeBytes",
              gi + ".TotalCommittedBytes", gi + ".FragmentedBytes", "__import__('System').GC.CollectionCount(2)"]
    fmt = "AET-STAT t=%d %%s gc_total=%%d heap=%%d committed=%%d frag=%%d gen2=%%d" % t
    if full:
        fields.append("__import__('System').GC.GetTotalMemory(True)")
        fmt += " gc_live=%d"
    cmds = ['python "print(\'%s\' %% (%s))"' % (fmt, ", ".join(fields))]
    if t in pauses:
        cmds.append(
            "python \"(lambda S: (S.GC.Collect(2, S.GCCollectionMode.Forced, True, True), S.GC.WaitForPendingFinalizers(), "
            "S.GC.Collect(2, S.GCCollectionMode.Forced, True, True), open('%s/pause-%d', 'w').close(), "
            "[S.Threading.Thread.Sleep(200) for _ in iter(lambda: S.IO.File.Exists('%s/go-%d'), True)]))(__import__('System'))\""
            % (out, t, out, t))
    return cmds

def at(b):
    out = probe(b // 10**9) if b % 10**9 == 0 else []
    if reads and b >= from_ns:
        out += ['echo "AET-READ t=%d"' % (b // 10**6)] + reads
    return out

def step(lines):
    """Each RunFor split at every multiple of the step (whole seconds, or STEP_MS with READS), with a probe at each
    second and the reads at each step."""
    now, res = 0, []
    for line in lines:
        m = RUN.fullmatch(line.strip())
        if not m:
            res.append(line); continue
        ns = int(m.group(1)) * 10**9 + int(m.group(2))
        end = now + ns
        while (now // g + 1) * g <= end:
            nxt = (now // g + 1) * g
            if nxt > now:
                res.append(run_for(nxt - now))
            now = nxt
            res.extend(at(now))
        if end > now:
            res.append(run_for(end - now)); now = end
    return res

if mode == "run":
    cmds = open(src).read().split("; ")
    runs = [i for i, c in enumerate(cmds) if RUN.fullmatch(c.strip())]
    assert len(runs) == 1, "expected one RunFor in the -e argument, found %d" % len(runs)
    i = runs[0]
    # The stepped run goes in a script file, as the CLI's input timeline does: inline, 10 ms reads pass the 128 KiB a
    # single argument may be.
    open(os.path.join(out, "steps.resc"), "w").write("\n".join(probe(0) + step([cmds[i]])) + "\n")
    cmds[i] = "include @%s" % os.path.join(out, "steps.resc")
    sys.stdout.write("; ".join(cmds))
else:
    lines = open(src).read().splitlines()
    open(src, "w").write("\n".join(probe(0) + step(lines)) + "\n")
