#!/usr/bin/env bash
# container/check-pins.sh: refuses a Dockerfile that can drift (INV-06). It
# accepts only known-safe shapes rather than hunting for bad ones:
#   - every FROM, in any case or indent, names image@sha256:<64 hex>, and a
#     `--from=` or a `--mount`'s `from=` names a stage declared above or a
#     digest-pinned image; ADD is refused (COPY a local file instead);
#   - every curl is exactly `curl -fsSLo PATH https://URL`, and the next line is
#     `&& echo "<64 hex>  PATH" | sha256sum -c -`, the only pipe allowed;
#   - apt runs only as `apt-get update` and one `apt-get install -y
#     --no-install-recommends` block whose every package is name=version;
#   - every stage that runs `apt-get update` first has `COPY NAME.sources
#     /etc/apt/sources.list.d/debian.sources`, NAME.sources beside this file;
#     every *.sources there names only snapshot.debian.org, all at one
#     timestamp; nothing else touches /etc/apt or sets APT_CONFIG
#     (AET-WO-4407: a build with no cache resolves what the tables name);
#   - `cargo install` is --locked with a --version, and rustup installs a
#     numbered toolchain;
#   - nothing is swallowed with `||`, and the common fetchers outside the curl
#     shape (wget, pip, npm, git clone, ADD <url>) are refused;
#   - read on whole instructions, continuations joined as Docker joins them
#     (AET-WO-4401, 4400a A7): git runs only as the clone shape, `git -C DIR
#     rev-parse HEAD` or `git -C DIR status --porcelain`, and `git submodule
#     update --remote`, `git fetch`, `git pull`, `pipx` and a `pip install`
#     without --require-hashes are refused by name, wherever the word stands
#     (in quotes, spliced as gi''t, in $(...), as an ENV value, as -mpip, as
#     /path/to/git);
#   - a `-m venv` takes Debian's pip, so python3-pip-whl must be pinned in the
#     apt block;
#   - no heredoc, no `# escape=` or `# syntax=` directive (in any case or
#     spacing, after `#` or `//`), no byte-order mark, no carriage return, and no COPY of
#     a script (a #! line, an executable bit or a .sh/.bash/.py/.pl name): the
#     check reads only this file and the hashed requirements.
# What is left of A7: a command name assembled at run time (`${G}t fetch`,
# `$(printf '\x67it')`), a plain file copied in and run by an interpreter
# (`sh /x.txt`), and installers this list does not name (uvx, conda). This
# script stops accidental drift, not an attacker; those are left to review.
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
snapped=0
stages=" "
refuse() {
  echo "check-pins: line $(($1 + 1)): $2" >&2
  bad=1
}

