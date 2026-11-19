#!/usr/bin/env bash
# AET-WO-1911 §1.1: route a pull request whose diff touches only documents. Bash, awk and git only (RUL-12).
#
# Docs-only is a positive list (is_docs below); anything else is code, and so is anything unknown:
#   docs/**; *.md, README* and LICENSE* at the repository root; *.md under fab/ outside any fab/**/package/**.
#   README* and LICENSE* are root-only: crate-level ones are package inputs, and kicad-lib include_bytes! its
#   data/aethelgard/README.md and data/kicad-10.0.7/LICENSE.md (FEEDBACK, the positive list's reading).
#
#   route.sh classify RAW         RAW is `git diff --raw --no-renames -z` output: prints each path's class, then
#                                 `class=docs` only when there is at least one path and every one is a regular file
#                                 (mode 100644, or absent on one side) on the positive list; else `class=code`
#   route.sh decide EVENT         the route job's step: on pull_request, classifies the merge commit against its
#                                 first parent; any other event, or a HEAD that is not a two-parent merge, is code
#   route.sh inventory            every string literal in a Rust source under crates/ or bridge/ that names a
#                                 docs-only path, as package<TAB>target<TAB>literal (target: a test target, `unit`
#                                 for src/, or a refusal kind); comments are skipped
#   route.sh check LIST           the inventory guard: every inventory row is in LIST (ci/docs-tests.tsv) and every
#                                 LIST row is in the inventory; refuses by name a build, example or bridge source
#                                 or an include_str!/include_bytes! that names a docs-only path
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
  local raw=$1 meta path om nm n=0 code=0 why
  readable "$raw"
  while IFS= read -r -d '' meta && IFS= read -r -d '' path; do
    n=$((n + 1))
    # `:OLDMODE NEWMODE OLDSHA NEWSHA STATUS`
    read -r om nm _ _ _ <<<"${meta#:}"
    why=""
    case $om in 100644 | 000000) ;; *) why="mode $om" ;; esac
    case $nm in 100644 | 000000) ;; *) why="mode $nm" ;; esac
    if [ -z "$why" ] && ! is_docs "$path"; then why="not on the docs-only list"; fi
    if [ -n "$why" ]; then code=1; printf 'code\t%s\t(%s)\n' "$path" "$why"; else printf 'docs\t%s\n' "$path"; fi
  done <"$raw"
  [ "$n" -gt 0 ] || { echo "route: no changed path; unknown is code"; code=1; }
  if [ "$code" -eq 0 ]; then echo "class=docs"; else echo "class=code"; fi
}

decide() {
  local event=$1 parents out class
  out=$(mktemp -d); trap 'rm -rf "$out"' RETURN
  if [ "$event" != pull_request ]; then
    echo "route: event $event runs the full suite"
    class=code
  else
    parents=$(git rev-list --parents -n 1 HEAD | wc -w)
    if [ "$parents" -ne 3 ]; then
      echo "route: HEAD $(git rev-parse HEAD) is not a two-parent merge; unknown is code"
      class=code
    else
      echo "route: the merge commit $(git rev-parse HEAD) against its first parent $(git rev-parse HEAD^1)"
      git diff --raw --no-renames -z HEAD^1 HEAD >"$out/raw"
      classify "$out/raw" >"$out/class"
      grep -v '^class=' "$out/class" || true
      class=$(sed -n 's/^class=//p' "$out/class")
    fi
  fi
  case $class in
    docs) echo "route: DOCS-ONLY: gates runs cargo fmt and ci/docs-tests.tsv's tests; tools-build, tools-shard and bench-package are skipped; tools reports the routing" ;;
    code) echo "route: CODE: every job runs" ;;
    *) die "no class" ;;
  esac
  echo "class=$class"
}

# The docs-only files, one per line.
docs_files() { git -C "$here/.." ls-files | while IFS= read -r f; do if is_docs "$f"; then printf '%s\n' "$f"; fi; done; }

