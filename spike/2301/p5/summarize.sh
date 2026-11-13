#!/bin/bash
# Builds spike/2301/out/p5-*.txt from /cache/p5/<label>/summary and stream.json (run inside img.sh).
O=/repo/spike/2301/out; C=/cache/p5
one() { # label ms quiet
  echo "## label=$1  virtual=$2 ms  quiet=$3 (0: logFile only; 1: logFile + logLevel 3 console; 2: neither)"
  echo "cmd: spike/2301/img.sh -- bash -c '/repo/spike/2301/p5/sbx.sh $1 $2 $3'"
  cat $C/$1/summary | cut -c1-400
  if [ -f $C/$1/stdout ] && grep -qx $'AETHELGARD-BOOT-BEGIN\r\r' $C/$1/stdout; then
    echo "marker block (whole-line BEGIN..END, $(sed -n '/^AETHELGARD-BOOT-BEGIN/,/^AETHELGARD-BOOT-END/p' $C/$1/stdout | wc -l) lines incl. markers): $(sed -n '/^AETHELGARD-BOOT-BEGIN/,/^AETHELGARD-BOOT-END/p' $C/$1/stdout | tr -d '\r' | sed '1d;$d' | tr '\n' ' ')"
  else echo "marker block: ABSENT (no whole-line AETHELGARD-BOOT-BEGIN in stdout)"; fi
  echo
}
{ echo "# cmd: spike/2301/p5/sbx.sh <label> <ms> <quiet>  (Renode under the rebuilt sandbox, CLI command list + logFile; flood firmware /cache/target/thumbv7em-none-eabihf/release/p5-flood); full command list: $(cat $C/flood-q1-10/cmds.txt)"
  echo; for a in "flood-q2-10 10 2" "flood-q0-10 10 0" "flood-q1-10 10 1" "flood-q1-100 100 1" "flood-q2-100 100 2" "flood-q0-100 100 0"; do set -- $a; one $1 $2 $3; done
} > $O/p5-b-sandbox-logfile.txt
{ echo "# cmd: spike/2301/p5/sbx.sh flood-q1-<ms> <ms> 1   (logFile + logLevel 3 console; flood firmware; log bytes and lines per virtual ms)"
  echo "ms  logfile_bytes  logfile_lines  bytes/ms  lines/ms  reaper-status"
  for ms in 1 2 5 10 20 25 30 100; do d=$C/flood-q1-$ms; b=$(stat -c %s $d/tool/renode.log); l=$(wc -l < $d/tool/renode.log)
    echo "$ms $b $l $((b/ms)) $((l/ms)) $(tr '\n' '|' < $d/status)"; done
  echo; echo "# run-to-run: same flood, 5 ms (instructions executed identical = 0x7A120 each), locked-write lines counted"
  for i in 1 2 3; do echo "both-5-$i: stdout_locked=$(grep -c 'locked peripheral' $C/both-5-$i/stdout) file_locked=$(grep -c 'locked peripheral' $C/both-5-$i/tool/renode.log)   [cmd: sbx.sh both-5-$i 5 0]"; done
  for i in 1 2; do echo "f-5-$i (quiet=1): file lines $(wc -l < $C/f-5-$i/tool/renode.log)   stream-5-$i (quiet=2, stream.py): $(python3 -c "import json;print(json.load(open('$C/stream-5-$i/stream.json'))['classes']['locked_peripheral_write'])")"; done
  for i in 1 2; do echo "f-10-$i: file lines $(wc -l < $C/f-10-$i/tool/renode.log)   stream-10-$i: $(python3 -c "import json;print(json.load(open('$C/stream-10-$i/stream.json'))['classes']['locked_peripheral_write'])")"; done
  echo "ms=25: file lines (flood-q1-25) $(wc -l < $C/flood-q1-25/tool/renode.log); stream-25 locked $(python3 -c "import json;print(json.load(open('$C/stream-25/stream.json'))['classes']['locked_peripheral_write'])")"
  echo; echo "first log lines:"; sed -n 1,5p $C/flood-q1-25/tool/renode.log | cut -c1-200
} > $O/p5-c-rates.txt
{ echo "# cmd: STREAM=1 spike/2301/p5/sbx.sh <label> <ms> 2   (stream.py owns the reaper/Renode; console default, no logFile; STREAM_ARGS=--direct for no sandbox)"
  for l in stream-25 stream-100 stream-300 stream-1000 stream-direct-100; do
    echo "## $l   cmd: spike/2301/img.sh -- bash -c 'STREAM=1 [STREAM_ARGS=--direct] /repo/spike/2301/p5/sbx.sh $l <ms> 2'"
    python3 - $C/$l/stream.json <<'P'
import json,sys
d=json.load(open(sys.argv[1]))
for k in ['mode','exit','status','wall_s','stdout_bytes','stdout_lines','stderr_bytes','begin_markers','end_markers','classes','reader_peak_rss_kib','children_peak_rss_kib','first_error','files_in_workdir','block']:
    print(f"{k}: {d[k]}")
print("first_of_class:", json.dumps(d['first_of_class'])[:600])
P
    echo; done
  echo "# reference firmware at 1000 ms, quiet=1 (is Renode's RSS growth the flood's?)"; cat $C/ref-q1-1000/summary | cut -c1-300
} > $O/p5-d-stream.txt
