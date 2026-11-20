#!/bin/bash
# AET-WO-2307 §2: every other refusal's bytes unchanged. Runs the base CLI and this branch's CLI on
# the same refusing inputs and compares stdout and exit byte for byte. Only
# UnwitnessedFieldNotAcknowledged may differ.
# Usage: spike/2307/differential.sh <base aethelgard> <branch aethelgard>   -> spike/2307/out/differential.txt
set -u
cd "$(dirname "$0")/../.."
B=$(realpath "$1") N=$(realpath "$2")
S=$HOME/.cache/aet-2307/diff; rm -rf "$S"; mkdir -p "$S/w"
SEN=$PWD/crates/aethelgard-cli/tests/fixtures/sentinel-03.json
REF=$PWD/crates/aethelgard-cli/tests/fixtures/four-layer.json
mk() { python3 -I -c "$1" "$SEN" "$S/w" "$REF"; }
# Inputs, each a scratch spec beside a scenario re-based on it, in $S/w/<case>/.
mk '
import json, sys, os, copy
sen, w = json.load(open(sys.argv[1])), sys.argv[2]
def case(name, spec, scen=None, raw=None):
    d = f"{w}/{name}"; os.makedirs(d + "/scen")
    if raw is not None: open(d + "/spec.json", "w").write(raw)
    else: json.dump(spec, open(d + "/spec.json", "w"), indent=2)
    sc = scen or {"schema_version": 3, "name": "baseline", "base": "../spec.json", "expect": "pass", "expect_findings": [], "overrides": []}
    json.dump(sc, open(d + "/scen/baseline.json", "w"))
s = copy.deepcopy(sen); s["schema_version"] = 16; case("unsupported-schema", s)
case("not-json", None, raw="{")
s = copy.deepcopy(sen); s.pop("board"); case("schema-violation", s)
s = copy.deepcopy(sen); s["acknowledge_unwitnessed"][0]["fields"].append("NOSUCHFIELD"); case("stale", s)
s = copy.deepcopy(sen); s["acknowledge_unwitnessed"][0]["reason"] = ""; case("invalid-source", s)
s = copy.deepcopy(sen); s["acknowledge_unwitnessed"].append(copy.deepcopy(s["acknowledge_unwitnessed"][0])); case("acknowledged-twice", s)
s = copy.deepcopy(sen); case("coverage-short", s)
s = copy.deepcopy(sen); case("base-mismatch", s, {"schema_version": 3, "name": "baseline", "base": "../other.json", "expect": "pass", "expect_findings": [], "overrides": []})
json.dump(sen, open(f"{w}/base-mismatch/other.json", "w"))
s = copy.deepcopy(sen); case("base-outside-root", s, {"schema_version": 3, "name": "baseline", "base": "/etc/hostname", "expect": "pass", "expect_findings": [], "overrides": []})
'
cmpone() { # cmpone <case> <args...>: run both binaries in $S/w/<case>, compare stdout and exit
  local c=$1; shift
  (cd "$S/w/$c" && "$B" "$@" > "$S/$c.base.out"; echo $? > "$S/$c.base.rc")
  (cd "$S/w/$c" && "$N" "$@" > "$S/$c.new.out"; echo $? > "$S/$c.new.rc")
  local kind=$(python3 -I -c "import json,sys;print(json.load(open(sys.argv[1]))['error']['kind'])" "$S/$c.base.out" 2>/dev/null || echo "-")
  if cmp -s "$S/$c.base.out" "$S/$c.new.out" && cmp -s "$S/$c.base.rc" "$S/$c.new.rc"; then r=same; else r=DIFFERS; fi
  printf '%s\t%s\texit %s\t%s\t%s\n' "$c" "$*" "$(cat "$S/$c.base.rc")" "$kind" "$r"
}
{
for c in unsupported-schema not-json schema-violation stale invalid-source acknowledged-twice base-mismatch base-outside-root; do
  cmpone "$c" verify --spec spec.json --scenarios scen/baseline.json --fab-out ../fab-$c
done
# Coverage short: Sentinel-03 with one baseline only (no fault anywhere).
cmpone coverage-short verify --spec spec.json --scenarios scen/baseline.json --fab-out ../fab-coverage
# validate on the same refusing inputs (main.rs's Refusal, untouched).
for c in unsupported-schema schema-violation stale invalid-source acknowledged-twice; do
  cmpone "$c" validate --file spec.json
done
# Usage errors.
cmpone stale verify --spec spec.json --fab-out ../fab-usage
cmpone stale verify
# And the one kind that must differ.
mkdir -p "$S/w/unack/scen"; cp scenarios/sentinel-03/*.json "$S/w/unack/scen/"
cp "$SEN" "$S/w/unack/spec.json"; sed -i 's|"base": "../../crates/aethelgard-cli/tests/fixtures/sentinel-03.json"|"base": "../spec.json"|' "$S/w/unack/scen/"*.json
(cd "$S/w/unack" && cmpone unack verify --spec spec.json --scenarios scen/*.json --fab-out ../fab-unack)
} | tee spike/2307/out/differential.txt
ls "$S"/fab-* 2>/dev/null && echo "A PACKAGE WAS WRITTEN"
rm -rf "$S"
