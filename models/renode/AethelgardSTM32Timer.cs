//
// AethelgardSTM32Timer: the STM32H7 advanced/general-purpose timer (TIM1, TIM8, TIM2-5, TIM15-17) for Renode 1.17.0,
// counting lazily. AET-WO-4412 (D4 a), PROTOTYPE. MIT OR Apache-2.0. Written for Aethelgard; no Antmicro source copied.
//
// Two sources, kept apart:
//  [RM pN]   RM0433 Rev 9 (held, sha256 672a9433...), page N. The DMA side is the reference manual's alone:
//            UDE and the update DMA request, TIMx_DCR/TIMx_DMAR burst, the request held until served.
//  [stock]   Renode 1.17.0's Timers.STM32_Timer, as a BEHAVIOURAL reference only (no text copied): every rule the twin's
//            committed boards are judged by today (aethelgard-twin src/timer.rs, "the model's conventions") is kept, so
//            a board that never sets UDE sees the same register reads, interrupts and pin edges at the same virtual
//            instants. Where the stock convention departs from RM0433 the rule is labelled [stock != RM] with the page.
//
// Time. Renode's clock entries count in whole entry ticks plus an exact fraction, wrap to 0 with the fraction dropped
// when they reach their limit, and are processed at the exact nanosecond of the limit (BaseClockSource.Advance splits
// an advance at the nearest limit). This model keeps the same arithmetic for its counter and four compare channels in
// software (struct Entry) and asks the clock source for ONE event, at the earliest instant something observable can
// happen. Everything else is computed when the firmware next touches the timer ("catch-up"). See LAZY below.
//
// Modelled: up-counting, ARR (ARPE as stock: the limit reloads at every update), PSC, UDIS, URS, OPM, UG (EGR), CNT,
// RCR (interrupt only, as stock), UIE/UIF, CCxIE/CCxIF, output compare modes 0-7 on CH1-4 with stock's edges, CCxE,
// UDE -> UpdateDmaRequest held until served [RM p.641, p.695], DCR/DMAR bursts [RM pp.1586, 1615-1617], OCxPE preload
// [RM p.1603] (constructor preload: false gives [stock]'s rule).
// Left out, each warned by name when selected: down-counting and centre-aligned counting (CR1 DIR/CMS), slave modes
// (SMCR SMS != 0), input capture (CCxS != 0), CCxDE/TDE requests, CH5/CH6, PSC preload, break, MOE
// gating. [stock] has the same gaps except input capture, encoder and the gated/reset/trigger slave modes.
//
// Log contract: nothing per DMAR burst or CCR write; the DMA logs one line per completed transfer with every word
// moved (AethelgardSTM32DMA, ruling D-b). A broken laziness rule (an observable change found during catch-up) logs a Warning.
//
using System;
using System.Collections.Generic;

using Antmicro.Renode.Core;
using Antmicro.Renode.Core.Structure;
using Antmicro.Renode.Exceptions;
using Antmicro.Renode.Logging;
using Antmicro.Renode.Peripherals.Bus;
using Antmicro.Renode.Peripherals.CPU;
using Antmicro.Renode.Time;

namespace Antmicro.Renode.Peripherals.Timers
{
    [AllowedTranslations(AllowedTranslation.ByteToDoubleWord | AllowedTranslation.WordToDoubleWord)]
    public class AethelgardSTM32Timer : IDoubleWordPeripheral, IKnownSize, INumberedGPIOOutput, IGPIOReceiver
    {
        // preload: honour CCMRx.OCxPE as RM0433 p.1603 defines it (CCR writes go to the preload register, loaded into
        // the active one at each update event). false keeps [stock] (OCxPE is a tag: a CCR write acts at once).
        // stockExits (default true): ask the CPU to leave its translation block on every register write where [stock]
        // touches a clock entry (each LimitTimer setter does). Measured (AET-WO-4412 spike/4412/proto/NOTES.md): without
        // it the firmware's own trajectory moves (ArduPilot's MAVLink bytes differ from 10 s on arm12); with it, on arm12
        // (49 updates skipped) every access, byte, edge and end read equals [stock]'s except CCMR's OCxPE read-back with
        // preload on.
        // schedule (default "hybrid", ruling D-c): "lazy" wakes only for what can be observed (below); "eager" wakes at every limit the counter or a
        // running channel reaches, as [stock]'s clock entries stop the CPU; "hybrid" is eager while a DMA request or an
        // interrupt is enabled (UDE, any CCxDE, UIE, any CCxIE) and lazy otherwise (ruling D-c, NOTES §7).
        public AethelgardSTM32Timer(IMachine machine, ulong frequency, uint initialLimit, bool preload = true, bool stockExits = true,
            string schedule = "hybrid")
        {
            if(initialLimit == 0)
            {
                throw new ConstructionException("initialLimit has to be greater than zero");
            }
            if(frequency == 0)
            {
                throw new ConstructionException("frequency has to be greater than zero");
            }
            this.machine = machine;
            this.frequency = frequency;
            this.preload = preload;
            if(schedule != "lazy" && schedule != "eager" && schedule != "hybrid")
            {
                throw new ConstructionException("schedule must be lazy, eager or hybrid");
            }
            this.schedule = schedule;
            this.stockExits = stockExits;
            this.initialLimit = initialLimit;
            counterBits = 0;
            for(var v = (ulong)initialLimit; v != 0; v >>= 1)
            {
                counterBits++;
            }
            counterMask = counterBits >= 32 ? 0xFFFFFFFFu : (uint)((1UL << counterBits) - 1);
            outputs = new GPIO[Channels];
            var conns = new Dictionary<int, IGPIO>();
            for(var i = 0; i < Channels; i++)
            {
                outputs[i] = new GPIO();
                conns[i] = outputs[i];
            }
            Connections = conns;
            ch = new Entry[Channels];
            chan = new Channel[Channels];
            // One-shot event source in nanoseconds (a bare clock entry; re-arming it asks the CPU for a return, as a
            // LimitTimer setter does), re-armed to the earliest instant needed.
            wakeHandler = OnWake;
            machine.ClockSource.ExchangeClockEntryWith(wakeHandler, e => e, NewWakeEntry);
            Reset();
        }

