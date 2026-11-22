#!/bin/bash
# AET-WO-1606 §1.1 (P4): which witness names the compiler cargo ran. Inside the tools image
# (~/.cache/aet-1606/sh/img.sh). Cargo runs as `build` runs it: an empty environment but for
# RUSTUP_HOME, PATH, CARGO_TARGET_DIR and RUSTUP_TOOLCHAIN, on a copy of a generated board crate,
# into a fresh target directory. Candidate (a) is `rustc -vV` under the same environment and cwd;
# candidate (b) is the target's `.rustc_info.json`.
# usage: CLI=/cache/target-X/debug/aethelgard CRATE=/cache/runs-main/s01min/gen/board OUT=/cache/p4 rustc.sh
set -u
O=${OUT:?}; rm -rf "$O"; mkdir -p "$O"
RH=/cache/rh   # a private RUSTUP_HOME; the image's /opt/rustup and Furnace's ~/.rustup are never written
if [ ! -d "$RH/toolchains/alt" ]; then
  rm -rf "$RH"; mkdir -p "$RH/toolchains"
  cp /opt/rustup/settings.toml "$RH/"
  cp -a /opt/rustup/toolchains/1.99.1-x86_64-unknown-linux-gnu "$RH/toolchains/"
  cp -al "$RH/toolchains/1.99.1-x86_64-unknown-linux-gnu" "$RH/toolchains/alt"
  rm "$RH/toolchains/alt/bin/rustc"
  cat > "$RH/toolchains/alt/bin/rustc" <<'EOF'
#!/bin/sh
# 1.99.1's rustc, except that -vV names another release and commit.
real=/cache/rh/toolchains/1.99.1-x86_64-unknown-linux-gnu/bin/rustc
case " $* " in
  *" -vV "*) "$real" "$@" | sed -e 's/^rustc 1\.99\.1 /rustc 1.99.1-alt /' -e 's/^release: .*/release: 1.99.1-alt/' \
      -e 's/^commit-hash: .*/commit-hash: a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1/'; exit 0 ;;
esac
exec "$real" "$@"
EOF
  chmod 755 "$RH/toolchains/alt/bin/rustc"
fi
P=/opt/cargo/bin:/usr/bin:/bin
# A cargo configuration outside the copy, in an ancestor of the scratch, naming another compiler.
mkdir -p "$O/anc/.cargo" "$O/anc/tmp"
printf 'build.rustc = "%s"\n' "$RH/toolchains/alt/bin/rustc" > "$O/anc/.cargo/config.toml"

vv() { sed -n -e 's/^release: //p' -e 's/^commit-hash: //p' | tr '\n' ' '; }
info() { # the -vV entry of a .rustc_info.json: release and commit, and how many entries had a release
  python3 -I -c '
import json,sys
d=json.load(open(sys.argv[1])); vs=[o["stdout"] for o in d["outputs"].values() if o["stdout"].startswith("rustc ") and "\nrelease: " in o["stdout"]]
pick=lambda s,k:[l.split(": ",1)[1] for l in s.splitlines() if l.startswith(k+": ")][0]
print(len(d["outputs"]), "entries;", len(vs), "-vV;", " ".join(pick(vs[0],k) for k in ("release","commit-hash")) if vs else "")' "$1"
}
cargo_run() { # name, dir-of-crate-copy, env...
  local n=$1 dir=$2; shift 2
  local T=$O/$n-target; rm -rf "$T"; rm -rf "$dir"; mkdir -p "$(dirname "$dir")"; cp -a "${CRATE:?}" "$dir"
  [ -e "$T/.rustc_info.json" ] && echo "$n: rustc_info before the run!"
  ( cd "$dir/firmware" && env -i PATH=$P CARGO_TARGET_DIR="$T" "$@" cargo build --release --offline --locked \
      --target thumbv7em-none-eabihf --config 'profile.release.strip="symbols"' --message-format=json-render-diagnostics \
      > "$O/$n.jsonl" 2> "$O/$n.err" ); local code=$?
  local a; a=$(cd "$dir/firmware" && env -i PATH=$P "$@" rustc -vV 2>&1 | vv)
  local c; c=$(cd "$dir/firmware" && env -i PATH=$P "$@" cargo --version 2>&1)
  local elf; elf=$(python3 -I -c '
import json,sys
for l in open(sys.argv[1]):
    m=json.loads(l)
    if m.get("reason")=="compiler-artifact" and m.get("executable"): print(m["executable"])' "$O/$n.jsonl" 2>/dev/null | tail -1); local sha=-
  [ -n "$elf" ] && [ -f "$elf" ] && sha=$(sha256sum "$elf" | cut -c1-16) && readelf -p .comment "$elf" > "$O/$n.comment" 2>&1
  echo "$n: exit $code | cargo --version: $c | (a) rustc -vV: $a | (b) $( [ -f "$T/.rustc_info.json" ] && info "$T/.rustc_info.json" || echo absent) | elf $sha"
}
echo "== plain cargo, build's environment"
cargo_run default "$O/w/default/crate" RUSTUP_HOME=/opt/rustup
cargo_run nightly "$O/w/nightly/crate" RUSTUP_HOME=/opt/rustup RUSTUP_TOOLCHAIN=nightly-2026-08-21-x86_64-unknown-linux-gnu
cargo_run private "$O/w/private/crate" RUSTUP_HOME=$RH
cargo_run alt "$O/w/alt/crate" RUSTUP_HOME=$RH RUSTUP_TOOLCHAIN=alt
cargo_run ancestor "$O/anc/tmp/crate" RUSTUP_HOME=$RH

# build asks `cargo --version` with the same CARGO_TARGET_DIR before it builds: does that write a record?
T=$O/version-only-target; rm -rf "$T"
( cd "$O/w/default/crate/firmware" && env -i PATH=$P CARGO_TARGET_DIR="$T" RUSTUP_HOME=/opt/rustup cargo --version > /dev/null )
echo "cargo --version alone: .rustc_info.json $( [ -e "$T/.rustc_info.json" ] && echo written || echo 'not written' ); target dir $( [ -d "$T" ] && echo exists || echo absent )"

echo "== the CLI's build (sandboxed): $CLI"
cli_run() { # name, env...
  local n=$1; shift
  rm -f "$O/$n.elf"
  env -i PATH=$P HOME=/root "$@" "${CLI:?}" build --crate "$CRATE" --out "$O/$n.elf" > "$O/$n.build.json" 2> "$O/$n.build.err"; local code=$?
  python3 -I -c '
import json,sys
v=json.load(open(sys.argv[1])); t=v.get("tool",{})
print(sys.argv[2]+":", "exit", sys.argv[3], "|", v.get("verdict"), "|", json.dumps(t), "|", (v.get("elf") or {}).get("sha256","-")[:16], "|", v.get("error",""))' "$O/$n.build.json" "$n" "$code" 2>/dev/null || { echo "$n: exit $code"; head -c 400 "$O/$n.build.json"; }
}
cli_run cli-default RUSTUP_HOME=/opt/rustup
cli_run cli-nightly RUSTUP_HOME=/opt/rustup RUSTUP_TOOLCHAIN=nightly-2026-08-21-x86_64-unknown-linux-gnu
cli_run cli-private RUSTUP_HOME=$RH
cli_run cli-alt RUSTUP_HOME=$RH RUSTUP_TOOLCHAIN=alt
cli_run cli-ancestor RUSTUP_HOME=$RH TMPDIR=$O/anc/tmp
