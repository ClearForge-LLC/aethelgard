#!/usr/bin/env bash
# AET-WO-1610 §2: the mutants only tier 2 can kill, each applied to a copy of
# the committed source (git archive of HEAD) and run in the pinned tools image
# by the kit (~/.cache/aet-1610/kit/img.sh: capped-run, 16 GiB, swap 0,
# --network none). Only a test failure (101) kills.
set -uo pipefail
C=$HOME/.cache/aet-1610
REPO=$(git rev-parse --show-toplevel)
F='--locked --offline --features aethelgard-cli/with-tools -p aethelgard-cli'
one() { # name, file, old, new, test args (one string)
  local name=$1 file=$2 old=$3 new=$4 test=$5
  rm -rf "$C/msrc" && mkdir -p "$C/msrc" && git -C "$REPO" archive HEAD | tar -x -C "$C/msrc"
  python3 -I - "$C/msrc/$file" "$old" "$new" <<'PY'
import sys
p, old, new = sys.argv[1:4]
s = open(p).read()
assert s.count(old) == 1, f"{p}: {old!r} occurs {s.count(old)} times"
open(p, "w").write(s.replace(old, new))
PY
  SRC=$C/msrc "$C/kit/img.sh" bash -c "cargo test $F $test >/dev/null 2>&1"
  local rc=$?
  case $rc in
    0) echo "SURVIVED $name" ;;
    101) echo "killed   $name (exit 101)" ;;
    *) echo "ERROR    $name (exit $rc, not a kill)" ;;
  esac
}
V=crates/aethelgard-cli/src/verify.rs
M=crates/aethelgard-cli/src/main.rs
one "T1 the seats judged are never counted" "$M" \
  'judged.seats = Some(stack.imus.len());' 'judged.seats = Some(0);' '--test with_tools a_turned_keepout'
one "T2 a gate that judged nothing reads judged 1 (board_tier2)" "$V" \
  '(_, Some(scope)) => Row::judged(name, scope.judged),' '(_, Some(scope)) => Row::judged(name, scope.judged.max(1)),' \
  '--test board_tier2 verify_names_what_judged_nothing'
one "T3 a gate judged whole counts 0 (verify_2k2_tier2)" "$V" \
  '        (None, _) => 1,' '        (None, _) => 0,' '--test verify_2k2_tier2'
rm -rf "$C/msrc"
