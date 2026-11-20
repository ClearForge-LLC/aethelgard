# measure.py ELF...: AET-WO-4406 §1.1, by a self-contained ELF32 reader (no nm
# or readelf). For each ELF (named by its directory) it prints:
#   SIG <board> load_defaults_file=<absent|type/section@addr> load_embedded_param_defaults=<…>
#       param_defaults_data=<absent|size/section magic=<ok|bad> max=<n> length=<n> text=<repr>>
#       romfs=<absent|n entries [name,…]>
#   NEAR <board> <every .symtab name holding "defaults" or "ROMFS", with type and section>
#   GPS <board> sym=<type>/<size>B sec=<name>:<flags> rows=<n> subgroups=<name:idx:flags:type,…> cap=<n> other=<flags set>/<types set> term=<hex>
#   OFFS <board> <n_ROW=offset ...> verdict=<all 24n+{0,4,8}|differs: ...>
# AP_Param::GroupInfo, 16 bytes on ARM32 (AP_Param.h:206-218); AP_GROUPEND
# `{"", 0, 0, 0, 0xFF, 0}` (:170); AP_ROMFS::embedded_file, 20 bytes
# (AP_ROMFS.h:30-36); param_defaults_struct (AP_Param.h:682-688).
import os, struct, sys

NAMES = {
    "load_defaults_file": b"_ZN8AP_Param18load_defaults_fileEPKcb",
    "load_embedded_param_defaults": b"_ZN8AP_Param28load_embedded_param_defaultsEb",
    "param_defaults_data": b"_ZN8AP_Param19param_defaults_dataE",
    "romfs": b"_ZN8AP_ROMFS5filesE",
    "gps": b"_ZN6AP_GPS8var_infoE",
    "serial": b"_ZN16AP_SerialManager8var_infoE",
}
MAGIC = b"PARMDEF\0" + bytes([0x55, 0x37, 0xF4, 0xA0, 0x38, 0x5D, 0x48, 0x5B])
FIELD = {"BAUD": 0, "OPTIONS": 4, "PROTOCOL": 8}


