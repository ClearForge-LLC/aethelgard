"""AET-WO-1317: count non-monotone cases in a sweep TSV (spike1317 sweep/specsweep).

hard: full fits, reduced refused (any class that is not fit, validation refusals aside).
soft(t): reduced w or h exceeds full's by more than t mm (reached size when refused).
Rows refused by validate/solve/parse are their own class and never counted.
"""
import sys, collections

def rows(path):
    with open(path) as f:
        head = f.readline().rstrip('\n').split('\t')
        for line in f:
            r = dict(zip(head, line.rstrip('\n').split('\t')))
            if r['removed'] != '-':
                yield r

def main(path, tol=1.0):
    per = collections.OrderedDict()
    for r in rows(path):
        v = per.setdefault(r['variant'], collections.Counter())
        v['removals'] += 1
        cls = r['class']
        if cls.split(':')[0] in ('validate', 'solve', 'parse', 'design'):
            v['spec-refused'] += 1
            continue
        v['placed'] += 1
        v['fit' if cls == 'fit' else cls] += 1
        fw, fh, w, h = map(float, (r['full_w'], r['full_h'], r['w_mm'], r['h_mm']))
        hard = r['full_class'] == 'fit' and cls != 'fit'
        v['hard'] += hard
        if w == 0:  # a refusal with no size (CompactionUnbounded etc.)
            v['nosize'] += 1
            continue
        g = max(w - fw, h - fh)
        v['soft>%.1f' % tol] += g > tol + 1e-9
        v['soft>0'] += g > 1e-9
        v['shrank'] += (w < fw - 1e-9 or h < fh - 1e-9)
        if hard or g > tol + 1e-9:
            print('  case', r['variant'], r['removed'], cls, f"{fw:.3f}x{fh:.3f} -> {w:.3f}x{h:.3f}")
    tot = collections.Counter()
    for k, v in per.items():
        tot.update(v)
        print(k, dict(v))
    print('TOTAL', dict(tot))

if __name__ == '__main__':
    main(sys.argv[1], float(sys.argv[2]) if len(sys.argv) > 2 else 1.0)
