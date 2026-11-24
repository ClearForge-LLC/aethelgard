# findpages.py PDF REGEX... : print the 1-based PDF page index and printed page number for each match
import sys; sys.path.insert(0, "/home/forge/.cache/uv/archive-v0/QrBn2gya0HHcphsZ")
import fitz, re
d = fitz.open(sys.argv[1])
for pat in sys.argv[2:]:
    for i, p in enumerate(d):
        t = p.get_text()
        if re.search(pat, t):
            m = re.findall(r'(?:Copyright|Submit).*?\n?(\d+)\s*$', t.strip()) 
            print(f"{pat!r}: pdf page {i+1}")
