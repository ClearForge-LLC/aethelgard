"""AET-WO-1219: a minimal PDF page-text reader, written because no extractor
is installed on Furnace (none on the host, none in the tools image).

It reads objects (plain and in /ObjStm streams), the page tree in order,
FlateDecode content streams, and Tj/TJ strings decoded through each font's
/ToUnicode CMap (bfchar, bfrange), else as Latin-1. It is not a general PDF
parser: its positive control is DS12117, whose held PyMuPDF text gives the
pages to compare against (FEEDBACK §7 F1).

Usage: pdftext.py <in.pdf> > out.txt, with "=== page N ===" markers (PDF page
indices, 1-based), as the held texts carry."""
import re
import sys
import zlib

data = open(sys.argv[1], "rb").read()
objs = {}


def inflate(raw, d):
    if b"FlateDecode" in d:
        try:
            return zlib.decompressobj().decompress(raw)
        except zlib.error:
            return b""
    return raw


def stream_of(body):
    m = re.search(rb"stream\r?\n", body)
    if not m:
        return None
    d = body[: m.start()]
    raw = body[m.end():]
    e = raw.rfind(b"endstream")
    raw = raw[:e] if e >= 0 else raw
    return inflate(raw, d)


for m in re.finditer(rb"(\d+)\s+0\s+obj\b(.*?)endobj", data, re.S):
    objs[int(m.group(1))] = m.group(2)
for n, body in list(objs.items()):
    if b"/ObjStm" in body:
        s = stream_of(body) or b""
        fm = re.search(rb"/First\s+(\d+)", body)
        if not fm:
            continue
        first = int(fm.group(1))
        head = s[:first].split()
        for i in range(0, len(head) - 1, 2):
            on, off = int(head[i]), int(head[i + 1])
            nxt = int(head[i + 3]) if i + 3 < len(head) else len(s) - first
            objs.setdefault(on, s[first + off: first + nxt])


def ref(b, key):
    m = re.search(rb"/" + key + rb"\s+(\d+)\s+0\s+R", b)
    return int(m.group(1)) if m else None


def refs(b, key):
    m = re.search(rb"/" + key + rb"\s*\[([^\]]*)\]", b)
    if m:
        return [int(x) for x in re.findall(rb"(\d+)\s+0\s+R", m.group(1))]
    r = ref(b, key)
    return [r] if r else []


def pages(n, seen=()):
    b = objs.get(n, b"")
    if re.search(rb"/Type\s*/Pages\b", b):
        out = []
        for k in refs(b, b"Kids"):
            if k not in seen:
                out += pages(k, seen + (n,))
        return out
    return [n]


def cmap(n):
    s = stream_of(objs.get(n, b"")) or b""
    m = {}
    for blk in re.findall(rb"beginbfchar(.*?)endbfchar", s, re.S):
        for a, b in re.findall(rb"<([0-9A-Fa-f]+)>\s*<([0-9A-Fa-f]+)>", blk):
            m[int(a, 16)] = bytes.fromhex(b.decode()).decode("utf-16-be", "replace")
    for blk in re.findall(rb"beginbfrange(.*?)endbfrange", s, re.S):
        for a, b, c in re.findall(rb"<([0-9A-Fa-f]+)>\s*<([0-9A-Fa-f]+)>\s*<([0-9A-Fa-f]+)>", blk):
            lo, hi, base = int(a, 16), int(b, 16), int(c, 16)
            for k in range(lo, min(hi, lo + 65535) + 1):
                m[k] = chr(min(base + k - lo, 0x10FFFF))
    width = 2 if re.search(rb"begincodespacerange\s*<[0-9A-Fa-f]{4}>", s) else 1
    return m, width


def fonts_of(page):
    res = page
    r = ref(page, b"Resources")
    if r:
        res = objs.get(r, b"")
    fm = re.search(rb"/Font\s*<<(.*?)>>", res, re.S)
    if fm:
        body = fm.group(1)
    else:
        fr = ref(res, b"Font")
        body = objs.get(fr, b"") if fr else b""
    out = {}
    for name, on in re.findall(rb"/([\w.+-]+)\s+(\d+)\s+0\s+R", body):
        tu = ref(objs.get(int(on), b""), b"ToUnicode")
        out[name] = cmap(tu) if tu else ({}, 1)
    return out


def lit(s):
    out, i = bytearray(), 0
    while i < len(s):
        c = s[i]
        if c == 0x5C and i + 1 < len(s):
            n = s[i + 1]
            i += 2
            if n in b"nrtbf":
                out += {110: b"\n", 114: b"\r", 116: b"\t", 98: b"\b", 102: b"\f"}[n]
            elif 48 <= n <= 55:
                j, o = i - 1, b""
                while j < len(s) and len(o) < 3 and 48 <= s[j] <= 55:
                    o += bytes([s[j]])
                    j += 1
                out.append(int(o, 8) & 255)
                i = j
            else:
                out.append(n)
            continue
        out.append(c)
        i += 1
    return bytes(out)


def decode(raw, font):
    m, w = font
    if not m:
        return raw.decode("latin-1")
    if w == 2:
        return "".join(m.get(int.from_bytes(raw[i:i + 2], "big"), "") for i in range(0, len(raw) - 1, 2))
    return "".join(m.get(b, chr(b)) for b in raw)


TOK = re.compile(rb"/([\w.+-]+)\s+[\d.]+\s+Tf|\((?:\\.|[^\\)])*\)|<[0-9A-Fa-f\s]*>|T\*|Td|TD|'|\"|ET")
root = ref(data[data.rfind(b"/Root") - 0:], b"Root") or ref(data, b"Root")
for i, pn in enumerate(pages(ref(objs[root], b"Pages")), 1):
    page = objs.get(pn, b"")
    fonts = fonts_of(page)
    s = b"".join(stream_of(objs.get(c, b"")) or b"" for c in refs(page, b"Contents"))
    font, text = ({}, 1), []
    for t in TOK.finditer(s):
        g = t.group(0)
        if t.group(1):
            font = fonts.get(t.group(1), ({}, 1))
        elif g.startswith(b"("):
            text.append(decode(lit(g[1:-1]), font))
        elif g.startswith(b"<"):
            h = re.sub(rb"\s", b"", g[1:-1])
            text.append(decode(bytes.fromhex(h.decode() + ("0" if len(h) % 2 else "")), font))
        else:
            text.append("\n")
    sys.stdout.write("=== page %d ===\n%s\n" % (i, "".join(text)))
