#!/usr/bin/env python3
"""AET-WO-4300j phase B2 mutants: one hand mutant per guard B2 adds or changes.

Each mutant replaces one exact span (it must occur once), runs the named test
binary with its filter, records killed (the run fails) or survived, and
restores the file from the copy taken before the edit, checking its sha256.
Nothing is restored with git: the B2 work is not committed while this runs.

usage: mutants_b2.py <out.tsv> [id ...]   (env CARGO_TARGET_DIR; run from the repo root)
"""
import hashlib, os, subprocess, sys

E = 'crates/aethelgard-eda/src/'
CLI = ('aethelgard-cli', '--test')
EDA = ('aethelgard-eda', '--lib')

M = [
 # id, guard, file, old, new, (crate, kind, binary), filter
 ('S1', 'motor_link_missing', E+'support.rs',
  '''            None => fail(
                "motor_link_missing",''', '''            None if false => fail(
                "motor_link_missing",''', CLI+('support_tier1',), 'a_motor_link_absent_is_missing'),
 ('S2', 'motor_link_part: the row\'s symbol', E+'support.rs',
  'if c.symbol.as_deref() != Some(l_row.symbol.as_str()) || c.value != "0" {', 'if c.value != "0" {',
  CLI+('support_tier1',), 'a_motor_link_of_another_part_or_footprint_fails_by_name'),
 ('S3', 'motor_link_part: the value 0', E+'support.rs',
  'if c.symbol.as_deref() != Some(l_row.symbol.as_str()) || c.value != "0" {', 'if c.symbol.as_deref() != Some(l_row.symbol.as_str()) {',
  CLI+('support_tier1',), 'a_motor_link_of_another_part_or_footprint_fails_by_name'),
 ('S4', 'motor_link_footprint', E+'support.rs',
  'if c.footprint.as_deref() != Some(l_row.footprint.as_str()) {', 'if c.footprint.is_none() {',
  CLI+('support_tier1',), 'a_motor_link_of_another_part_or_footprint_fails_by_name'),
 ('S5', 'motor_link_off_net', E+'support.rs',
  'if !joins(linked, &line, &pd) {', 'if !joins(linked, &line, &pd) && false {',
  CLI+('support_tier1',), 'a_motor_link_off_its_nets_fails_by_name'),
 ('S6', 'motor_link_fitment: fitted is populated', E+'support.rs',
  'PullFitment::Fitted => !c.unpopulated(),', 'PullFitment::Fitted => !c.marks.contains("dnp"),',
  CLI+('support_tier1',), 'a_motor_links_fitment_that_disagrees_with_the_declaration_fails_by_name'),
 ('S7', 'motor_link_fitment: open is dnp alone', E+'support.rs',
  'PullFitment::Open => c.marks.iter().eq(["dnp"].iter()),', 'PullFitment::Open => c.marks.contains("dnp"),',
  CLI+('support_tier1',), 'a_motor_links_fitment_that_disagrees_with_the_declaration_fails_by_name'),
 ('S8', 'motor_link_fitment: open is not populated', E+'support.rs',
  'PullFitment::Open => c.marks.iter().eq(["dnp"].iter()),', 'PullFitment::Open => c.marks.iter().all(|k| *k == "dnp"),',
  CLI+('support_tier1',), 'a_motor_links_fitment_that_disagrees_with_the_declaration_fails_by_name'),
 ('S9', 'motor_link_count', E+'support.rs',
  'if !bridged.is_empty() {', 'if false && !bridged.is_empty() {',
  CLI+('support_tier1',), 'a_second_part_across_a_motor_link_fails_by_name'),
 ('S10', 'motor_pull_off_net: on the pull net', E+'support.rs',
  'if !joins(reference, &pd, to) {', 'if !joins(reference, &pd, to) && !joins(reference, &line, to) {',
  CLI+('support_tier1',), 'a_motor_pull_off_its_nets_fails_by_name'),
 ('S11', 'motor_pull_count: past the link to `to`', E+'support.rs',
  'parallel.extend(across(&line, to));', '',
  CLI+('support_tier1',), 'a_second_resistor_beside_a_motor_pull_fails_by_name'),
 ('S12', 'motor_pull_count: beside the resistor', E+'support.rs',
  'let mut parallel = across(&pd, to);', 'let mut parallel: Vec<&String> = Vec::new();',
  CLI+('support_tier1',), 'a_second_resistor_beside_a_motor_pull_fails_by_name'),
 ('S13', 'a second path excludes the link', E+'support.rs',
  '.filter(|q| q.as_str() != reference && q.as_str() != linked && joins(q, a, b))', '.filter(|q| q.as_str() != reference && joins(q, a, b))',
  CLI+('support_tier1',), 'a_motor_link_off_its_nets_fails_by_name'),
 ('S14', 'motor_net_extra_resistor excludes the link', E+'support.rs',
  '''                    && q.as_str() != linked
                    && is_resistor(x)''', '''                    && is_resistor(x)''',
  CLI+('support_tier1',), 'the_motor_fixtures_pulls_pass_the_support_gate'),
 ('S15', 'motor_net_extra_resistor: the pull net too', E+'support.rs',
  '.any(|(_, n)| n == line || n == pd)', '.any(|(_, n)| n == line)',
  CLI+('support_tier1',), 'another_resistor_on_a_motor_line_fails_by_name'),
 ('S16', 'the netlist\'s marks, each by name', E+'support.rs',
  '                marks.insert(*mark);', '                marks.insert("dnp");',
  CLI+('support_tier1',), 'a_motor_links_fitment_that_disagrees_with_the_declaration_fails_by_name'),
 ('E1', 'schematic: (dnp yes)', E+'schematic.rs',
  '*dnp = Sx::node("dnp", &["yes"]);', '*dnp = Sx::node("dnp", &["no"]);',
  CLI+('emit_tier1',), 'an_open_motor_pulls_link_is_drawn_not_fitted_and_its_resistor_fitted'),
 ('E2', 'schematic: an open link is dnp', E+'schematic.rs',
  '            part.dnp = true;', '            part.dnp = false;',
  CLI+('emit_tier1',), 'an_open_motor_pulls_link_is_drawn_not_fitted_and_its_resistor_fitted'),
 ('E3', 'BOM: DNP in the note', E+'schematic.rs',
  '            note: if p.dnp {', '            note: if false && p.dnp {',
  CLI+('emit_tier1',), 'an_open_motor_pulls_link_is_drawn_not_fitted_and_its_resistor_fitted'),
 ('E4', 'board: the footprint\'s dnp attribute', E+'pcb.rs',
  '                    items.push(Sx::atom("dnp"));', '',
  CLI+('emit_tier1',), 'an_open_motor_pulls_link_is_drawn_not_fitted_and_its_resistor_fitted'),
 ('E5', 'JLCPCB: a DNP row is found by its note', E+'fab.rs',
  '.filter(|[.., note]| note == crate::schematic::DNP)', '.filter(|[.., note]| note == "never")',
  EDA, 'a_dnp_part_is_in_neither_jlcpcb_file'),
 ('E6', 'JLCPCB: no placement row for a DNP part', E+'fab.rs',
  '    placed.retain(|p| !dnp.contains(&p.reference));', '',
  EDA, 'a_dnp_part_is_in_neither_jlcpcb_file'),
 ('E7', 'JLCPCB: no BOM row for a DNP part', E+'fab.rs',
  '''        if dnp.contains(&reference.as_str()) {
            continue;
        }''', '',
  EDA, 'a_dnp_part_is_in_neither_jlcpcb_file'),
 ('E8', 'schematic: every link drawn', E+'schematic.rs',
  '''    for m in &spec.motor_outputs {
        let link = &m.pull.link;''', '''    for m in spec.motor_outputs.iter().filter(|_| false) {
        let link = &m.pull.link;''',
  CLI+('emit_tier1',), 'every_motor_net_carries_its_declared_pull_down_in_the_emitted_files'),
 ('E9', 'schematic: the resistor on the pull net', E+'schematic.rs',
  '''            Conn::Net(aethelgard_ir::names::motor_pull_net(&m.label)),
            to,''', '''            Conn::Net(m.label.clone()),
            to,''',
  CLI+('emit_tier1',), 'every_motor_net_carries_its_declared_pull_down_in_the_emitted_files'),
 ('E10', 'schematic: the resistor placed beside its motor net', E+'schematic.rs',
  '        part.beside = Some(m.label.clone());', '        part.beside = None;',
  CLI+('outline_tier1',), 'the_fixture_places_at_the_spikes_outline_with_every_gap_kept'),
 ('E11', 'placer: a part shares its beside net', E+'pcb.rs',
  '            mine.insert(format!("/{}", escape(label)));', '',
  EDA, 'a_part_placed_beside_a_net_goes_by_that_nets_ic_pad'),
]

