#!/usr/bin/env bash
# container/gcc-source.sh: the mirror of Arm's GCC source next to the ArduPilot
# image (AET-WO-4403; 4400a D-2, ruled by Scotty 2026-11-15: mirror the exact
# corresponding-source tarball by sha256, do not rely on Arm's host). The pin
# and the manifest come from check-gcc-source.sh; nothing here takes a digest
# from the registry it judges (INV-07).
#
#   fetch REF@sha256:D DIR [--any]
#       Anonymous: curl reads no netrc and no docker config, and sends only the
#       anonymous token the registry's own challenge hands out. Fetches the
#       manifest by digest and refuses it unless its sha256 is D. Without
#       --any, D is the pin, so the manifest is container/gcc-source.manifest.json
#       byte for byte (check-gcc-source.sh), and the tarball is fetched into DIR
#       and refused unless its sha256 and size are the manifest's. Exit 2 when the manifest is not there (404);
#       1 for every other refusal.
#   oras DIR        oras ORAS_VERSION into DIR, refused unless its sha256 is
#                   ORAS_SHA256; prints the binary's path.
#   publish REPO DIR
#                   fetch the pin anonymously. Only if it is not there (exit
#                   2): download Arm's tarball from the manifest's URL, refuse
#                   it unless its sha256 and size are the manifest's, push the
#                   empty config, the tarball and the manifest with oras (its
#                   docker credentials) under the tag TAG, refuse unless the
#                   digest oras reports is the pin's, and fetch it anonymously
#                   again.
#
# A REPO or REF whose registry is localhost:PORT or 127.0.0.1:PORT is spoken
# to over plain HTTP (the pull request rehearsal); every other one over HTTPS.
set -euo pipefail
export LC_ALL=C
here=$(dirname "$0")
ORAS_VERSION=1.3.6
ORAS_SHA256=66fb0ad7206f3f6fa77ed5b552049d364ccf4b50b1684554f731f76302b52ca0
MANIFEST=$here/gcc-source.manifest.json
MT_MANIFEST=application/vnd.oci.image.manifest.v1+json

die() { echo "gcc-source: $*" >&2; exit 1; }
IFS=$'\t' read -r PIN LAYER SIZE ARM_URL TITLE < <(bash "$here/check-gcc-source.sh" "$here/GCC-SOURCE" "$MANIFEST") \
  || die "the pin or the manifest is refused (above)"
[ -n "${TITLE:-}" ] || die "check-gcc-source.sh printed nothing"
TAG=${TITLE%.tar.bz2}