        // ---- GPIO --------------------------------------------------------------------------------------------------

        [DefaultInterrupt]
        public GPIO IRQ { get; } = new GPIO();
        public GPIO BreakInterrupt { get; } = new GPIO();
        public GPIO UpdateInterrupt { get; } = new GPIO();
        public GPIO TriggerInterrupt { get; } = new GPIO();
        public GPIO CommutationInterrupt { get; } = new GPIO();
        public GPIO CaptureCompareInterrupt { get; } = new GPIO();
        // The update DMA request [RM p.1586 "generate multiple DMA requests upon a single event"; RM p.695 "pending
        // until served"]: set at an update event while DIER.UDE = 1, cleared when the burst's last DMAR access is made.
        // Wired by the platform to its DMAMUX1 input (RM0433 Table 124, p.697: TIM1_UP = 15).
        public GPIO UpdateDmaRequest { get; } = new GPIO();

        public IReadOnlyDictionary<int, IGPIO> Connections { get; }

        public long Size => 0x400;

        public void OnGPIO(int number, bool value)
        {
            if(number == ResetPin)
            {
                if(value)
                {
                    Reset();
                }
                return;
            }
            if(number < 0 || number >= Channels)
            {
                return;
            }
            // The pin loops back to its own channel's input (the platform wires gpioPortA 8 -> timer1@0): an output
            // channel ignores it, as [stock] does. Input capture is not modelled.
            if(chan[number].Mode != 0 && !warnedInput)
            {
                warnedInput = true;
                this.Log(LogLevel.Warning, "input capture on CH{0} is not modelled; the input is ignored", number + 1);
            }
        }

        public void Reset()
        {
            depth = 0;
            machine.ClockSource.ExchangeClockEntryWith(wakeHandler, e => e.With(enabled: false), NewWakeEntry);
            armedAt = ulong.MaxValue;
            lastT = Now();
            main = new Entry { Period = initialLimit, Freq = frequency, Enabled = false, OneShot = false };
            divider = 1;
            for(var i = 0; i < Channels; i++)
            {
                ch[i] = new Entry { Period = initialLimit, Freq = frequency, Enabled = false, OneShot = true };
                // [stock != RM p.1610]: CCRx reads back initialLimit after reset (the compare timer's first limit), not 0.
                // The preload starts equal to it, so enabling OCxPE alone leaves nothing pending (measured: a pending
                // preload here re-armed the event on a CCMR write, where [stock] makes the CPU leave no block).
                chan[i] = new Channel { Preload = initialLimit };
                outputs[i].Unset();
            }
            autoReload = initialLimit;
            enableRequested = false;
            udis = urs = arpe = false;
            cms = 0;
            ckd = 0;
            ti1s = false;
            sms = ts = 0;
            uie = tie = ude = false;
            dierDma = 0;
            uif = tif = false;
            rcr = 0;
            repetitionsLeft = 0;
            dba = dbl = 0;
            dmarIndex = 0;
            burstWords.Clear();
            if(requestPending)
            {
                requestPending = false;
            }
            UpdateDmaRequest.Unset();
            UpdateInterruptsInner();
        }

        // ---- bus ---------------------------------------------------------------------------------------------------

        public uint ReadDoubleWord(long offset)
        {
            depth++;
            try
            {
                if(offset == (long)Reg.CNT && machine.SystemBus.TryGetCurrentCPU(out var cpu))
                {
                    cpu.SyncTime(); // [stock] reads CNT at the CPU's own instant
                }
                CatchUp(Now());
                if(offset == (long)Reg.DMAR)
                {
                    return DmarAccess(false, 0);
                }
                return ReadRegister(offset);
            }
            finally
            {
                Leave();
            }
        }

        public void WriteDoubleWord(long offset, uint value)
        {
            depth++;
            try
            {
                CatchUp(Now());
                if(offset == (long)Reg.DMAR)
                {
                    DmarAccess(true, value);
                    return;
                }
                WriteRegister(offset, value);
            }
            finally
            {
                Leave();
            }
        }

