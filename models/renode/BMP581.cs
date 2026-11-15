//
// BMP581: a behavioural model of the Bosch BMP581 barometric pressure sensor on an I2C bus, for Renode 1.17.0.
// AET-WO-4220. MIT OR Apache-2.0. Written for Aethelgard; no Antmicro source copied.
//
// Facts come from the hand-read datasheet BST-BMP581-DS004-13 (page numbers beside each reset value).
// Modelled: the register map of section 7, the power modes (standby, normal, forced, continuous) with
// their start-up and conversion times in virtual time, the data registers, the data-ready flag, the INT
// pin. Not modelled: the FIFO (FIFO_COUNT reads 0, FIFO_DATA reads the empty frame 0x7F), the IIR
// filters (data is the unfiltered value), the out-of-range interrupt, the NVM, SPI and I3C, the
// data shadowing across a burst read, t_standby and the INT de-assertion gap. Registers without a
// datasheet fact read 0x00, ignore writes and log a warning.
//
// Log contract: at the end of an I2C transaction (FinishTransmission) one Noisy line for its writes and
// one for its reads; a transaction that moved no byte logs nothing.
//
using System;
using System.Collections.Generic;
using System.Text;

using Antmicro.Renode.Core;
using Antmicro.Renode.Logging;
using Antmicro.Renode.Peripherals.I2C;
using Antmicro.Renode.Peripherals.Timers;
using Antmicro.Renode.Time;

namespace Antmicro.Renode.Peripherals.Sensors
{
    public class BMP581 : II2CPeripheral
    {
        public BMP581(IMachine machine)
        {
            this.machine = machine;
            INT = new GPIO();
            foreach(var d in Defs)
            {
                kind[d[0]] = (byte)d[2];
                mask[d[0]] = (byte)d[3];
            }
            for(var a = 0x1D; a <= 0x22; a++)
            {
                kind[a] = RO;
            }
            for(var a = 0x23; a <= 0x26; a++)
            {
                kind[a] = RO;
            }
            // One-shot timer in nanoseconds, re-armed to the earliest pending event.
            timer = new LimitTimer(machine.ClockSource, 1000000000, this, "bmp581", limit: 1000000000, workMode: WorkMode.OneShot, eventEnabled: true, autoUpdate: true);
            timer.LimitReached += OnTimer;
            Pressure = 101325;
            Temperature = 25;
            Reset();
        }

        // Stimulus, sampled when a measurement completes. Pressure in Pa, temperature in degC.
        public double Pressure { get; set; }
        public double Temperature { get; set; }

        public GPIO INT { get; private set; }

        public void Reset()
        {
            ResetRegisters();
            pointer = 0; // "By default the start address is set at 8h'00", DS p.40
            lastWriteAddress = 0;
            resetUntil = 0;
            ClearTransaction();
            timer.Reset();
        }

        public void Write(byte[] data)
        {
            foreach(var b in data)
            {
                if(!txPointerSet || txLastWasRead)
                {
                    pointer = (byte)(b & 0x7F);
                    lastWriteAddress = pointer;
                    txPointerSet = true;
                    txLastWasRead = false;
                    txPointerByte = pointer;
                    continue;
                }
                if(txWritten.Count == 0)
                {
                    txWriteStart = pointer;
                }
                WriteRegister(pointer, b);
                txWritten.Add(b);
                pointer = (byte)((pointer + 1) & 0x7F);
            }
        }

        public byte[] Read(int count = 1)
        {
            var result = new byte[Math.Max(count, 0)];
            for(var i = 0; i < result.Length; i++)
            {
                if(txRead.Count == 0)
                {
                    // "Once a new data read transmission starts, the start address is set to the
                    // register address specified in the latest I2C write command", DS p.40.
                    pointer = lastWriteAddress;
                    txReadStart = pointer;
                }
                result[i] = ReadRegister(pointer);
                txRead.Add(result[i]);
                if(pointer != 0x29) // FIFO_DATA does not increment, DS p.36
                {
                    pointer = (byte)((pointer + 1) & 0x7F);
                }
                txLastWasRead = true;
            }
            return result;
        }

