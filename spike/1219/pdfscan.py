import re, sys, zlib
pats = [b"RP2350", b"rp2350", b"Raspberry", b"\x00R\x00P\x002\x003\x005\x000", b"\x00R\x00a\x00s\x00p\x00b"]
n = hit = streams = 0
for f in open(sys.argv[1]).read().split("\n"):
    if not f: continue
    n += 1
    try: data = open(f, "rb").read()
    except OSError: continue
    found = [p for p in pats if p in data]
    for m in re.finditer(rb"stream\r?\n", data):
        s = m.end(); e = data.find(b"endstream", s)
        if e < 0: break
        try:
            d = zlib.decompressobj().decompress(data[s:e])
            streams += 1
        except zlib.error:
            continue
        found += [p for p in pats if p in d]
    if found:
        hit += 1; print("HIT", f, sorted(set(found)))
print("pdfs %d, inflated streams %d, hits %d" % (n, streams, hit))
