#!/usr/bin/env bash
# container/check-sources.sh: the written source offer of a public image
# (AET-WO-4403). The expected values are read from the image itself (the
# ArduPilot commit, the toolchain release, the KiCad and Renode versions, the
# dpkg database) and the document is judged against them, never the reverse.
#
#   check-sources.sh labels KIND IMAGE [REF]   on the host: the image's
#       licences label is KIND's, and its documentation label is this
#       repository's container/<KIND>-SOURCES.md at REF (default main).
#   check-sources.sh inside KIND               inside the image, with this
#       directory at /container: /SOURCES.md is container/<KIND>-SOURCES.md
#       byte for byte, /SOURCES-debian.tsv is the image's dpkg database, and
#       every version the image holds is the one its row names.
#
# KIND is ardupilot or tools. A refusal is printed by name, exit 1 (INV-06).
set -euo pipefail
export LC_ALL=C
die() { echo "check-sources: $*" >&2; exit 1; }
mode=${1:-}; kind=${2:-}
case $kind in
  ardupilot) doc=ARDUPILOT-SOURCES.md licences='GPL-3.0-or-later' ;;
  tools) doc=TOOLS-SOURCES.md licences='GPL-3.0-or-later AND CC-BY-SA-4.0' ;;
  *) die "usage: check-sources.sh labels|inside ardupilot|tools ..." ;;
esac

label() { "${DOCKER:-docker}" image inspect --format "{{index .Config.Labels \"org.opencontainers.image.$2\"}}" "$1"; }

if [ "$mode" = labels ]; then
  img=${3:?usage: check-sources.sh labels KIND IMAGE [REF]} ref=${4:-main}
  want=https://github.com/ClearForge-LLC/aethelgard/blob/$ref/container/$doc
  got=$(label "$img" documentation)
  [ "$got" = "$want" ] || die "$img: documentation label is [$got], not [$want]"
  got=$(label "$img" licenses)
  [ "$got" = "$licences" ] || die "$img: licenses label is [$got], not [$licences]"
  echo "ok   $img labels: documentation $want; licenses $licences"
  exit 0
fi
[ "$mode" = inside ] || die "usage: check-sources.sh labels|inside ardupilot|tools ..."

f=/SOURCES.md
[ -f "$f" ] || die "$f is missing from the image"
cmp -s "$f" "/container/$doc" || die "$f is not container/$doc byte for byte"
[ -f /SOURCES-debian.tsv ] || die "/SOURCES-debian.tsv is missing from the image"
dpkg-query -W -f '${binary:Package}\t${Version}\t${source:Package}\t${source:Version}\thttps://snapshot.debian.org/package/${source:Package}/${source:Version}/\n' \
  | sort | cmp -s - /SOURCES-debian.tsv || die "/SOURCES-debian.tsv is not this image's dpkg database"
echo "ok   /SOURCES.md is container/$doc; /SOURCES-debian.tsv lists all $(wc -l < /SOURCES-debian.tsv) dpkg packages"

# cell NAME COLUMN: that cell of the table row whose first cell is exactly NAME
# (columns: 1 component, 2 version in this image, 3 licence, 4 source).
cell() { awk -F'|' -v n="$1" -v c="$2" '/^\| /{k=$2; gsub(/^ +| +$/, "", k); if (k == n) print $(c + 1)}' "$f"; }
# need NAME COLUMN FACT: that cell exists and names FACT.
need() {
  local v
  v=$(cell "$1" "$2")
  [ -n "$v" ] || die "$f has no row for $1"
  [ "$(wc -l <<< "$v")" -eq 1 ] || die "$f has more than one row for $1"
  grep -qF -- "$3" <<< "$v" || die "$f's $1 row, column $2, does not name $3, which is what the image holds"
  echo "ok   $1: $3"
}

case $kind in
  ardupilot)
    need ArduPilot 2 "\`$(git -C /opt/ardupilot rev-parse HEAD)\`"
    rel=$(arm-none-eabi-gcc --version | head -n 1 | sed -n 's/.*(GNU Arm Embedded Toolchain \([^)]*\)).*/\1/p')
    [ -n "$rel" ] || die "arm-none-eabi-gcc names no toolchain release"
    need 'GNU Arm Embedded Toolchain' 2 "\`gcc-arm-none-eabi-$rel\`"
    IFS=$'\t' read -r pin layer _ < <(bash /container/check-gcc-source.sh /container/GCC-SOURCE /container/gcc-source.manifest.json)
    need 'GNU Arm Embedded Toolchain' 4 "\`$pin\`"
    need 'GNU Arm Embedded Toolchain' 4 "sha256 \`$layer\`"
    # Every Python distribution whose metadata says GPL has a row, with its version.
    while read -r name ver gpl; do
      if [ -n "$(cell "$name" 1)" ]; then
        need "$name" 2 "\`$ver\`"
      elif [ "$gpl" = gpl ]; then
        die "$f has no row for $name ($ver), whose metadata says GPL"
      fi
    done < <(/opt/venv/bin/python -I -c '
import importlib.metadata as m
for d in m.distributions():
    md = d.metadata
    text = " ".join([md.get("License") or "", md.get("License-Expression") or ""] + (md.get_all("Classifier") or []))
    print(md["Name"], md["Version"], "gpl" if "GPL" in text or "General Public License" in text else "-")')
    ;;
  tools)
    need KiCad 2 "\`$(kicad-cli version)\`"
    deps=/opt/renode/Renode.deps.json
    v() { python3 -I -c 'import json,sys; print(next(k.split("/")[1] for k in json.load(open(sys.argv[1]))["libraries"] if k.split("/")[0] == sys.argv[2]))' "$deps" "$1"; }
    need Renode 2 "\`$(v Renode)\`"
    need 'GTK#' 2 "\`$(v GtkSharp)\`"
    need NetMQ 2 "\`$(v NetMQ)\`"
    # Every licence Renode ships under a GNU GPL or LGPL header is named.
    for l in /opt/renode/licenses/*; do
      head -n 15 "$l" | grep -qiE 'GNU (LESSER |LIBRARY )?GENERAL PUBLIC LICENSE|LGPL licensed' || continue
      n=$(basename "$l" -license)
      grep -qF -- "$n" "$f" || die "$f does not name $n, which Renode ships under $(basename "$l")"
      echo "ok   Renode's $n (GNU licence) is named"
    done
    ;;
esac
