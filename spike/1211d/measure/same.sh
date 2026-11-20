#!/usr/bin/env bash
# Two sweep.sh directories, byte for byte: prints each differing file and exits 1 on any.
set -euo pipefail
diff -rq "$1" "$2" && echo "identical: $(find "$1" -type f | wc -l) files"
