import sys
sys.path.insert(0, "/home/forge/.cache/uv/archive-v0/QrBn2gya0HHcphsZ")
import pymupdf
path, page, out = sys.argv[1], int(sys.argv[2]), sys.argv[3]
zoom = float(sys.argv[4]) if len(sys.argv) > 4 else 2.0
d = pymupdf.open(path)
d[page - 1].get_pixmap(matrix=pymupdf.Matrix(zoom, zoom)).save(out)
