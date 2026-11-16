#!/usr/bin/env bash
# container/check-gcc-source.sh [PIN [MANIFEST]]: the pin of the GCC source
# mirror (AET-WO-4403; 4400a D-2, ruled by Scotty 2026-11-15). The mirror is an
# OCI artifact in the image's own package, so PIN (default container/GCC-SOURCE)
# has ARDUPILOT-IMAGE's form and goes through check-ardupilot-pin.sh first.
# MANIFEST (default container/gcc-source.manifest.json) is the manifest pushed
# byte for byte: PIN's digest must be its sha256, and it must have the one shape
# gcc-source.sh relies on (the empty config, one layer, Arm's URL).
#
# On success prints, tab-separated: the pin, the layer's sha256 (hex), its size,
# Arm's URL and the file name. Otherwise the refusal goes to stderr, naming the
# file, with exit 1 (INV-06). test-check-gcc-source.sh holds the fixtures.
set -euo pipefail
export LC_ALL=C
here=$(dirname "$0")
pinf=${1:-container/GCC-SOURCE}
man=${2:-container/gcc-source.manifest.json}
pin=$(bash "$here/check-ardupilot-pin.sh" "$pinf")
{ [ -f "$man" ] && [ -r "$man" ]; } || { echo "$man missing or unreadable" >&2; exit 1; }
want=${pin#*@sha256:}
got=$(sha256sum < "$man" | cut -d' ' -f1)
[ "$got" = "$want" ] || { echo "$pinf names sha256:$want but $man is sha256:$got" >&2; exit 1; }
out=$(python3 -I - "$man" <<'PY'
import json, re, sys
path = sys.argv[1]
def refuse(why):
    print(f"{path} {why}", file=sys.stderr)
    sys.exit(1)
def no_dupes(pairs):
    keys = [k for k, _ in pairs]
    if len(keys) != len(set(keys)):
        refuse("repeats a key")
    return dict(pairs)
try:
    with open(path, "rb") as f:
        m = json.loads(f.read().decode("utf-8"), object_pairs_hook=no_dupes)
except (UnicodeDecodeError, ValueError):
    refuse("is not JSON")
EMPTY = {"mediaType": "application/vnd.oci.empty.v1+json",
         "digest": "sha256:44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a",
         "size": 2}
ARM = "https://developer.arm.com/-/media/Files/downloads/gnu-rm/"
if not isinstance(m, dict) or set(m) != {"schemaVersion", "mediaType", "artifactType", "config", "layers", "annotations"}:
    refuse("does not have exactly the keys of an OCI artifact manifest")
if m["schemaVersion"] != 2 or m["mediaType"] != "application/vnd.oci.image.manifest.v1+json":
    refuse("is not an OCI image manifest, schema 2")
if m["artifactType"] != "application/vnd.clearforge.aethelgard.corresponding-source.v1":
    refuse("has another artifactType")
if m["config"] != EMPTY:
    refuse("has a config other than the empty descriptor")
layers = m["layers"]
if not isinstance(layers, list) or len(layers) != 1 or not isinstance(layers[0], dict):
    refuse("does not have exactly one layer")
l = layers[0]
if set(l) != {"mediaType", "digest", "size", "annotations"} or l["mediaType"] != "application/octet-stream":
    refuse("has a layer that is not one application/octet-stream file")
d, size, ann = l["digest"], l["size"], l["annotations"]
if not isinstance(d, str) or not re.fullmatch(r"sha256:[0-9a-f]{64}", d):
    refuse("has a layer digest that is not sha256:<64 hex>")
if type(size) is not int or size <= 0:
    refuse("has a layer size that is not a positive integer")
if not isinstance(ann, dict) or set(ann) != {"org.opencontainers.image.title"}:
    refuse("has a layer without exactly its title annotation")
title = ann["org.opencontainers.image.title"]
if not isinstance(title, str) or not re.fullmatch(r"gcc-arm-none-eabi-[0-9a-z-]+-src\.tar\.bz2", title):
    refuse("has a layer title that is not an Arm GCC source tarball")
top = m["annotations"]
if not isinstance(top, dict) or set(top) != {"org.opencontainers.image.description", "org.opencontainers.image.url", "org.opencontainers.image.source"}:
    refuse("does not have exactly the three annotations")
url = top["org.opencontainers.image.url"]
if not isinstance(url, str) or not url.startswith(ARM) or not url.endswith("/" + title) or not re.fullmatch(r"[A-Za-z0-9:/._-]+", url):
    refuse("has a URL that is not the title on Arm's download site")
if top["org.opencontainers.image.source"] != "https://github.com/ClearForge-LLC/aethelgard":
    refuse("names another source repository")
print(f"{d[7:]}\t{size}\t{url}\t{title}")
PY
)
printf '%s\t%s\n' "$pin" "$out"
