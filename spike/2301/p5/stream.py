#!/usr/bin/env python3
"""AET-WO-2301 P5.3: a process that owns Renode and reads its stdout as a stream, in bounded memory.

It is the "CLI" of the sandbox (the reaper checks its parent pid), starts the reaper the way
run.rs does, and keeps: the lines between the whole-line markers AETHELGARD-BOOT-BEGIN/END, the
first [ERROR] line, the first line of each class, a count per class, and the last 20 lines.
Nothing else is retained; a line is read at most 64 KiB at a time.

Usage: stream.py <run-dir> <workdir> <cmds-file> [--direct]
  default: through the reaper/trampoline (as sbx.sh); --direct: renode itself, fresh HOME.
"""
import collections, json, os, re, resource, subprocess, sys, threading, time

R, WD, CMDS = sys.argv[1], sys.argv[2], open(sys.argv[3]).read().strip()
DIRECT = "--direct" in sys.argv
BEGIN, END = "AETHELGARD-BOOT-BEGIN", "AETHELGARD-BOOT-END"
LOCKED = re.compile(r"Tried to (write|read) a locked peripheral")
LEVEL = re.compile(r"\] \[(DEBUG|INFO|WARNING|ERROR)\]|\]\s+\[(DEBUG|INFO|WARNING|ERROR)\]")
AETH = os.environ.get("AETH", "/cache/target/release/aethelgard")
ENVV = {"HOME": WD, "TMPDIR": WD, "DOTNET_EnableDiagnostics": "0", "DOTNET_EnableWriteXorExecute": "0"}
renode = ["/usr/local/bin/renode", "--disable-gui", "--console", "--plain", "-e", CMDS]
if DIRECT:
    argv = renode
else:
    ro = []
    for d in "/usr /lib /lib64 /bin /sbin /etc /usr/local/bin /opt/renode /opt/renode /proc /dev/urandom".split():
        ro += ["--ro", d]
    argv = [AETH, "--aethelgard-internal-reaper", "--cli-pid", str(os.getpid()), "--status", f"{R}/status",
            "--wall-ms", "30000", "--rss-kib", "1048576"] + ro + ["--rw", WD, "--"] + renode
t0 = time.monotonic()
p = subprocess.Popen(argv, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                     env=ENVV, cwd=WD, process_group=0)
err = {"bytes": 0}
def drain():
    while True:
        b = p.stderr.read(65536)
        if not b:
            return
        err["bytes"] += len(b)
th = threading.Thread(target=drain); th.start()

total_bytes = total_lines = 0
long_lines = 0
inblock, block, begins, ends = False, [], 0, 0
classes = collections.Counter()
first = {}
last = collections.deque(maxlen=20)
first_error = None
KEYS_MAX = 64
while True:
    raw = p.stdout.readline(65536)
    if not raw:
        break
    total_bytes += len(raw)
    if not raw.endswith(b"\n"):
        long_lines += 1          # a >64 KiB line, or the final line: counted, content not kept beyond this chunk
    total_lines += 1
    line = raw.decode("utf-8", "replace").strip()
    if line == BEGIN:
        begins += 1; inblock = True; block = []; continue
    if line == END:
        ends += 1; inblock = False; continue
    if inblock:
        block.append(line); continue
    m = LOCKED.search(line)
    if m:
        k = f"locked_peripheral_{m.group(1)}"
    else:
        lv = LEVEL.search(line)
        k = f"log_{(lv.group(1) or lv.group(2)).lower()}" if lv else "other"
    classes[k] += 1
    if k not in first:
        first[k] = line[:300]
    if k == "log_error" and first_error is None:
        first_error = line[:300]
    last.append(line[:200])
rc = p.wait(); th.join()
wall = time.monotonic() - t0
status = open(f"{R}/status").read().replace("\n", "|") if not DIRECT else ""
res = {
    "mode": "direct" if DIRECT else "sandbox", "exit": rc, "status": status, "wall_s": round(wall, 2),
    "stdout_bytes": total_bytes, "stdout_lines": total_lines, "stderr_bytes": err["bytes"],
    "begin_markers": begins, "end_markers": ends, "block": block, "classes": dict(classes),
    "first_of_class": first, "first_error": first_error, "last_lines": list(last), "chunked_lines": long_lines,
    "reader_peak_rss_kib": resource.getrusage(resource.RUSAGE_SELF).ru_maxrss,
    "children_peak_rss_kib": resource.getrusage(resource.RUSAGE_CHILDREN).ru_maxrss,
    "files_in_workdir": sorted(os.listdir(WD)),
}
print(json.dumps(res, indent=1))
