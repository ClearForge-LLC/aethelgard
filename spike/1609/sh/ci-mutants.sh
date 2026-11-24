#!/bin/bash
# AET-WO-1609: the two mutants only ci/foreign-judge.sh can kill, each in the source copy
# (~/.cache/aet-1609/mut) built to /cache/mtarget, with the script run whole as CI runs it.
set -u
C=$HOME/.cache/aet-1609
for m in no-finding no-refusal; do
  rsync -a --delete --exclude target --exclude .git $HOME/aethelgard-1609/ $C/mut/
  python3 -I - "$C/mut" "$m" <<'PY'
import sys
root, m = sys.argv[1:]
if m == "no-finding":
    p, a, b = "crates/aethelgard-cli/src/renode/foreign.rs", '            "foreign_panic",\n', '            "foreign_panic_info",\n'
    s = open(f"{root}/{p}").read(); assert s.count("findings.push(Finding::plain(\n            \"foreign_panic\",") == 1
    s = s.replace('findings.push(Finding::plain(\n            "foreign_panic",', 'findings.push(Finding::info(\n            "foreign_panic",')
else:
    p = "crates/aethelgard-cli/src/main.rs"
    s = open(f"{root}/{p}").read()
    a = "elf::function(&bytes, renode::foreign::PANIC_SYMBOL)?.ok_or_else(|| {"
    assert s.count(a) == 1
    s = s.replace(a, "elf::function(&bytes, renode::foreign::PANIC_SYMBOL)?.or(Some(0x0800_0000)).ok_or_else(|| {")
open(f"{root}/{p}", "w").write(s)
print("mutant", m)
PY
  sed 's#^bin=target/debug/aethelgard#bin=/cache/mtarget/debug/aethelgard#' $C/mut/ci/foreign-judge.sh > $C/fj-mut.sh
  REPO=$C/mut MEM=8G $C/kit/img.sh bash -c 'export CARGO_TARGET_DIR=/cache/mtarget; cargo build -q --locked --offline -p aethelgard-cli && cargo build -q --locked --offline --manifest-path bridge/Cargo.toml && bash /cache/fj-mut.sh /cache/boards' > $C/ci-mut-$m.log 2>&1
  echo "$m: foreign-judge.sh exit=$? (nonzero = killed); $(grep -E 'late-panic|panic-unnamed' $C/ci-mut-$m.log | grep -o 'ANSWER 4400e: [a-z-]* exit=[0-9]* verdict=[a-z]* error=[A-Za-z]* kinds=[^w]*' | tr '\n' ' ') $(grep -E 'does not hold|exit [0-9]+, not' $C/ci-mut-$m.log | head -2)"
done
rsync -a --delete --exclude target --exclude .git $HOME/aethelgard-1609/ $C/mut/
