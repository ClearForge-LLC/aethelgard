#!/bin/bash
# AET-WO-2310 §1.1 sweep T, second pass, inside the tools image: the digests timed (the image has no bc) and libngspice's install history.
T=/opt/rustup/toolchains/1.99.1-x86_64-unknown-linux-gnu
for r in 1 2; do s=$(date +%s%N); h=$(find $T -type f -print0 | LC_ALL=C sort -z | xargs -0 sha256sum | sha256sum | cut -c1-64); e=$(date +%s%N); echo "tree digest run $r: $h in $(( (e-s)/1000000 )) ms"; done
s=$(date +%s%N); sha256sum /usr/lib/x86_64-linux-gnu/libngspice.so.0.0.12 >/dev/null; e=$(date +%s%N); echo "libngspice sha256 in $(( (e-s)/1000000 )) ms"
grep -n "libngspice0" /var/log/dpkg.log | head; grep -n "^Start-Date\|^Commandline\|libngspice0" /var/log/apt/history.log | cut -c1-160
grep -c . /var/log/apt/history.log; ls -la --time-style=+%F /var/log/apt/ /var/lib/dpkg/info/libngspice0*
grep -n "rust-lld\|linker" $T/lib/rustlib/thumbv7em-none-eabihf/target.json 2>/dev/null; rustc --print target-spec-json -Z unstable-options --target thumbv7em-none-eabihf 2>&1 | grep -n "linker" | head -3
rustc --target thumbv7em-none-eabihf --print link-args 2>&1 | head -2