        // ---- registers ---------------------------------------------------------------------------------------------

        private uint ReadRegister(long offset)
        {
            switch((Reg)offset)
            {
            case Reg.CR1:
                return (enableRequested ? 1u : 0) | (udis ? 2u : 0) | (urs ? 4u : 0) | (main.OneShot ? 8u : 0)
                    | ((uint)cms << 5) | (arpe ? 0x80u : 0); // DIR reads 0 (up only); CKD is a [stock] tag, reads 0
            case Reg.CR2:
                return ti1s ? 0x80u : 0; // [stock]: only TI1S is stored
            case Reg.SMCR:
                return (uint)sms | ((uint)ts << 4);
            case Reg.DIER:
            {
                uint v = uie ? 1u : 0;
                for(var i = 0; i < Channels; i++)
                {
                    v |= chan[i].InterruptEnable ? (2u << i) : 0;
                }
                v |= tie ? 0x40u : 0;
                v |= ude ? 0x100u : 0; // [RM p.1595] rw; [stock] tags it and reads 0
                v |= dierDma;          // CCxDE, TDE: stored, not modelled
                return v;
            }
            case Reg.SR:
            {
                uint v = uif ? 1u : 0;
                for(var i = 0; i < Channels; i++)
                {
                    v |= chan[i].InterruptFlag ? (2u << i) : 0;
                    v |= chan[i].Overcapture ? (0x200u << i) : 0;
                }
                v |= tif ? 0x40u : 0;
                return v;
            }
            case Reg.EGR:
                return 0;
            case Reg.CCMR1:
                return ReadCcmr(0);
            case Reg.CCMR2:
                return ReadCcmr(2);
            case Reg.CCER:
            {
                uint v = 0;
                for(var i = 0; i < Channels; i++)
                {
                    v |= (chan[i].OutputEnable ? 1u : 0) << (4 * i);
                    v |= (chan[i].Polarity ? 1u : 0) << (4 * i + 1);
                    v |= (chan[i].NPolarity ? 1u : 0) << (4 * i + 3);
                }
                return v; // CCxNE are [stock] tags, read 0
            }
            case Reg.CNT:
                return (uint)main.Value & counterMask;
            case Reg.PSC:
                return (uint)(divider - 1);
            case Reg.ARR:
                return autoReload & counterMask;
            case Reg.RCR:
                return rcr;
            case Reg.CCR1:
            case Reg.CCR2:
            case Reg.CCR3:
            case Reg.CCR4:
            {
                var i = (int)((offset - (long)Reg.CCR1) / 4);
                if(PreloadOn(i))
                {
                    return chan[i].Preload & counterMask; // "Read/Write operations access the preload register" [RM p.1603]
                }
                return (uint)ch[i].Period & counterMask; // output mode; input capture is not modelled
            }
            case Reg.BDTR:
                return 0; // [stock] tags every BDTR field
            case Reg.DCR:
                return (uint)dba | ((uint)dbl << 8); // [RM p.1615]
            default:
                this.Log(LogLevel.Warning, "read from unmodelled offset 0x{0:X}, returning 0", offset);
                return 0;
            }
        }

