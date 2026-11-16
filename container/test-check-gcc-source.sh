#!/usr/bin/env bash
# container/test-check-gcc-source.sh: check-gcc-source.sh against its fixtures
# (AET-WO-4403). Each directory under container/fixtures/gcc-source/ holds a
# GCC-SOURCE, a manifest.json or both, and is named in the table below with
# the one refusal it must draw, exactly, on stderr, with exit 1 and nothing
# on stdout:
#   form       the pin's form, from check-ardupilot-pin.sh;
#   nopin      "DIR/GCC-SOURCE missing or unreadable";
#   mismatch   "DIR/GCC-SOURCE names sha256:<pin> but DIR/manifest.json is
#              sha256:<its hash>";
#   noman      "DIR/manifest.json missing or unreadable";
#   shape      "DIR/manifest.json <the reason in the table>".
# The committed pair must pass, and every fixture must be in the table. The
# check runs under en_US.UTF-8 when the host has it, as the pin check's test
# does.
#
#   bash container/test-check-gcc-source.sh [path/to/check-gcc-source.sh]
set -euo pipefail

check=${1:-container/check-gcc-source.sh}
dir=container/fixtures/gcc-source
form='ghcr.io/clearforge-llc/aethelgard-ardupilot@sha256:<64 hex>'
locales=$(locale -a 2>/dev/null || true)
loc=$(grep -ixm1 'en_US\.utf-\?8' <<< "$locales" || echo C.UTF-8)
echo "the check runs under LC_ALL=$loc"
fail=0
tmp=$(mktemp)
trap 'rm -f "$tmp"' EXIT

run() {
  local d=$dir/$1 verdict=$2 reason=$3 out err rc want pin
  set +e
  out=$(LC_ALL=$loc bash "$check" "$d/GCC-SOURCE" "$d/manifest.json" 2> "$tmp"); rc=$?
  set -e
  err=$(cat "$tmp")
  case $verdict in
    form) want="$d/GCC-SOURCE is not $form: $(tr -d '\000' < "$d/GCC-SOURCE")" ;;
    nopin) want="$d/GCC-SOURCE missing or unreadable" ;;
    mismatch)
      pin=$(head -n 1 "$d/GCC-SOURCE")
      want="$d/GCC-SOURCE names sha256:${pin#*@sha256:} but $d/manifest.json is sha256:$(sha256sum < "$d/manifest.json" | cut -d' ' -f1)" ;;
    noman) want="$d/manifest.json missing or unreadable" ;;
    shape) want="$d/manifest.json $reason" ;;
    *) echo "FAIL $1: unknown verdict $verdict"; fail=1; return ;;
  esac
  if [ "$rc" -eq 1 ] && [ -z "$out" ] && [ "$err" = "$want" ]; then
    echo "ok   $verdict $1"
  else
    echo "FAIL $1: want [$want], got exit $rc, stdout [${out:0:120}], stderr [${err:0:200}]"
    fail=1
  fi
}

# name                     verdict   reason (shape only)
table='
pin-uppercase              form
pin-tag                    form
pin-nul                    form
pin-other-repository       form
pin-missing                nopin
image-pin-pasted           mismatch
manifest-reformatted       mismatch
manifest-missing           noman
not-json                   shape     is not JSON
not-utf8                   shape     is not JSON
nul-in-string              shape     is not JSON
duplicate-key              shape     repeats a key
extra-key                  shape     does not have exactly the keys of an OCI artifact manifest
docker-manifest            shape     is not an OCI image manifest, schema 2
artifact-type-other        shape     has another artifactType
config-not-empty           shape     has a config other than the empty descriptor
two-layers                 shape     does not have exactly one layer
layer-mediatype-tar        shape     has a layer that is not one application/octet-stream file
layer-digest-uppercase     shape     has a layer digest that is not sha256:<64 hex>
layer-size-string          shape     has a layer size that is not a positive integer
layer-size-float           shape     has a layer size that is not a positive integer
layer-size-zero            shape     has a layer size that is not a positive integer
title-not-gcc              shape     has a layer title that is not an Arm GCC source tarball
layer-annotation-extra     shape     has a layer without exactly its title annotation
annotation-extra           shape     does not have exactly the three annotations
url-other-host             shape     has a URL that is not the title on Arm'"'"'s download site
url-other-file             shape     has a URL that is not the title on Arm'"'"'s download site
url-query                  shape     has a URL that is not the title on Arm'"'"'s download site
source-other-repository    shape     names another source repository
'

listed=()
while read -r name verdict reason; do
  [ -n "$name" ] || continue
  listed+=("$name")
  run "$name" "$verdict" "$reason"
done <<< "$table"

for f in "$dir"/*; do
  name=${f##*/}
  printf '%s\n' "${listed[@]}" | grep -qxF -- "$name" || { echo "FAIL $f is not in the table"; fail=1; }
done

if out=$(bash "$check") && [ "${out%%$'\t'*}" = "$(head -n 1 container/GCC-SOURCE)" ]; then
  echo "ok   container/GCC-SOURCE and container/gcc-source.manifest.json pass"
else
  echo "FAIL the committed pin and manifest are refused"; fail=1
fi
exit "$fail"
