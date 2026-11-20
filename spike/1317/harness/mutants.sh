#!/bin/bash
# AET-WO-1317 mutants, in the adv-branch copy: each one replacement in pcb.rs, outline_tier1 run, restored.
set -u
export PATH=$HOME/.cargo/bin:$PATH
C=$HOME/.cache/aet-1317/adv-branch; F=$C/crates/aethelgard-eda/src/pcb.rs; O=$HOME/.cache/aet-1317/mut
: # orig set by caller
mutate() { # name old new
  cp $O/pcb.rs.orig $F
  python3 -I - "$F" "$2" "$3" <<'PY' || { echo "$1 NOT APPLIED"; return 1; }
import sys; f,o,n=sys.argv[1:]; s=open(f).read(); assert s.count(o)==1,(o,s.count(o)); open(f,'w').write(s.replace(o,n))
PY
  (cd $C && nice -n 10 systemd-run --user --scope -q -p MemoryMax=24G -p MemorySwapMax=0 -- env CARGO_TARGET_DIR=$HOME/.cache/aet-1317/target-mut timeout 1800 cargo test -q -p aethelgard-cli --test outline_tier1 -- --test-threads 4) > $O/$1.log 2>&1
  echo "$1: exit $? $(grep -E '^test result' $O/$1.log) FAILED: $(grep -E '^    [a-z_]+$' $O/$1.log | tr -d ' ' | tr '\n' ' ')"
}
mutate M0-none 'const RESEEDS: u64 = 8;' 'const RESEEDS: u64 = 8;'
mutate M1-reseeds-0 'const RESEEDS: u64 = 8;' 'const RESEEDS: u64 = 0;'
mutate M2-seed-ignored '        s => fnv(FNV_BASIS, &s.to_le_bytes()),' '        _ => FNV_BASIS,'
mutate M3-least-over-flipped '                if over(&e) < over(&best) {' '                if over(&e) > over(&best) {'
mutate M4-ties-to-later '                if over(&e) < over(&best) {' '                if over(&e) <= over(&best) {'
mutate M5-keep-last-fit '            done => return done,
        }
    }
    Err(best)' '            Ok(l) => { let _ = l; continue; }
            done => return done,
        }
    }
    Err(best)'
cp $O/pcb.rs.orig $F
cmp $F $O/pcb.rs.orig && echo restored
