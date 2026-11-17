#!/usr/bin/env bash
# AET-WO-1313b: order independence of each method, emit only (no KiCad), against the method's own
# unrenamed signature (sig.py): 10 seeded shuffles of the parts inside the emitter
# (AET_1313B_SHUFFLE), the canonical key's last tie-break reversed (AET_1313B_REVTIE), and the 20
# order-only renamings (rename_perm.py). Usage, from the scratch tree with placer.patch applied:
#   spike/1313b/independence.sh <spec.json> <outdir> <method>...     ("-" is main's placer)
set -u
F=$1; T=$(realpath -m "$2"); shift 2
HERE=$(dirname "$(realpath "$0")"); BIN=${AET_BIN:-$(pwd)/target/release/aethelgard}
mkdir -p "$T"
for s in $(seq 0 19); do python3 -I "$HERE/rename_perm.py" "$F" "$s" "$T/perm-$s.json"; done
sig() { AET_1313B=$1 AET_1313B_DUMP="$T/x.json" "${@:2}" "$BIN" emit --file "$F2" --out "$T/e" >/dev/null 2>&1; python3 -I "$HERE/sig.py" "$T/x.json"; }
for m in "$@"; do
  M=$m; [ "$m" = - ] && M=
  F2=$F; base=$(sig "$M" env)
  sh=0; for k in $(seq 1 10); do [ "$(sig "$M" env AET_1313B_SHUFFLE=$k)" = "$base" ] && sh=$((sh+1)); done
  rt=$([ "$(sig "$M" env AET_1313B_REVTIE=1)" = "$base" ] && echo same || echo DIFFERS)
  pm=0; for s in $(seq 0 19); do F2=$T/perm-$s.json; [ "$(sig "$M" env)" = "$base" ] && pm=$((pm+1)); done; F2=$F
  sig "$M" env > /dev/null
  o=$(python3 -I -c 'import json,sys; d=json.load(open(sys.argv[1])); o=d["outline"]; print("%.2f x %.2f"%((o[2]-o[0])/1e6,(o[3]-o[1])/1e6))' "$T/x.json")
  echo "$m $base | $o | shuffled $sh/10 | tie reversed $rt | order-only renamings $pm/20"
done
