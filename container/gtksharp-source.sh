#!/usr/bin/env bash
# container/gtksharp-source.sh: GtkSharp's source at commit 964cb9c, mirrored
# next to the tools image (AET-WO-4407; 4403 D4). GTK# 3.24.24.95 names no
# commit; 964cb9c is the one it was built from by inference (TOOLS-SOURCES.md),
# and this keeps that tree's bytes beside the image, as gcc-source.sh does for
# Arm's GCC. The pin is the sha256 of gtksharp-source.manifest.json, whose one
# layer is GitHub's archive of the commit; oci-mirror.sh does the registry
# work, anonymously for every fetch.
#
#   fetch REPO DIR    the pin from REPO into DIR, anonymously; exit 2 when it
#                     is not there (404), 1 for every other refusal.
#   publish REPO DIR  fetch; only if it is not there, download the archive
#                     from the manifest's url, push it (oci-mirror.sh refuses a
#                     file whose sha256 or size is not the manifest's), and
#                     fetch it anonymously again.
set -euo pipefail
export LC_ALL=C
here=$(dirname "$0")
MANIFEST=$here/gtksharp-source.manifest.json
die() { echo "gtksharp-source: $*" >&2; exit 1; }
read -r TITLE URL < <(python3 -I -c '
import json, re, sys
m = json.load(open(sys.argv[1]))
(l,) = m["layers"]
t, u = l["annotations"]["org.opencontainers.image.title"], m["annotations"]["org.opencontainers.image.url"]
c = re.fullmatch(r"https://github\.com/GtkSharp/GtkSharp/archive/([0-9a-f]{40})\.tar\.gz", u)
if not (c and t == "GtkSharp-" + c[1] + ".tar.gz"):
    sys.exit("not one GitHub archive of a GtkSharp commit")
print(t, u)' "$MANIFEST") || die "$MANIFEST is refused (above)"
PIN=$(sha256sum < "$MANIFEST" | cut -d' ' -f1)
oci() { bash "$here/oci-mirror.sh" "$@"; }
fetch() { oci fetch "$1@sha256:$PIN" "$2" --manifest "$MANIFEST"; }

[ $# -eq 3 ] || die "usage: gtksharp-source.sh fetch|publish REPO DIR"
case $1 in
  fetch) fetch "$2" "$3" ;;
  publish)
    rc=0; fetch "$2" "$3" || rc=$?
    case $rc in
      0) echo "already mirrored: $2@sha256:$PIN" ;;
      2) mkdir -p "$3"
         curl -q -fsSL --proto '=https' --proto-redir '=https' --max-time 300 -o "$3/$TITLE" "$URL" || die "download failed: $URL"
         oci push "$2" "${TITLE%.tar.gz}" "$MANIFEST" "$3"
         rm -f "$3/$TITLE"; fetch "$2" "$3" ;;
      *) die "the mirror is there but refused (above); nothing pushed" ;;
    esac ;;
  *) die "usage: gtksharp-source.sh fetch|publish REPO DIR" ;;
esac
