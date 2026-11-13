import json,sys
for l in open(sys.argv[1]):
    if not l.startswith('{'): continue
    d=json.loads(l)
    print(f"{d['case']:>13} q={d['quantum_ns']/1000:g}us tmax={d.get('tmax_ns') or 'def'} span={d['span_ns']/1e6:g}ms pts={d['accepted_points']} min={d['rail_min_v']:.4f} run={d['run_s']}s wall={d['wall_s']}s resets={d['reset_intervals_ns']}")
    for k in ['tolerance_floor','bor3_fall','bor3_rise','bor0_fall','bor0_rise']:
        x=d[k]
        if x['exact_ns'] or x['seen_at_boundary_ns']:
            ex=[e[0] for e in x['exact_ns']]
            lag=[round(s-e,3) for s,e in zip(x['seen_at_boundary_ns'],ex)]
            print(f"     {k:16} exact={[round(e,3) for e in ex]} seen={x['seen_at_boundary_ns']} lag_ns={lag} meas=({x['meas_fall_ns']},{x['meas_rise_ns']})")
