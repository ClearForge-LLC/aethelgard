#!/bin/bash
# Fetches each candidate datasheet into ~/.cache/aet-4300d/ds/ and logs status, type, bytes, sha256.
# Usage: fetch.sh <name> <url> [<name> <url> ...]
set -u
DS=$HOME/.cache/aet-4300d/ds
LOG=$(cd "$(dirname "$0")" && pwd)/fetch.log
mkdir -p "$DS"
while [ $# -ge 2 ]; do
  name=$1 url=$2; shift 2
  out=$DS/$name
  code=$(curl -sS -L --max-time 60 -A 'Mozilla/5.0 (X11; Linux x86_64)' -o "$out.part" -w '%{http_code}' "$url" 2>&1)
  if [ -s "$out.part" ]; then
    mv "$out.part" "$out"
    printf '%s %s %s http=%s type=%s bytes=%s sha256=%s\n' "$(date -u +%FT%TZ)" "$name" "$url" "$code" \
      "$(file -b --mime-type "$out")" "$(stat -c %s "$out")" "$(sha256sum "$out" | cut -c1-64)" | tee -a "$LOG"
  else
    rm -f "$out.part"
    printf '%s %s %s http=%s FAILED\n' "$(date -u +%FT%TZ)" "$name" "$url" "$code" | tee -a "$LOG"
  fi
done
