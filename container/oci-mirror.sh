#!/usr/bin/env bash
# container/oci-mirror.sh: push and fetch the source mirrors that sit next to
# the public images (AET-WO-4403): Arm's GCC source (gcc-source.sh) and the
# GPLv2-only Debian sources (debian-src.py). Every artifact has a committed or
# derived manifest whose sha256 is its pin; nothing here takes a digest from
# the registry it judges (INV-07).
#
#   fetch REF@sha256:D DIR --manifest FILE [--head|--discard]
#       Anonymous: curl reads no .curlrc, no netrc and no docker config, and
#       sends only the anonymous token the registry's own challenge hands out.
#       FILE's sha256 must be D; the manifest served must be sha256 D; then
#       each layer FILE names is fetched into DIR/<title> and refused unless
#       its sha256 and size are FILE's. --head stops after the manifest;
#       --discard deletes each layer once it is verified.
#   fetch REF@sha256:D DIR --any
#       Any image: its manifest by digest and its config blob.
#   push REPO TAG FILE DIR
#       With oras (its docker credentials): the empty config, each layer from
#       DIR/<title> (refused unless its sha256 and size are FILE's), then FILE
#       under TAG. Refused if TAG already names another digest (a mirror's tag
#       is never moved) or if oras reports a digest other than FILE's sha256.
#   oras DIR        oras ORAS_VERSION into DIR, refused unless its sha256 is
#                   ORAS_SHA256; prints the binary's path.
#
# fetch exits 2 when the manifest is not there (404), 1 on every other
# refusal. A registry localhost:PORT or 127.0.0.1:PORT is spoken to over plain
# HTTP (the pull request rehearsal); every other one over HTTPS.
set -euo pipefail
export LC_ALL=C
ORAS_VERSION=1.3.6
ORAS_SHA256=66fb0ad7206f3f6fa77ed5b552049d364ccf4b50b1684554f731f76302b52ca0
MT_MANIFEST=application/vnd.oci.image.manifest.v1+json