        public void FinishTransmission()
        {
            if(txWritten.Count > 0)
            {
                this.Log(LogLevel.Noisy, "write 0x{0:X2} <- {1}", txWriteStart, Join(txWritten));
            }
            if(txRead.Count > 0)
            {
                this.Log(LogLevel.Noisy, "read 0x{0:X2} -> {1}", txReadStart, Join(txRead));
            }
            if(txPointerSet && txWritten.Count == 0 && txRead.Count == 0)
            {
                this.Log(LogLevel.Noisy, "write 0x{0:X2} <- (none)", txPointerByte);
            }
            ClearTransaction();
        }

        private static string Join(List<byte> bytes)
        {
            var text = new StringBuilder();
            foreach(var b in bytes)
            {
                if(text.Length > 0)
                {
                    text.Append(' ');
                }
                text.Append("0x").Append(b.ToString("X2"));
            }
            return text.ToString();
        }

        private void ClearTransaction()
        {
            txPointerSet = false;
            txLastWasRead = false;
            txWritten.Clear();
            txRead.Clear();
        }

        // ---- registers -------------------------------------------------------------------------

        private void ResetRegisters()
        {
            Array.Clear(regs, 0, regs.Length);
            foreach(var d in Defs)
            {
                regs[d[0]] = (byte)d[1];
            }
            for(var a = 0x1D; a <= 0x22; a++)
            {
                regs[a] = 0x7F; // TEMP_DATA, PRESS_DATA reset 0x7F each, DS p.56-57
            }
            regs[0x27] = 0x10; // INT_STATUS.por: power-on, DS p.58 and 4.3.9 p.18
            actualMode = 0;
            measArmed = false;
            pulseActive = false;
            intAsserted = false;
            timer.Enabled = false;
            UpdatePin(false);
        }

        // t_soft_res: the part comes out of a soft reset after 2 ms (DS p.11, p.18); an access
        // before then is the firmware's fault, which fails its step. The access goes ahead.
        private void CheckReset(byte reg)
        {
            if(Now() < resetUntil)
            {
                this.Log(LogLevel.Warning, "access to 0x{0:X2} within 2 ms of a soft reset", reg);
            }
        }

        private byte ReadRegister(byte reg)
        {
            CheckReset(reg);
            if(kind[reg] == 0)
            {
                this.Log(LogLevel.Warning, "access to unmodelled register 0x{0:X2}", reg);
                return 0;
            }
            switch(reg)
            {
            case 0x27: // INT_STATUS, clear on read
                var status = regs[0x27];
                regs[0x27] = 0;
                if(intAsserted && !pulseActive)
                {
                    intAsserted = false;
                    UpdatePin(true);
                }
                return status;
            case 0x37: // ODR_CONFIG: a read of pwr_mode returns the actual mode, DS p.64
                return (byte)((regs[0x37] & 0xFC) | actualMode);
            case 0x7E: // CMD always reads 0x00, DS p.67
                return 0;
            default:
                return regs[reg];
            }
        }

        private void WriteRegister(byte reg, byte value)
        {
            CheckReset(reg);
            if(kind[reg] == 0)
            {
                this.Log(LogLevel.Warning, "access to unmodelled register 0x{0:X2}", reg);
                return;
            }
            if(kind[reg] == RO)
            {
                this.Log(LogLevel.Warning, "write to read-only register 0x{0:X2} ignored", reg);
                return;
            }
            if(reg == 0x7E)
            {
                if(value == 0xB6) // soft reset, DS p.67
                {
                    ResetRegisters();
                    resetUntil = Now() + SoftResetNs;
                }
                return;
            }
            // The state the write leaves decides the start-up: after a reset the part is in DEEP
            // STANDBY (DS p.16), whatever the same write sets deep_dis to.
            leavingDeep = actualMode == 0 && DeepStandby();
            regs[reg] = (byte)((regs[reg] & ~mask[reg]) | (value & mask[reg]));
            switch(reg)
            {
            case 0x14:
                UpdatePin(true);
                break;
            case 0x36:
            case 0x37:
                OnConfigurationWrite();
                break;
            }
        }

        // ---- measurement -----------------------------------------------------------------------

