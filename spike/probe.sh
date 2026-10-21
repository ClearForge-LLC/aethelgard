#!/usr/bin/env bash
# AET-WO-0300 spike probe. A MEASUREMENT, not a gate: it runs the four tools
# inside the published image and records what happened, failures included, so
# one CI run answers the work order's §0 (S1-S6). Every exit code is captured
# on purpose rather than stopping at the first nonzero one. Each result is one
# line `ANSWER <key>=<value>`; the build half deletes this file.
#
# Expected mounts: the repository read-only at /repo, the built probes at
# /target/release, the Kani fixture crate read-only at /ro-crate.
set -u

REPO=/repo
FIX=$REPO/container/fixtures
SPK=$REPO/spike/fixtures
BIN=/target/release
OUT=$(mktemp -d /tmp/probe.XXXXXX)
NUCLEO='mach create; machine LoadPlatformDescription @platforms/boards/nucleo_h753zi.repl; peripherals; quit'

answer() { printf 'ANSWER %s=%s\n' "$1" "$2"; }
now_ms() { echo $(($(date +%s%N) / 1000000)); }
group() { echo "::group::$*"; }
endgroup() { echo "::endgroup::"; }

KICAD=$(command -v kicad-cli)
RENODE=$(command -v renode)
CARGO=$(command -v cargo)
answer kernel "$(uname -r)"
answer tools "kicad-cli=$KICAD renode=$RENODE cargo=$CARGO"
group "image environment"
env | sort
endgroup

# ---------------------------------------------------------------- S6 versions
for t in "kicad-cli version" "renode --version" "ngspice --version" "cargo kani --version"; do
  key=$(echo "$t" | cut -d' ' -f1)
  for rep in 1 2 3; do
    s=$(now_ms)
    $t > "$OUT/v.txt" 2>&1 < /dev/null
    rc=$?
    answer "s6.version.$key.run$rep" "ms=$(($(now_ms) - s)) rc=$rc"
  done
  answer "s6.version.$key.text" "$(head -1 "$OUT/v.txt")"
done

# One real Renode run: load the reference board and list its peripherals.
s=$(now_ms)
timeout 120 "$RENODE" --disable-gui --console --plain -e "$NUCLEO" > "$OUT/renode.log" 2>&1 < /dev/null
answer s6.renode.nucleo "ms=$(($(now_ms) - s)) rc=$? cpu=$(grep -c 'cpu (CortexM)' "$OUT/renode.log")"

# ---------------------------------------------------------------- S1 DRC/ERC
# report NAME CMD... : run a KiCad check writing $OUT/NAME.json, record rc, time,
# the report summary, and the report itself.
report() {
  local name=$1
  shift
  group "S1 $name"
  local s
  s=$(now_ms)
  "$@" > "$OUT/$name.log" 2>&1 < /dev/null
  local rc=$?
  local ms=$(($(now_ms) - s))
  cat "$OUT/$name.log"
  if [ -f "$OUT/$name.json" ]; then
    "$BIN/json-summary" "$OUT/$name.json"
    cat "$OUT/$name.json"
    echo
  fi
  endgroup
  local findings
  findings=$("$BIN/json-summary" "$OUT/$name.json" 2> /dev/null | awk '$1 == "findings" { print $2 }')
  answer "s1.$name" "rc=$rc findings=${findings:-noreport} ms=$ms"
}
drc() { "$KICAD" pcb drc --format json -o "$OUT/$1.json" "${@:3}" "$2"; }
erc() { "$KICAD" sch erc --format json -o "$OUT/$1.json" "${@:3}" "$2"; }

for b in drc-clean drc-violation; do
  report "drc.$b.flag" drc "drc.$b.flag" "$FIX/$b.kicad_pcb" --exit-code-violations
  report "drc.$b.noflag" drc "drc.$b.noflag" "$FIX/$b.kicad_pcb"
  report "drc.$b.flag.severity_error" drc "drc.$b.flag.severity_error" "$FIX/$b.kicad_pcb" --exit-code-violations --severity-error
done
for s in erc-clean erc-dangling-wire erc-dangling-label erc-unconnected-input; do
  report "erc.$s.flag" erc "erc.$s.flag" "$SPK/$s.kicad_sch" --exit-code-violations
  report "erc.$s.noflag" erc "erc.$s.noflag" "$SPK/$s.kicad_sch"
  report "erc.$s.flag.severity_error" erc "erc.$s.flag.severity_error" "$SPK/$s.kicad_sch" --exit-code-violations --severity-error