local_registry() { [[ ${1%%/*} =~ ^(localhost|127\.0\.0\.1):[0-9]+$ ]]; }

# file_ok FILE SHA256 SIZE: the file's bytes are exactly these.
file_ok() {
  [ -f "$1" ] && [ "$(stat -c %s "$1")" = "$3" ] && [ "$(sha256sum < "$1" | cut -d' ' -f1)" = "$2" ]
}

fetch() {
  local ref=$1 dir=$2 any=${3:-}
  [[ $ref =~ ^([a-z0-9.-]+(:[0-9]+)?)/([a-z0-9._/-]+)@sha256:([0-9a-f]{64})$ ]] \
    || die "not REGISTRY/REPOSITORY@sha256:<64 hex>: $ref"
  local reg=${BASH_REMATCH[1]} repo=${BASH_REMATCH[3]} hex=${BASH_REMATCH[4]}
  case $any in
    --any) ;;
    '') [ "$hex" = "${PIN#*@sha256:}" ] || die "$ref is not the pin's digest; fetch another image with --any" ;;
    *) die "unknown option: $any" ;;
  esac
  local scheme=https proto='=https'
  if local_registry "$ref"; then scheme=http proto='=http'; fi
  # A redirect (ghcr sends blobs to its CDN) keeps the scheme, and curl drops
  # the token when the host changes.
  local base=$scheme://$reg/v2/$repo tok='' code hdrs=$dir/.headers
  mkdir -p "$dir"
  local accept='application/vnd.oci.image.manifest.v1+json, application/vnd.oci.image.index.v1+json, application/vnd.docker.distribution.manifest.v2+json, application/vnd.docker.distribution.manifest.list.v2+json'
  get() { # URL OUT: no netrc, no config file; the token only once one was issued.
    local auth=()
    [ -z "$tok" ] || auth=(-H "Authorization: Bearer $tok")
    curl -q -sS -L --proto "$proto" --proto-redir "$proto" --max-time 600 -D "$hdrs" -o "$2" \
      -w '%{http_code}' -H "Accept: $accept" "${auth[@]}" "$1"
  }
  code=$(get "$base/manifests/sha256:$hex" "$dir/manifest.json") || die "manifest request failed: $ref"
  if [ "$code" = 401 ]; then
    # The registry's Bearer challenge names where an anonymous token comes from.
    local ch realm service scope tcode
    ch=$(grep -i '^www-authenticate: *bearer ' "$hdrs" | tail -1 | tr -d '\r') || die "401 without a Bearer challenge: $ref"
    realm=$(sed -n 's/.*realm="\([^"]*\)".*/\1/ip' <<< "$ch")
    service=$(sed -n 's/.*service="\([^"]*\)".*/\1/ip' <<< "$ch")
    scope=$(sed -n 's/.*scope="\([^"]*\)".*/\1/ip' <<< "$ch")
    # belt-and-suspenders: the token request's --proto '=https' also refuses
    # it; this names the refusal.
    [[ $realm =~ ^https:// ]] || die "the token realm is not HTTPS: $realm"
    tcode=$(curl -q -sS --proto '=https' --max-time 60 -o "$dir/.token" -w '%{http_code}' -G \
      --data-urlencode "service=$service" --data-urlencode "scope=$scope" "$realm") || die "token request failed: $realm"
    [ "$tcode" = 200 ] || die "anonymous pull refused: the token endpoint answered HTTP $tcode for $ref"
    tok=$(python3 -I -c 'import json,sys; d=json.load(open(sys.argv[1])); print(d.get("token") or d.get("access_token") or "")' "$dir/.token")
    rm -f "$dir/.token"
    [ -n "$tok" ] || die "anonymous pull refused: no token for $ref"
    code=$(get "$base/manifests/sha256:$hex" "$dir/manifest.json") || die "manifest request failed: $ref"
  fi
  case $code in
    200) ;;
    404) echo "gcc-source: not there (HTTP 404): $ref" >&2; exit 2 ;;
    *) die "anonymous pull refused: manifest HTTP $code for $ref" ;;
  esac
  [ "$(sha256sum < "$dir/manifest.json" | cut -d' ' -f1)" = "$hex" ] || die "the manifest served for $ref is not sha256:$hex"
  echo "manifest sha256:$hex fetched anonymously from $reg/$repo"
  if [ "$any" = --any ]; then
    # Any image: its config blob too, so the blob route (ghcr's redirect to
    # its CDN) is exercised as well as the manifest's.
    local cd cs
    read -r cd cs < <(python3 -I -c 'import json,sys; c=json.load(open(sys.argv[1])).get("config") or {}; print(c.get("digest",""), c.get("size",""))' "$dir/manifest.json")
    [[ $cd =~ ^sha256:[0-9a-f]{64}$ && $cs =~ ^[0-9]+$ ]] || die "the manifest at $ref names no config blob"
    code=$(get "$base/blobs/$cd" "$dir/config") || die "blob request failed: $ref"
    [ "$code" = 200 ] || die "anonymous pull refused: blob HTTP $code for $ref"
    file_ok "$dir/config" "${cd#sha256:}" "$cs" || die "the config blob served for $ref is not $cd"
    rm -f "$hdrs"
    echo "config $cd ($cs bytes) fetched anonymously"
    return 0
  fi
  code=$(get "$base/blobs/sha256:$LAYER" "$dir/$TITLE") || die "blob request failed: $ref"
  [ "$code" = 200 ] || die "anonymous pull refused: blob HTTP $code for $ref"
  file_ok "$dir/$TITLE" "$LAYER" "$SIZE" || die "the blob served for $ref is not sha256:$LAYER, $SIZE bytes"
  rm -f "$hdrs"
  echo "$TITLE sha256:$LAYER ($SIZE bytes) fetched anonymously"
}

arm() {
  curl -q -fsSL --proto '=https' --proto-redir '=https' --max-time 900 -o "$1" "$ARM_URL" || die "download failed: $ARM_URL"
  file_ok "$1" "$LAYER" "$SIZE" || { rm -f "$1"; die "$ARM_URL is not sha256:$LAYER, $SIZE bytes; refused and deleted"; }
  echo "$TITLE from Arm: sha256:$LAYER ($SIZE bytes)"
}

get_oras() {
  local dir=$1 tgz=$1/oras.tar.gz
  mkdir -p "$dir"
  curl -q -fsSL --proto '=https' --proto-redir '=https' -o "$tgz" \
    "https://github.com/oras-project/oras/releases/download/v$ORAS_VERSION/oras_${ORAS_VERSION}_linux_amd64.tar.gz"
  [ "$(sha256sum < "$tgz" | cut -d' ' -f1)" = "$ORAS_SHA256" ] || die "oras $ORAS_VERSION is not sha256:$ORAS_SHA256"
  tar -xzf "$tgz" -C "$dir" oras
  rm "$tgz"
  echo "$dir/oras"
}

push() {
  local repo=$1 file=$2 oras=${ORAS:-oras} plain=() out digest
  [[ $repo =~ ^[a-z0-9.-]+(:[0-9]+)?/[a-z0-9._/-]+$ ]] || die "not REGISTRY/REPOSITORY: $repo"
  local_registry "$repo" && plain=(--plain-http)
  file_ok "$file" "$LAYER" "$SIZE" || die "$file is not sha256:$LAYER, $SIZE bytes"
  local tmp; tmp=$(mktemp -d)
  printf '{}' > "$tmp/config"
  "$oras" blob push "${plain[@]}" "$repo" "$tmp/config"
  "$oras" blob push "${plain[@]}" "$repo@sha256:$LAYER" "$file"
  out=$("$oras" manifest push "${plain[@]}" --media-type "$MT_MANIFEST" "$repo:$TAG" "$MANIFEST")
  rm -r "$tmp"
  echo "$out"
  digest=$(sed -n 's/^Digest: *//p' <<< "$out")
  [ "$digest" = "sha256:${PIN#*@sha256:}" ] || die "oras pushed $digest, not the pin's sha256:${PIN#*@sha256:}"
  echo "pushed $repo:$TAG at $digest"
}

cmd=${1:-}
case $cmd in
  fetch) [ $# -ge 3 ] && [ $# -le 4 ] || die "usage: fetch REF@sha256:D DIR [--any]"; fetch "$2" "$3" "${4:-}" ;;
  oras) [ $# -eq 2 ] || die "usage: oras DIR"; get_oras "$2" ;;
  publish)
    [ $# -eq 3 ] || die "usage: publish REPO DIR"
    ref=$2@sha256:${PIN#*@sha256:}
    rc=0; (fetch "$ref" "$3") || rc=$?
    case $rc in
      0) echo "already mirrored: $ref" ;;
      2) arm "$3/$TITLE"; push "$2" "$3/$TITLE"; rm -f "$3/$TITLE"; fetch "$ref" "$3" ;;
      *) die "the mirror is there but refused (above); nothing pushed" ;;
    esac ;;
  *) die "usage: gcc-source.sh fetch|oras|publish ..." ;;
esac
