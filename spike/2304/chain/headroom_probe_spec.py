import json,sys
s=json.load(open(sys.argv[1])); add=int(sys.argv[2]); cm=int(sys.argv[3]); p=s['power']
p['regulators']=[r for r in p['regulators'] if r['reference']!='U10']
p['loads']=[l for l in p['loads'] if l['domain']!='VDD_5V']
s['voltage_domains']=[d for d in s['voltage_domains'] if d['id']!='VDD_5V']
s['test_points']=[t for t in s['test_points'] if t.get('net')!='VDD_5V']
for c in s['connectors']:
  for pin in c['pins']:
    if pin['net']=='VDD_5V': pin['net']=None; c['standard']=None
for nc in s['net_classes']: nc['nets']=[n for n in nc['nets'] if n!='VDD_5V']
p['battery']['cell_min_millivolts']['value']=cm
if add: p['loads'].append({'domain':'VDD_3V3','milliamps':add,'source':'mirror probe'})
print(json.dumps(s))
