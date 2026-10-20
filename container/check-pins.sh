#!/usr/bin/env bash
# container/check-pins.sh: refuses a Dockerfile that can drift (INV-06). It
# accepts only known-safe shapes rather than hunting for bad ones:
#   - every FROM, in any case or indent, names image@sha256:<64 hex>, and a
#     COPY --from names a build stage or a digest-pinned image;
#   - every curl is exactly `curl -fsSLo PATH https://URL`, and the next line is
#     `&& echo "<64 hex>  PATH" | sha256sum -c -`, the only pipe allowed;
#   - apt runs only as `apt-get update` and one `apt-get install -y
#     --no-install-recommends` block whose every package is name=version;
#   - `cargo install` is --locked with a --version, and rustup installs a
#     numbered toolchain;
#   - nothing is swallowed with `||`, and the common fetchers outside the curl
#     shape (wget, pip, npm, git clone, ADD <url>) are refused. That last list
#     is a deny-list, so a deliberate fetch another way (python, git fetch) is
#     left to review; this script stops accidental drift, not an attacker.
# Needs no container runtime.
#
#   bash container/check-pins.sh [container/Dockerfile]
set -euo pipefail

file=${1:-container/Dockerfile}
[ -f "$file" ] || { echo "check-pins: $file not found" >&2; exit 1; }
mapfile -t lines < "$file"

hex='[0-9a-f]{64}'
bad=0
from_seen=0
in_apt=0
refuse() {
  echo "check-pins: line $(($1 + 1)): $2" >&2
  bad=1
}

for i in "${!lines[@]}"; do
  line=${lines[$i]}
  [[ "$line" =~ ^[[:space:]]*(#|$) ]] && continue

  # Inside the apt install block: one name=version per line until the cleanup.
  if [ "$in_apt" -eq 1 ]; then
    if [[ "$line" =~ ^[[:space:]]+\&\&\ rm\ -rf\ /var/lib/apt/lists/\*$ ]]; then
      in_apt=0
    elif ! [[ "$line" =~ ^[[:space:]]+[a-z0-9][a-z0-9.+-]*=[0-9][^[:space:]=*]*\ \\$ ]]; then
      refuse "$i" "apt package without an exact version: $line"
    fi
    continue
  fi

  if [[ "$line" == *"||"* ]]; then refuse "$i" "'||' would swallow a failure"; fi

  if [[ "${line,,}" =~ ^[[:space:]]*from[[:space:]] ]]; then
    from_seen=1
    read -ra words <<< "$line"
    image=
    for word in "${words[@]:1}"; do
      [[ "$word" == --* ]] || { image=$word; break; }
    done
    [[ "$image" =~ ^[^@[:space:]]+@sha256:${hex}$ ]] || refuse "$i" "unpinned base: $line"
  fi

  if [[ "$line" =~ --from=([^[:space:]]+) ]]; then
    ref=${BASH_REMATCH[1]}
    if [[ "$ref" =~ [:/@.] ]] && ! [[ "$ref" =~ @sha256:${hex}$ ]]; then
      refuse "$i" "COPY --from an unpinned image: $ref"
    fi
  fi

  if [[ "$line" =~ (^|[^a-z-])(wget|pip3?|npm|git[[:space:]]+clone)([^a-z-]|$) ]] || [[ "${line,,}" =~ ^[[:space:]]*add[[:space:]].*:// ]]; then
    refuse "$i" "fetches outside the checked curl shape: $line"
  fi

  if [[ "$line" == *curl* ]]; then
    if [[ "$line" =~ ^[[:space:]]*(RUN|\&\&)\ curl\ -fsSLo\ (/[^[:space:]]+)\ https://[^[:space:]]+\ \\$ ]]; then
      out=${BASH_REMATCH[2]}
      next=${lines[$((i + 1))]:-}
      [[ "$next" =~ ^[[:space:]]*\&\&\ echo\ \"${hex}\ \ ${out//./\\.}\"\ \|\ sha256sum\ -c\ -\ \\$ ]] \
        || refuse "$i" "download not checked by the next line: $out"
    else
      refuse "$i" "curl not in the checked shape: $line"
    fi
  elif [[ "$line" == *"|"* ]] && ! [[ "$line" =~ ^[[:space:]]*\&\&\ echo\ \"${hex}\ \ /[^[:space:]]+\"\ \|\ sha256sum\ -c\ -\ \\$ ]]; then
    refuse "$i" "pipe other than a sha256 check: $line"
  fi

  if [[ "$line" =~ (^|[^a-z-])apt(-get)?[[:space:]] ]]; then
    if [[ "$line" =~ ^[[:space:]]*\&\&\ apt-get\ install\ -y\ --no-install-recommends\ \\$ ]]; then
      in_apt=1
    elif ! [[ "$line" =~ ^RUN\ apt-get\ update\ \\$ ]]; then
      refuse "$i" "apt used outside the pinned install block: $line"
    fi
  fi

  if [[ "$line" == *"cargo install"* ]] && ! [[ "$line" =~ cargo\ install\ --locked\ .*--version\ [0-9] ]]; then
    refuse "$i" "cargo install without --locked and a --version: $line"
  fi
  if [[ "$line" == *"toolchain"* ]] && [[ "$line" =~ (--default-toolchain|toolchain\ install)\ ([^[:space:]]+) ]] \
    && ! [[ "${BASH_REMATCH[2]}" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    refuse "$i" "Rust toolchain not pinned to a numbered release: $line"
  fi
done

[ "$from_seen" -eq 1 ] || refuse -1 "no FROM line"
[ "$in_apt" -eq 0 ] || refuse -1 "apt install block never closed"
[ "$bad" -eq 0 ] || exit 1
echo "check-pins: $file ok"