        private ulong Now()
        {
            if(machine.SystemBus.TryGetCurrentCPU(out var cpu))
            {
                cpu.SyncTime();
            }
            return machine.LocalTimeSource.ElapsedVirtualTime.Ticks;
        }

        private int RequestedMode()
        {
            return regs[0x37] & 0x03;
        }

        private int ConfigurationKey()
        {
            return (regs[0x36] << 16) | (regs[0x37] << 8) | (regs[0x18] & 0x03);
        }

        private bool PressureOn()
        {
            // OSR_CONFIG.press_en, or a FIFO frame that holds pressure (FIFO_SEL.fifo_frame_sel 0b10, 0b11), DS p.18
            return (regs[0x36] & 0x40) != 0 || (regs[0x18] & 0x03) >= 2;
        }

        private double OdrHz()
        {
            return OdrTable[(regs[0x37] >> 2) & 0x1F];
        }

        private bool DeepStandby()
        {
            // DS p.16, 4.3.2: deep_dis = 0, odr < 5 Hz, no FIFO frame, both IIR filters bypassed
            return (regs[0x37] & 0x80) == 0 && OdrHz() < 5.0 && (regs[0x18] & 0x03) == 0 && (regs[0x31] & 0x3F) == 0;
        }

        // The oversampling a measurement runs with. In NORMAL mode with pressure on, an ODR that the
        // OSRs cannot reach is replaced with OSR 2 below 160 Hz and OSR 1 from 160 Hz up (DS p.17, 4.3.4).
        private void EffectiveOsr(out int osrP, out int osrT, out bool valid)
        {
            osrP = (regs[0x36] >> 3) & 0x07;
            osrT = regs[0x36] & 0x07;
            valid = true;
            if(actualMode == 1 && PressureOn())
            {
                if(OdrHz() > MaxOdr[osrP, osrT])
                {
                    valid = false;
                    osrP = OdrHz() >= 160.0 ? 0 : 1;
                    osrT = osrP;
                }
                regs[0x38] = (byte)((valid ? 0x80 : 0) | (osrP << 3) | osrT); // OSR_EFF, DS p.66
            }
        }

        private ulong ConversionNs()
        {
            int osrP, osrT;
            bool valid;
            EffectiveOsr(out osrP, out osrT, out valid);
            // The conversion's maximum, typ + 5 % (Table 4, DS p.12): the latest the data can come.
            var us = ConvTempUs[osrT] + (PressureOn() ? ConvPressUs[osrP] : 0UL);
            return us * 1050UL;
        }

        private void OnConfigurationWrite()
        {
            var key = ConfigurationKey();
            var mode = RequestedMode();
            if(mode == 0)
            {
                actualMode = 0;
                measArmed = false;
                runKey = -1;
                Arm(Now());
                return;
            }
            if(actualMode == mode && key == runKey)
            {
                return; // the same configuration written again: nothing restarts
            }
            var now = Now();
            actualMode = mode;
            runKey = key;
            var startup = leavingDeep ? StartupDeepNs : StartupNs; // t_startup_deep 4 ms, t_startup 3 ms (max), DS p.11
            measStart = now + startup;
            measDone = measStart + ConversionNs();
            measArmed = true;
            Arm(now);
        }

        private void Arm(ulong now)
        {
            var target = Never;
            if(measArmed)
            {
                target = measDone;
            }
            if(pulseActive && pulseEnd < target)
            {
                target = pulseEnd;
            }
            timer.Enabled = false;
            if(target == Never)
            {
                return;
            }
            var delta = target > now ? target - now : 1UL;
            timerTarget = now + delta;
            timer.Limit = delta;
            timer.Enabled = true;
        }

        private void OnTimer()
        {
            var t = timerTarget;
            while(true)
            {
                if(measArmed && measDone <= t)
                {
                    MeasurementDone();
                }
                else if(pulseActive && pulseEnd <= t)
                {
                    pulseActive = false;
                    intAsserted = false;
                    UpdatePin(true);
                }
                else
                {
                    break;
                }
            }
            Arm(t);
        }

