#!/usr/bin/env bash
# AET-WO-1911 §1.1: route a pull request whose diff touches only documents. Bash, awk and git only (RUL-12).
#
# Docs-only is a positive list (is_docs below); anything else is code, and so is anything unknown.
#   README* and LICENSE* are root-only: crate-level ones are package inputs, and kicad-lib include_bytes! its
#   data/aethelgard/README.md and data/kicad-10.0.7/LICENSE.md (FEEDBACK, the positive list's reading).
#
#   route.sh classify RAW         RAW is `git diff --raw --no-renames -z` output: prints each path's class, then
#                                 `class=docs` only when there is at least one path and every one is a regular file
#                                 (mode 100644, or absent on one side) on the positive list; else `class=code`
#   route.sh decide EVENT         the route job's step: on pull_request, classifies the merge commit against its
#                                 first parent; any other event, a HEAD that is not a two-parent merge, or one whose
#                                 second parent is not $PR_HEAD_SHA (the pull request's head), is code
#   route.sh inventory            every string literal in a Rust source in the tree that names a docs-only path,
#                                 directory or name, as package<TAB>target<TAB>literal<TAB>uses (target: a test target,
#                                 or a refusal kind ending in `!`); comments are skipped
#   route.sh check LIST           the inventory guard: the inventory equals LIST (ci/docs-tests.tsv), and no refusal
#   route.sh docs-tests LIST      run LIST's `reads` targets on the host; each must run at least one test, all pass
set -euo pipefail
export LC_ALL=C
here=$(cd "$(dirname "$0")" && pwd)
TAB=$(printf '\t')

die() { echo "route: $*" >&2; exit 1; }
readable() { [ -r "$1" ] || die "cannot read $1"; }

# is_docs PATH: exit 0 when PATH is on the positive list. In a case pattern `*` also matches `/`.
is_docs() {
  case $1 in
    fab/package/* | fab/*/package/*) return 1 ;;
    docs/*) return 0 ;;
    fab/*.md) return 0 ;;
    */*) return 1 ;;
    *.md | README* | LICENSE*) return 0 ;;
    *) return 1 ;;
  esac
}

classify() {
  local raw=$1 meta path om nm m n=0 code=0 why
  readable "$raw"
  while IFS= read -r -d '' meta && IFS= read -r -d '' path; do
    n=$((n + 1))
    # `:OLDMODE NEWMODE OLDSHA NEWSHA STATUS`
    read -r om nm _ _ _ <<<"${meta#:}"
    why=""
    for m in $om $nm; do case $m in 100644 | 000000) ;; *) why="mode $m" ;; esac; done
    if [ -z "$why" ] && ! is_docs "$path"; then why="not on the docs-only list"; fi
    if [ -n "$why" ]; then code=1; printf 'code\t%s\t(%s)\n' "$path" "$why"; else printf 'docs\t%s\n' "$path"; fi
  done <"$raw"
  [ "$n" -gt 0 ] || { echo "route: no changed path; unknown is code"; code=1; }
  if [ "$code" -eq 0 ]; then echo "class=docs"; else echo "class=code"; fi
}

decide() {
  local event=$1 parents res class
  if [ "$event" != pull_request ]; then
    echo "route: event $event runs the full suite"
    class=code
  else
    parents=$(git rev-list --parents -n 1 HEAD | wc -w)
    if [ "$parents" -ne 3 ]; then
      echo "route: HEAD $(git rev-parse HEAD) is not a two-parent merge; unknown is code"
      class=code
    elif [ -z "${PR_HEAD_SHA:-}" ] || [ "$(git rev-parse HEAD^2)" != "$PR_HEAD_SHA" ]; then
      echo "route: HEAD's second parent is not the pull request's head (${PR_HEAD_SHA:-unset}); unknown is code"
      class=code
    else
      echo "route: the merge commit $(git rev-parse HEAD) against its first parent $(git rev-parse HEAD^1)"
      res=$(classify <(git diff --raw --no-renames -z HEAD^1 HEAD))
      grep -v '^class=' <<<"$res" || true
      class=$(sed -n 's/^class=//p' <<<"$res")
    fi
  fi
  case $class in
    docs) echo "route: DOCS-ONLY: gates runs cargo fmt and ci/docs-tests.tsv's tests; tools-build, tools-shard and bench-package are skipped; tools reports the routing" ;;
    code) echo "route: CODE: every job runs" ;;
    *) die "no class" ;;
  esac
  echo "class=$class"
}

