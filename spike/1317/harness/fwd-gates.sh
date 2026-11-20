#!/bin/bash
# AET-WO-1317: every gate board_tier2's bounded_gates runs, on the forward turn (now placed on seed 1).
set -u
A=${A:-/cache/target-t2/debug/aethelgard}
S=/repo/s03-turn-forward.json
W=/tmp/fwd; rm -rf $W; mkdir -p $W
echo "validate: $($A validate --file $S | head -c 200)"
echo "generate: $($A generate --spec $S --out $W/gen | head -c 200)"
E=$($A emit --file $S --out $W/emitted); echo "emit: $(echo "$E" | head -c 300)"
B=$(echo "$E" | python3 -c 'import json,sys; print(json.load(sys.stdin)["board"]["path"])')
C=$(echo "$E" | python3 -c 'import json,sys; print(json.load(sys.stdin)["schematic"]["path"])')
grep -c " signal)" $B
grep -m1 -A2 gr_rect $B | tr -s ' \n\t' ' '; echo
J='import json,sys; x=json.load(sys.stdin); print(x["verdict"], "findings", len(x["findings"]))'
echo "erc: $($A check erc --schematic $C | python3 -c "$J")"
echo "drc: $($A check drc --board $B --spec $S | python3 -c "$J")"
echo "drc+parity: $($A check drc --board $B --spec $S --schematic $C | python3 -c "$J")"
echo "support: $($A check support --schematic $C --board $B --spec $S | python3 -c "$J")"
echo "extract: $($A extract --board $B --spec $S | python3 -c 'import json,sys; x=json.load(sys.stdin); print(x["verdict"], x["findings"], min((l["declared_femtofarads"]-l["c_bound_femtofarads"])/1000 for l in x["lines"]))')"
