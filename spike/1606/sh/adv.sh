#!/bin/bash
# AET-WO-1606 §5, adversarial: a banner the bridge did not get from its library. Inside the tools image,
# with the branch's bridge in $BIN. ngspice runs `.spiceinit` from the working directory and from
# HOME at start-up, before the bridge reads the log (measured below), so a file there can print a line.
set -u
B=${BIN:?}/aethelgard-bridge
W=${OUT:?}/adv; rm -rf "$W"; mkdir -p "$W"
probe() { # name, dir, spiceinit text, env...
  local n=$1 d=$W/$1 text=$2; shift 2
  mkdir -p "$d"; printf '%s\n' "$text" > "$d/.spiceinit"
  ( cd "$d" && env -i PATH=/usr/bin:/bin "$@" "$B" --version > "$W/$n.out" 2> "$W/$n.err" ); local code=$?
  echo "$n: exit $code"; sed 's/^/  /' "$W/$n.out" | cut -c1-400
}
probe marker 'echo aethelgard-marker-line'
probe same-banner 'echo ** ngspice-44.2 shared library'
probe other-banner 'echo ** ngspice-45.0 shared library'
mkdir -p "$W/home-only"
probe home-banner 'echo ** ngspice-44.2 shared library' HOME="$W/home-banner"
( cd "$W/home-only" && env -i PATH=/usr/bin:/bin HOME="$W/home-banner" "$B" --version > "$W/home-only.out" 2>&1 ); echo "home-only (cwd clean, HOME has .spiceinit): exit $?"; sed 's/^/  /' "$W/home-only.out" | cut -c1-400
# The CLI's own invocation: an empty environment and a fresh directory of its own (analog.rs), so
# neither file is there unless the CLI writes it.
( mkdir -p "$W/clean" && cd "$W/clean" && env -i "$B" --version > "$W/clean.out" 2>&1 ); echo "clean (env -i, fresh dir): exit $?"; sed 's/^/  /' "$W/clean.out" | cut -c1-400
