#!/usr/bin/env bash
# AET-WO-1911 §1.1: the routing and the inventory guard fail each case they exist to fail, on scratch git repositories.
# No cargo, no image.
set -uo pipefail
export LC_ALL=C
here=$(cd "$(dirname "$0")" && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
fail=0
ok() { echo "PASS $1"; }
bad() { echo "FAIL $1"; fail=1; }
git_() { git -c user.name=t -c user.email=t@t -c init.defaultBranch=main -c commit.gpgsign=false "$@"; }

# ---- classify: one commit's change against its parent ------------------------------------------------------------

base() {
  rm -rf "$tmp/r"; mkdir -p "$tmp/r"; cd "$tmp/r" || exit 1
  git_ init -q .
  mkdir -p docs fab/b/package crates/x/src crates/x/tests examples
  echo a >docs/a.md; echo r >README.md; echo l >LICENSE-MIT; echo b >fab/b/BENCH.md; echo o >fab/b/ORDER.md; echo p >fab/b/package/FAB-NOTES.md
  echo s >crates/x/src/lib.rs; echo c >crates/x/README.md; echo e >examples/e.json
  git_ add -A; git_ commit -qm base
}
# class NAME WANT: commit the working tree's changes, classify against HEAD^, expect WANT
class() {
  local name=$1 want=$2 got
  git_ add -A; git_ commit -qm change --allow-empty
  git diff --raw --no-renames -z HEAD^ HEAD >"$tmp/raw"
  got=$(bash "$here/route.sh" classify "$tmp/raw" | sed -n 's/^class=//p')
  [ "$got" = "$want" ] && ok "classify: $name -> $want" || bad "classify: $name: got '$got', want $want"
}

base; echo more >>docs/a.md; class "a document edited" docs
base; echo more >>README.md; echo x >CLAUDE.md; echo m >>LICENSE-MIT; class "root README, a new root .md, a root LICENSE" docs
base; echo more >>fab/b/ORDER.md; class "fab/**/*.md outside package/" docs
base; echo more >>fab/b/BENCH.md; class "a bench sheet is code (AET-WO-3308: a test binds its ELF)" code
base; echo h >fab/b/ORDERED.sha256; class "an ordered-files list is code" code
base; git rm -q docs/a.md; class "a document deleted" docs
base; mkdir -p "docs/with space"; printf 'n\n' >"docs/with space/new
line.md"; class "a document whose path holds a space and a newline" docs
base; echo more >>fab/b/package/FAB-NOTES.md; class "fab/**/package/** is code" code
base; echo more >>crates/x/README.md; class "a crate README is code (README* is root-only)" code
base; mkdir -p crates/x/data; echo l >crates/x/data/LICENSE.md; class "a nested LICENSE.md is code" code
base; echo more >>examples/e.json; class "examples/ is code" code
base; mkdir -p .github/workflows ci; echo w >.github/workflows/ci.yml; echo s >ci/SHARDS; class "a workflow and ci/ are code" code
base; echo more >>docs/a.md; echo more >>crates/x/src/lib.rs; class "docs and code together" code
base; git mv crates/x/src/lib.rs docs/lib.md; class "a source renamed into docs/ (its old path is code)" code
base; git mv docs/a.md crates/x/src/a.rs; class "a document renamed into a crate" code
base; ln -s ../crates/x/src/lib.rs docs/link.md; class "a symlink under docs/" code
base; rm docs/a.md; ln -s ../crates/x/src/lib.rs docs/a.md; class "a document replaced by a symlink" code
base; chmod +x docs/a.md; class "a document made executable" code
base; class "an empty diff (unknown is code)" code

# decide: the event and the merge commit
base; git_ checkout -qb pr; echo more >>docs/a.md; git_ commit -qam pr; git_ checkout -q main; echo e2 >>examples/e.json; git_ commit -qam main
git_ merge -q --no-ff -m merge pr
prhead=$(git rev-parse pr)
got=$(PR_HEAD_SHA=$prhead bash "$here/route.sh" decide pull_request | sed -n 's/^class=//p'); [ "$got" = docs ] && ok "decide: a docs-only PR's merge commit, main having moved -> docs" || bad "decide: merge: got '$got'"
got=$(bash "$here/route.sh" decide pull_request | sed -n 's/^class=//p'); [ "$got" = code ] && ok "decide: no PR head given -> code" || bad "decide: no head: got '$got'"
got=$(PR_HEAD_SHA=$(git rev-parse main^) bash "$here/route.sh" decide pull_request | sed -n 's/^class=//p'); [ "$got" = code ] && ok "decide: a second parent that is not the PR head (parents swapped, say) -> code" || bad "decide: wrong head: got '$got'"
got=$(PR_HEAD_SHA=$prhead bash "$here/route.sh" decide push | sed -n 's/^class=//p'); [ "$got" = code ] && ok "decide: a push runs everything" || bad "decide: push: got '$got'"
got=$(bash "$here/route.sh" decide workflow_dispatch | sed -n 's/^class=//p'); [ "$got" = code ] && ok "decide: a dispatch runs everything" || bad "decide: dispatch: got '$got'"
git_ checkout -qb pr2 main; echo p2 >>docs/a.md; git_ commit -qam pr2
git_ checkout -qb third main; echo t >third.md; git_ add -A; git_ commit -qm third
git_ checkout -qb octo main; git_ merge -q --no-ff -m octopus pr2 third
[ "$(git rev-list --parents -n 1 HEAD | wc -w)" -eq 4 ] || bad "decide: the octopus fixture has no three parents"
got=$(PR_HEAD_SHA=$(git rev-parse HEAD^2) bash "$here/route.sh" decide pull_request | sed -n 's/^class=//p'); [ "$got" = code ] && ok "decide: an octopus merge whose second parent is the PR head -> code" || bad "decide: octopus: got '$got'"
git_ checkout -q pr
got=$(PR_HEAD_SHA=$prhead bash "$here/route.sh" decide pull_request | sed -n 's/^class=//p'); [ "$got" = code ] && ok "decide: a HEAD that is not a merge -> code" || bad "decide: no merge: got '$got'"

# ---- the inventory guard, on a scratch tree with its own copy of route.sh ----------------------------------------

inv() {
  rm -rf "$tmp/i"; mkdir -p "$tmp/i/ci" "$tmp/i/docs" "$tmp/i/crates/d/src" "$tmp/i/crates/d/tests/common"; cd "$tmp/i" || exit 1
  cp "$here/route.sh" ci/
  printf '[package]\nname = "demo"\n' >crates/d/Cargo.toml
  echo a >docs/a.md; echo r >README.md
  printf 'fn f() { let _ = std::fs::read("../../docs/a.md"); }\n' >crates/d/tests/reads.rs
  printf '// docs/a.md in a comment\n/* README.md in a /* nested */ comment */\nfn g() { let _ = "crates/d/README.md"; }\n' >crates/d/tests/quiet.rs
  printf 'pub fn s() -> char { \x27"\x27 }\n' >crates/d/src/lib.rs
  printf 'demo\treads\t../../docs/a.md\t1\treads\n' >ci/list.tsv
  git_ init -q .; git_ add -A; git_ commit -qm t
}
expect() { # NAME WANT(ok|refused) [PATTERN...]: every pattern in the refusal
  local name=$1 want=$2 out rc p hit=1; shift 2
  git_ add -A >/dev/null 2>&1; git_ commit -qm t --allow-empty
  out=$(bash ci/route.sh check ci/list.tsv 2>&1); rc=$?
  for p in "$@"; do grep -qF -- "$p" <<<"$out" || hit=0; done
  if [ "$want" = ok ]; then [ $rc -eq 0 ] && ok "guard: $name" || bad "guard: $name: $out"
  elif [ $rc -ne 0 ] && [ $hit -eq 1 ]; then ok "guard: $name: refused ($(cut -c1-110 <<<"$out"))"
  else bad "guard: $name: rc=$rc out=$out"; fi
}
inv; expect "the listed reader passes; comments, a crate path and a char quote are ignored" ok
inv; printf 'fn h() { let _ = r#"see docs/a.md"#; let _ = format!("{root}/docs/a.md"); }\n' >crates/d/tests/more.rs
expect "a raw string and a placeholder path, unlisted, are refused by name" refused "demo more {root}/docs/a.md"
inv; printf 'fn h() { let _ = root().join("docs").join("x"); }\n' >crates/d/tests/walk.rs
expect "a join of \"docs\" is refused by name" refused "demo walk docs"
inv; printf 'fn h() { let _ = "LICENSE"; let _ = "README.md"; }\n' >crates/d/tests/bare.rs
expect "a bare root name is refused by name" refused "demo bare README.md"
inv; printf 'pub fn r() -> &str { "docs/a.md" }\n' >crates/d/tests/common/mod.rs; printf 'mod common;\n' >crates/d/tests/uses.rs
expect "a helper module's literal is charged to the binary that includes it" refused "demo uses docs/a.md"
inv; printf 'pub fn r() -> &str { "docs/a.md" }\n' >crates/d/tests/common/mod.rs
expect "a helper no binary includes is refused as unmapped" refused "unmapped! crates/d/tests/common/mod.rs" "no docs-only run can judge"
inv; printf 'pub const D: &str = include_str!(\n    "../../../docs/a.md"\n);\n' >>crates/d/src/lib.rs
expect "an include_str! of a docs-only path is refused" refused "include! crates/d/src/lib.rs docs/a.md" "no docs-only run can judge"
inv; printf '#[path = "../../../docs/gen.rs"]\npub mod gen;\npub const R: &str = "docs/a.md";\n' >>crates/d/src/lib.rs; echo 'pub fn f() {}' >docs/gen.rs
expect "production src/ naming a docs-only path (a #[path] module, a constant) is refused" refused "src! crates/d/src/lib.rs" "no docs-only run can judge"
inv; printf 'fn h() { let _ = "../../docs/a.md"; let _ = "../../docs/a.md"; }\n' >crates/d/tests/reads.rs
expect "a second use of a listed literal changes its count and is refused" refused "demo reads ../../docs/a.md 2"
inv; printf '#[path = "common/stub.rs"]\nmod stub;\n' >crates/d/tests/viapath.rs; printf 'pub fn r() -> &str { "docs/a.md" }\n' >crates/d/tests/common/stub.rs; printf 'mod common;\n' >crates/d/tests/viamod.rs
expect "a helper included by #[path] is charged to that binary too" refused "demo viapath docs/a.md"
inv; printf 'fn h() { let _ = cr"x\\"; let _ = "docs\\x2fa.md"; let _ = "\\u{64}ocs/a.md"; }\n' >crates/d/tests/esc.rs
expect "escapes decoded, cr raw strings: \\x2f and \\u{64} found" refused "demo esc docs/a.md 2"
inv; printf 'fn h() { let _ = "../../do\\\n     cs/a.md"; }\n' >crates/d/tests/cont.rs
expect "a line continuation joined" refused "demo cont ../../docs/a.md"
inv; mkdir -p fab/b; echo o >fab/b/ORDER.md; printf 'fn h() { let _ = std::fs::read_dir("../../fab/b"); }\n' >crates/d/tests/walkfab.rs
expect "a walker of a fab directory holding a document is refused" refused "demo walkfab ../../fab/b"
inv; mkdir -p boards/x/src; printf 'const R: &str = include_str!("../../../README.md");\n' >boards/x/src/lib.rs
expect "a Rust source outside crates/ naming a document is refused" refused "boards/x/src/lib.rs" "no docs-only run can judge"
inv; printf 'fn main() { println!("cargo:rerun-if-changed=../../README.md"); let _ = "../../README.md"; }\n' >crates/d/build.rs
expect "a build script naming a root document is refused" refused "build! crates/d/build.rs" "no docs-only run can judge"
inv; mkdir -p bridge/src; printf 'fn f() { let _ = "docs/a.md"; }\n' >bridge/src/main.rs
expect "the bridge naming a document is refused" refused "bridge! bridge/src/main.rs" "no docs-only run can judge"
inv; printf 'demo\tgone\tdocs/a.md\t1\treads\n' >>ci/list.tsv
expect "a row the inventory does not find is refused" refused "demo gone docs/a.md"
inv; printf 'demo\treads\t../../docs/a.md\treads\n' >ci/list.tsv
expect "a malformed row is refused" refused "want package<TAB>target<TAB>literal<TAB>occurrences<TAB>reads|cites"
inv; git_ rm -q docs/a.md README.md
expect "a tree with no docs-only file is refused" refused "no docs-only file"

cd "$here" || exit 1
[ "$fail" -eq 0 ] && echo "test-route: all passed" || { echo "test-route: FAILED"; exit 1; }
