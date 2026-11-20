# varinfo.py ELF...: AP_SerialManager::var_info decoded, with its layout, by a
# self-contained ELF32 reader (no nm or readelf). For each ELF it prints:
#   LAYOUT <name> sym=<type>/<size> sec=<name>:<flags> rows=<n> term=<hex> names=<sections> flags=<set> types=<name:type,...>
#   ROWS <name> n_PROTOCOL=v n_BAUD=v->baud n_OPTIONS=v ... _PASS1=v _PASS2=v _PASSTIMO=v
# AP_Param::GroupInfo, 16 bytes on ARM32 (AP_Param.h:206-218): const char *name,
# ptrdiff_t offset, union { float def_value; ... }, uint16_t flags, uint8_t idx,
# uint8_t type. ArduPilot stops at the first type 0, AP_PARAM_NONE (AP_Param.cpp:1498-1500).
import os, struct, sys

SYM = b"_ZN16AP_SerialManager8var_infoE"


def mapb(r):
    r = 57 if r <= 0 else r
    t = {1: 1200, 2: 2400, 4: 4800, 9: 9600, 19: 19200, 38: 38400, 57: 57600, 100: 100000, 111: 111100,
         115: 115200, 230: 230400, 256: 256000, 460: 460800, 500: 500000, 921: 921600, 1500: 1500000, 2000: 2000000}
    return t.get(r, r if r > 2000 else r * 1000)


def decode(path):
    b = open(path, "rb").read()
    assert b[:4] == b"\x7fELF" and b[4] == 1 and b[5] == 1
    shoff, = struct.unpack_from("<I", b, 32)
    shnum, shstrndx = struct.unpack_from("<HH", b, 48)
    secs = [struct.unpack_from("<IIIIIIIIII", b, shoff + 40 * i) for i in range(shnum)]
    shstr = secs[shstrndx]
    def secname(s):
        o = shstr[4] + s[0]
        return b[o:b.index(b"\0", o)].decode()
    symtab = [s for s in secs if s[1] == 2][0]
    strtab = secs[symtab[6]]
    found = []
    for i in range(symtab[5] // 16):
        st_name, value, size, info, other, shndx = struct.unpack_from("<IIIBBH", b, symtab[4] + 16 * i)
        o = strtab[4] + st_name
        if b[o:o + len(SYM) + 1] == SYM + b"\0":
            found.append((value, size, info & 0xF, shndx))
    assert len(found) == 1, found
    value, size, kind, shndx = found[0]
    sec = secs[shndx]
    def at(addr, n):
        for s in secs:
            if s[1] == 1 and s[2] & 2 and s[3] <= addr and addr + n <= s[3] + s[5]:
                return b[s[4] + addr - s[3]: s[4] + addr - s[3] + n], s
        return None, None
    data, _ = at(value, size)
    rows, names_in, flags, types, term = [], set(), set(), [], None
    for i in range(0, size, 16):
        name_p, off, val, fl, idx, typ = struct.unpack_from("<IiIHBB", data, i)
        if typ == 0:
            term = data[i:i + 16].hex()
            tail = (size - i - 16) // 16
            break
        chunk, s = at(name_p, 1)
        names_in.add(secname(s) + (":W" if s[2] & 1 else ":RO"))
        o = s[4] + name_p - s[3]
        name = b[o:b.index(b"\0", o)].decode()
        f = struct.unpack("<f", struct.pack("<I", val))[0]
        flags.add(fl)
        types.append(f"{name}:{typ}")
        rows.append((name, f, idx))
    else:
        tail = None
    return dict(kind=kind, size=size, sec=secname(sec), secflags=sec[2], rows=rows, term=term, tail=tail,
                names=sorted(names_in), flags=sorted(flags), types=types)


for path in sys.argv[1:]:
    d = decode(path)
    name = os.path.basename(os.path.dirname(path))
    print(f"LAYOUT {name} sym=type{d['kind']}/{d['size']}B sec={d['sec']}:flags{d['secflags']} rows={len(d['rows'])} "
          f"term={d['term']} after_term={d['tail']} names={','.join(d['names'])} flags={d['flags']} types={' '.join(d['types'])}")
    out = []
    for n, f, idx in d["rows"]:
        v = int(f) if f == int(f) else f
        out.append(f"{n}={v}->{mapb(int(f))}" if n.endswith("_BAUD") else f"{n}={v}")
    print(f"ROWS {name} " + " ".join(out))