        private void WriteRegister(long offset, uint value)
        {
            if(stockExits && !inBurst)
            {
                StockExit(offset, value); // [stock] never sees a DMAR burst (DMAR is unhandled there), so a burst asks for none
            }
            switch((Reg)offset)
            {
            case Reg.CR1:
                WriteCr1(value);
                break;
            case Reg.CR2:
                ti1s = (value & 0x80) != 0;
                break;
            case Reg.SMCR:
                WriteSmcr(value);
                break;
            case Reg.DIER:
                WriteDier(value);
                break;
            case Reg.SR:
                // rc_w0 [RM p.1597]: writing 0 clears, writing 1 leaves the flag.
                if((value & 1) == 0)
                {
                    uif = false;
                }
                for(var i = 0; i < Channels; i++)
                {
                    if((value & (2u << i)) == 0)
                    {
                        chan[i].InterruptFlag = false;
                    }
                    if((value & (0x200u << i)) == 0)
                    {
                        chan[i].Overcapture = false;
                    }
                }
                if((value & 0x40) == 0)
                {
                    tif = false;
                }
                UpdateInterrupts();
                break;
            case Reg.EGR:
                WriteEgr(value);
                break;
            case Reg.CCMR1:
                WriteCcmr(0, value);
                break;
            case Reg.CCMR2:
                WriteCcmr(2, value);
                break;
            case Reg.CCER:
                for(var i = 0; i < Channels; i++)
                {
                    chan[i].Polarity = ((value >> (4 * i + 1)) & 1) != 0;
                    chan[i].NPolarity = ((value >> (4 * i + 3)) & 1) != 0;
                    var oe = ((value >> (4 * i)) & 1) != 0;
                    chan[i].OutputEnable = oe;
                    UpdateChannel(i);
                    if(!oe)
                    {
                        SetOutput(i, false);
                    }
                }
                break;
            case Reg.CNT:
            {
                var v = value & counterMask;
                main.Value = Math.Min(v, initialLimit); // keeps the fraction, as a clock entry's Value does
                for(var i = 0; i < Channels; i++)
                {
                    if(v < ch[i].Period)
                    {
                        ch[i].Value = v;
                    }
                }
                ReachNow();
                UpdateInterrupts();
                break;
            }
            case Reg.PSC:
            {
                // [stock != RM p.1609]: the prescaler applies at once, not at the next update event.
                var d = (ulong)(value & 0xFFFF) + 1;
                if(d != divider)
                {
                    divider = d;
                    main.Freq = frequency / divider;
                    main.Resid = 0;
                    for(var i = 0; i < Channels; i++)
                    {
                        ch[i].Freq = main.Freq;
                        ch[i].Resid = 0;
                    }
                }
                UpdateInterrupts();
                break;
            }
            case Reg.ARR:
                autoReload = value & counterMask;
                main.Enabled = enableRequested && autoReload > 0; // [stock]: no slave-mode check here
                if(!arpe)
                {
                    main.Period = autoReload;
                }
                ReachNow();
                UpdateInterrupts();
                break;
            case Reg.RCR:
                rcr = value & 0xFF;
                break;
            case Reg.CCR1:
            case Reg.CCR2:
            case Reg.CCR3:
            case Reg.CCR4:
                WriteCcr((int)((offset - (long)Reg.CCR1) / 4), value & counterMask);
                break;
            case Reg.BDTR:
                if((value & ~0x8000u) != 0 && !warnedBdtr)
                {
                    warnedBdtr = true;
                    this.Log(LogLevel.Warning, "BDTR 0x{0:X}: break and dead time are not modelled", value);
                }
                break;
            case Reg.DCR:
                dba = (int)(value & 0x1F);
                dbl = (int)((value >> 8) & 0x1F);
                FlushBurst();
                dmarIndex = 0;
                break;
            default:
                this.Log(LogLevel.Warning, "write to unmodelled offset 0x{0:X}, value 0x{1:X}", offset, value);
                break;
            }
        }

        private void WriteCr1(uint value)
        {
            enableRequested = (value & 1) != 0;
            if(!SlaveStopped)
            {
                main.Enabled = enableRequested && autoReload > 0;
            }
            udis = (value & 2) != 0;
            urs = (value & 4) != 0;
            main.OneShot = (value & 8) != 0; // OPM: the counter stops at the next update event [RM p.1589]
            if((value & 0x10) != 0 && !warnedDir)
            {
                warnedDir = true;
                this.Log(LogLevel.Warning, "CR1.DIR (down-counting) is not modelled; counting up");
            }
            cms = (int)((value >> 5) & 3);
            if(cms != 0 && !warnedCms)
            {
                warnedCms = true;
                this.Log(LogLevel.Warning, "CR1.CMS {0} (centre-aligned) is not modelled; counting up, edge-aligned", cms);
            }
            arpe = (value & 0x80) != 0;
            ckd = (int)((value >> 8) & 3);
            for(var i = 0; i < Channels; i++)
            {
                UpdateChannel(i);
            }
            UpdateInterrupts();
        }

        private void WriteSmcr(uint value)
        {
            sms = (int)(value & 7);
            ts = (int)((value >> 4) & 7);
            if(sms != 0 && !warnedSms)
            {
                warnedSms = true;
                this.Log(LogLevel.Warning, "SMCR.SMS {0} (slave mode) is not modelled", sms);
            }
            if(sms == 7)
            {
                return; // [stock] refuses external clock mode 1 and leaves the counter as it was
            }
            main.Enabled = !SlaveStopped && enableRequested && autoReload > 0;
        }

        private void WriteDier(uint value)
        {
            uie = (value & 1) != 0;
            for(var i = 0; i < Channels; i++)
            {
                chan[i].InterruptEnable = (value & (2u << i)) != 0;
                UpdateChannel(i);
            }
            tie = (value & 0x40) != 0;
            var newUde = (value & 0x100) != 0;
            dierDma = value & 0x5E00; // CC1DE..CC4DE, TDE
            if(dierDma != 0 && !warnedDierDma)
            {
                warnedDierDma = true;
                this.Log(LogLevel.Warning, "DIER 0x{0:X}: CCxDE/TDE DMA requests are not modelled (UDE is)", value);
            }
            if(ude && !newUde && requestPending)
            {
                // The request is UDE-gated: clearing UDE withdraws one not yet served.
                requestPending = false;
                UpdateDmaRequest.Unset();
            }
            ude = newUde;
            UpdateInterrupts();
        }