        private void MeasurementDone()
        {
            var t = measDone;
            var pressure = PressureOn();
            var rawT = Raw24(Temperature * 65536.0); // DS p.22: T = raw / 2^16
            regs[0x1D] = (byte)(rawT & 0xFF);
            regs[0x1E] = (byte)((rawT >> 8) & 0xFF);
            regs[0x1F] = (byte)((rawT >> 16) & 0xFF);
            if(pressure) // with pressure off the pressure registers keep their last value
            {
                var rawP = Raw24(Pressure * 64.0); // DS p.22: P = raw / 2^6
                regs[0x20] = (byte)(rawP & 0xFF);
                regs[0x21] = (byte)((rawP >> 8) & 0xFF);
                regs[0x22] = (byte)((rawP >> 16) & 0xFF);
            }
            regs[0x27] |= 0x01; // INT_STATUS.drdy_data_reg
            if((regs[0x14] & 0x08) != 0 && (regs[0x15] & 0x01) != 0) // int_en and drdy_data_reg_en
            {
                intAsserted = true;
                if((regs[0x14] & 0x01) != 0) // int_mode latched
                {
                    pulseActive = false;
                }
                else
                {
                    pulseActive = true;
                    pulseEnd = t + PulseNs; // t_int_pulse typ 105 us, DS p.12
                }
                UpdatePin(true);
            }
            if(actualMode == 2) // FORCED: one measurement, then STANDBY
            {
                actualMode = 0;
                regs[0x37] = (byte)(regs[0x37] & 0xFC);
                measArmed = false;
                runKey = -1;
            }
            else if(actualMode == 1) // NORMAL: every 1/ODR
            {
                var period = (ulong)Math.Round(1e9 / OdrHz());
                var conv = ConversionNs();
                measStart = measStart + Math.Max(period, conv);
                measDone = measStart + conv;
            }
            else // CONTINUOUS: back to back
            {
                measStart = t;
                measDone = measStart + ConversionNs();
            }
        }

        private void UpdatePin(bool log)
        {
            var activeHigh = (regs[0x14] & 0x02) != 0; // int_pol
            var level = intAsserted == activeHigh;
            if(INT.IsSet == level)
            {
                return;
            }
            INT.Set(level);
            if(log)
            {
                this.Log(LogLevel.Noisy, level ? "INT set" : "INT cleared");
            }
        }

        private static int Raw24(double value)
        {
            if(double.IsNaN(value))
            {
                return 0;
            }
            var r = Math.Round(value, MidpointRounding.AwayFromZero);
            if(r < -8388608.0)
            {
                r = -8388608.0;
            }
            if(r > 8388607.0)
            {
                r = 8388607.0;
            }
            return (int)r;
        }

        // ---- state -----------------------------------------------------------------------------

        private const byte RW = 1;
        private const byte RO = 2;
        private const ulong Never = ulong.MaxValue;
        private const ulong StartupNs = 3000000UL;
        private const ulong StartupDeepNs = 4000000UL;
        private const ulong PulseNs = 105000UL;
        private const ulong SoftResetNs = 2000000UL; // t_soft_res max 2 ms, DS p.11
        private byte lastWriteAddress;
        private ulong resetUntil;
        private bool leavingDeep;