# lits FILE...: file<TAB>kind<TAB>literal for every string literal outside comments; kind is `include` for the
# literal argument of include_str!/include_bytes!, else `lit`. Handles escapes, raw strings, char literals and
# nested block comments; newlines and tabs inside a literal become spaces.
lits() {
  awk '
    function hex(h,   v, k, d) { v = 0; h = tolower(h); for (k = 1; k <= length(h); k++) { d = index("0123456789abcdef", substr(h, k, 1)) - 1; v = v * 16 + d }; return v }
    function emit(   k) { gsub(/[\t\n]/, " ", buf); k = (tail ~ /include_(str|bytes)!\($/) ? "include" : "lit"; print FILENAME "\t" k "\t" buf }
    FNR == 1 { st = 0; depth = 0; tail = "" }
    {
      line = $0; n = length(line); i = 1
      if (st == 2 || (st == 1 && !cont)) buf = buf " "
      if (st == 1 && cont) { sub(/^[ \t]+/, "", line); n = length(line) }
      cont = 0
      while (i <= n) {
        c = substr(line, i, 1)
        if (st == 0) {
          two = substr(line, i, 2)
          if (two == "//") break
          if (two == "/*") { st = 3; depth = 1; i += 2; continue }
          if (c == "\x27") {
            if (substr(line, i, 3) == "\x27\"\x27") { i += 3; continue }
            if (substr(line, i, 4) == "\x27\\\"\x27") { i += 4; continue }
            i++; continue
          }
          if (c == "r" && (i == 1 || substr(line, i - 1, 1) !~ /[A-Za-z0-9_]/ || (i > 1 && substr(line, i - 1, 1) ~ /[bc]/ && (i == 2 || substr(line, i - 2, 1) !~ /[A-Za-z0-9_]/)))) {
            j = i + 1; h = 0
            while (substr(line, j, 1) == "#") { h++; j++ }
            if (substr(line, j, 1) == "\"") { st = 2; hashes = h; buf = ""; i = j + 1; continue }
          }
          if (c == "\"") { st = 1; buf = ""; i++; continue }
          if (c !~ /[ \t]/) { tail = tail c; if (length(tail) > 32) tail = substr(tail, length(tail) - 31) }
          i++; continue
        }
        if (st == 1) {
          if (c == "\\") {
            e = substr(line, i + 1, 1)
            if (i == n) { cont = 1; i++; continue }
            if (e == "x" && substr(line, i + 2, 2) ~ /^[0-7][0-9A-Fa-f]$/) { buf = buf sprintf("%c", hex(substr(line, i + 2, 2))); i += 4; continue }
            if (e == "u" && match(substr(line, i + 2), /^\{[0-9A-Fa-f]+\}/)) { v = hex(substr(line, i + 3, RLENGTH - 2)); buf = buf (v < 128 ? sprintf("%c", v) : "?"); i += 2 + RLENGTH; continue }
            buf = buf e; i += 2; continue }
          if (c == "\"") { emit(); st = 0; tail = tail "\""; i++; continue }
          buf = buf c; i++; continue
        }
        if (st == 2) {
          if (c == "\"") { close_ = "\""; for (k = 0; k < hashes; k++) close_ = close_ "#"
            if (substr(line, i, hashes + 1) == close_) { emit(); st = 0; tail = tail "\""; i += hashes + 1; continue } }
          buf = buf c; i++; continue
        }
        if (st == 3) {
          two = substr(line, i, 2)
          if (two == "*/") { depth--; i += 2; if (depth == 0) st = 0; continue }
          if (two == "/*") { depth++; i += 2; continue }
          i++; continue
        }
      }
    }' "$@"
}

inventory() {
  local root dfile files paths
  root=$(cd "$here/.." && pwd)
  dfile=$(mktemp) paths=$(mktemp); trap 'rm -f "$dfile" "$paths"' RETURN
  git -C "$root" ls-files | while IFS= read -r f; do if is_docs "$f"; then printf '%s\n' "$f"; fi; done >"$dfile"
  [ -s "$dfile" ] || die "no docs-only file in the tree"
  mapfile -t files < <(git -C "$root" ls-files '*.rs')
  [ "${#files[@]}" -gt 0 ] || die "no Rust source in the tree"
  path_map "$root" >"$paths"
  (cd "$root" && lits "${files[@]}") | awk -F'\t' -v df="$dfile" '
    # norm: strip leading "/", "./" and "../" runs; a docs-only path is matched repository-relative.
    function norm(p) { while (sub(/^(\.\.\/|\.\/|\/)/, "", p)) ; return p }
    # resolve: DIR/P with . and .. folded, as include_str! resolves it.
    function resolve(dir, p,   parts, out, n, i, m) {
      if (p ~ /^\//) return "/" p
      n = split(dir "/" p, parts, "/"); m = 0
      for (i = 1; i <= n; i++) { if (parts[i] == "" || parts[i] == ".") continue
        if (parts[i] == "..") { if (m > 0) m--; continue }
        out[++m] = parts[i] }
      p = ""; for (i = 1; i <= m; i++) p = p (i > 1 ? "/" : "") out[i]; return p }
    function base(p) { sub(/.*\//, "", p); return p }
    # A file, its name, its directory (a walker of fab/sentinel-01-bench reads BENCH.md), every docs/ directory,
    # and the bare stems of the root names (README*, LICENSE*: format!("{}.md", "README")).
    FILENAME == df { full[$0] = 1; b[base($0)] = 1
      d = $0; if (sub(/\/[^\/]*$/, "", d)) dir[d] = 1
      while (sub(/\/[^\/]*$/, "", d)) if (d ~ /^docs(\/|$)/) dir[d] = 1
      next }
    BEGIN { b["README"] = 1; b["LICENSE"] = 1 }
    {
      f = $1; kind = $2; L = $3
      if (kind == "include") {
        d = f; sub(/\/[^\/]*$/, "", d); r = resolve(d, L)
        if (r in full) { print "include\t" f "\t" r; next }
      }
      n = norm(L)
      # A docs path after a format placeholder or any prefix ("{root}/docs/roadmap.md"); a root name after a
      # placeholder ("{root}/README.md").
      s = L; sub(/^.*(^|[\/}])docs\//, "docs/", s)
      p = L; sub(/^.*\}\//, "", p)
      if ((n in full) || (n in b) || (n in dir) || (s in full) || (s in dir) || (p != L && (p in full)) || L == "md" || L == ".md") print "lit\t" f "\t" L
    }' "$dfile" - | while IFS="$TAB" read -r kind f lit; do
      if [ "$kind" = include ]; then printf 'include!\t%s\t%s\n' "$f" "$lit"; continue; fi
      target_of "$root" "$f" "$paths" | while IFS= read -r t; do printf '%s\t%s\n' "$t" "$lit"; done
    done | LC_ALL=C sort | uniq -c | awk '{ n = $1; sub(/^ *[0-9]+ /, ""); print $0 "\t" n }' | LC_ALL=C sort
}

pkg_of() { awk -F'"' '/^\[/ { sec = $0 } sec == "[package]" && /^name *=/ { print $2; exit }' "$1/Cargo.toml"; }

# path_map ROOT: file<TAB>package<TAB>binary for every `#[path = "P"]` in a crate's tests/*.rs, P resolved from tests/;
# a P that is a mod.rs stands for its whole directory (file ends in "/").
path_map() {
  local root=$1 t d pkg b p r
  for t in "$root"/crates/*/tests/*.rs; do
    d=${t%/tests/*}; pkg=$(pkg_of "$d"); b=${t##*/}; b=${b%.rs}
    { grep -oE '#\[path *= *"[^"]+"\]' "$t" || true; } | sed -E 's/^[^"]*"//; s/"\]$//' | while IFS= read -r p; do
      r=$(realpath -m --relative-to="$root" "${t%/*}/$p")
      case $r in */mod.rs) r=${r%mod.rs} ;; esac
      printf '%s\t%s\t%s\n' "$r" "$pkg" "$b"
    done
  done
}

# target_of ROOT FILE PATHMAP: package<TAB>target for each test target FILE is compiled into, or a refusal kind.
# src/ is a refusal: a docs path named in production code is a build input or a constant a test may read (AET-WO-1911
# adversarial A4, B3).
target_of() {
  local root=$1 f=$2 map=$3 dir pkg rest sub t found=0
  case $f in
    bridge/*) printf 'bridge!\t%s\n' "$f"; return ;;
    crates/*/*) ;;
    *) printf 'unmapped!\t%s\n' "$f"; return ;;
  esac
  dir=${f#crates/}; dir=${dir%%/*}; rest=${f#crates/"$dir"/}
  pkg=$(pkg_of "$root/crates/$dir")
  [ -n "$pkg" ] || die "crates/$dir/Cargo.toml names no package"
  case $rest in
    src/*) printf 'src!\t%s\n' "$f" ;;
    tests/*/*)
      sub=${rest#tests/}; sub=${sub%%/*}
      for t in "$root/crates/$dir"/tests/*.rs; do
        if grep -Eq "^[[:space:]]*(pub[[:space:]]+)?mod[[:space:]]+$sub[[:space:]]*;" "$t"; then
          found=1; t=${t##*/}; printf '%s\t%s\n' "$pkg" "${t%.rs}"
        fi
      done
      # Binaries that include it by #[path], from any crate (adversarial B2).
      while IFS="$TAB" read -r p ppkg pb; do
        case $f in "$p" | "$p"*) case $p in */ | "$f") found=1; printf '%s\t%s\n' "$ppkg" "$pb" ;; esac ;; esac
      done <"$map"
      [ "$found" -eq 1 ] || printf 'unmapped!\t%s\n' "$f" ;;
    tests/*.rs) t=${rest#tests/}; printf '%s\t%s\n' "$pkg" "${t%.rs}" ;;
    build.rs) printf 'build!\t%s\n' "$f" ;;
    *) printf 'unmapped!\t%s\n' "$f" ;;
  esac
}

# The list's rows (format: ci/docs-tests.tsv's header).
rows() {
  local list=$1
  readable "$list"
  awk -F'\t' -v lf="$list" '
    /^#/ || NF == 0 { next }
    NF != 5 || $4 !~ /^[1-9][0-9]*$/ || ($5 != "reads" && $5 != "cites") { print "route: " lf ":" FNR ": want package<TAB>target<TAB>literal<TAB>occurrences<TAB>reads|cites" > "/dev/stderr"; exit 1 }
    { print }' "$list"
}

check() {
  local list=$1 inv have h refused missing stale
  inv=$(inventory)
  have=$(rows "$list")
  refused=$(awk -F'\t' '$1 ~ /!$/' <<<"$inv")
  [ -z "$refused" ] || die "a source no docs-only run can judge names a docs-only path (production src/, a build input, an example, the bridge, or a file no test target includes): $(tr '\t\n' ' ;' <<<"$refused")"
  h=$(cut -f1-4 <<<"$have" | sort -u)
  missing=$(comm -23 <(printf '%s\n' "$inv" | sort -u) <(printf '%s\n' "$h") | sed '/^$/d')
  stale=$(comm -13 <(printf '%s\n' "$inv" | sort -u) <(printf '%s\n' "$h") | sed '/^$/d')
  [ -z "$missing" ] || die "a test target names a docs-only path, or names it more often, than $(basename "$list") says (classify it reads or cites): $(tr '\t\n' ' ;' <<<"$missing")"
  [ -z "$stale" ] || die "$(basename "$list") lists a row the inventory no longer finds (renamed or removed?): $(tr '\t\n' ' ;' <<<"$stale")"
  echo "route: inventory ok: $(wc -l <<<"$inv") rows; docs-only set: $(awk -F'\t' '$5 == "reads" { print $1 " " $2 }' <<<"$have" | sort -u | tr '\n' ';')"
}

docs_tests() {
  local list=$1 pkg t log status=0
  mapfile -t targets < <(rows "$list" | awk -F'\t' '$5 == "reads" { print $1 "\t" $2 }' | sort -u)
  [ "${#targets[@]}" -gt 0 ] || die "$list has no reads row"
  log=$(mktemp); trap 'rm -f "$log"' RETURN
  for row in "${targets[@]}"; do
    pkg=${row%%"$TAB"*}; t=${row#*"$TAB"}
    echo "route: docs-only test: $pkg $t"
    cargo test --locked -p "$pkg" --test "$t" 2>&1 | tee "$log" || status=1
    # A target that runs no test (a tier-2 binary on the host, say) judges nothing.
    awk '/^test result: [A-Za-z]+\. [0-9]+ passed; [0-9]+ failed;/ { n += $4 + $6 } END { exit !(n > 0) }' "$log" \
      || die "$pkg $t ran no test on the host; a docs-only run cannot judge it"
  done
  [ "$status" -eq 0 ] || die "a docs-only test failed"
}

cmd=${1:?usage: route.sh classify RAW | decide EVENT | inventory | check LIST | docs-tests LIST}
shift
case $cmd in
  classify | decide | inventory | check) "$cmd" "$@" ;;
  docs-tests) docs_tests "$@" ;;
  *) echo "usage: route.sh classify RAW | decide EVENT | inventory | check LIST | docs-tests LIST" >&2; exit 2 ;;
esac
