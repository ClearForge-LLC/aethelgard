#!/bin/bash
# AET-WO-1609 §1.1 (the late panic): `simulate --foreign` on sentinel-02-own, as
# ci/foreign-judge.sh's positive case runs it, on one ELF. Inside the tools image
# (~/.cache/aet-1609/kit/img.sh), the CLI built at /cache/target.
#   foreign.sh NAME ELF [WINDOW]   -> /cache/run/NAME/{simulate.json,exit}
set -u
cd /repo
bin=/cache/target/debug/aethelgard
spec=crates/aethelgard-cli/tests/fixtures/sentinel-02-own.json
o=/cache/run/$1; rm -rf "$o"; mkdir -p "$o"
[ -d /cache/run/gen ] || "$bin" generate --spec "$spec" --out /cache/run/gen > /cache/run/generate.json
status=0
"$bin" simulate --spec "$spec" --platform /cache/run/gen/board/sentinel-02-own.repl \
  --firmware "$2" --foreign "${3:-5000ms}" > "$o/simulate.json" || status=$?
echo "$status" > "$o/exit"
python3 -I - "$o/simulate.json" "$1" "$status" <<'PY'
import json, sys
path, name, status = sys.argv[1:]
v = json.load(open(path))
boot = v.get("boot") or {}
kinds = [f["kind"] for f in v.get("findings", []) if f["severity"] != "info"]
w = {(d["reference"], d["step"]): d["observed_ns"] for d in boot.get("devices", [])}
print(f"ANSWER 1609: {name} exit={status} verdict={v.get('verdict')} "
      f"error={(v.get('error') or {}).get('kind')} kinds={kinds} booted={boot.get('booted')} "
      f"pc={boot.get('pc')} icsr={boot.get('icsr')} witnessed={w}")
PY