        // address, reset, kind, write mask
        private static readonly int[][] Defs = new int[][]
        {
            new[] { 0x01, 0x50, RO, 0x00 }, // CHIP_ID DS p.51
            new[] { 0x02, 0x32, RO, 0x00 }, // REV_ID p.51
            new[] { 0x11, 0x00, RO, 0x00 }, // CHIP_STATUS p.51
            new[] { 0x13, 0x30, RW, 0xFF }, // DRIVE_CONFIG p.52
            new[] { 0x14, 0x35, RW, 0xFF }, // INT_CONFIG p.52-53
            new[] { 0x15, 0x00, RW, 0x0F }, // INT_SOURCE p.54
            new[] { 0x16, 0x00, RW, 0x3F }, // FIFO_CONFIG p.54
            new[] { 0x17, 0x00, RO, 0x00 }, // FIFO_COUNT p.55
            new[] { 0x18, 0x00, RW, 0x1F }, // FIFO_SEL p.55
            new[] { 0x1C, 0x00, RO, 0x00 }, // reserved, reads 0x00 p.56
            // 0x1D-0x22 TEMP_DATA, PRESS_DATA read only, reset 0x7F each: p.56-57
            // 0x23-0x26 reserved, read 0x00: p.57-58
            new[] { 0x27, 0x00, RO, 0x00 }, // INT_STATUS, clear on read, p.58 (0x10 after power-on)
            new[] { 0x28, 0x02, RO, 0x00 }, // STATUS p.58
            new[] { 0x29, 0x7F, RO, 0x00 }, // FIFO_DATA p.59
            new[] { 0x2B, 0x00, RW, 0x7F }, // NVM_ADDR p.60 (NVM not modelled)
            new[] { 0x2C, 0x00, RW, 0xFF }, // NVM_DATA_LSB p.60
            new[] { 0x2D, 0x00, RW, 0xFF }, // NVM_DATA_MSB p.60
            new[] { 0x30, 0x03, RW, 0xFF }, // DSP_CONFIG p.61
            new[] { 0x31, 0x00, RW, 0x3F }, // DSP_IIR p.62
            new[] { 0x32, 0x00, RW, 0xFF }, // OOR_THR_P_LSB p.63
            new[] { 0x33, 0x00, RW, 0xFF }, // OOR_THR_P_MSB p.63
            new[] { 0x34, 0x00, RW, 0xFF }, // OOR_RANGE p.63
            new[] { 0x35, 0x00, RW, 0xC1 }, // OOR_CONFIG p.63
            new[] { 0x36, 0x00, RW, 0x7F }, // OSR_CONFIG p.64
            new[] { 0x37, 0x70, RW, 0xFF }, // ODR_CONFIG p.64
            new[] { 0x38, 0x00, RO, 0x00 }, // OSR_EFF p.66
            new[] { 0x7E, 0x00, RW, 0xFF }, // CMD p.67
        };

        // ODR_CONFIG.odr code to Hz, DS p.65
        private static readonly double[] OdrTable =
        {
            240.0, 218.537, 199.111, 179.2, 160.0, 149.333, 140.0, 129.855,
            120.0, 110.164, 100.299, 89.6, 80.0, 70.0, 60.0, 50.056,
            45.025, 40.0, 35.0, 30.0, 25.005, 20.0, 15.0, 10.0,
            5.0, 4.0, 3.0, 2.0, 1.0, 0.5, 0.25, 0.125
        };

        // Table 7 p.17: maximum nominal ODR in NORMAL mode, [osr_p code, osr_t code]
        private static readonly double[,] MaxOdr =
        {
            { 240, 240, 240, 240, 200, 130, 80, 40 },
            { 240, 240, 240, 220, 180, 120, 70, 40 },
            { 220, 220, 200, 180, 140, 100, 70, 40 },
            { 140, 140, 130, 120, 100, 80, 50, 35 },
            { 80, 80, 80, 70, 70, 50, 45, 30 },
            { 45, 45, 40, 40, 40, 35, 30, 20 },
            { 20, 20, 20, 20, 20, 20, 15, 15 },
            { 10, 10, 10, 10, 10, 10, 10, 5 },
        };

        // Table 4 p.12: typical conversion time in microseconds by OSR code (1x to 128x)
        private static readonly ulong[] ConvPressUs = { 1000, 1700, 2900, 5400, 10400, 20400, 40400, 80400 };
        private static readonly ulong[] ConvTempUs = { 1000, 1100, 1500, 2100, 3300, 5800, 10800, 20800 };

        private readonly IMachine machine;
        private readonly LimitTimer timer;
        private readonly byte[] regs = new byte[128];
        private readonly byte[] kind = new byte[128];
        private readonly byte[] mask = new byte[128];
        private readonly List<byte> txWritten = new List<byte>();
        private readonly List<byte> txRead = new List<byte>();

        private byte pointer;
        private bool txPointerSet;
        private bool txLastWasRead;
        private byte txPointerByte;
        private byte txWriteStart;
        private byte txReadStart;

        private int actualMode;
        private int runKey = -1;
        private bool measArmed;
        private ulong measStart;
        private ulong measDone;
        private bool intAsserted;
        private bool pulseActive;
        private ulong pulseEnd;
        private ulong timerTarget;
    }
}