        private void WriteEgr(uint value)
        {
            // [stock != RM p.1599]: any EGR write acts as UG (the stock callback ignores the written bit); RM0433 acts
            // on UG = 1 only. CCxG, COMG, TG, BG are not modelled.
            if(udis)
            {
                UpdateInterrupts();
                return;
            }
            main.Value = 0; // keeps the fraction (a clock entry's Value setter does)
            repetitionsLeft = rcr;
            LoadPreloads(); // UG "generates an update of the registers" [RM p.1599]
            // [stock != RM p.1597]: UIF is set by UG only while UIE = 1 (RM0433 sets it whenever URS = 0).
            if(!urs && uie)
            {
                uif = true;
            }
            for(var i = 0; i < Channels; i++)
            {
                if(ch[i].Enabled)
                {
                    ch[i].Value = main.Value;
                }
            }
            // [RM p.1589, CR1.URS]: with URS = 0 a UG update also issues the update DMA request.
            if(!urs && ude && !requestPending)
            {
                RaiseRequest();
            }
            UpdateInterrupts();
        }

        private uint ReadCcmr(int first)
        {
            uint v = 0;
            for(var h = 0; h < 2; h++)
            {
                var c = chan[first + h];
                uint half = (uint)c.Mode;
                if(c.Mode == 0)
                {
                    half |= (uint)c.CompareMode << 4; // OCxFE, OCxCE are [stock] tags, read 0
                    half |= preload && c.PreloadEnable ? 8u : 0; // OCxPE: read back with preload on; [stock] reads 0
                }
                else
                {
                    half |= c.InputBits; // ICxPSC, ICxF
                }
                v |= half << (8 * h);
            }
            return v;
        }

        private void WriteCcmr(int first, uint value)
        {
            for(var h = 0; h < 2; h++)
            {
                var i = first + h;
                var half = (value >> (8 * h)) & 0xFF;
                var wasOutput = chan[i].Mode == 0;
                var sel = (int)(half & 3);
                if(sel == 3)
                {
                    this.Log(LogLevel.Warning, "CC{0}S = TRC is not modelled; ignored", i + 1);
                }
                else
                {
                    if(sel != 0 && !warnedInput)
                    {
                        warnedInput = true;
                        this.Log(LogLevel.Warning, "CC{0}S {1}: input capture is not modelled", i + 1, sel);
                    }
                    chan[i].Mode = sel;
                }
                if(wasOutput)
                {
                    var mode = (int)((half >> 4) & 7);
                    chan[i].CompareMode = mode;
                    chan[i].PreloadEnable = (half & 8) != 0;
                    // Forced levels apply at once ([stock] sets the pin for modes 1, 2, 4, 5 when written).
                    switch(mode)
                    {
                    case ForceInactive:
                    case SetActiveOnMatch:
                        SetOutput(i, false);
                        break;
                    case SetInactiveOnMatch:
                    case ForceActive:
                        SetOutput(i, true);
                        break;
                    }
                }
                else
                {
                    chan[i].InputBits = half & 0xFC;
                }
            }
        }

        private void WriteCcr(int i, uint value)
        {
            chan[i].Preload = value;
            if(PreloadOn(i))
            {
                return; // the active register follows at the next update event [RM p.1603]
            }
            if(value == 0)
            {
                ch[i].Enabled = false;
            }
            ch[i].Period = value;
            // A limit at or below the channel's count is reached at once (a clock entry's limit setter updates it).
            if(ch[i].Enabled && ch[i].Value >= ch[i].Period)
            {
                ReachNow();
            }
            UpdateChannel(i);
            UpdateInterrupts();
        }

        // ---- DMA burst [RM pp.1586-1587, 1615-1617] ----------------------------------------------------------------

        private uint DmarAccess(bool write, uint value)
        {
            // "(TIMx_CR1 address) + (DBA + DMA index) x 4 ... DMA index ... ranges from 0 to DBL" [RM p.1617]
            var target = (long)(dba + dmarIndex) * 4;
            uint result = 0;
            if(target == (long)Reg.DMAR)
            {
                this.Log(LogLevel.Warning, "DMAR burst points at DMAR itself (DBA {0}, index {1}); access dropped", dba, dmarIndex);
            }
            else if(write)
            {
                inBurst = true;
                try
                {
                    WriteRegister(target, value);
                }
                finally
                {
                    inBurst = false;
                }
            }
            else
            {
                result = ReadRegister(target);
            }
            burstWords.Add(write ? value : result);
            if(burstStart < 0)
            {
                burstStart = target;
            }
            dmarIndex++;
            if(dmarIndex > dbl) // DBL 0 = 1 transfer, ..., so the burst holds DBL + 1 accesses [RM p.1616]
            {
                dmarIndex = 0;
                FlushBurst();
                if(requestPending)
                {
                    // "The peripheral releases its request as soon as it gets the Acknowledge" [RM p.641]
                    requestPending = false;
                    UpdateDmaRequest.Unset();
                }
            }
            return result;
        }

        // Where [stock]'s write handlers call a LimitTimer setter (each requests a return of the current CPU).
        private void StockExit(long offset, uint value)
        {
            switch((Reg)offset)
            {
            case Reg.CR1:
            case Reg.SMCR:
            case Reg.DIER:
            case Reg.EGR:
            case Reg.CCER:
            case Reg.CNT:
            case Reg.ARR:
            case Reg.CCR1:
            case Reg.CCR2:
            case Reg.CCR3:
            case Reg.CCR4:
                break;
            case Reg.PSC:
                if((ulong)(value & 0xFFFF) + 1 == divider)
                {
                    return;
                }
                break;
            default:
                return;
            }
            if(machine.SystemBus.TryGetCurrentCPU(out var cpu))
            {
                (cpu as IControllableCPU)?.RequestReturn();
            }
        }

