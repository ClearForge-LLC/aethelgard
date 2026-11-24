#!/usr/bin/env python3 -I
"""Merge the pass results, classify, write SWEEP.md's generated table (stdout).
Class: relaxes | conservative | refused | refused-later | no change.  MANUAL holds the judgement made by reading each diff."""
import json, re, collections, os
C = '/home/forge/.cache/aet-4300k/sweep/'
rows = collections.OrderedDict()
for f in ('results-pass1', 'results-pass2', 'results-pass3', 'results-pass4'):
    if not os.path.exists(C + f + '.json'): continue
    for r in json.load(open(C + f + '.json')):
        rows[(r['path'], r.get('carrier'), r.get('variant'))] = r
# (path, carrier|None, variant|None) -> (class, note, [substrings selecting the quoted diff entries])
M = {
 ('mcu.hse', None, 'null'): ('conservative', 'removes the crystal (BOM loses Y1, C20, C21, C26-C29; adds bypass caps); the oscillator gate goes off with its subject; clock falls to HSI', ['ran[', 'oscillator[U1_OSC_IN].crystal', 'clock.source']),
 ('mcu.reset_switch', None, 'delete', ): ('conservative', 'removes SW1 from the BOM; no judgement removed (placement only)', ['ladder']),
 ('mcu.foreign_clock', None, 'delete'): ('conservative', 'solve drops the foreign_clock block; the reader is simulate --foreign, which was not run', ['foreign_clock']),
 ('i2c_devices[].pin_capacitance', None, 'null'): ('conservative', 'STRICTER: the declared 10000 fF per pin becomes the UM10204 10 pF default, terms.pins 20000 -> 30000', ['conditions', 'declared_pins', 'terms.pins']),
 ('spi_devices[].cs.pin', None, 'null'): ('no change', 'solver chooses the pin (IMU_CS PB12 -> PA1); no judgement removed', ['gpio.IMU_CS']),
 ('board.stackup.fab_tolerances', 'sentinel-03', 'delete'): ('relaxes', 'RingRule::of is None: emit writes no capacitance_ladder and raises no thermal via; the board and BOM are the same', ['capacitance_ladder.rung', 'notes[*]', 'judged_by']),
 ('board.stackup.fab_tolerances', 'sentinel-02-power', 'delete'): ('relaxes', 'same; emit loses the ladder, its unjudged TranslatorPinUnbounded entry and the via-ring notes (extract refuses TranslatorPinUnbounded on this baseline, so extract is not a witness here)', ['notes[*]', 'capacitance_ladder.judged_by']),
 ('board.stackup.fab_tolerances', 'sentinel-03+gated', 'delete'): ('relaxes', 'emit relaxes as on sentinel-03; with drop_limits declared the supply gate then refuses (extract SupplyInputUndeclared), so only the emit-side relaxation survives', ['notes[*]', 'error.message', 'capacitance_ladder.judged_by']),
 ('board.max_outline_um', None, 'delete'): ('no change', 'no gate I ran reads it; absence is an unbounded outline (ladder numbers move with placement). Ruled optional in PR #191 Q1', ['ladder']),
 ('board.drawn', None, 'delete'): ('conservative', 'absent is true, the strict case: emit refuses the twin stand-in by another rule (HseSourceNotDrawn)', ['error.kind']),
 ('net_classes[]', None, 'empty'): ('no change', 'no judgement removed on host; routing and the ladder rung move', ['ladder']),
 ('net_classes[].via', None, 'null'): ('no change', 'geometry only', []),
 ('diff_pairs[]', None, 'empty'): ('relaxes', 'extract.pairs goes off and the pair rows go; BOM identical; the same nets are still extracted as lines', ['ran[3]', 'mismatch', 'pairs[USB_STANDIN].name']),
 ('power.conditions.dc_bias_derating[]', None, 'empty'): ('relaxes', 'the cout bound is judged at rated, not derated, capacitance; BOM identical', ['conditions[*]']),
 ('power.loads[]', None, 'empty'): ('relaxes', 'regulators judged with no load: loss at 2000 mA -> 0 mA; required key, [] accepted; BOM identical', ['conditions[*]', 'load_milliamps', 'input_steady']),
 ('power.sense[]', None, 'empty'): ('conservative', 'removes the VBAT_SENSE network (BOM loses R50, R51, C50) and its ADC judgement with it', ['VBAT_SENSE.pin', 'consumers']),
 ('power.drop_limits[]', None, 'delete'): ('relaxes', 'the supply gate is off: ran[1] judged 1 -> off power.drop_limits', ['ran[1]', 'supplies[VDD_3V3].domain', 'drop_bound', 'verdict']),
 ('power.drop_limits[]', None, 'empty'): ('relaxes', 'same as delete; [] is accepted and means none', ['ran[1]', 'supplies[VDD_3V3].domain', 'drop_bound', 'verdict']),
 ('test_points[]', None, 'delete'): ('conservative', 'removes the pads; the probe-load condition lines go with them (named in the RUL-14 amendment)', ['conditions[*]']),
 ('test_points[]', None, 'empty'): ('conservative', 'same as delete', ['conditions[*]']),
 ('board.stackup.dielectrics[].max_thickness_um', None, 'delete'): ('no change', 'doc (board.rs:249-253): absent prices the via barrel at the finished maximum less the other layers\' minima, i.e. a longer barrel, stricter; supplies[VDD_3V3].drop_bound_microvolts identical on the gated carrier', []),
 ('serial_ports[].negotiated', None, 'null'): ('refused', 'validate refuses; SerialFramingContradictsRole', []),
}
GENERIC = {
 'board.stackup.fab_tolerances.hole_size_minus_um': 'read only by the supply gate (supply.rs:1384): no change unless power.drop_limits is declared; with it, see the gated rows (refused at extract)',
 'board.stackup.fab_tolerances.track_width_permille': 'read only by the supply gate (supply.rs:1386): no change unless power.drop_limits is declared; with it, see the gated rows (refused at extract)',
 'net_constraints[].impedance_ohms': 'declared 50 vs null: no output of validate, solve, emit, extract or check speeds differs; no host gate reads it',
 'power.battery.series_milliohms': 'no output differs on the host commands',
 'connectors[].standard': 'no output differs on the host commands',  # keys whose accepted variants only re-run the solver
 'spi_buses[].sck.pin': 'solver freedom: a pinned pin becomes the solver\'s choice; may move the board (on pair-standin emit then says Unroutable, on sentinel-02-own a line fails); no judgement removed',
 'motor_outputs[].pin': 'solver freedom; pins move, no judgement removed',
 'serial_ports[].peripheral': 'solver freedom; no change',
}
def cls(r):
    if r.get('carrier') is None: return ('untested', 'no committed specification declares a value for it', [])
    if r.get('na'): return ('n/a', 'scalar: {} / [] does not apply', [])
    if r.get('refused'): return ('refused', f"validate exit 2 {r['refused']['kind']}", [])
    d = r.get('diff', [])
    errs = [x for x in d if x[0] == 'new' and x[1].endswith('.error.kind')]
    key = (r['path'], r['carrier'], r['variant'])
    for k in (key, (r['path'], None, r['variant'])):
        if k in M:
            c, n, q = M[k]
            return (c, n, q)
    if errs: return ('refused-later', f"{errs[0][1].split(':')[0]} {errs[0][2][0].strip(chr(34))}", ['error.message'])
    if not d: return ('no change', GENERIC.get(r['path'], ''), [])
    if r['path'] in GENERIC: return ('no change', GENERIC[r['path']], [])
    return ('UNCLASSIFIED', '', [])