# lits FILE...: file<TAB>kind<TAB>literal for every string literal outside comments; kind is `include` for the
# literal argument of include_str!/include_bytes!, else `lit`. Handles escapes, raw strings, char literals and
# nested block comments; newlines and tabs inside a literal become spaces.
lits() {
  awk '
    function emit(   k) { gsub(/[\t\n]/, " ", buf); k = (tail ~ /include_(str|bytes)!\($/) ? "include" : "lit"; print FILENAME "\t" k "\t" buf }
    FNR == 1 { st = 0; depth = 0; tail = "" }
    {
      line = $0; n = length(line); i = 1
      if (st == 1 || st == 2) buf = buf " "
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
          if (c == "r" && (i == 1 || substr(line, i - 1, 1) !~ /[A-Za-z0-9_]/ || (i > 1 && substr(line, i - 1, 1) == "b" && (i == 2 || substr(line, i - 2, 1) !~ /[A-Za-z0-9_]/)))) {
            j = i + 1; h = 0
            while (substr(line, j, 1) == "#") { h++; j++ }
            if (substr(line, j, 1) == "\"") { st = 2; hashes = h; buf = ""; i = j + 1; continue }
          }
          if (c == "\"") { st = 1; buf = ""; i++; continue }
          if (c !~ /[ \t]/) { tail = tail c; if (length(tail) > 32) tail = substr(tail, length(tail) - 31) }
          i++; continue
        }
        if (st == 1) {
          if (c == "\\") { buf = buf substr(line, i + 1, 1); i += 2; continue }
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
  local root dfile files
  root=$(cd "$here/.." && pwd)
  dfile=$(mktemp); trap 'rm -f "$dfile"' RETURN
  docs_files >"$dfile"
  [ -s "$dfile" ] || die "no docs-only file in the tree"
  mapfile -t files < <(git -C "$root" ls-files 'crates/*.rs' 'bridge/*.rs')
  [ "${#files[@]}" -gt 0 ] || die "no Rust source under crates/ or bridge/"
  (cd "$root" && lits "${files[@]}") | awk -F'\t' -v df="$dfile" -v root="$root" '
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
    FILENAME == df { full[$0] = 1; b[base($0)] = 1
      d = $0; while (sub(/\/[^\/]*$/, "", d)) if (d ~ /^docs(\/|$)/) dir[d] = 1
      next }
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
      target_of "$root" "$f" | while IFS= read -r t; do printf '%s\t%s\n' "$t" "$lit"; done
    done | LC_ALL=C sort -u
}

# target_of ROOT FILE: package<TAB>target for each test target FILE is compiled into, or a refusal kind.
target_of() {
  local root=$1 f=$2 dir pkg rest sub t found=0
  case $f in
    bridge/*) printf 'bridge!\t%s\n' "$f"; return ;;
    crates/*/*) ;;
    *) printf 'unmapped!\t%s\n' "$f"; return ;;
  esac
  dir=${f#crates/}; dir=${dir%%/*}; rest=${f#crates/"$dir"/}
  pkg=$(awk -F'"' '/^\[/ { sec = $0 } sec == "[package]" && /^name *=/ { print $2; exit }' "$root/crates/$dir/Cargo.toml")
  [ -n "$pkg" ] || die "crates/$dir/Cargo.toml names no package"
  case $rest in
    src/*) printf '%s\tunit\n' "$pkg" ;;
    tests/*/*)
      sub=${rest#tests/}; sub=${sub%%/*}
      for t in "$root/crates/$dir"/tests/*.rs; do
        if grep -Eq "^[[:space:]]*(pub[[:space:]]+)?mod[[:space:]]+$sub[[:space:]]*;" "$t"; then
          found=1; t=${t##*/}; printf '%s\t%s\n' "$pkg" "${t%.rs}"
        fi
      done
      [ "$found" -eq 1 ] || printf 'unmapped!\t%s\n' "$f" ;;
    tests/*.rs) t=${rest#tests/}; printf '%s\t%s\n' "$pkg" "${t%.rs}" ;;
    build.rs) printf 'build!\t%s\n' "$f" ;;
    examples/*) printf 'example!\t%s\n' "$f" ;;
    *) printf 'unmapped!\t%s\n' "$f" ;;
  esac
}

# The list: package<TAB>target<TAB>literal<TAB>reads|cites. A `cites` row names a docs-only name that the target
# does not read (a crate-level README, a citation in a message); `reads` rows run on a docs-only pull request.
rows() {
  local list=$1
  readable "$list"
  awk -F'\t' -v lf="$list" '
    /^#/ || NF == 0 { next }
    NF != 4 || ($4 != "reads" && $4 != "cites") { print "route: " lf ":" FNR ": want package<TAB>target<TAB>literal<TAB>reads|cites" > "/dev/stderr"; bad = 1; exit 1 }
    { print $1 "\t" $2 "\t" $3 "\t" $4 }
    END { if (bad) exit 1 }' "$list"
}

check() {
  local list=$1 inv have refused missing stale
  inv=$(inventory) || exit 1
  have=$(rows "$list") || exit 1
  refused=$(awk -F'\t' '$1 ~ /!$/' <<<"$inv")
  [ -z "$refused" ] || die "a source no docs-only run can judge names a docs-only path (a build input, an example, the bridge, or a file no test target includes): $(tr '\t\n' ' ;' <<<"$refused")"
  missing=$(comm -23 <(printf '%s\n' "$inv" | sort -u) <(cut -f1-3 <<<"$have" | sort -u) | sed '/^$/d')
  stale=$(comm -13 <(printf '%s\n' "$inv" | sort -u) <(cut -f1-3 <<<"$have" | sort -u) | sed '/^$/d')
  [ -z "$missing" ] || die "a test target names a docs-only path and is not in $(basename "$list") (classify it reads or cites): $(tr '\t\n' ' ;' <<<"$missing")"
  [ -z "$stale" ] || die "$(basename "$list") lists a row the inventory no longer finds (renamed or removed?): $(tr '\t\n' ' ;' <<<"$stale")"
  echo "route: inventory ok: $(wc -l <<<"$inv") rows; docs-only set: $(awk -F'\t' '$4 == "reads" { print $1 " " $2 }' <<<"$have" | sort -u | tr '\n' ';')"
}

docs_tests() {
  local list=$1 pkg t log status=0 args
  mapfile -t targets < <(rows "$list" | awk -F'\t' '$4 == "reads" { print $1 "\t" $2 }' | sort -u)
  [ "${#targets[@]}" -gt 0 ] || die "$list has no reads row"
  log=$(mktemp); trap 'rm -f "$log"' RETURN
  for row in "${targets[@]}"; do
    pkg=${row%%"$TAB"*}; t=${row#*"$TAB"}
    if [ "$t" = unit ]; then args=(--lib --bins); else args=(--test "$t"); fi
    echo "route: docs-only test: $pkg $t"
    set +e
    cargo test --locked -p "$pkg" "${args[@]}" 2>&1 | tee "$log"
    [ "${PIPESTATUS[0]}" -eq 0 ] || status=1
    set -e
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
  is_docs) is_docs "$@" ;;
  *) echo "usage: route.sh classify RAW | decide EVENT | inventory | check LIST | docs-tests LIST" >&2; exit 2 ;;
esac