        private void FlushBurst()
        {
            if(burstWords.Count == 0)
            {
                return;
            }
            // The words are logged by the DMA, once per transfer (ruling D-b).
            burstWords.Clear();
            burstStart = -1;
        }

        private void RaiseRequest()
        {
            requestPending = true;
            if(late)
            {
                Violation("update DMA request");
            }
            // The DMA serves the request inside this call, through DMAR; a served burst clears requestPending.
            UpdateDmaRequest.Set();
        }

        // ---- the counter in virtual time ---------------------------------------------------------------------------

        // A clock entry's arithmetic: Value whole ticks, Resid the fraction in 1e-9 tick, Freq ticks per second.
        // Advancing by d ns adds d * Freq units of 1e-9 tick; the limit is reached when Value >= Period, which drops
        // the fraction ([stock] convention: a period is Period ticks realised in whole ns, rounded up; src/timer.rs).
        private struct Entry
        {
            public ulong Value;
            public ulong Resid;
            public ulong Period;
            public ulong Freq;
            public bool Enabled;
            public bool OneShot;

            public void Advance(ulong ns)
            {
                if(!Enabled || ns == 0)
                {
                    return;
                }
                while(ns > 0)
                {
                    var step = Math.Min(ns, 1000000000UL); // keeps ns * Freq inside 64 bits
                    var total = Resid + step * Freq;
                    Value += total / Unit;
                    Resid = total % Unit;
                    ns -= step;
                }
            }

            // ns until Value reaches Period: ceil(((Period - Value) * 1e9 - Resid) / Freq), 0 if reached.
            public ulong NsToReach()
            {
                if(Value >= Period)
                {
                    return 0;
                }
                var units = (Period - Value) * Unit - Resid;
                return (units + Freq - 1) / Freq;
            }

            public bool Reached => Enabled && Value >= Period;
        }

        private ulong Now()
        {
            return machine.ClockSource.CurrentValue.Ticks;
        }

        private void AdvanceAll(ulong ns)
        {
            main.Advance(ns);
            for(var i = 0; i < Channels; i++)
            {
                ch[i].Advance(ns);
            }
        }

        // Process every limit reached up to t, in time order; limits reached at the same instant run as one clock-source
        // update does: the counter's first, then the channels' in index order, each decided before any handler runs.
        private void CatchUp(ulong t)
        {
            if(t < lastT)
            {
                return;
            }
            while(true)
            {
                var next = ulong.MaxValue;
                if(main.Enabled)
                {
                    next = Math.Min(next, main.NsToReach());
                }
                for(var i = 0; i < Channels; i++)
                {
                    if(ch[i].Enabled)
                    {
                        next = Math.Min(next, ch[i].NsToReach());
                    }
                }
                if(next == ulong.MaxValue || next > t - lastT)
                {
                    break;
                }
                AdvanceAll(next);
                lastT += next;
                ProcessReached(lastT < t);
            }
            AdvanceAll(t - lastT);
            lastT = t;
        }

        // Limits reached at the current instant (a register write lowered a limit to the count).
        private void ReachNow()
        {
            if(main.Reached || ch[0].Reached || ch[1].Reached || ch[2].Reached || ch[3].Reached)
            {
                ProcessReached(false);
            }
        }

        private void ProcessReached(bool isLate)
        {
            var mainHit = main.Reached;
            var chHit = new bool[Channels];
            if(mainHit)
            {
                main.Value = 0;
                main.Resid = 0;
                if(main.OneShot)
                {
                    main.Enabled = false;
                }
            }
            for(var i = 0; i < Channels; i++)
            {
                chHit[i] = ch[i].Reached;
                if(chHit[i])
                {
                    ch[i].Value = 0;
                    ch[i].Resid = 0;
                    ch[i].Enabled = false;
                }
            }
            var saved = late;
            late = isLate;
            try
            {
                if(mainHit)
                {
                    OnUpdate();
                }
                for(var i = 0; i < Channels; i++)
                {
                    if(chHit[i])
                    {
                        OnCompare(i);
                    }
                }
            }
            finally
            {
                late = saved;
            }
        }

        // The update event ([stock] order: reload, flag, channels, interrupt; then the DMA request [RM p.1586]).
        private void OnUpdate()
        {
            if(udis)
            {
                return; // [RM p.1589] UDIS: no update event
            }
            if(main.OneShot)
            {
                enableRequested = false;
            }
            main.Period = autoReload;
            uif = true;
            LoadPreloads();
            for(var i = 0; i < Channels; i++)
            {
                UpdateChannel(i);
                if(!ch[i].Enabled || chan[i].Mode != 0)
                {
                    continue;
                }
                if(chan[i].CompareMode == PwmMode1)
                {
                    SetOutput(i, true);
                }
                else if(chan[i].CompareMode == PwmMode2)
                {
                    SetOutput(i, false);
                }
            }
            if(uie && repetitionsLeft == 0)
            {
                repetitionsLeft = 1u + rcr * (cms == 1 || cms == 2 ? 2u : 1u);
                UpdateInterrupts();
            }
            if(repetitionsLeft > 0)
            {
                repetitionsLeft--;
            }
            if(ude && !requestPending)
            {
                RaiseRequest();
            }
        }