done
group "kicad-cli sch erc --help"
"$KICAD" sch erc --help 2>&1
endgroup

# Count against exit code on KiCad's own demos: do they ever disagree?
for f in $(ls /usr/share/kicad/demos/*/*.kicad_pcb 2> /dev/null | head -8) $(ls /usr/share/kicad/demos/*/*.kicad_sch 2> /dev/null | head -8); do
  [ -e "$f" ] || continue
  base=$(basename "$f")
  case "$f" in
    *.kicad_pcb) "$KICAD" pcb drc --format json -o "$OUT/demo.json" --exit-code-violations "$f" > /dev/null 2>&1 < /dev/null ;;
    *) "$KICAD" sch erc --format json -o "$OUT/demo.json" --exit-code-violations "$f" > /dev/null 2>&1 < /dev/null ;;
  esac
  rc=$?
  findings=$("$BIN/json-summary" "$OUT/demo.json" 2> /dev/null | awk '$1 == "findings" { print $2 }')
  agree=no
  { [ "$rc" -eq 5 ] && [ "${findings:-0}" -gt 0 ]; } && agree=yes
  { [ "$rc" -eq 0 ] && [ "${findings:-x}" = 0 ]; } && agree=yes
  answer "s1.demo.$base" "rc=$rc findings=${findings:-noreport} agree=$agree"
  rm -f "$OUT/demo.json"
done

# ---------------------------------------------------------------- tool runs
# Each takes a working directory as $1 (also HOME and TMPDIR, the posture the
# CLI will use) and runs with stdin from
# /dev/null. $2... is an `env -i` assignment list. Success is the expected exit
# code plus the expected key in the output.
try_drc() {
  local w=$1
  shift
  (cd "$w" && env -i HOME="$w" TMPDIR="$w" "$@" "$KICAD" pcb drc --exit-code-violations --format json -o "$w/r.json" "$FIX/drc-violation.kicad_pcb") > "$w/log" 2>&1 < /dev/null
  [ $? -eq 5 ] && grep -qE '"type": *"clearance"' "$w/r.json" 2> /dev/null
}
try_erc() {
  local w=$1
  shift
  (cd "$w" && env -i HOME="$w" TMPDIR="$w" "$@" "$KICAD" sch erc --exit-code-violations --format json -o "$w/r.json" "$SPK/erc-clean.kicad_sch") > "$w/log" 2>&1 < /dev/null
  [ $? -eq 0 ] && [ -s "$w/r.json" ]
}
try_renode() {
  local w=$1
  shift
  (cd "$w" && env -i HOME="$w" TMPDIR="$w" "$@" timeout 120 "$RENODE" --disable-gui --console --plain -e "$NUCLEO") > "$w/log" 2>&1 < /dev/null
  [ $? -eq 0 ] && grep -qF 'cpu (CortexM)' "$w/log"
}
try_kani() {
  local w=$1
  shift
  cp -r "$FIX/kani-smoke" "$w/crate"
  (cd "$w/crate" && env -i HOME="$w" TMPDIR="$w" "$@" "$CARGO" kani --harness add_in_range) > "$w/log" 2>&1 < /dev/null
  [ $? -eq 0 ] && grep -qF 'VERIFICATION:- SUCCESSFUL' "$w/log"
}

