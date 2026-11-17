#!/bin/bash
# Inside the tools image: every generated board's `simulate --for 100ms` verdict, by main's CLI on
# main's platform and by this branch's CLI on this branch's platform. Only the platform's sha256
# may differ. main's tree is extracted at /cache/main-src (git archive origin/main).
set -u
( cd /cache/main-src && CARGO_TARGET_DIR=/cache/target-main cargo build --locked --offline -p aethelgard-cli 2>&1 | tail -1 && CARGO_TARGET_DIR=/cache/target-main cargo build --offline --manifest-path bridge/Cargo.toml --locked 2>&1 | tail -1 )
( cd /repo && CARGO_TARGET_DIR=/repo/target cargo build --offline --manifest-path bridge/Cargo.toml --locked 2>&1 | tail -1 )
for b in sentinel-01:examples/sentinel-01-minimal.json sentinel-01-bench:examples/sentinel-01.json; do
  n=${b%%:*}; s=${b##*:}
  for side in main:/cache/main-src:/cache/target-main branch:/repo:/repo/target; do
    tag=${side%%:*}; rest=${side#*:}; src=${rest%%:*}; tgt=${rest##*:}
    O=/cache/verdicts/$n-$tag; rm -rf $O; mkdir -p $O
    ( cd $src && $tgt/debug/aethelgard build --crate boards/$n --out $O/fw.elf > $O/build.json 2>&1
      $tgt/debug/aethelgard simulate --platform boards/$n/$n.repl --firmware $O/fw.elf --spec $s --for 100ms > $O/verdict.json 2>&1; echo "$n $tag exit=$?" )
  done
  python3 -I - "$n" <<'PY'
import json, sys
n = sys.argv[1]
def load(t):
    v = json.load(open(f"/cache/verdicts/{n}-{t}/verdict.json"))
    v.pop("memory", None); v.get("boot", {}).pop("memory", None)
    return v
a, b = load("main"), load("branch")
def flat(o, p=""):
    if isinstance(o, dict):
        for k, x in o.items(): yield from flat(x, p + "/" + k)
    elif isinstance(o, list):
        for i, x in enumerate(o): yield from flat(x, p + "/" + str(i))
    else: yield p, o
fa, fb = dict(flat(a)), dict(flat(b))
diff = sorted(k for k in set(fa) | set(fb) if fa.get(k) != fb.get(k))
print(n, "verdict", a.get("verdict"), b.get("verdict"), "differing fields:", diff)
PY
done
