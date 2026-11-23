"""AET-WO-2311 §1.1: a mutation of an unmodified ArduPilot ELF with no rebuild: the default of one
AP_SerialManager parameter, in its var_info row (AP_Param::GroupInfo, AP_Param.h:206-218: name
pointer, ptrdiff_t offset, float def_value, u16 flags, u8 idx, u8 type; 16 bytes on ARM32), set to
VALUE. Pure Python (the tools image has no ARM binutils). Refuses unless exactly one row has NAME and
its default is FROM.  python3 -I default.py IN OUT NAME FROM VALUE   e.g. 1_OPTIONS 0 8"""
import struct, sys
src, dst, name, old, new = sys.argv[1:]
data = bytearray(open(src, "rb").read())
assert data[:4] == b"\x7fELF" and data[4] == 1 and data[5] == 1, "not a 32-bit little-endian ELF"
shoff, = struct.unpack_from("<I", data, 0x20)
shentsize, shnum, _ = struct.unpack_from("<HHH", data, 0x2E)
secs = [struct.unpack_from("<IIIIIIIIII", data, shoff + i * shentsize) for i in range(shnum)]
symtab, = [s for s in secs if s[1] == 2]
strtab = secs[symtab[6]]
def cstr(off):
    return bytes(data[off:data.index(0, off)]).decode()
want = b"_ZN16AP_SerialManager8var_infoE"
hits = []
for i in range(symtab[5] // 16):
    st_name, value, size, *_ = struct.unpack_from("<IIIBBH", data, symtab[4] + i * 16)
    if bytes(data[strtab[4] + st_name:data.index(0, strtab[4] + st_name)]) == want:
        hits.append((value, size))
(addr, size), = hits
def file_off(a):
    (s,) = [s for s in secs if s[1] == 1 and s[3] <= a < s[3] + s[5]]  # SHT_PROGBITS holding a
    return a - s[3] + s[4]
rows = []
for i in range(0, size, 16):
    at = file_off(addr + i)
    name_p, off, val, flags, idx, typ = struct.unpack_from("<IifHBB", data, at)
    if typ == 0 and idx == 255:
        break
    if cstr(file_off(name_p)) == name:
        rows.append((at, val))
(at, val), = rows
assert val == float(old), f"{name} defaults to {val:g}, not {old}"
struct.pack_into("<f", data, at + 8, float(new))
open(dst, "wb").write(data)
print(f"{name}: {val:g} -> {float(new):g} at file offset {at + 8:#x}")
