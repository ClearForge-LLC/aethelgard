#!/usr/bin/env python3
"""AET-WO-4241 mutants, one per guard this order adds (CLAUDE.md: sized to the diff). Each mutant is applied to a
fresh copy of the committed tree (git archive), then the named tests run on the host under hc.sh's cap with a
shared target dir; a mutant is KILLED when the tests fail. Usage: python3 -I mutants.py <tree-sha> [ids...]"""
import os, subprocess, sys
SHA = sys.argv[1]; ONLY = set(sys.argv[2:])
K = os.path.expanduser('~/.cache/aet-4241/mut')
FW = 'crates/aethelgard-svd-hal/src/board/firmware.rs'
BT = 'crates/aethelgard-twin/src/boot.rs'
RN = 'crates/aethelgard-cli/src/renode.rs'
LG = 'crates/aethelgard-cli/src/ledger.rs'
MUTANTS = [
  # The WO's two named mutants.
  ('M1-pass', RN, '        findings.push(unmodelled_finding(u, seen, booted));', '        let _ = (u, seen, booted);',
   ['-p', 'aethelgard-cli', '--test', 'boot_tier1']),
  ('M1b-pass-kind', RN, 'pub const NOT_WITNESSABLE: &str = "identity_not_witnessable";', 'pub const NOT_WITNESSABLE: &str = "identity_pass";',
   ['-p', 'aethelgard-cli', '--test', 'boot_tier1']),
  ('M2-answers-filter', FW, '    kind == StepKind::Identity || answers(part)', '    answers(part) && kind == kind',
   ['-p', 'aethelgard-svd-hal', '--test', 'board']),
  # Guards this order adds.
  ('M3-order-interleaved', FW, '        let into = if answers(&d.part) {\n            &mut out\n        } else {\n            &mut unanswered\n        };\n        for step in steps\n            .into_iter()\n            .filter(|s| step_read(&d.part, s.kind, answers))\n        {\n            into.push(Read {\n                bus: d.peripheral.to_ascii_lowercase(),\n                target: Target::I2c',
   '        let into = &mut out;\n        for step in steps\n            .into_iter()\n            .filter(|s| step_read(&d.part, s.kind, answers))\n        {\n            into.push(Read {\n                bus: d.peripheral.to_ascii_lowercase(),\n                target: Target::I2c',
   ['-p', 'aethelgard-svd-hal', '--test', 'board']),
  ('M4-twin-slot-first', BT, '    for c in &mut plan.checks {\n        c.status_at = place(&c.reference, c.width)?;\n        c.value_at = c.status_at + 4;\n    }\n    for u in &mut plan.unmodelled {\n        if let Some(n) = u.identity.as_mut() {\n            n.status_at = place(&u.reference, n.width)?;\n            n.value_at = n.status_at + 4;\n        }\n    }',
   '    for u in &mut plan.unmodelled {\n        if let Some(n) = u.identity.as_mut() {\n            n.status_at = place(&u.reference, n.width)?;\n            n.value_at = n.status_at + 4;\n        }\n    }\n    for c in &mut plan.checks {\n        c.status_at = place(&c.reference, c.width)?;\n        c.value_at = c.status_at + 4;\n    }',
   ['-p', 'aethelgard-twin', '--lib']),
  ('M5-d2-made-always', RN, '    match u.wire.status(status) {\n        Some(_) => {\n            let status = status_name(&u.wire, status);', '    match Some(u.wire.status(status)) {\n        Some(_) => {\n            let status = status_name(&u.wire, status);',
   ['-p', 'aethelgard-cli', '--bins', 'an_unmodelled']),
  ('M6-absent-others-ignored', RN, '        .chain(others.iter().copied())\n', '        .chain(others.iter().copied().filter(|_| false))\n',
   ['-p', 'aethelgard-cli', '--bins', 'an_unmodelled']),
  ('M7-spi-window-ignored', RN, '    for &(mux, address) in others {\n        every_window', '    for &(mux, address) in others.iter().filter(|_| false) {\n        every_window',
   ['-p', 'aethelgard-cli', '--bins', 'a_line_in_an_unmodelled']),
  ('M8-hook-unmodelled-dropped', RN, '        c.extend(\n            unwitnessable(identity).map(|(j, _, n)| {\n                crate::anchor::step_hook(unmodelled_hook(identity, j), n.status_at)\n            }),\n        );',
   '        let _ = unmodelled_hook;',
   ['-p', 'aethelgard-cli', '--bins', 'renode::tests::a_']),
  ('M9-rearm-unmodelled-dropped', RN, '            .chain(unmodelled)\n', '            .chain(unmodelled.filter(|_| false))\n',
   ['-p', 'aethelgard-cli', '--test', 'boot_tier1']),
  ('M10-cost-once', RN, '        writes.next().filter(|_| writes.next().is_none())', '        writes.next()',
   ['-p', 'aethelgard-cli', '--bins', 'an_unmodelled']),
  ('M11-twin-limit-row', LG, '    rows.extend(identity_rows(topology)?);\n', '',
   ['-p', 'aethelgard-cli', '--test', 'ledger_tier1']),
  ('M12-status-on-own-bus', BT, '            (STATUS_TXIS, false) => Some(Some("TXIS")),', '            (STATUS_TXIS, _) => Some(Some("TXIS")),',
   ['-p', 'aethelgard-cli', '--bins', 'an_unmodelled']),
]
res = []
for mid, path, old, new, test in MUTANTS:
    if ONLY and mid not in ONLY: continue
    d = f'{K}/tree'
    subprocess.run(['rm', '-rf', d], check=True); os.makedirs(d)
    subprocess.run(f'git -C /home/forge/aethelgard-4241 archive {SHA} | tar -x -C {d}', shell=True, check=True)
    p = f'{d}/{path}'; s = open(p).read()
    if s.count(old) != 1:
        res.append(f'{mid}: NOT APPLIED (anchor count {s.count(old)})'); print(res[-1], flush=True); continue
    open(p, 'w').write(s.replace(old, new))
    cmd = ['systemd-run', '--user', '--scope', '-q', '-p', 'MemoryMax=8G', '-p', 'MemorySwapMax=0', '--expand-environment=no',
           'nice', '-n', '10', 'env', f'PATH={os.path.expanduser("~/.cargo/bin")}:{os.environ["PATH"]}', f'CARGO_TARGET_DIR={K}/target', os.path.expanduser('~/.cargo/bin/cargo'), 'test', '-q', '--offline'] + test
    r = subprocess.run(cmd, cwd=d, capture_output=True, text=True)
    out = r.stdout + r.stderr
    tail = [l for l in out.splitlines() if 'test result' in l or l.startswith('error') or 'panicked' in l][:4]
    verdict = 'KILLED' if r.returncode != 0 and 'test result: FAILED' in out else ('COMPILE-FAIL' if r.returncode != 0 else 'SURVIVED')
    res.append(f'{mid}: {verdict} ({" ".join(test)}) {tail}'); print(res[-1], flush=True)
open(f'{K}/results.txt', 'a').write('\n'.join(res) + '\n')
