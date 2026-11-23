#!/bin/bash
# AET-WO-2310 §1.1 sweep T (item 5), inside the tools image: what libngspice and the toolchain are on disk, and
# where they come from. Read-only; prints one fact a line.
set -u
B=${B:-/cache/target-main/debug}
echo "== libngspice"
ldd $B/aethelgard-bridge | grep -i ngspice
L=$(ldd $B/aethelgard-bridge | awk '/libngspice/{print $3}')
echo "mapped path: $L"; echo "realpath: $(realpath $L)"; ls -la /usr/lib/x86_64-linux-gnu/libngspice*
echo "owner: $(dpkg -S "$(realpath $L)" 2>&1)"
P=$(dpkg -S "$(realpath $L)" 2>/dev/null | cut -d: -f1)
dpkg-query -W -f='package: ${Package} ${Version} ${Architecture} status=${Status}\n' "$P"
apt-cache policy "$P" 2>/dev/null | head -8
grep -h "$(realpath $L | sed 's#^/##')" /var/lib/dpkg/info/$P*.md5sums
md5sum "$(realpath $L)"; sha256sum "$(realpath $L)"
echo "ngspice binary: $(command -v ngspice) $(sha256sum $(command -v ngspice) | cut -c1-64) owner $(dpkg -S $(realpath $(command -v ngspice)) 2>&1)"
ls /var/log/apt 2>/dev/null; zgrep -h -i ngspice /var/log/apt/history.log* /var/log/dpkg.log* 2>/dev/null | head -8
echo "== toolchain"
echo "PATH=$PATH"; echo "RUSTUP_HOME=${RUSTUP_HOME:-} CARGO_HOME=${CARGO_HOME:-}"
for t in rustc cargo rustup; do W=$(command -v $t); echo "$t: $W -> $(realpath $W) $(stat -c %s $(realpath $W)) B sha256 $(sha256sum $(realpath $W) | cut -c1-64)"; done
rustup toolchain list -v 2>&1; rustc -vV 2>&1
T=$(rustc --print sysroot); echo "sysroot: $T"
ls -la $T/bin; ls -la $T/lib | head -20
for f in $T/bin/rustc $T/lib/librustc_driver-*.so; do echo "$(stat -c %s $f) B $(sha256sum $f | cut -c1-64) $f"; done
echo "readelf NEEDED of rustc:"; readelf -d $T/bin/rustc 2>/dev/null | grep NEEDED
find $T/lib/rustlib -maxdepth 1 | sort
find $T/lib/rustlib -name 'rust-lld' -o -name 'gcc-ld' | head
ls $T/lib/rustlib/*/ 2>/dev/null | head -20
echo "component manifests:"; ls $T/lib/rustlib/ ; head -c 600 $T/lib/rustlib/multirust-channel-manifest.toml 2>/dev/null; echo; cat $T/lib/rustlib/components 2>/dev/null
grep -h -c sha256 $T/lib/rustlib/multirust-channel-manifest.toml 2>/dev/null
echo "tree: $(du -sb $T | cut -f1) B, $(find $T -type f | wc -l) files"
s=$(date +%s.%N); find $T -type f -print0 | LC_ALL=C sort -z | xargs -0 sha256sum | sha256sum; e=$(date +%s.%N); echo "tree digest took $(echo "$e - $s" | bc) s"
s=$(date +%s.%N); find $T/lib/rustlib/thumbv7em-none-eabihf -type f -print0 | LC_ALL=C sort -z | xargs -0 sha256sum | sha256sum; e=$(date +%s.%N); echo "thumbv7em target dir: $(du -sb $T/lib/rustlib/thumbv7em-none-eabihf | cut -f1) B, digest took $(echo "$e - $s" | bc) s"
echo "== a generated board's linker"
G=$(ls -d /cache/runs/fw/gen-min 2>/dev/null); [ -n "$G" ] && { find $G/board -name config.toml -path '*.cargo*' -exec cat {} \; ; grep -rn "linker\|link-arg" $G/board/.cargo $G/board/*/build.rs 2>/dev/null | head; }