# ---------------------------------------------------------------- S2 env
# The image's own variables, each a candidate for the allowlist.
mapfile -t FULL < <(env | grep -E '^(PATH|RUSTUP_HOME|CARGO_HOME|KANI_HOME|DOTNET_[A-Z_]+|LANG|LC_[A-Z]+|TZ|USER|TERM|XDG_[A-Z_]+)=' | sort)
answer s2.candidates "$(printf '%s ' "${FULL[@]}")"
for tool in drc erc renode kani; do
  w=$(mktemp -d /tmp/s2.XXXXXX)
  if "try_$tool" "$w" "${FULL[@]}"; then full=works; else full=FAILS; fi
  w=$(mktemp -d /tmp/s2.XXXXXX)
  if "try_$tool" "$w"; then empty=works; else empty=fails; fi
  # Sequential elimination: drop each variable in turn and keep it dropped if
  # the tool still works. The result is a minimal sufficient set in FULL's
  # order, not necessarily the only one (two variables may stand in for each
  # other).
  cur=("${FULL[@]}")
  for var in "${FULL[@]}"; do
    rest=()
    for v in "${cur[@]}"; do [ "$v" = "$var" ] || rest+=("$v"); done
    w=$(mktemp -d /tmp/s2.XXXXXX)
    "try_$tool" "$w" "${rest[@]}" && cur=("${rest[@]}")
  done
  required=("${cur[@]}")
  # The minimal set, run once more with a marker to see what it writes outside $w.
  w=$(mktemp -d /tmp/s2.XXXXXX)
  touch /tmp/s2-marker
  sleep 1
  if "try_$tool" "$w" "${required[@]}"; then minimal=works; else minimal=FAILS; fi
  outside=$(find / -xdev \( -path /proc -o -path /sys -o -path /dev -o -path "$w" -o -path /tmp/s2-marker \) -prune -o -newer /tmp/s2-marker -print 2> /dev/null | head -40 | tr '\n' ' ')
  group "S2 $tool minimal-run log"
  tail -20 "$w/log"
  endgroup
  answer "s2.$tool" "full=$full empty=$empty minimal=$minimal required=[$(printf '%s ' "${required[@]}" | sed 's/=[^ ]*//g')]"
  answer "s2.$tool.required_values" "$(printf '%s ' "${required[@]}")"
  answer "s2.$tool.writes_outside_workdir" "${outside:-none}"
done

# ---------------------------------------------------------------- S3 Landlock
group "S3 landlock status"
"$BIN/confine" --ro / --rw /tmp -- true
echo "confine rc=$?"
endgroup
answer s3.status "$("$BIN/confine" --ro / --rw /tmp -- true 2>&1 | grep '^landlock' | head -1)"
SYS=/usr:/lib:/lib64:/bin:/sbin:/etc
# confined NAME POLICY RO RW -- tool function : run a tool under Landlock.
for policy in strict strict+proc+dev; do
  for tool in drc erc renode kani; do
    w=$(mktemp -d /tmp/s3.XXXXXX)
    case $tool in
      drc) ro="$FIX:$SYS" ;;
      erc) ro="$SPK:$SYS" ;;
      renode) ro="/opt/renode:$SYS" ;;
      kani) ro="$FIX:/opt/cargo:/opt/rustup:/opt/kani:$SYS" ;;
    esac
    rw="$w"
    if [ "$policy" = strict+proc+dev ]; then
      ro="$ro:/proc"
      rw="$rw:/dev"
    fi
    case $tool in
      drc) cmd=("$KICAD" pcb drc --exit-code-violations --format json -o "$w/r.json" "$FIX/drc-violation.kicad_pcb"); want=5 ;;
      erc) cmd=("$KICAD" sch erc --exit-code-violations --format json -o "$w/r.json" "$SPK/erc-clean.kicad_sch"); want=0 ;;
      renode) cmd=(timeout 120 "$RENODE" --disable-gui --console --plain -e "$NUCLEO"); want=0 ;;
      kani)
        cp -r "$FIX/kani-smoke" "$w/crate"
        cmd=("$CARGO" kani --harness add_in_range)
        want=0
        ;;
    esac
    dir=$w
    [ "$tool" = kani ] && dir=$w/crate
    (cd "$dir" && HOME="$w" TMPDIR="$w" "$BIN/confine" --ro "$ro" --rw "$rw" -- "${cmd[@]}") > "$w/log" 2>&1 < /dev/null
    rc=$?
    case $tool in
      drc) grep -qE '"type": *"clearance"' "$w/r.json" 2> /dev/null && key=yes || key=no ;;
      erc) [ -s "$w/r.json" ] && key=yes || key=no ;;
      renode) grep -qF 'cpu (CortexM)' "$w/log" && key=yes || key=no ;;
      kani) grep -qF 'VERIFICATION:- SUCCESSFUL' "$w/log" && key=yes || key=no ;;
    esac
    works=no
    [ "$rc" -eq "$want" ] && [ "$key" = yes ] && works=yes
    group "S3 $policy $tool log"
    tail -25 "$w/log"
    endgroup
    answer "s3.$policy.$tool" "works=$works rc=$rc want=$want key=$key landlock=$(grep -m1 '^landlock ' "$w/log" | cut -c1-120) first_error=$(grep -m1 -iE 'denied|error|cannot|failed|exception' "$w/log" | grep -v '^landlock ' | cut -c1-200)"
  done
done