def sha(b): return hashlib.sha256(b).hexdigest()

def main():
    out = sys.argv[1]; only = set(sys.argv[2:])
    rows = []
    for (i, guard, path, old, new, (crate, kind, *binary), flt) in M:
        if only and i not in only: continue
        orig = open(path, 'rb').read(); h = sha(orig)
        text = orig.decode()
        assert text.count(old) == 1, (i, 'span occurs %d times' % text.count(old))
        open(path, 'w').write(text.replace(old, new))
        cmd = ['cargo', 'test', '-p', crate, kind] + binary + ['--', flt]
        try:
            r = subprocess.run(['systemd-run', '--user', '--scope', '-q', '-p', 'MemoryMax=16G', '-p', 'MemorySwapMax=0', '--', 'nice', '-n', '10'] + cmd,
                               capture_output=True, text=True)
        finally:
            open(path, 'wb').write(orig)
            assert sha(open(path, 'rb').read()) == h, (i, 'not restored')
        log = r.stdout + r.stderr
        ran = 'running 0 tests' not in log and ('test result' in log or 'error[' in log)
        compiled = 'error[' not in log and 'could not compile' not in log
        killed = r.returncode != 0 and compiled
        verdict = 'killed' if killed else ('COMPILE-ERROR' if not compiled else 'SURVIVED')
        if 'running 0 tests' in log and r.returncode == 0: verdict = 'NO-TEST-RAN'
        open(os.path.join(os.path.dirname(out), 'mut-%s.log' % i), 'w').write(log)
        rows.append((i, guard, path, flt, verdict)); print(i, verdict, flush=True)
    with open(out, 'a') as w:
        for row in rows: w.write('\t'.join(row) + '\n')

main()
