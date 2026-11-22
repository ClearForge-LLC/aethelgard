"""AET-WO-1219 RULES amendment 2: ST facts held in code, not in a table.
Every function in crates/aethelgard-ir/src (outside #[cfg(test)]) with a
string literal in ST's vocabulary, one line each: where, how many, samples.
Comments are stripped first, so a citation in a doc comment does not count;
a literal inside an error message does (it is printed to the user)."""
import collections, pathlib, re, sys
sys.dont_write_bytecode = True
sys.path.insert(0, str(pathlib.Path(__file__).parent))
from reader import strip_comments, line_of  # noqa: E402

VOCAB = re.compile(r'"([^"\n]*\b(?:P[A-K]\d{1,2}|SPI\d|I2C\d|USART\d|UART\d|LPUART\d|TIM\d+|ADC\d|RCC\w*|HSE|HSI|LSE|CSI|PLL\d?|VOS\d?|AF\d+|DS12117|RM0433|ES0392|STM32\w*|OSPEEDR|PUPDR|MODER|BOR|MCO\d?)\b[^"\n]*)"')
src = pathlib.Path(sys.argv[1]) / "crates/aethelgard-ir/src"
rows = collections.OrderedDict()
for p in sorted(src.glob("*.rs")):
    t = strip_comments(p.read_text())
    m = re.search(r"#\[cfg\(test\)\]\s*mod\s+\w+\s*\{", t)
    t = t[: m.start()] if m else t
    fns = [(mm.start(), mm.group(1)) for mm in re.finditer(r"\bfn\s+(\w+)", t)]
    for mm in VOCAB.finditer(t):
        before = [(s, n) for s, n in fns if s < mm.start()]
        s, n = before[-1] if before else (0, "<item level>")
        key = (p.name, n, line_of(t, s) if before else 0)
        rows.setdefault(key, []).append(mm.group(1))
print("| function | at | ST-vocabulary literals | samples |")
print("|---|---|---|---|")
for (f, n, l), lits in rows.items():
    sample = ", ".join("`%s`" % x[:40].replace("|", "\\|") for x in list(dict.fromkeys(lits))[:3])
    print("| `%s` | `%s:%d` | %d | %s |" % (n, f, l, len(lits), sample))
print()
print("%d functions, %d literals" % (len(rows), sum(len(v) for v in rows.values())))
