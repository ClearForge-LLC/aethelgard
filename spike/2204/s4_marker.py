#!/usr/bin/env python3
"""AET-WO-2204 spike S4: where the BEGIN marker goes under `flood-max` with one
console level. Ten runs of s4c's `flood-max-onelevel` case; for each, the byte
offset of the BEGIN and END markers in stdout, and every line that is neither a
whole log line, a bare value nor a marker. Run after s4.py's s4c tag (it reads that case file).

Usage: s4_marker.py > out/s4-marker.txt
"""
import json, os, re, subprocess, sys, tempfile

sys.path.insert(0, "/repo/spike/2204")
import drive  # noqa: E402

# What a line of this run may be: the flood's two warnings, the boot's own log
# lines, or a bare value. Anything else is a fragment.
WHOLE = re.compile(rb"^\[\d\d:\d\d:\d\d\.\d{4}\] \[(WARNING|INFO)\] (i2c1: (Receive buffer underflow!|Unhandled write"
                   rb" to offset 0x1[0C]\..*)|gpioPortB: Unhandled.*|sysbus: Loading.*|cpu: Setting.*|machine-0: Machine "
                   rb"(started|paused)\..*|Pseudorandom.*)$|^0x[0-9A-Fa-f]+$|^AETHELGARD-BOOT-(BEGIN|END)$")
case = json.load(open("/cache/raw/s4c/flood-max-onelevel.case.json"))
cmds, _ = drive.commands(case)
for k in range(10):
    home = tempfile.mkdtemp(prefix="aet2204-home-", dir="/cache/homes")
    os.makedirs(f"{home}/.config/renode")
    open(f"{home}/.config/renode/config", "w").write("[general]\nhistory-path = /dev/null\n")
    env = {"HOME": home, "PATH": "/usr/local/bin:/usr/bin:/bin", "DOTNET_EnableDiagnostics": "0",
           "DOTNET_EnableWriteXorExecute": "0"}
    p = subprocess.run(["renode", "--disable-gui", "--console", "--plain", "-e", "; ".join(cmds)],
                       stdin=subprocess.DEVNULL, capture_output=True, env=env)
    d = p.stdout
    # The -e echo near the start holds both markers inside quotes; skip it.
    i, e = d.find(b"AETHELGARD-BOOT-BEGIN", 3000), d.find(b"AETHELGARD-BOOT-END", 3000)
    lines = d.split(b"\n")
    # Lines 0-4 are the banner and the -e echo; the last 160 are the listing.
    fragments = [l.strip(b"\r")[:120].decode("utf-8", "replace") for l in lines[5:-160]
                 if not WHOLE.match(l.strip(b"\r"))]
    print(json.dumps({"run": k, "exit": p.returncode, "stdout_bytes": len(d), "begin_at": i, "end_at": e,
                      "fragments": len(fragments), "first_fragments": fragments[:10]}))
