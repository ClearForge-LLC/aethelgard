"""mkplat.py VARIANT TIMER(0|1) DMA(0|1) [timer param lines...]: the stock stm32h753 -> h743 -> h7 chain copied under
/cache/proto/platforms/ (host ~/.cache/aet-4412/proto/platforms) as *-VARIANT.repl, with timer1 and/or dma1/dma2 retyped
as Aethelgard's models (timer params appended to timer1's entry, e.g. 'preload: false')."""
import sys
V = '/home/forge/aethelgard-4412/crates/aethelgard-twin/tests/renode-1.17.0/platforms/cpus/'
O = '/home/forge/.cache/aet-4412/proto/platforms/'
var, tim, dma, params = sys.argv[1], sys.argv[2] == '1', sys.argv[3] == '1', sys.argv[4:]
s = open(V + 'stm32h7.repl').read()
def sub(old, new):
    global s
    assert s.count(old) == 1, old
    s = s.replace(old, new)
if dma:
    for n, base, first in (('dma2', '0x40020400', 8), ('dma1', '0x40020000', 0)):
        sub('%s: DMA.STM32DMA @ {\n        sysbus %s;\n        rcc RegistrationPeripheralAlias.%s\n    }\n' % (n, base, n.upper()),
            '%s: DMA.AethelgardSTM32DMA @ {\n        sysbus %s;\n        rcc RegistrationPeripheralAlias.%s\n    }\n    dmamux: dmamux1\n    firstMuxChannel: %d\n    pacedRequests: "15"\n' % (n, base, n.upper(), first))
if tim:
    sub('timer1: Timers.STM32_Timer @ {\n        sysbus 0x40010000;\n        rcc RegistrationPeripheralAlias.TIM1\n    }\n',
        'timer1: Timers.AethelgardSTM32Timer @ {\n        sysbus 0x40010000;\n        rcc RegistrationPeripheralAlias.TIM1\n    }\n' + ''.join('    %s\n' % p for p in params) + '    UpdateDmaRequest -> dmamux1@15\n')
open(O + 'stm32h7-%s.repl' % var, 'w').write('// AET-WO-4412 PROTOTYPE variant %s: timer1 %s, dma1/dma2 %s %s\n' % (var, 'ours' if tim else 'stock', 'ours' if dma else 'stock', params) + s)
h743 = open(V + 'stm32h743.repl').read()
assert h743.startswith('using "platforms/cpus/stm32h7.repl"')
open(O + 'stm32h743-%s.repl' % var, 'w').write(h743.replace('using "platforms/cpus/stm32h7.repl"', 'using "/cache/proto/platforms/stm32h7-%s.repl"' % var, 1))
h753 = open(V + 'stm32h753.repl').read()
assert h753.startswith('using "./stm32h743.repl"')
open(O + 'stm32h753-%s.repl' % var, 'w').write(h753.replace('using "./stm32h743.repl"', 'using "/cache/proto/platforms/stm32h743-%s.repl"' % var, 1))
print("made", var)
