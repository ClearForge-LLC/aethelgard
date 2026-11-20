"""AET-WO-4240a mutants: one per guard this work order adds or changes. Each is (id, file, old, new, test args).
Run in a copy of the tree (MUT_SRC); every mutant must make a named test fail."""
import os, subprocess, sys, shutil
SRC = os.path.expanduser('~/.cache/aet-4240a/mut/src'); PRISTINE = os.path.expanduser('~/.cache/aet-4240a/mut/pristine')
IR = ['-p', 'aethelgard-ir', '--lib', '--test', 'hse', '--test', 'solve', '--test', 'motor', '--test', 'validate', '--test', 'bench']
CLK = 'crates/aethelgard-ir/src/clock.rs'; SOL = 'crates/aethelgard-ir/src/solve.rs'; LIB = 'crates/aethelgard-ir/src/lib.rs'
M = [
 ('M1 reference on a boundary passes', CLK, 'if rn <= u128::from(min_hz) * rd || rn >= u128::from(max_hz) * rd {', 'if rn < u128::from(min_hz) * rd || rn > u128::from(max_hz) * rd {', IR),
 ('M2 the budget drops aging', CLK, '&crystal.tolerance_ppm,\n            &crystal.stability_ppm,\n            &crystal.aging_ppm,\n        ]', '&crystal.tolerance_ppm,\n            &crystal.stability_ppm,\n        ]', IR),
 ('M3 the HSI ppm for a crystal board', 'crates/aethelgard-ir/src/motor.rs', '.map_or(crate::clock::Budget::HSI, crate::clock::SolvedHse::budget)', '.map_or(crate::clock::Budget::HSI, |_| crate::clock::Budget::HSI)', IR),
 ('M4 VCOSEL vs reference', CLK, 'if (vcosel == 1) != (rge == 0) {', 'if false {', IR),
 ('M5 VCO range', CLK, 'if vn < u128::from(vmin) * rd || vn > u128::from(vmax) * rd {', 'if false {', IR),
 ('M6 DIVP1EN', CLK, 'if field(h.pllcfgr, 16, 1) == 0 {', 'if false {', IR),
 ('M7 PLLSRC', CLK, 'if field(h.pllckselr, 0, 2) != 0b10 {', 'if false {', IR),
 ('M8 PLL1FRACEN', CLK, 'if field(h.pllcfgr, 0, 1) != 0 {', 'if false {', IR),
 ('M9 stored budget', CLK, 'if budget != h.budget() {', 'if false {', IR),
 ('M10 words decode to the clock', CLK, 'if !exact || expected != *clock {', 'if false {', IR),
 ('M11 temperature inside the part', CLK, 'if lo > hi || i64::from(lo) < t.min || i64::from(hi) > t.max {', 'if false {', IR),
 ('M12 life within aging span', CLK, 'if i64::from(c.life_hours) > crystal.aging_hours.value {', 'if false {', IR),
 ('M13 pull 0', CLK, 'if c.pull_ppm == 0 {', 'if false {', IR),
 ('M14 frequency range', CLK, 'if n < u128::from(min_hz) * d || n > u128::from(max_hz) * d {', 'if false {', IR),
 ('M15 Gm_crit', CLK, 'if !g.holds {', 'if false {', IR),
 ('M16 an external declared 0', CLK, 'if v == 0 {', 'if false {', IR),
 ('M17 PH1 beside a crystal', SOL, 'pins.extend(["PH0", "PH1"])', 'pins.push("PH0")', IR),
 ('M18 PH0 beside an external clock', SOL, 'Some(crate::clock::Hse::External(_)) => pins.push("PH0"),', 'Some(crate::clock::Hse::External(_)) => {}', IR),
 ('M19 solver reference strictly inside', SOL, '.position(|&(lo, hi)| lo < reference && reference < hi)', '.position(|&(lo, hi)| lo <= reference && reference <= hi)', IR),
 ('M20 F-PANIC refusal', SOL, '.ok_or_else(|| refuse(mode, kernel_hz))', '.ok_or_else(|| -> SolveError { panic!("F-PANIC") })', IR),
 ('M21 legacy hse_hz non-zero', LIB, '(None, Some(0)) => None,', '(None, Some(_)) => None,', IR),
 ('M22 hse_hz beside hse', LIB, '(Some(hse), None) => hse,', '(Some(hse), _) => hse,', IR),
 ('M23 a PLL1 clock without its tree', LIB, 'if self.clock.hse.is_none() && self.clock != SolvedClock::reset() {', 'if false {', IR),
 ('M25 budget bound', CLK, '(Ok(fast_ppm), Ok(slow_ppm)) if fast < 1_000_000 && slow < 1_000_000 => {', '(Ok(fast_ppm), Ok(slow_ppm)) => {', IR),
 ('M26 caps equal', CLK, 'if c1 != c2 {', 'if false {', IR),
 ('M27 caps in series under CL', CLK, '    if u128::from(c1) * u128::from(c2)\n', '    if false && u128::from(c1) * u128::from(c2)\n', IR),
 ('M28 unread bits', CLK, 'if word & !read != 0 {', 'if false {', IR),
 ('M29 DIVR1 0', CLK, 'if field(h.pll1divr, 24, 7) == 0 {', 'if false {', IR),
 ('M30 pll1_q limit', CLK, 'if fastest_hz > limit {', 'if false {', IR),
 ('M31 SPI at the HSI', 'crates/aethelgard-ir/src/vos.rs', 'let b = if spi(c) {\n                crate::clock::Budget::HSI', 'let b = if false {\n                crate::clock::Budget::HSI', IR),
 ('M24 HseNotGenerated', 'crates/aethelgard-cli/src/generate.rs', 'if topology.clock.hse.is_some() {', 'if false {', ['-p', 'aethelgard-cli', '--test', 'generate_tier1', 'a_declared_hse']),
]
only = sys.argv[1:]
env = dict(os.environ, CARGO_TARGET_DIR=os.path.expanduser('~/.cache/aet-4240a/mut/target'), PATH=os.path.expanduser('~/.cargo/bin') + ':' + os.environ['PATH'])
for mid, f, old, new, args in M:
    if only and mid.split()[0] not in only: continue
    p = os.path.join(SRC, f); s = open(p).read()
    if s.count(old) != 1: print(f'{mid}: ANCHOR {s.count(old)}', flush=True); continue
    open(p, 'w').write(s.replace(old, new))
    r = subprocess.run(['nice', '-n', '10', 'cargo', 'test', '--no-fail-fast', *args], cwd=SRC, env=env, capture_output=True, text=True)
    shutil.copy(os.path.join(PRISTINE, f), p)
    out = r.stdout + r.stderr
    failed = sorted({l.split()[1] for l in out.splitlines() if l.startswith('test ') and l.endswith('FAILED')})
    compile_err = 'error[' in out and not failed
    print(f'{mid}: {"KILLED" if failed else ("COMPILE-ERROR" if compile_err else "SURVIVED")} exit={r.returncode} {failed[:4]}', flush=True)
