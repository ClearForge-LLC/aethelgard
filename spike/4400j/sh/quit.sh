#!/bin/bash
# AET-WO-4400j CI hang (23:07Z note): how Renode ends on a failed command,
# on /dev/null and on a held pipe with "quit" written first. Run in the tools image.
set -u
R=/opt/renode/renode; O=/cache/kit/quit; rm -rf "$O"; mkdir -p "$O"
BAD=/repo/crates/aethelgard-cli/tests/fixtures/renode-missing-include.repl
GOOD=/opt/renode/platforms/cpus/stm32h743.repl
run() { # name stdin-mode commands
  local s=$SECONDS rc
  case $2 in
    null) timeout 120 $R --disable-gui --console --plain -e "$3" </dev/null >"$O/$1.out" 2>"$O/$1.err"; rc=$? ;;
    held) { printf 'quit\n'; sleep 200; } | timeout 120 $R --disable-gui --console --plain -e "$3" >"$O/$1.out" 2>"$O/$1.err"; rc=${PIPESTATUS[1]} ;;
    bare) { sleep 200; } | timeout 120 $R --disable-gui --console --plain -e "$3" >"$O/$1.out" 2>"$O/$1.err"; rc=${PIPESTATUS[1]} ;;
  esac
  # The wall includes the held pipe's writer, `sleep 200`, which the pipeline waits for.
  printf '%s exit=%s wall=%ss out=%s\n' "$1" "$rc" "$((SECONDS - s))" "$(sha256sum <"$O/$1.out" | cut -c1-12)"
}
L="mach create; machine LoadPlatformDescription @$BAD; peripherals; quit"
run bad-null null "$L"
run bad-held held "$L"
run bad-bare bare "$L"
G="mach create; machine LoadPlatformDescription @$GOOD; emulation RunFor \"2\"; echo \"DONE-RUNFOR\"; quit"
run good-held held "$G"
run good-null null "$G"
diff <(sed 's/[0-9:.]*\] //' $O/bad-null.out) <(sed 's/[0-9:.]*\] //' $O/bad-held.out) > $O/bad.diff; echo "bad diff lines: $(wc -l < $O/bad.diff)"
grep -c DONE-RUNFOR $O/good-held.out $O/good-null.out
