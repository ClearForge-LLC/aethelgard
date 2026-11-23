#!/usr/bin/env bash
# AET-WO-1917 §1.1 (6): the result-line forms in raw tools-shard job logs (`gh api .../jobs/ID/logs`), each line read past
# GitHub's timestamp and `run`'s stamp, ANSI stripped. Classes:
#   whole       `test NAME ... RESULT` alone on its line (what libtest writes)
#   tool-before `TOOL test NAME ... RESULT`: times() matches its end today
#   name-open   `[TOOL ]test NAME ... TOOL` with no result at its end: libtest's name, then another process's line
#   lone-result a line that is exactly `ok`, `FAILED` or `ignored`, or ends in one with no `test NAME ...` before it
#               (the far half of a name-open); printed with the line before it
# Usage: bash spike/1917/split-forms.sh LOG...
set -euo pipefail
export LC_ALL=C
for f in "$@"; do
  awk -v f="${f##*/}" '
    { line = $0; sub(/^[0-9T:.-]+Z /, "", line); sub(/^[0-9]+[.][0-9]+\t/, "", line); gsub(/\033\[[0-9;]*[A-Za-z]/, "", line) }
    line ~ /^test [A-Za-z0-9_:]+( - should panic)? [.][.][.] (ok|FAILED|ignored)$/ { c["whole"]++; prev = line; next }
    match(line, /test [A-Za-z0-9_:]+( - should panic)? [.][.][.] (ok|FAILED|ignored)$/) { c["tool-before"]++; print f "\ttool-before\t" substr(line, 1, 160); prev = line; next }
    match(line, /test [A-Za-z0-9_:]+( - should panic)? [.][.][.] /) && line !~ /[.][.][.] (ok|FAILED|ignored)$/ { c["name-open"]++; print f "\tname-open\t" substr(line, 1, 160); prev = line; next }
    line ~ /^(ok|FAILED|ignored)$/ || (line ~ /(ok|FAILED|ignored)$/ && prevopen) { c["lone-result"]++; print f "\tlone-result\t" substr(line, 1, 80) "\t(after: " substr(prev, 1, 100) ")" }
    { prevopen = (prev ~ /[.][.][.] [^.]*$/); prev = line }
    END { for (k in c) printf "%s\tcount\t%s=%d\n", f, k, c[k] }
  ' "$f"
done