# ---------------------------------------------------------------- S4 children
# desc PID : PIDs descending from PID right now (not PID itself).
desc() {
  local root=$1 changed=1
  local -A in=([$root]=1)
  local p pp
  while [ "$changed" -eq 1 ]; do
    changed=0
    for d in /proc/[0-9]*; do
      p=${d#/proc/}
      [ -n "${in[$p]:-}" ] && continue
      pp=$(awk '/^PPid:/ { print $2 }' "$d/status" 2> /dev/null)
      if [ -n "$pp" ] && [ -n "${in[$pp]:-}" ]; then
        in[$p]=1
        changed=1
      fi
    done
  done
  for p in "${!in[@]}"; do [ "$p" = "$root" ] || echo "$p"; done
}
alive() { # PIDS... : those still running (not zombies), with their names
  for p in "$@"; do
    st=$(awk '/^State:/ { print $2 }' "/proc/$p/status" 2> /dev/null)
    [ -n "$st" ] && [ "$st" != Z ] && printf '%s(%s) ' "$p" "$(cat "/proc/$p/comm" 2> /dev/null)"
  done
}
for tool in renode kani; do
  for mode in bare pdeathsig reaper; do
    for sig in TERM KILL; do
      w=$(mktemp -d /tmp/s4.XXXXXX)
      if [ "$tool" = renode ]; then
        cmd=("$RENODE" --disable-gui --console --plain -e 'mach create; machine LoadPlatformDescription @platforms/boards/nucleo_h753zi.repl')
        settle=4
      else
        cp -r "$FIX/kani-smoke" "$w/crate"
        cmd=(sh -c "cd $w/crate && exec $CARGO kani --harness add_in_range")
        settle=1
      fi
      # stdin is a FIFO held open by this shell, so it outlives the probe.
      mkfifo "$w/stdin"
      exec 9<> "$w/stdin"
      "$BIN/spawn-tree" "$mode" -- "${cmd[@]}" > "$w/log" 2>&1 < "$w/stdin" &
      probe=$!
      sleep "$settle"
      mapfile -t kids < <(desc "$probe")
      before=$(alive "${kids[@]}")
      kill -"$sig" "$probe"
      sleep 1
      after=$(alive "${kids[@]}")
      answer "s4.$tool.$mode.$sig" "before=[${before}] survivors=[${after}]"
      for p in "${kids[@]}"; do kill -9 "$p" 2> /dev/null; done
      wait "$probe" 2> /dev/null
      exec 9>&-
    done
  done
done

# ---------------------------------------------------------------- S5 kani
kani_run() { # NAME DIR HARNESS : run cargo kani in DIR with the target dir outside it
  local name=$1 dir=$2 harness=$3 w
  w=$(mktemp -d /tmp/s5.XXXXXX)
  local s
  s=$(now_ms)
  (cd "$dir" && CARGO_TARGET_DIR="$w/target" "$CARGO" kani --harness "$harness") > "$w/log" 2>&1 < /dev/null
  local rc=$?
  answer "s6.kani.$name.ms" "$(($(now_ms) - s))"
  group "S5 $name"
  cat "$w/log"
  endgroup
  answer "s5.$name" "rc=$rc"
  answer "s5.$name.lines" "$(grep -E '^(VERIFICATION:|Complete -|Failed Checks:|error(\[E[0-9]+\])?:|Summary:|SUMMARY:| \*\* [0-9]+ of [0-9]+ failed)' "$w/log" | sort -u | tr '\n' '|' | cut -c1-600)"
}
kani_run readonly_mount /ro-crate add_in_range
copy=$(mktemp -d /tmp/s5copy.XXXXXX)
cp -r "$FIX/kani-smoke" "$copy/crate"
(cd "$copy/crate" && find . -type f -printf '%P %s %T@\n' | sort) > "$OUT/before.txt"
kani_run copy_pass "$copy/crate" add_in_range
kani_run copy_fail "$copy/crate" add_overflows
(cd "$copy/crate" && find . -type f -printf '%P %s %T@\n' | sort) > "$OUT/after.txt"
answer s5.files_created_or_changed "$(diff "$OUT/before.txt" "$OUT/after.txt" | grep '^>' | cut -d' ' -f2 | tr '\n' ' ')"
broken=$(mktemp -d /tmp/s5broken.XXXXXX)
cp -r "$REPO/spike/kani-broken" "$broken/crate"
kani_run copy_compile_error "$broken/crate" add_in_range
kani_run copy_no_such_harness "$copy/crate" no_such_harness

answer probe "done"
