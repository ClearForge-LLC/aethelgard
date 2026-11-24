#!/usr/bin/env bash
# AET-WO-1610 §2: one mutant per guard this WO adds or changes, each run
# against the suite it lives in (host, tier 1 and unit). A mutant is killed
# when that suite fails. Each file is restored from git and touched after its
# run (a restored file keeps an old mtime; cargo would reuse the mutant).
# Tier-2 mutants (verify_2k2_tier2, board_tier2, with_tools) run in the tools
# image: tier2-mutants.sh.
set -uo pipefail
cd "$(git rev-parse --show-toplevel)"
export PATH=$HOME/.cargo/bin:$PATH

V=crates/aethelgard-cli/src/verify.rs
M=crates/aethelgard-cli/src/main.rs
L=crates/aethelgard-cli/src/ledger.rs
CLI=(-p aethelgard-cli)

run() { # name, file, old, new, cargo test args...
  local name=$1 file=$2 old=$3 new=$4; shift 4
  # ONLY="M5 M8": run just those (a rerun after a fix).
  if [ -n "${ONLY:-}" ] && [[ " $ONLY " != *" ${name%% *} "* ]]; then return; fi
  python3 -I - "$file" "$old" "$new" <<'PY'
import sys
p, old, new = sys.argv[1:4]
s = open(p).read()
assert s.count(old) == 1, f"{p}: {old!r} occurs {s.count(old)} times"
open(p, "w").write(s.replace(old, new))
PY
  # Only a test failure (cargo's 101) kills: a build or argument error (1)
  # is reported as such, never counted.
  timeout 1800 nice -n 10 cargo test -q "$@" >/dev/null 2>&1
  local rc=$?
  case $rc in
    0) echo "SURVIVED $name" ;;
    101) echo "killed   $name (exit 101)" ;;
    *) echo "ERROR    $name (exit $rc, not a kill)" ;;
  esac
  git checkout -q -- "$file"
  touch "$file"
}

U=("${CLI[@]}" --bin aethelgard)
run "M1 no harness pushes no prove gate" "$V" \
  'gates.push(not_reached("prove", NO_HARNESS));' '' "${U[@]}" verify::
run "M2 prove absent reads off again" "$V" \
  '("prove", Absent::Never),' '("prove", Absent::Off("a harness")),' "${U[@]}" verify::
run "M3 the not_reached reason is not the finding's kind" "$V" \
  'const NO_HARNESS: &str = "no_harness";' 'const NO_HARNESS: &str = "no harness";' "${U[@]}" verify::
run "M4 a gate that judged nothing reads judged 1" "$V" \
  '(_, Some(scope)) => Row::judged(name, scope.judged),' '(_, Some(scope)) => Row::judged(name, scope.judged.max(1)),' "${U[@]}" verify::
run "M5 drc.orientation on without a declared orientation" "$M" \
  'n(judged.seats) > 0,' 'true,' "${U[@]}" _parts_are_judged_or_off
run "M6 the outline's height is not compared" "$M" \
  'if w > bw || h > bh {' 'if w > bw {' "${CLI[@]}" --test outline_tier1 check_drc
run "M7 a second Edge.Cuts graphic by layers is not counted" "$M" \
  'if named.is_some_and(|l| l.texts().any(|t| t == "Edge.Cuts")) {' 'if key == "layer" && named.is_some_and(|l| l.texts().any(|t| t == "Edge.Cuts")) {' "${CLI[@]}" --test outline_tier1 check_drc
run "M8 a non-rectangle outline is accepted" "$M" \
  'let [("gr_rect", edge)] = edges.as_slice() else {' 'let [(_, edge)] = edges.as_slice() else {' "${CLI[@]}" --test outline_tier1 check_drc
run "M9 an empty enum is not looked for" "$M" \
  'if input.get(at) != Some(&b'"'"'}'"'"') {' 'if true {' "${CLI[@]}" --test validate_cli an_enum_left
run "M10 an array index never advances" "$M" \
  'Some(Frame::Array(n)) => *n += 1,' 'Some(Frame::Array(_)) => {}' "${U[@]}" the_path_of_an_empty
run "M11 an odd key reads bare" "$M" \
  'Frame::Object(Some(key)) if plain(key) => {' 'Frame::Object(Some(key)) if true => {' "${U[@]}" the_path_of_an_empty
run "M12 well-formed by IgnoredAny again" "$M" \
  '    serde_json::from_slice::<serde_json::Value>(input).is_ok()' '    serde_json::from_slice::<serde::de::IgnoredAny>(input).is_ok()' "${CLI[@]}" --test validate_cli an_enum_left
run "M13 the no-ledger message quotes solve's text" "$M" \
  '.map_err(|_| "`solve` refuses it; `aethelgard solve` names why")?;' '.map_err(|e| e.to_string().leak() as &str)?;' "${CLI[@]}" --test validate_cli stale
run "M14 the ledger's fields of every class" "$L" \
  '.filter(|r| r.register == a.register && r.class.unwitnessed() == Some(class))' '.filter(|r| r.register == a.register)' "${CLI[@]}" --test ledger_tier1
run "M15 the stale fields quoted by their text" "$L" \
  'let at: Vec<String> = self.at.iter().map(|i| format!("`fields[{i}]`")).collect();' 'let at: Vec<String> = self.at.iter().map(|i| format!("`fields[{i}]` MODER3 the fix")).collect();' "${CLI[@]}" --test validate_cli stale