# The apt sources (AET-WO-4407): each *.sources beside this file holds only
# comments, blank lines and these deb822 fields, and every URIs line names
# snapshot.debian.org at the one timestamp all of them share.
snapshot=''
for s in "$(dirname "$file")"/*.sources; do
  [ -f "$s" ] || continue
  n=0
  while IFS= read -r l || [ -n "$l" ]; do
    n=$((n + 1))
    [[ "$l" =~ ^(#.*)?$ ]] && continue
    if [[ "$l" =~ ^URIs:\ http://snapshot\.debian\.org/archive/debian(-security)?/([0-9]{8}T[0-9]{6}Z)$ ]]; then
      [ -n "$snapshot" ] || snapshot=${BASH_REMATCH[2]}
      [ "${BASH_REMATCH[2]}" = "$snapshot" ] \
        || refuse -1 "$s:$n: snapshot ${BASH_REMATCH[2]}, not $snapshot: every *.sources here names one timestamp"
    elif ! [[ "$l" =~ ^(Types:\ deb|Suites:\ [a-z]+(-updates|-security)?(\ [a-z]+(-updates|-security)?)*|Components:\ main|Signed-By:\ /usr/share/keyrings/debian-archive-keyring\.(pgp|gpg)|Check-Valid-Until:\ no)$ ]]; then
      refuse -1 "$s:$n: not a snapshot.debian.org source line: $l"
    fi
  done < "$s"
done
snapcopy='^COPY ([a-z0-9][a-z0-9.-]*\.sources) /etc/apt/sources\.list\.d/debian\.sources$'

# Whole instructions (AET-WO-4401). Docker drops a trailing `\` (spaces and
# tabs may follow it) with its newline and joins the next line as it stands,
# with no space added, skipping comment and empty lines in between.
logical=() starts=() ends=()
cur='' open=0 start=0
for i in "${!lines[@]}"; do
  l=${lines[$i]}
  [[ "$l" =~ ^[[:space:]]*(#|$) ]] && continue
  [ "$open" -eq 1 ] || { cur=''; start=$i; }
  if [[ "$l" =~ ^(.*)\\[[:blank:]]*$ ]]; then
    cur+=${BASH_REMATCH[1]}
    open=1
  else
    logical+=("$cur$l") starts+=("$start") ends+=("$i")
    open=0
  fi
done
[ "$open" -eq 0 ] || logical+=("$cur") starts+=("$start") ends+=("$i")

# Each instruction as words: separators become `;`; quotes and backslashes go,
# as the shell splices `gi''t` and `g\it` into `git`; brackets, backquotes and
# `$` become spaces, so a command word is found inside a string, a $(...) or a
# ${VAR}. A word's name drops an `X=` prefix, a `-m` prefix and its directory.
named=()
venv=0
for n in "${!logical[@]}"; do
  t=${logical[$n]}
  t=${t//&&/ ; } t=${t//||/ ; }
  t=${t//|/ ; } t=${t//&/ ; }
  t=${t//[\"\'\\]/}
  t=${t//[\(\)\`\$\{\}\[\],<>]/ }
  read -ra w <<< "$t"
  # Only the snapshot COPY writes apt's configuration (AET-WO-4407).
  if [[ "$t" == *"/etc/apt"* || "$t" == *APT_CONFIG* ]] && ! [[ "${logical[$n]}" =~ $snapcopy ]]; then
    refuse "${starts[$n]}" "only 'COPY NAME.sources /etc/apt/sources.list.d/debian.sources' may change apt's configuration: ${logical[$n]}"
  fi
  for k in "${!w[@]}"; do
    name=${w[$k]##*=}
    [[ "$name" == -m?* ]] && name=${name#-m}
    name=${name##*/}
    rest=()
    for ((j = k + 1; j < ${#w[@]}; j++)); do
      [ "${w[$j]}" = ";" ] && break
      rest+=("${w[$j]}")
    done
    msg=''
    if [ "$name" = git ]; then
      j=0
      while [ "$j" -lt "${#rest[@]}" ] && [[ "${rest[$j]}" == -* ]]; do
        case ${rest[$j]} in
          -C | -c | --git-dir | --work-tree | --namespace | --config-env) j=$((j + 2)) ;;
          *) j=$((j + 1)) ;;
        esac
      done
      sub=${rest[$j]:-}
      case $sub in
        fetch | pull) msg="git $sub moves the tree past its pinned commit" ;;
        submodule)
          if [[ " ${rest[*]} " == *" --remote "* ]]; then
            msg="git submodule update --remote checks out a branch tip, not the commit the tree records"
          fi
          ;;
        clone)
          [ "$j" -eq 0 ] && [[ "${rest[*]}" =~ ^clone\ --depth\ 1\ -b\ [A-Za-z0-9._-]+\ --recurse-submodules\ --shallow-submodules\ https://[^[:space:]]+\ /[^[:space:]]+$ ]] \
            || msg="git clone outside the checked shape"
          ;;
        rev-parse) [ "$j" -eq 2 ] && [ "${rest[0]}" = -C ] && [ "${rest[3]:-}" = HEAD ] && sub=ok ;;
        status) [ "$j" -eq 2 ] && [ "${rest[0]}" = -C ] && [ "${rest[3]:-}" = --porcelain ] && sub=ok ;;
      esac
      [ -n "$msg" ] || [ "$sub" = ok ] || [ "$sub" = clone ] \
        || msg="git subcommand '$sub' is outside the checked shapes (clone, -C DIR rev-parse HEAD, -C DIR status --porcelain)"
    elif [ "$name" = pipx ]; then
      msg="pipx installs from an index with no hash check"
    elif [[ "$name" =~ ^pip[0-9.]*$ ]] && [[ " ${rest[*]} " == *" install "* ]] \
      && [[ " ${rest[*]} " != *" --require-hashes "* ]]; then
      msg="pip install without --require-hashes"
    elif [ "$name" = venv ] && { [ "${w[$k]}" = -mvenv ] || [ "${w[$((k > 0 ? k - 1 : 0))]}" = -m ]; } \
      && [[ " ${rest[*]} " != *" --without-pip "* ]]; then
      venv=1
    fi
    if [ -n "$msg" ]; then
      refuse "${starts[$n]}" "$msg: ${logical[$n]}"
      for ((p = starts[n]; p <= ends[n]; p++)); do named[p]=1; done
    fi
  done
done