def quote(r, q):
    out = []
    d = r.get('diff', [])
    for sub in q:
        for k, p, v in d:
            if sub in p and k != 'chg-num' and not (k == 'chg' and all(re.fullmatch(r'-?[\d.]+', x) for x in v)):
                out.append(f'{k} `{p}`: ' + ' / '.join(x[:230] for x in v[:2])); break
    return out[:3]
UNT = {r['path'] for k, r in rows.items() if r.get('carrier') and not r.get('na')}
rows = collections.OrderedDict((k, r) for k, r in rows.items() if not (r.get('carrier') is None and r['path'] in UNT) and not str(r.get('carrier')).startswith('syn:d1-drv8876'))
print('| key path | carrier | variant | validate | class | what changed (quoted) | type |')
print('|---|---|---|---|---|---|---|')
for key, r in rows.items():
    c, n, q = cls(r)
    val = '-' if r.get('carrier') is None or r.get('na') else ('exit 2' if r.get('refused') else 'exit 0')
    ex = ''
    if r.get('exits') and not r.get('refused'):
        bad = {k: v for k, v in r['exits'].items() if v}
        if bad: ex = ' [exit ' + ','.join(f'{k}={v}' for k, v in bad.items()) + ']'
    qs = quote(r, q)
    cell = (n + (': ' if n and qs else '') + '<br>'.join(qs)).replace('|', '\\|').replace('\n', ' ')
    print(f"| `{r['path']}` | {r.get('carrier') or '-'} | {r.get('variant') or '-'} | {val}{ex} | {c} | {cell} | {r.get('file','').split('/')[-1]}:{r.get('line','')} |")
