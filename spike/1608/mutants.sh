#!/usr/bin/env bash
# AET-WO-1608 §2: one mutant per guard this WO adds, each run against the
# suite it lives in. A mutant is killed when that suite fails. Each file is
# restored from git after its run.
set -uo pipefail
cd "$(git rev-parse --show-toplevel)"
export PATH=$HOME/.cargo/bin:$PATH

T=crates/aethelgard-cli/templates/i2c-spi-bench.json
I=crates/aethelgard-cli/src/init.rs
L=crates/aethelgard-ir/src/lib.rs

run() { # name, file, python replacement (old, new), cargo test args...
  local name=$1 file=$2 old=$3 new=$4; shift 4
  python3 -I - "$file" "$old" "$new" <<'EOF'
import sys
p, old, new = sys.argv[1:4]
s = open(p).read()
assert s.count(old) == 1, f"{p}: {old!r} occurs {s.count(old)} times"
open(p, "w").write(s.replace(old, new))
EOF
  if nice -n 10 cargo test -q "$@" >/dev/null 2>&1; then
    echo "SURVIVED $name"
  else
    echo "killed   $name"
  fi
  git checkout -q -- "$file"
}

run "M1 a placeholder given a value" "$T" \
  '"source": ""
      },
      "relative_permittivity' '"source": "a value"
      },
      "relative_permittivity' -p aethelgard-cli --test init_tier1
run "M2 the template drifted from its example" "$T" \
  '"pull_up_ohms": 2200' '"pull_up_ohms": 2400' -p aethelgard-cli --test init_tier1
run "M3 an existing --out truncated" "$I" \
  '.create_new(true)' '.create(true).truncate(true)' -p aethelgard-cli --test init_tier1
run "M4 the fix names the wrong key" "$L" \
  "let last = path.rsplit('.').next().unwrap_or(path);" \
  "let last = path.split('.').next().unwrap_or(path);" -p aethelgard-ir --test identifier_fix
run "M5 a placeholder dropped from the printed list" "$I" \
  '        "board.stackup.copper_max_um.source",
' '' -p aethelgard-cli --test init_tier1
run "M6 a shape that a next rule refuses (no date shape in a project name)" "$L" \
  'with no -- and no date shape dddd-dd-dd, and not' 'with no --, and not' -p aethelgard-ir --test identifier_fix
run "M7 the base rule's fix quotes the failed rule, not the family" "$L" \
  'let shape = family.shape();' 'let shape = IdentifierKind::Plain.shape();' -p aethelgard-ir --test identifier_fix
