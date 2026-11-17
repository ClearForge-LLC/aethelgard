#!/bin/bash
# Inside the image: the 100 ms matrix, each case twice, then sha256 of each trace.
set -u
for c in healthy weak; do for k in 1 2; do
  echo "== $c run $k"; bash /repo/spike/2303/sh/run.sh $c 100000000 /cache/runs/$c-100ms-$k
done; done
for c in healthy weak; do for t in samples records events; do
  sha256sum /cache/runs/$c-100ms-*/trace-$t/trace.cbor | awk '{print $1}' | uniq -c | sed "s/^/$c $t /"
done; done
