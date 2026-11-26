#!/bin/bash
# As quit.sh: whether an EOT byte (Ctrl-D) on a held pipe ends Renode at its prompt as EOF does.
set -u
R=/opt/renode/renode; O=/cache/kit/quit
BAD=/repo/crates/aethelgard-cli/tests/fixtures/renode-missing-include.repl
L="mach create; machine LoadPlatformDescription @$BAD; peripherals; quit"
s=$SECONDS; { printf '\004'; sleep 40; } | timeout 30 $R --disable-gui --console --plain -e "$L" >"$O/bad-eot.out" 2>&1; echo "bad-eot exit=${PIPESTATUS[1]} wall=$((SECONDS-s))s"
diff <(sed 's/[0-9:.]*\] //' $O/bad-null.out) <(sed 's/[0-9:.]*\] //' $O/bad-eot.out); echo "diff exit $?"
G="mach create; machine LoadPlatformDescription @/opt/renode/platforms/cpus/stm32h743.repl; emulation RunFor \"2\"; echo \"DONE-RUNFOR\"; quit"
s=$SECONDS; { printf 'quit\n'; sleep 40; } | timeout 30 $R --disable-gui --console --plain -e "$G" >"$O/good-held2.out" 2>&1; echo "good-held2 exit=${PIPESTATUS[1]} wall=$((SECONDS-s))s"; grep -n "DONE-RUNFOR\|quit" $O/good-held2.out | cut -c1-120
