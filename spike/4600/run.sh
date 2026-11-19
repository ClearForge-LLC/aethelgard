#!/bin/bash
# AET-WO-4600: remake every run. One container per step (sh/img.sh: capped-run, --network none,
# nice -n 10, one at a time), so each row's memory.peak is that step's own scope peak.
# Usage: spike/4600/run.sh <spec-name> [step ...]     (no step: every step, in the work order's order)
# Steps: validate solve emit erc drc support speeds extract generate build prove simulate campaign
#        replay attribute verify verify-fab hwdef pinout
# Outputs: ~/.cache/aet-4600/runs/<spec>/<step>.{json,err}, rows.tsv (spec step exit wall_ms peak max).
# A step whose input another step makes is `not_run` when that step's exit was not 0 or its output is
# absent: the row says not_run and names the step it needed.
# The CLI is target/debug/aethelgard built inside the image from the base SHA (FEEDBACK §0).
set -u
cd "$(dirname "$0")/../.."
S=$1; shift
STEPS=${*:-validate solve emit erc drc support speeds extract generate build prove simulate campaign replay attribute verify verify-fab hwdef pinout}
IMG=spike/4600/sh/img.sh
A=/cache/target/debug/aethelgard
SPEC=/repo/spike/4600/$S.json
O=/cache/runs/$S
H=$HOME/.cache/aet-4600/runs/$S
mkdir -p "$H"
PROJ=$(python3 -I -c "import json,sys; print(json.load(open(sys.argv[1]))['project_name'])" "spike/4600/$S.json")
SCEN=$(python3 -I -c "import json,sys; print(' '.join('/repo/'+f for f in json.load(open('spike/4600/scenarios/lists.json')).get(sys.argv[1], [])))" "$S")
rc() { awk -F'\t' -v s="$1" '$2==s {r=$3} END {print (r==""?"absent":r)}' "$H/rows.tsv" 2>/dev/null || echo absent; }
need() { # need <step> <dependency>...: print a not_run row and return 1 unless each dependency exited 0
  local step=$1; shift
  for d in "$@"; do
    if [ "$(rc "$d")" != 0 ]; then
      printf '%s\t%s\tnot_run\t-\t-\tneeds %s (exit %s)\n' "$S" "$step" "$d" "$(rc "$d")" | tee -a "$H/rows.tsv"
      return 1
    fi
  done
}
# Before each container: wait until the host has the step's cap plus 4 GiB available (Furnace is
# shared), logging what free -g showed.
roomy() { local want=$(( ${1%G} + 4 )); local n=0
  until [ "$(free -g | awk '/^Mem:/ {print $7}')" -ge "$want" ]; do n=$((n+1)); [ $n -gt 120 ] && break; sleep 30; done
  echo "$(date -u -Is) $S $2 mem=$1 free -g available=$(free -g | awk '/^Mem:/ {print $7}')" >> "$H/free.log"; }
step() { local name=$1; shift; local m=${MEM:-8G}; roomy "$m" "$name"; MEM=$m bash $IMG bash /repo/spike/4600/sh/step.sh "$S" "$name" "$O" "$@" 2>>"$H/caps.log"; }
# prove runs at 16G on any board whose schedule harness is unsplit, i.e. with no timer (no motor
# output; svd-hal board.rs L3897), whatever its buses (the architect's ruling, 18:06Z, re-keying the
# 17:08Z one); every other step but verify at 8G.
PROVE_MEM=$(python3 -I -c "import json,sys; print('8G' if json.load(open(sys.argv[1]))['motor_outputs'] else '16G')" "spike/4600/$S.json")
for st in $STEPS; do
  case $st in
    validate) step validate $A validate --file $SPEC ;;
    solve) step solve $A solve --file $SPEC ;;
    emit) rm -rf "$H/emit"; step emit $A emit --file $SPEC --out $O/emit ;;
    erc) need erc emit && step erc $A check erc --schematic $O/emit/$PROJ.kicad_sch ;;
    drc) need drc emit && step drc $A check drc --board $O/emit/$PROJ.kicad_pcb --spec $SPEC --schematic $O/emit/$PROJ.kicad_sch ;;
    support) need support emit && step support $A check support --schematic $O/emit/$PROJ.kicad_sch --board $O/emit/$PROJ.kicad_pcb --spec $SPEC ;;
    speeds) step speeds $A check speeds --spec $SPEC ;;
    extract) need extract emit && step extract $A extract --board $O/emit/$PROJ.kicad_pcb --spec $SPEC ;;
    generate) rm -rf "$H/gen"; step generate $A generate --spec $SPEC --out $O/gen ;;
    build) rm -f "$H/fw.elf"; need build generate && step build $A build --crate $O/gen/board --out $O/fw.elf ;;
    prove)
      if need prove generate; then
        for h in $(awk '/^ *#\[kani::proof\]/{a=1;next} a&&/^ *fn /{sub(/^ *fn /,"");sub(/[^A-Za-z0-9_].*/,"");print "proofs::"$0;a=0}' "$H/gen/board/src/proofs.rs"); do
          MEM=$PROVE_MEM step "prove:${h#proofs::}" $A prove --crate $O/gen/board --harness "$h" --spec $SPEC
        done
      fi ;;
    simulate) need simulate generate build && step simulate $A simulate --platform $O/gen/board/$PROJ.repl --firmware $O/fw.elf --spec $SPEC --for 100000000ns ;;
    campaign)
      if [ -z "$SCEN" ]; then printf '%s\tcampaign\tnot_run\t-\t-\tno scenarios written\n' "$S" | tee -a "$H/rows.tsv"
      elif need campaign build; then rm -rf "$H/archive"; step campaign $A campaign --scenarios $SCEN --firmware $O/fw.elf --archive $O/archive; fi ;;
    replay|attribute)
      # The first archived run (campaign exits nonzero when any run misses its expectation, and
      # still archives every run, so the archive, not the exit, is the dependency).
      run=$(ls -1 "$H/archive" 2>/dev/null | head -1)
      if [ -n "$run" ]; then step $st $A $st --archive $O/archive/$run
      else printf '%s\t%s\tnot_run\t-\t-\tneeds an archived run (campaign exit %s)\n' "$S" "$st" "$(rc campaign)" | tee -a "$H/rows.tsv"; fi ;;
    verify) MEM=16G step verify $A verify --spec $SPEC ;;
    verify-fab)
      if [ -z "$SCEN" ]; then printf '%s\tverify-fab\tnot_run\t-\t-\tno scenarios written\n' "$S" | tee -a "$H/rows.tsv"
      else rm -rf "$H/fab"; MEM=16G step verify-fab $A verify --spec $SPEC --scenarios $SCEN --fab-out $O/fab; fi ;;
    hwdef) rm -f "$H/hwdef.dat"; need hwdef solve && step hwdef $A generate hwdef --topology $O/solve.json --out $O/hwdef.dat ;;
    pinout) need pinout solve build && step pinout $A check pinout --topology $O/solve.json --elf $O/fw.elf ;;
    *) echo "unknown step $st" >&2; exit 2 ;;
  esac
done