die() { echo "oci-mirror: $*" >&2; exit 1; }
local_registry() { [[ ${1%%/*} =~ ^(localhost|127\.0\.0\.1):[0-9]+$ ]]; }
sha() { sha256sum < "$1" | cut -d' ' -f1; }
# file_ok FILE SHA256 SIZE: the file's bytes are exactly these.
file_ok() { [ -f "$1" ] && [ "$(stat -c %s "$1")" = "$3" ] && [ "$(sha "$1")" = "$2" ]; }
# layers FILE: "sha256hex size title" per layer; a title is a plain file name.
layers() {
  python3 -I -c '
import json, re, sys
for l in json.load(open(sys.argv[1]))["layers"]:
    d, n, t = l["digest"], l["size"], l["annotations"]["org.opencontainers.image.title"]
    if not (re.fullmatch(r"sha256:[0-9a-f]{64}", d) and type(n) is int and n > 0 and re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9.+~_-]*", t)):
        sys.exit("bad layer in " + sys.argv[1])
    print(d[7:], n, t)' "$1" || die "$1 names a layer that is not sha256, size and a plain file name"
}

fetch() {
  local ref=$1 dir=$2 mode=$3 man=${4:-} extra=${5:-}
  [[ $ref =~ ^([a-z0-9.-]+(:[0-9]+)?)/([a-z0-9._/-]+)@sha256:([0-9a-f]{64})$ ]] \
    || die "not REGISTRY/REPOSITORY@sha256:<64 hex>: $ref"
  local reg=${BASH_REMATCH[1]} repo=${BASH_REMATCH[3]} hex=${BASH_REMATCH[4]}
  case $mode in
    --any) ;;
    --manifest)
      [ -f "$man" ] || die "no manifest file: $man"
      [ "$(sha "$man")" = "$hex" ] || die "$man is not sha256:$hex, the digest asked for: $ref"
      case $extra in ''|--head|--discard) ;; *) die "unknown option: $extra" ;; esac ;;
    *) die "fetch needs --manifest FILE or --any" ;;
  esac
  local scheme=https proto='=https'
  if local_registry "$ref"; then scheme=http proto='=http'; fi
  # A redirect (ghcr sends blobs to its CDN) keeps the scheme, and curl drops
  # the token when the host changes.
  local base=$scheme://$reg/v2/$repo tok='' code hdrs=$dir/.headers got=$dir/.manifest
  mkdir -p "$dir"
  local accept='application/vnd.oci.image.manifest.v1+json, application/vnd.oci.image.index.v1+json, application/vnd.docker.distribution.manifest.v2+json, application/vnd.docker.distribution.manifest.list.v2+json'
  get() { # URL OUT: no .curlrc, no netrc; the token only once one was issued.
    local auth=()
    [ -z "$tok" ] || auth=(-H "Authorization: Bearer $tok")
    curl -q -sS -L --proto "$proto" --proto-redir "$proto" --max-time 900 -D "$hdrs" -o "$2" \
      -w '%{http_code}' -H "Accept: $accept" "${auth[@]}" "$1"
  }
  code=$(get "$base/manifests/sha256:$hex" "$got") || die "manifest request failed: $ref"
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
    code=$(get "$base/manifests/sha256:$hex" "$got") || die "manifest request failed: $ref"
  fi
  case $code in
    200) ;;
    404) rm -f "$hdrs" "$got"; echo "oci-mirror: not there (HTTP 404): $ref" >&2; exit 2 ;;
    *) die "anonymous pull refused: manifest HTTP $code for $ref" ;;
  esac
  [ "$(sha "$got")" = "$hex" ] || die "the manifest served for $ref is not sha256:$hex"
  echo "manifest sha256:$hex fetched anonymously from $reg/$repo"
  if [ "$mode" = --any ]; then
    # Its config blob too, so the blob route (ghcr's redirect to its CDN) is
    # exercised as well as the manifest's.
    local cd cs
    read -r cd cs < <(python3 -I -c 'import json,sys; c=json.load(open(sys.argv[1])).get("config") or {}; print(c.get("digest",""), c.get("size",""))' "$got")
    [[ $cd =~ ^sha256:[0-9a-f]{64}$ && $cs =~ ^[0-9]+$ ]] || die "the manifest at $ref names no config blob"
    code=$(get "$base/blobs/$cd" "$dir/config") || die "blob request failed: $ref"
    [ "$code" = 200 ] || die "anonymous pull refused: blob HTTP $code for $ref"
    file_ok "$dir/config" "${cd#sha256:}" "$cs" || die "the config blob served for $ref is not $cd"
    rm -f "$hdrs" "$got"
    echo "config $cd ($cs bytes) fetched anonymously"
    return 0
  fi
  rm -f "$got"
  [ "$extra" = --head ] && { rm -f "$hdrs"; return 0; }
  local h n t
  while read -r h n t; do
    code=$(get "$base/blobs/sha256:$h" "$dir/$t") || die "blob request failed: $ref $t"
    [ "$code" = 200 ] || die "anonymous pull refused: blob HTTP $code for $ref $t"
    file_ok "$dir/$t" "$h" "$n" || die "the blob served for $ref is not sha256:$h, $n bytes: $t"
    echo "$t sha256:$h ($n bytes) fetched anonymously"
    [ "$extra" = --discard ] && rm -f "$dir/$t"
  done < <(layers "$man")
  rm -f "$hdrs"
}

get_oras() {
  local dir=$1 tgz=$1/oras.tar.gz
  mkdir -p "$dir"
  curl -q -fsSL --proto '=https' --proto-redir '=https' -o "$tgz" \
    "https://github.com/oras-project/oras/releases/download/v$ORAS_VERSION/oras_${ORAS_VERSION}_linux_amd64.tar.gz"
  [ "$(sha "$tgz")" = "$ORAS_SHA256" ] || die "oras $ORAS_VERSION is not sha256:$ORAS_SHA256"
  tar -xzf "$tgz" -C "$dir" oras
  rm "$tgz"
  echo "$dir/oras"
}

push() {
  local repo=$1 tag=$2 man=$3 dir=$4 oras=${ORAS:-oras} plain=() out digest want h n t
  [[ $repo =~ ^[a-z0-9.-]+(:[0-9]+)?/[a-z0-9._/-]+$ ]] || die "not REGISTRY/REPOSITORY: $repo"
  [[ $tag =~ ^[A-Za-z0-9_][A-Za-z0-9._-]{0,127}$ ]] || die "not an OCI tag: $tag"
  local_registry "$repo" && plain=(--plain-http)
  want=sha256:$(sha "$man")
  if out=$("$oras" resolve "${plain[@]}" "$repo:$tag" 2>/dev/null); then
    [ "$out" = "$want" ] || die "$repo:$tag already names $out, not $want; a mirror's tag is never moved"
  fi
  while read -r h n t; do
    file_ok "$dir/$t" "$h" "$n" || die "$dir/$t is not sha256:$h, $n bytes"
  done < <(layers "$man")
  local tmp; tmp=$(mktemp -d)
  printf '{}' > "$tmp/config"
  "$oras" blob push "${plain[@]}" "$repo" "$tmp/config"
  rm -r "$tmp"
  while read -r h n t; do
    "$oras" blob push "${plain[@]}" "$repo@sha256:$h" "$dir/$t"
  done < <(layers "$man")
  out=$("$oras" manifest push "${plain[@]}" --media-type "$MT_MANIFEST" "$repo:$tag" "$man")
  echo "$out"
  digest=$(sed -n 's/^Digest: *//p' <<< "$out")
  [ "$digest" = "$want" ] || die "oras pushed $digest, not $want"
  echo "pushed $repo:$tag at $digest"
}

case ${1:-} in
  fetch) [ $# -ge 4 ] && [ $# -le 6 ] || die "usage: fetch REF@sha256:D DIR --manifest FILE [--head|--discard] | --any"
         if [ "$4" = --any ]; then [ $# -eq 4 ] || die "--any takes nothing more"; fetch "$2" "$3" --any
         else [ "$4" = --manifest ] && [ $# -ge 5 ] || die "fetch needs --manifest FILE or --any"; fetch "$2" "$3" --manifest "$5" "${6:-}"; fi ;;
  push) [ $# -eq 5 ] || die "usage: push REPO TAG FILE DIR"; push "$2" "$3" "$4" "$5" ;;
  oras) [ $# -eq 2 ] || die "usage: oras DIR"; get_oras "$2" ;;
  *) die "usage: oci-mirror.sh fetch|push|oras ..." ;;
esac
