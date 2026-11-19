"""usage: pdfgrep.py <pdf> <regex> [context]: print page number and context of each match (PyMuPDF)."""
import re, sys
import fitz
d = fitz.open(sys.argv[1]); rx = re.compile(sys.argv[2], re.I); c = int(sys.argv[3]) if len(sys.argv) > 3 else 120
for i, p in enumerate(d, 1):
    t = " ".join(p.get_text().split())
    for m in rx.finditer(t):
        print(f"p{i}: ...{t[max(0,m.start()-c):m.end()+c]}...")