        private void OnCompare(int i)
        {
            if(chan[i].Mode != 0)
            {
                return;
            }
            switch(chan[i].CompareMode)
            {
            case SetActiveOnMatch:
                SetOutput(i, true);
                SetOutput(i, false); // [stock]: a high pulse of no width
                break;
            case SetInactiveOnMatch:
                SetOutput(i, false);
                SetOutput(i, true);  // [stock]: a low pulse of no width
                break;
            case ToggleOnMatch:
                SetOutput(i, !outputs[i].IsSet);
                break;
            case PwmMode1:
                SetOutput(i, false);
                break;
            case PwmMode2:
                SetOutput(i, true);
                break;
            }
            if(chan[i].InterruptEnable)
            {
                chan[i].InterruptFlag = true;
                UpdateInterrupts();
            }
        }

        // [stock]: a channel's compare counts while the counter runs, the channel is enabled (CCxE or CCxIE) and the
        // count is below CCR; it restarts from the counter's value.
        private void UpdateChannel(int i)
        {
            ch[i].Enabled = main.Enabled && (chan[i].InterruptEnable || chan[i].OutputEnable) && main.Value < ch[i].Period;
            if(ch[i].Enabled)
            {
                ch[i].Value = main.Value;
            }
        }

        private void SetOutput(int i, bool value)
        {
            if(outputs[i].IsSet == value)
            {
                return;
            }
            if(late)
            {
                Violation("CH" + (i + 1) + " edge");
            }
            outputs[i].Set(value);
        }

        private void UpdateInterrupts()
        {
            var before = IRQ.IsSet;
            UpdateInterruptsInner();
            if(late && IRQ.IsSet != before)
            {
                Violation("interrupt");
            }
        }

        private void UpdateInterruptsInner()
        {
            var cc = false;
            for(var i = 0; i < Channels; i++)
            {
                cc |= chan[i].InterruptFlag && chan[i].InterruptEnable;
            }
            var up = uif && uie;
            var trg = tif && tie;
            IRQ.Set(cc || up || trg);
            BreakInterrupt.Set(false);
            UpdateInterrupt.Set(up);
            TriggerInterrupt.Set(trg);
            CommutationInterrupt.Set(false);
            CaptureCompareInterrupt.Set(cc);
        }

        private void Violation(string what)
        {
            violations++;
            if(violations <= 20)
            {
                this.Log(LogLevel.Warning, "lazy: a skipped event at {0} ns had an effect ({1}), applied at {2} ns", lastT, what, Now());
            }
        }

        // ---- LAZY: when an event is needed -------------------------------------------------------------------------
        // An update event can be observed only through: the update interrupt (UIE), an update DMA request not already
        // pending (UDE), or a channel that acts at the update or at its compare (an output in an active compare mode
        // with CCR > 0, or CCxIE). Anything else an update changes (UIF, CNT, OPM's CEN, RCR's count, ARR's reload) is
        // read through the bus, and every bus access catches up first. So: if any of those holds, wake at the earliest
        // limit (counter or channel); else sleep until the firmware next writes. The first of those is exact; the
        // second is checked: an observable change found during catch-up is counted and warned (violations).
        private bool PreloadOn(int i)
        {
            return preload && chan[i].PreloadEnable && chan[i].Mode == 0;
        }

        // Preload -> active at an update event (RM p.1603); the channel restarts as the update re-arms it.
        private void LoadPreloads()
        {
            for(var i = 0; i < Channels; i++)
            {
                if(!PreloadOn(i) || ch[i].Period == chan[i].Preload)
                {
                    continue;
                }
                if(chan[i].Preload == 0)
                {
                    ch[i].Enabled = false;
                }
                ch[i].Period = chan[i].Preload;
            }
        }

        private bool Observable()
        {
            if(schedule == "eager")
            {
                return true;
            }
            if(schedule == "hybrid" && (ude || dierDma != 0 || uie || chan[0].InterruptEnable || chan[1].InterruptEnable || chan[2].InterruptEnable || chan[3].InterruptEnable))
            {
                return true; // a DMA request or an interrupt is enabled: wake at every limit (ruling D-c)
            }
            if(uie || (ude && !requestPending))
            {
                return true;
            }
            for(var i = 0; i < Channels; i++)
            {
                if(chan[i].InterruptEnable)
                {
                    return true;
                }
                if(PreloadOn(i) && chan[i].Preload != ch[i].Period)
                {
                    return true; // a pending preload changes what the next update does
                }
                if(chan[i].Mode == 0 && chan[i].OutputEnable && ch[i].Period > 0 && chan[i].CompareMode >= SetActiveOnMatch && chan[i].CompareMode <= ToggleOnMatch)
                {
                    return true;
                }
                if(chan[i].Mode == 0 && chan[i].OutputEnable && ch[i].Period > 0 && (chan[i].CompareMode == PwmMode1 || chan[i].CompareMode == PwmMode2))
                {
                    return true;
                }
            }
            return false;
        }