for i in "${!lines[@]}"; do
  line=${lines[$i]}
  # A parser directive: `escape` changes the continuation character, so the
  # joining above would no longer be Docker's.
  [[ "${line,,}" =~ ^#[[:space:]]*escape[[:space:]]*= ]] && refuse "$i" "an escape directive changes how lines join: $line"
  # BuildKit reads `syntax` after `#` or `//`, in any case and spacing, and
  # pulls that frontend image by tag (AET-WO-4407; 4401 Q2). It skips a
  # byte-order mark first, so a BOM would hide either directive from this check.
  [[ "${line,,}" =~ ^(#|//)[[:space:]]*syntax[[:space:]]*= ]] && refuse "$i" "a syntax directive pulls a BuildKit frontend by tag: $line"
  [[ "$line" == *$'\xef\xbb\xbf'* ]] && refuse "$i" "a byte-order mark, which hides a parser directive from this check"
  # A carriage return: whether Docker then joins on `\` is not this check's to guess.
  [[ "$line" == *$'\r'* ]] && refuse "$i" "a carriage return (CRLF line ends)"
  [[ "$line" =~ ^[[:space:]]*(#|$) ]] && continue
  if [[ "$line" == *"<<"* ]]; then
    refuse "$i" "a heredoc, whose body the shell joins and quotes its own way: write it as RUN lines: $line"
  fi
  # A COPY'd script would run unread, so only plain files beside this file.
  if [[ "${line,,}" =~ ^[[:space:]]*copy[[:space:]] ]] && [[ "$line" != *--from=* ]]; then
    read -ra words <<< "$line"
    srcs=()
    for s in "${words[@]:1}"; do [[ "$s" == --* ]] || srcs+=("$s"); done
    for s in "${srcs[@]:0:${#srcs[@]}-1}"; do
      p=$(dirname "$file")/$s
      if ! [ -f "$p" ]; then
        refuse "$i" "COPY source is not a file beside this Dockerfile: $s"
      elif [ -x "$p" ] || [ "$(head -c2 "$p")" = '#!' ] || [[ "$s" =~ \.(sh|bash|py|pl)$ ]]; then
        refuse "$i" "COPY of a script, which this check cannot read: write its commands as RUN lines: $s"
      fi
    done
  fi

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
    snapped=0
    read -ra words <<< "$line"
    image=
    for k in "${!words[@]}"; do
      [ "$k" -eq 0 ] && continue
      [[ "${words[$k]}" == --* ]] || { image=${words[$k]}; break; }
    done
    # FROM IMAGE AS NAME declares a stage a later --from= may name.
    nextw=${words[$((k + 1))]:-}
    if [ -n "$image" ] && [ "${nextw,,}" = as ] && [ -n "${words[$((k + 2))]:-}" ]; then
      stages="$stages${words[$((k + 2))]} "
    fi
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

  # A name that is not a declared stage is pulled from a registry at its
  # latest tag (AET-WO-4400a adversarial A4), so only a stage or a digest.
  if [[ "$line" =~ (^|[[:space:],-])from=([^[:space:],]+) ]]; then
    ref=${BASH_REMATCH[2]}
    if ! [[ "$ref" =~ @sha256:${hex}$ ]] && [[ "$stages" != *" $ref "* ]]; then
      refuse "$i" "from= names neither a stage declared above nor a digest-pinned image: $ref"
    fi
  fi

  if [[ "${line,,}" =~ ^[[:space:]]*add[[:space:]] ]]; then
    refuse "$i" "ADD can fetch and unpack (a URL, a git source): COPY a local file instead: $line"
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

  if [ "$shaped" -eq 0 ] && [ -z "${named[$i]:-}" ] \
    && [[ "$line" =~ (^|[^a-z-])(wget|pip3?|npm|git[[:space:]].*clone)([^a-z-]|$) ]]; then
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

  if [[ "$line" =~ $snapcopy ]]; then
    if [ -n "$snapshot" ] && [ -f "$(dirname "$file")/${BASH_REMATCH[1]}" ]; then
      snapped=1
    else
      refuse "$i" "no snapshot sources beside this file: ${BASH_REMATCH[1]}"
    fi
  fi

  if [[ "$line" =~ (^|[^a-z-])apt(-get)?[[:space:]] ]]; then
    if [[ "$line" =~ ^[[:space:]]*\&\&\ apt-get\ install\ -y\ --no-install-recommends\ \\$ ]]; then
      in_apt=1
    elif ! [[ "$line" =~ ^RUN\ apt-get\ update\ \\$ ]]; then
      refuse "$i" "apt used outside the pinned install block: $line"
    elif [ "$snapped" -eq 0 ]; then
      refuse "$i" "apt-get update in a stage with no snapshot source COPYed above it: it would read Debian's moving index"
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
if [ "$venv" -eq 1 ] && ! printf '%s\n' "${lines[@]}" | grep -qE '^[[:space:]]+python3-pip-whl=[0-9]'; then
  refuse -1 "a -m venv takes Debian's pip: pin python3-pip-whl in the apt block"
fi
[ "$bad" -eq 0 ] || exit 1
echo "check-pins: $file ok"
