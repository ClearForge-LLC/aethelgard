#!/usr/bin/env bash
# AET-WO-1313 §1.3: every gate (run.sh) for each placement option on one specification; one TSV row
# per option. Usage, from a repository root holding a CLI built with placer.patch applied:
#   spike/1313/options.sh <spec.json> <outdir> <opt>...      ("-" is today's placer)
# Option letters (placer.patch, env AET_1313_OPT; AET_1313_BOUND=100,100 throughout):
#   a compaction, b bounded placer (box area), B bounded first-fit, c connectors on the left and right
#   edges, C connectors on the bottom and top edges, d the row's gap at 0.5 mm, o ICs in geometric order.
# Columns: option, the eight gates' exits, outline (mm), segments, track (mm), vias, rip-ups, emit (s),
# extract's findings (net=bound fF).
set -u
SPEC=$1; OUT=$(realpath -m "$2"); shift 2
HERE=$(dirname "$(realpath "$0")")
mkdir -p "$OUT"
for o in "$@"; do
  oo=$o; [ "$o" = - ] && oo=
  t=${o/-/none}
  r=$(AET_ENV="AET_1313_OPT=$oo AET_1313_BOUND=100,100 AET_1313_DUMP=$OUT/$t.layout.json AET_1313_ROUTE=$OUT/$t.route.json" \
      bash "$HERE/run.sh" "$SPEC" "$OUT/$t")
  python3 -I - "$t" "$r" "$OUT" <<'PY'
import json, re, sys
t, r, out = sys.argv[1:4]
gates = " ".join(re.findall(r"\d-[a-z-]+=\d+", r))
emit = re.search(r"emit=([\d.]+)s", r).group(1)
try:
    o = json.load(open(f"{out}/{t}.layout.json"))["outline"]
    outline = f"{(o[2]-o[0])/1e6:.2f} x {(o[3]-o[1])/1e6:.2f}"
    k = json.load(open(f"{out}/{t}.route.json"))
    route = f"{k['segments']}\t{k['track_mm']}\t{k['vias']}\t{k['ripups']}"
except (OSError, KeyError):
    outline, route = "-", "-\t-\t-\t-"
try:
    x = json.load(open(f"{out}/{t}/8-extract.json"))
    f = ",".join(m["message"].split(":")[0].strip("`") + "=" + m["message"].split("bound ")[1].split(" ")[0]
                 for m in x.get("findings", []))
except (OSError, ValueError, IndexError):
    f = "-"
print(f"{t}\t{gates}\t{outline}\t{route}\t{emit}\t{f or 'none'}")
PY
done