        private void Arm()
        {
            var now = Now();
            CatchUp(now);
            var next = ulong.MaxValue;
            if(Observable())
            {
                if(main.Enabled)
                {
                    next = Math.Min(next, main.NsToReach());
                }
                for(var i = 0; i < Channels; i++)
                {
                    if(ch[i].Enabled)
                    {
                        next = Math.Min(next, ch[i].NsToReach());
                    }
                }
            }
            var target = next == ulong.MaxValue ? ulong.MaxValue : lastT + next;
            if(target == armedAt)
            {
                return; // already armed there: no clock-source write
            }
            armedAt = target;
            var delta = target == ulong.MaxValue ? 0 : (target > now ? target - now : 1);
            machine.ClockSource.ExchangeClockEntryWith(wakeHandler,
                e => target == ulong.MaxValue ? e.With(enabled: false) : e.With(period: delta, value: 0, enabled: true), NewWakeEntry);
            if(machine.SystemBus.TryGetCurrentCPU(out var cpu))
            {
                (cpu as IControllableCPU)?.RequestReturn(); // as a LimitTimer setter does
            }
        }

        private ClockEntry NewWakeEntry()
        {
            return new ClockEntry(1, 1000000000, wakeHandler, this, "aet-tim", enabled: false, direction: Direction.Ascending, workMode: WorkMode.OneShot);
        }

        private void OnWake()
        {
            depth++;
            try
            {
                armedAt = ulong.MaxValue;
                CatchUp(Now());
            }
            finally
            {
                Leave();
            }
        }

        private void Leave()
        {
            depth--;
            if(depth == 0)
            {
                depth++;
                try
                {
                    Arm();
                }
                finally
                {
                    depth--;
                }
            }
        }

        private bool SlaveStopped => sms == 4 + 2 || (sms >= 1 && sms <= 3); // [stock]: trigger and encoder modes stop it

        private struct Channel
        {
            public int Mode;            // CCxS: 0 output, 1-3 input (not modelled)
            public int CompareMode;     // OCxM[2:0]
            public uint InputBits;
            public uint Preload;        // CCRx as written (the preload register when OCxPE and preload are on)
            public bool PreloadEnable;  // OCxPE
            public bool InterruptEnable;
            public bool InterruptFlag;
            public bool Overcapture;
            public bool OutputEnable;
            public bool Polarity;
            public bool NPolarity;
        }

        private enum Reg : long
        {
            CR1 = 0x00,
            CR2 = 0x04,
            SMCR = 0x08,
            DIER = 0x0C,
            SR = 0x10,
            EGR = 0x14,
            CCMR1 = 0x18,
            CCMR2 = 0x1C,
            CCER = 0x20,
            CNT = 0x24,
            PSC = 0x28,
            ARR = 0x2C,
            RCR = 0x30,
            CCR1 = 0x34,
            CCR2 = 0x38,
            CCR3 = 0x3C,
            CCR4 = 0x40,
            BDTR = 0x44,
            DCR = 0x48,
            DMAR = 0x4C,
        }

        private const ulong Unit = 1000000000UL;
        private const int Channels = 4;
        private const int ResetPin = 0xFF;
        private const int SetActiveOnMatch = 1;
        private const int SetInactiveOnMatch = 2;
        private const int ToggleOnMatch = 3;
        private const int ForceInactive = 4;
        private const int ForceActive = 5;
        private const int PwmMode1 = 6;
        private const int PwmMode2 = 7;

        private Entry main;
        private readonly Entry[] ch;
        private readonly Channel[] chan;
        private readonly GPIO[] outputs;
        private readonly Action wakeHandler;
        private readonly IMachine machine;
        private readonly ulong frequency;
        private readonly bool preload;
        private readonly string schedule;
        private readonly bool stockExits;
        private readonly uint initialLimit;
        private readonly int counterBits;
        private readonly uint counterMask;
        private readonly List<uint> burstWords = new List<uint>();

        private ulong lastT;
        private ulong armedAt = ulong.MaxValue;
        private ulong divider;
        private uint autoReload;
        private bool enableRequested;
        private bool udis, urs, arpe, ti1s, uie, tie, ude, uif, tif;
        private int cms, ckd, sms, ts;
        private uint dierDma;
        private uint rcr;
        private uint repetitionsLeft;
        private int dba, dbl, dmarIndex;
        private long burstStart = -1;
        private bool requestPending;
        private bool late;
        private bool inBurst;
        private int depth;
        private bool warnedInput, warnedDir, warnedCms, warnedSms, warnedDierDma, warnedBdtr;
        private ulong violations;
    }
}
