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
# One extension (AET-WO-1501): a Dockerfile may `FROM ${TOOLS_IMAGE}` when it
# declares `ARG TOOLS_IMAGE` with no default and container/IMAGE, read here, is
# image@sha256:<64 hex>; the workflow passes that file's content as the argument.
#
#   bash container/check-pins.sh [container/Dockerfile]
set -euo pipefail

file=${1:-container/Dockerfile}
image_file=$(dirname "$file")/IMAGE
[ -f "$file" ] || { echo "check-pins: $file not found" >&2; exit 1; }
mapfile -t lines < "$file"

hex='[0-9a-f]{64}'
bad=0
from_seen=0
arg_seen=0
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
  # Docker's /bin/sh -c has no -e: after `;` or a lone `&`, a failed check
  # above it no longer stops the build (AET-WO-4400a adversarial A3).
  if [[ "$line" == *";"* ]] || [[ "$line" =~ (^|[^&])\&([^&]|$) ]]; then
    refuse "$i" "';' or a lone '&' would let a failed check pass: $line"
  fi

  if [[ "${line,,}" =~ ^[[:space:]]*from[[:space:]] ]]; then
    from_seen=1
    read -ra words <<< "$line"
    image=
    for word in "${words[@]:1}"; do
      [[ "$word" == --* ]] || { image=$word; break; }
    done
    if [ "$image" = '${TOOLS_IMAGE}' ] && [ "$arg_seen" -eq 1 ]; then
      [ -f "$image_file" ] || refuse "$i" "$image_file not found: the base digest is unreadable"
      [ -f "$image_file" ] && ! [[ "$(<"$image_file")" =~ ^[^@[:space:]]+@sha256:${hex}$ ]] \
        && refuse "$i" "$image_file is not image@sha256:<64 hex>"
    else
      [[ "$image" =~ ^[^@[:space:]]+@sha256:${hex}$ ]] || refuse "$i" "unpinned base: $line"
    fi
  fi

  if [[ "$line" =~ ^[[:space:]]*[Aa][Rr][Gg][[:space:]] ]]; then
    if [ "${line//[[:space:]]/}" = "ARGTOOLS_IMAGE" ] && [ "$from_seen" -eq 0 ]; then
      arg_seen=1
    else
      refuse "$i" "ARG other than a bare 'ARG TOOLS_IMAGE' before FROM: $line"
    fi
  fi

  if [[ "$line" =~ --from=([^[:space:]]+) ]]; then
    ref=${BASH_REMATCH[1]}
    if [[ "$ref" =~ [:/@.] ]] && ! [[ "$ref" =~ @sha256:${hex}$ ]]; then
      refuse "$i" "COPY --from an unpinned image: $ref"
    fi
  fi

  # Two more fetch shapes (AET-WO-4400a), each checked as tightly as curl's:
  #   - `git clone --depth 1 -b REF --recurse-submodules --shallow-submodules
  #     https://URL DIR`, and the next line is `&& test "$(git -C DIR rev-parse
  #     HEAD)" = <40 hex>`: the commit, not the ref, is the pin;
  #   - `pip install --no-cache-dir --require-hashes --no-deps
  #     [--no-build-isolation] -r PATH`, where PATH is the target of a `COPY
  #     NAME PATH` in this file and NAME, beside this file, holds only
  #     `name==version --hash=sha256:<64 hex>...` lines (and comments).
  shaped=0
  if [[ "$line" =~ ^[[:space:]]*(RUN|\&\&)\ git\ clone\ --depth\ 1\ -b\ [A-Za-z0-9._-]+\ --recurse-submodules\ --shallow-submodules\ https://[^[:space:]]+\ (/[^[:space:]]+)\ \\$ ]]; then
    shaped=1
    dir=${BASH_REMATCH[2]}
    next=${lines[$((i + 1))]:-}
    [[ "$next" =~ ^[[:space:]]*\&\&\ test\ \"\$\(git\ -C\ ${dir//./\\.}\ rev-parse\ HEAD\)\"\ =\ [0-9a-f]{40}(\ \\)?$ ]] \
      || refuse "$i" "clone not checked against a commit by the next line: $dir"
  elif [[ "$line" =~ ^[[:space:]]*(RUN|\&\&)\ (/[^[:space:]]+/)?pip3?\ install\ --no-cache-dir\ --require-hashes\ --no-deps\ (--no-build-isolation\ )?-r\ (/[^[:space:]]+)(\ \\)?$ ]]; then
    shaped=1
    req=${BASH_REMATCH[4]}
    name=${req##*/}
    copied=0
    for l in "${lines[@]}"; do [ "$l" = "COPY $name $req" ] && copied=1; done
    [ "$copied" -eq 1 ] || refuse "$i" "requirements $req is not exactly 'COPY $name $req' from beside this file"
    reqfile=$(dirname "$file")/$name
    if [ -f "$reqfile" ]; then
      n=0
      while IFS= read -r r || [ -n "$r" ]; do
        n=$((n + 1))
        [[ "$r" =~ ^[[:space:]]*(#|$) ]] && continue
        [[ "$r" =~ ^[A-Za-z0-9][A-Za-z0-9._-]*==[0-9][A-Za-z0-9.+!-]*(\ --hash=sha256:${hex})+$ ]] \
          || refuse "$i" "$name:$n: not name==version with sha256 hashes: $r"
      done < "$reqfile"
    else
      refuse "$i" "$reqfile not found: the hashes are unreadable"
    fi
  fi

  if [ "$shaped" -eq 0 ] && { [[ "$line" =~ (^|[^a-z-])(wget|pip3?|npm|git[[:space:]].*clone)([^a-z-]|$) ]] || [[ "${line,,}" =~ ^[[:space:]]*add[[:space:]].*:// ]]; }; then
    refuse "$i" "fetches outside the checked curl, clone and pip shapes: $line"
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
