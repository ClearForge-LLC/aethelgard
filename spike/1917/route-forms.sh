#!/usr/bin/env bash
# AET-WO-1917 §1.1 (4): which docs-path literal forms `ci/route.sh inventory` reports, each planted alone in a scratch
# tree with the docs-only files docs/a.md, docs/work-orders/W.md, docs/reviews/R.md, fab/b/ORDER.md and README.md.
# Usage: bash spike/1917/route-forms.sh [ROUTE_SH]   (default: this checkout's ci/route.sh)
set -uo pipefail
export LC_ALL=C
here=$(cd "$(dirname "$0")/../.." && pwd)
route=${1:-$here/ci/route.sh}
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
git_() { git -c user.name=t -c user.email=t@t -c init.defaultBranch=main -c commit.gpgsign=false "$@"; }

form() { # LABEL RUST-EXPRESSION
  local label=$1 expr=$2 out
  rm -rf "$tmp/i"; mkdir -p "$tmp/i/ci" "$tmp/i/docs/work-orders" "$tmp/i/docs/reviews" "$tmp/i/fab/b" "$tmp/i/crates/d/tests"
  cd "$tmp/i" || exit 1
  cp "$route" ci/route.sh
  printf '[package]\nname = "demo"\n' >crates/d/Cargo.toml
  echo a >docs/a.md; echo w >docs/work-orders/W.md; echo r >docs/reviews/R.md; echo o >fab/b/ORDER.md; echo r >README.md
  printf 'fn f(x: &str) { let _ = %s; }\n' "$expr" >crates/d/tests/t.rs
  git_ init -q .; git_ add -A; git_ commit -qm t
  out=$(bash ci/route.sh inventory 2>&1 | tr '\t\n' ' ;')
  if [ -n "$out" ]; then printf 'SEEN\t%s\t%s\n' "$label" "$out"; else printf 'MISSED\t%s\t%s\n' "$label" "$expr"; fi
}

form "control: a plain path" '"docs/work-orders/W.md"'
form "control: a directory" '"docs/work-orders"'
form "control: {root}/ before a docs path" 'format!("{root}/docs/a.md")'
form "positional placeholder in the file name" 'format!("docs/work-orders/{}.md", x)'
form "named placeholder in the file name" 'format!("docs/work-orders/{x}.md")'
form "placeholder as a directory" 'format!("docs/{}/W.md", x)'
form "placeholder after docs/" 'format!("docs/{}", x)'
form "placeholder after a prefix and docs/" 'format!("{}/docs/{}.md", x, x)'
form "../../ then a placeholder" 'format!("../../docs/reviews/{}", x)'
form "trailing slash on a docs directory" '"docs/reviews/"'
form "trailing slash on docs" '"docs/"'
form "../../ and a trailing slash" '"../../docs/work-orders/"'
form "fab directory with a placeholder" 'format!("fab/{}/ORDER.md", x)'
form "fab directory, trailing slash" '"fab/b/"'
form "a root name with a placeholder stem" 'format!("{}.md", x)'
form "README with a placeholder suffix" 'format!("README{}", x)'
form "a fully unresolvable path" 'format!("{}/{}.md", x, x)'
form "concat! of a docs directory and a name" 'concat!("docs/reviews/", "R.md")'
