import sys
sys.path.insert(0, "/home/forge/.cache/uv/archive-v0/QrBn2gya0HHcphsZ")
import fitz
for path in sys.argv[1:]:
    d = fitz.open(path)
    print(path, d.page_count, d.metadata.get("title"), d.metadata.get("modDate"))
    with open(path[:-4] + ".txt", "w") as out:
        for i, p in enumerate(d):
            out.write(f"\n=== PAGE {i+1} ===\n" + p.get_text())
