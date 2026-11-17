#!/usr/bin/env bash
# container/gcc-source.sh: the mirror of Arm's GCC source next to the ArduPilot
# image (AET-WO-4403; 4400a D-2, ruled by Scotty 2026-11-15: mirror the exact
# corresponding-source tarball by sha256, do not rely on Arm's host). The pin
# and the manifest come from check-gcc-source.sh; oci-mirror.sh does the
# registry work, anonymously for every fetch.
#
#   fetch REF@sha256:D DIR [--any]
#       Without --any, D must be the pin: the manifest served must be
#       container/gcc-source.manifest.json byte for byte, and the tarball is
#       fetched into DIR and refused unless its sha256 and size are the
#       manifest's. --any fetches any image's manifest and config instead.
#       Exit 2 when the manifest is not there (404); 1 for every other refusal.
#   oras DIR        oras into DIR, checked by sha256 (oci-mirror.sh oras).
#   publish REPO DIR
#       Fetch the pin anonymously. Only if it is not there (exit 2): take the
#       tarball from the published mirror (CANON, anonymously, verified), or,
#       only if that is not there either, from Arm's URL in the manifest,
#       refused unless its sha256 and size are the manifest's; push it with
#       oci-mirror.sh under the tag named after it, and fetch it anonymously
#       again. So once main has published the mirror, nothing asks Arm's host,
#       which answered HTTP 503 in a rehearsal on 2026-11-16 (4400a D-2).
set -euo pipefail
export LC_ALL=C
here=$(dirname "$0")
MANIFEST=$here/gcc-source.manifest.json

die() { echo "gcc-source: $*" >&2; exit 1; }
IFS=$'\t' read -r PIN LAYER SIZE ARM_URL TITLE < <(bash "$here/check-gcc-source.sh" "$here/GCC-SOURCE" "$MANIFEST") \
  || die "the pin or the manifest is refused (above)"
[ -n "${TITLE:-}" ] || die "check-gcc-source.sh printed nothing"
TAG=${TITLE%.tar.bz2}
oci() { bash "$here/oci-mirror.sh" "$@"; }
CANON=ghcr.io/clearforge-llc/aethelgard-ardupilot

fetch() {
  if [ "${3:-}" = --any ]; then oci fetch "$1" "$2" --any; else
    [ -z "${3:-}" ] || die "unknown option: $3"
    oci fetch "$1" "$2" --manifest "$MANIFEST"
  fi
}

arm() {
  curl -q -fsSL --proto '=https' --proto-redir '=https' --max-time 900 -o "$1/$TITLE" "$ARM_URL" || die "download failed: $ARM_URL"
  [ "$(stat -c %s "$1/$TITLE")" = "$SIZE" ] && [ "$(sha256sum < "$1/$TITLE" | cut -d' ' -f1)" = "$LAYER" ] \
    || { rm -f "$1/$TITLE"; die "$ARM_URL is not sha256:$LAYER, $SIZE bytes; refused and deleted"; }
  echo "$TITLE from Arm: sha256:$LAYER ($SIZE bytes)"
}

case ${1:-} in
  fetch) [ $# -ge 3 ] && [ $# -le 4 ] || die "usage: fetch REF@sha256:D DIR [--any]"; fetch "$2" "$3" "${4:-}" ;;
  oras) [ $# -eq 2 ] || die "usage: oras DIR"; oci oras "$2" ;;
  publish)
    [ $# -eq 3 ] || die "usage: publish REPO DIR"
    ref=$2@sha256:${PIN#*@sha256:}
    rc=0; fetch "$ref" "$3" || rc=$?
    case $rc in
      0) echo "already mirrored: $ref" ;;
      2) mkdir -p "$3"
         seed=1
         if [ "$2" != "$CANON" ]; then
           seed=0; fetch "$CANON@sha256:${PIN#*@sha256:}" "$3" || seed=$?
           [ "$seed" -ne 1 ] || die "the published mirror is there but refused (above); Arm not asked"
         fi
         case $seed in
           0) echo "$TITLE seeded from $CANON" ;;
           *) arm "$3" ;;
         esac
         oci push "$2" "$TAG" "$MANIFEST" "$3"; rm -f "$3/$TITLE"; fetch "$ref" "$3" ;;
      *) die "the mirror is there but refused (above); nothing pushed" ;;
    esac ;;
  *) die "usage: gcc-source.sh fetch|oras|publish ..." ;;
esac
