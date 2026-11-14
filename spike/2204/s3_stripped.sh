#!/bin/bash
# AET-WO-2204 spike S3: the ELF the pipeline judges. Builds the reference with
# `aethelgard build` (which strips symbols, firmware.rs L37), reports its symbols,
# entry point and the bytes at the fixed point, compares its .text with the cargo
# build's, and boots it in the CLI's shape. Run inside img.sh after fw.py ref.
set -eu
OUT=/repo/spike/2204/out/s3-stripped.txt
ELF=/cache/fw/ref-built.elf
CARGO_ELF=/cache/fw/ref/target/thumbv7em-none-eabihf/release/sentinel-01-board-firmware
rm -f "$ELF"
{
  /cache/target/debug/aethelgard build --crate /repo/boards/sentinel-01 --out "$ELF" | python3 -c 'import json,sys; v=json.load(sys.stdin); print("build:", v["verdict"], v["elf"])'
  echo "sha256: $(sha256sum "$ELF" | cut -c1-64)"
  python3 - "$ELF" "$CARGO_ELF" <<'PY'
import struct, sys
def sections(p):
    d = open(p, "rb").read()
    shoff, = struct.unpack_from("<I", d, 0x20)
    entry, = struct.unpack_from("<I", d, 0x18)
    size, num, stri = struct.unpack_from("<HHH", d, 0x2E)
    secs = [struct.unpack_from("<IIIIIIIIII", d, shoff + i * size) for i in range(num)]
    names = d[secs[stri][4]:secs[stri][4] + secs[stri][5]]
    out = {}
    for s in secs:
        n = names[s[0]:names.index(b"\0", s[0])].decode()
        out[n] = (s[3], d[s[4]:s[4] + s[5]])
    return entry, out
e1, s1 = sections(sys.argv[1]); e2, s2 = sections(sys.argv[2])
print("entry:", hex(e1), "cargo build:", hex(e2))
print("stripped sections:", sorted(s1)); print("symtab in stripped:", ".symtab" in s1, "in cargo build:", ".symtab" in s2)
print(".text identical:", s1[".text"] == s2[".text"], "bytes:", len(s1[".text"][1]), "at", hex(s1[".text"][0]))
a, t = s1[".text"]; o = 0x08000588 - a
print("bytes at 0x08000588..0x08000598:", t[o:o + 16].hex())
PY
} > "$OUT"
python3 - <<'PY'
import json
c = {"name": "ref-built-10ms", "platform": "/repo/boards/sentinel-01/sentinel-01.repl", "elf": "/cache/fw/ref-built.elf",
     "for_ns": 10000000, "tail": "cli", "extra": [["icsr", "sysbus ReadDoubleWord 0xe000ed04"], ["cpsr", "cpu CPSR"]]}
json.dump(c, open("/cache/raw/ref-built.case.json", "w"))
PY
rm -f /cache/raw/ref-built.jsonl
PYTHONDONTWRITEBYTECODE=1 python3 /repo/spike/2204/drive.py /cache/raw/ref-built.case.json /cache/raw/ref-built.jsonl 1
python3 -c 'import json; r=json.loads(open("/cache/raw/ref-built.jsonl").read()); print("boot:", {k: r["reads"][k] for k in ("boot","pc","icsr","cpsr")})' >> "$OUT"
