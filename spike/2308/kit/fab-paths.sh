#!/usr/bin/env bash
# AET-WO-2308: every committed HSE specification and every committed board that
# draws a twin stand-in, under `verify --fab-out` with an empty PATH (so the run
# stops at its first pre-gate refusal or at the first missing tool), with the base
# binary and the branch binary. Prints each verdict line's kind, id and exit, then
# whether base and branch print the same line.
# Usage: fab-paths.sh <base aethelgard> <branch aethelgard> <repo> <scratch>
set -u
base=$1 branch=$2 repo=$3 work=$4
rm -rf "$work"; mkdir -p "$work"
cases=(
  "crates/aethelgard-cli/tests/fixtures/hse-crystal-16.json|"
  "crates/aethelgard-cli/tests/fixtures/hse-external-8.json|"
  "examples/sentinel-02-bench-hse8.json|"
  "crates/aethelgard-ir/tests/fixtures/sentinel-02-power.json|scenarios/sentinel-02-power"
  "examples/sentinel-01.json|scenarios/sentinel-01-bench"
)
bad=0
for c in "${cases[@]}"; do
  spec=${c%%|*} scen=${c#*|}
  d="$work/$(basename "$spec" .json)"; mkdir -p "$d"
  cp "$repo/$spec" "$d/spec.json"
  if [ -z "$scen" ]; then
    printf '{"schema_version":1,"name":"baseline","base":"spec.json","expect":"pass","expect_findings":[],"overrides":[]}' > "$d/s0.json"
  else
    i=0
    for f in "$repo/$scen"/*.json; do
      python3 -I -c 'import json,sys; v=json.load(open(sys.argv[1])); v["base"]="spec.json"; json.dump(v,open(sys.argv[2],"w"))' "$f" "$d/s$i.json"
      i=$((i+1))
    done
  fi
  for who in base branch; do
    bin=$base; [ "$who" = branch ] && bin=$branch
    (cd "$d" && env PATH= "$bin" verify --spec spec.json --scenarios s*.json --fab-out "pkg-$who" > "$who.out" 2>/dev/null; echo $? > "$who.exit")
  done
  same=same; cmp -s "$d/base.out" "$d/branch.out" || { same=DIFFERS; bad=1; }
  for who in base branch; do
    printf '%-28s %-6s exit %s %s\n' "$(basename "$spec")" "$who" "$(cat "$d/$who.exit")" \
      "$(python3 -I -c 'import json,sys; v=json.load(open(sys.argv[1])); e=v.get("error",{}); print(e.get("kind"), e.get("id"))' "$d/$who.out")"
  done
  echo "  -> $same"
done
exit $bad
