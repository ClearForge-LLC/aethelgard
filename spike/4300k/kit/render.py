# render.py PDF OUTDIR TAG DPI PAGE... -> OUTDIR/TAG-pN.png (1-based pages); "info" as PAGE prints page count.
import sys; sys.path.insert(0, "/home/forge/.cache/uv/archive-v0/QrBn2gya0HHcphsZ")
import sys, fitz
doc = fitz.open(sys.argv[1]); out, tag, dpi = sys.argv[2], sys.argv[3], int(sys.argv[4])
for p in sys.argv[5:]:
    if p == "info":
        print(len(doc)); continue
    doc[int(p) - 1].get_pixmap(dpi=dpi).save(f"{out}/{tag}-p{p}.png")