class Elf:
    def __init__(self, path):
        b = self.b = open(path, "rb").read()
        assert b[:4] == b"\x7fELF" and b[4] == 1 and b[5] == 1
        shoff, = struct.unpack_from("<I", b, 32)
        shnum, shstrndx = struct.unpack_from("<HH", b, 48)
        self.secs = [struct.unpack_from("<IIIIIIIIII", b, shoff + 40 * i) for i in range(shnum)]
        symtab = [s for s in self.secs if s[1] == 2]
        assert len(symtab) == 1
        symtab = symtab[0]
        strtab = self.secs[symtab[6]]
        self.syms = []
        for i in range(symtab[5] // 16):
            st_name, value, size, info, other, shndx = struct.unpack_from("<IIIBBH", b, symtab[4] + 16 * i)
            o = strtab[4] + st_name
            self.syms.append((b[o:b.index(b"\0", o)], value, size, info & 0xF, shndx))

    def secname(self, s):
        o = self.secs[struct.unpack_from("<H", self.b, 50)[0]][4] + s[0]
        return self.b[o:self.b.index(b"\0", o)].decode()

    def named(self, name):
        return [s for s in self.syms if s[0] == name]

    def at(self, addr, n):
        for s in self.secs:
            if s[1] == 1 and s[2] & 2 and s[3] <= addr and addr + n <= s[3] + s[5]:
                o = s[4] + addr - s[3]
                return self.b[o:o + n], s
        return None, None

    def cstr(self, addr):
        _, s = self.at(addr, 1)
        if s is None:
            return None
        o = s[4] + addr - s[3]
        return self.b[o:self.b.index(b"\0", o)].decode(errors="replace")

    def where(self, sym):
        name, value, size, kind, shndx = sym
        sec = self.secs[shndx] if 0 < shndx < len(self.secs) else None
        sn = self.secname(sec) if sec else f"shndx{shndx}"
        return f"type{kind}/{size}B/{sn}@{value:#010x}"

    def one(self, key):
        found = self.named(NAMES[key])
        assert len(found) <= 1, (key, found)
        return found[0] if found else None

    def rows(self, sym):
        data, sec = self.at(sym[1], sym[2])
        out, term = [], None
        for i in range(0, sym[2], 16):
            name_p, off, val, fl, idx, typ = struct.unpack_from("<IiIHBB", data, i)
            if typ == 0:
                term = (data[i:i + 16].hex(), (sym[2] - i - 16) // 16)
                break
            out.append((self.cstr(name_p), off, val, fl, idx, typ))
        return out, term, sec


def measure(path):
    e = Elf(path)
    board = os.path.basename(os.path.dirname(path))
    sig = []
    for key in ("load_defaults_file", "load_embedded_param_defaults"):
        s = e.one(key)
        sig.append(f"{key}={'absent' if s is None else e.where(s)}")
    s = e.one("param_defaults_data")
    if s is None:
        sig.append("param_defaults_data=absent")
    else:
        data, sec = e.at(s[1], s[2])
        mx, ln = struct.unpack_from("<HH", data, 16)
        text = bytes(data[20:20 + ln])
        sig.append(f"param_defaults_data={e.where(s)} ro={not (sec[2] & 1)} magic={'ok' if data[:16] == MAGIC else data[:16].hex()} "
                   f"max={mx} length={ln} text={text!r}")
    s = e.one("romfs")
    if s is None:
        sig.append("romfs=absent")
    else:
        data, sec = e.at(s[1], s[2])
        names = []
        for i in range(0, s[2], 20):
            fn, csz, dsz, crc, cp = struct.unpack_from("<IIIII", data, i)
            names.append(f"{e.cstr(fn)}({csz}/{dsz})")
        sig.append(f"romfs={e.where(s)} ro={not (sec[2] & 1)} n={s[2] // 20} rem={s[2] % 20} [{','.join(names)}]")
    print(f"SIG {board} " + " ".join(sig))
    near = sorted({f"{n.decode()}:{e.where((n, v, z, k, x))}" for n, v, z, k, x in e.syms
                   if (b"defaults" in n.lower() or b"ROMFS" in n or b"romfs" in n) and k in (1, 2)})
    print(f"NEAR {board} " + " ".join(near))

    s = e.one("gps")
    if s is None:
        print(f"GPS {board} absent")
    else:
        rows, term, sec = e.rows(s)
        subs = [r for r in rows if r[0] is not None and len(r[0]) == 2 and r[0][0].isdigit() and r[0][1] == "_"]
        other = [r for r in rows if r not in subs]
        print(f"GPS {board} sym=type{s[3]}/{s[2]}B sec={e.secname(sec)}:flags{sec[2]} rows={len(rows)} "
              f"subgroups={','.join(f'{r[0]}:{r[4]}:{r[3]}:{r[5]}' for r in subs)} cap={len(subs)} "
              f"other_flags={sorted({r[3] for r in other})} other_types={sorted({r[5] for r in other})} "
              f"other_groups={[r[0] for r in other if r[5] == 6]} term={term}")

    s = e.one("serial")
    rows, term, _ = e.rows(s)
    offs, bad = [], []
    for name, off, *_ in rows:
        n, _, field = name.partition("_")
        if n.isdigit() and field in FIELD:
            offs.append(f"{name}={off}")
            if off != 24 * int(n) + FIELD[field]:
                bad.append(f"{name}={off}!={24 * int(n) + FIELD[field]}")
        else:
            offs.append(f"{name}={off}(not a port row)")
    print(f"OFFS {board} {' '.join(offs)} verdict={'all 24n+{0,4,8}' if not bad else 'differs: ' + ' '.join(bad)}")


for p in sys.argv[1:]:
    measure(p)
