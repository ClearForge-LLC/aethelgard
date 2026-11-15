//
// ICM42688P: a behavioural model of the TDK ICM-42688-P six-axis IMU on a SPI bus, for Renode 1.17.0.
// AET-WO-4220. MIT OR Apache-2.0. Written for Aethelgard; no Antmicro source copied.
//
// Facts come from the hand-read datasheet DS-000347 Rev 1.9 (page numbers beside each reset value).
// Modelled: the register banks 0, 1 and 2 as far as the datasheet read gives a reset; accel and gyro
// sampling in virtual time; the data-ready flag; the INT1 pin. Not modelled: the FIFO (FIFO_COUNT
// reads 0, FIFO_DATA reads its reset 0xFF), I2C/I3C, the DMP and APEX, FSYNC and the timestamps,
// the low-pass filters (data is the unfiltered stimulus), drive circuit of INT1, and the INT1
// de-assertion gap. Registers without a datasheet fact read 0x00, ignore writes and log a warning.
//
// Log contract: one Noisy line per SPI transaction, written when it ends (FinishTransmission); a
// transaction that moved no byte logs nothing. The stimulus is a public double property each.
//
using System;
using System.Collections.Generic;
using System.Text;

using Antmicro.Renode.Core;
using Antmicro.Renode.Logging;
using Antmicro.Renode.Peripherals.SPI;
using Antmicro.Renode.Peripherals.Timers;
using Antmicro.Renode.Time;

namespace Antmicro.Renode.Peripherals.Sensors
{
    public class ICM42688P : ISPIPeripheral
    {
        public ICM42688P(IMachine machine)
        {
            this.machine = machine;
            INT1 = new GPIO();
            regs = new byte[Banks][];
            kind = new byte[Banks][];
            for(var b = 0; b < Banks; b++)
            {
                regs[b] = new byte[128];
                kind[b] = new byte[128];
            }
            foreach(var d in Defs)
            {
                kind[d[0]][d[1]] = (byte)d[3];
            }
            for(var a = 0x1D; a <= 0x2A; a++)
            {
                kind[0][a] = RO;
            }
            // One-shot timer in nanoseconds, re-armed to the earliest pending event.
            timer = new LimitTimer(machine.ClockSource, 1000000000, this, "icm42688p", limit: 1000000000, workMode: WorkMode.OneShot, eventEnabled: true, autoUpdate: true);
            timer.LimitReached += OnTimer;
            AccelerationX = 0;
            AccelerationY = 0;
            AccelerationZ = 0;
            AngularRateX = 0;
            AngularRateY = 0;
            AngularRateZ = 0;
            Temperature = 25;
            Reset();
        }

        // Stimulus, sampled at each sample instant. Acceleration in g, angular rate in deg/s, temperature in degC.
        public double AccelerationX { get; set; }
        public double AccelerationY { get; set; }
        public double AccelerationZ { get; set; }
        public double AngularRateX { get; set; }
        public double AngularRateY { get; set; }
        public double AngularRateZ { get; set; }
        public double Temperature { get; set; }

        public GPIO INT1 { get; private set; }

        public void Reset()
        {
            ResetRegisters();
            txStarted = false;
            timer.Reset();
        }

        public byte Transmit(byte data)
        {
            if(!txStarted)
            {
                txStarted = true;
                txRead = (data & 0x80) != 0;
                txStartReg = (byte)(data & 0x7F);
                txReg = txStartReg;
                txBank = bankSel;
                txBytes.Clear();
                return 0;
            }
            byte reply = 0;
            if(txRead)
            {
                reply = ReadRegister(txBank, txReg);
                txBytes.Add(reply);
            }
            else
            {
                WriteRegister(txBank, txReg, data);
                txBytes.Add(data);
            }
            txReg = (byte)((txReg + 1) & 0x7F);
            return reply;
        }

        public void FinishTransmission()
        {
            if(!txStarted)
            {
                return;
            }
            var text = new StringBuilder();
            foreach(var b in txBytes)
            {
                if(text.Length > 0)
                {
                    text.Append(' ');
                }
                text.Append("0x").Append(b.ToString("X2"));
            }
            var bytes = txBytes.Count == 0 ? "(none)" : text.ToString();
            if(txRead)
            {
                this.Log(LogLevel.Noisy, "bank {0} read 0x{1:X2} -> {2}", txBank, txStartReg, bytes);
            }
            else
            {
                this.Log(LogLevel.Noisy, "bank {0} write 0x{1:X2} <- {2}", txBank, txStartReg, bytes);
            }
            txStarted = false;
            txBytes.Clear();
        }

        // ---- registers -------------------------------------------------------------------------

        private void ResetRegisters()
        {
            for(var b = 0; b < Banks; b++)
            {
                Array.Clear(regs[b], 0, 128);
            }
            foreach(var d in Defs)
            {
                regs[d[0]][d[1]] = (byte)d[2];
            }
            for(var i = 0; i < words.Length; i++)
            {
                words[i] = 0x8000; // DS-000347 p.65-68: every data pair resets to 0x8000
                sampled[i] = false;
            }
            bankSel = 0;
            regs[0][0x2D] = 0x10; // power-on status: RESET_DONE_INT
            accelOn = false;
            gyroOn = false;
            accelNext = Never;
            gyroNext = Never;
            pulseActive = false;
            intAsserted = false;
            timer.Enabled = false;
            UpdatePin(false);
        }

        private byte ReadRegister(int bank, byte reg)
        {
            if(reg == 0x76)
            {
                return bankSel;
            }
            if(bank >= Banks || kind[bank][reg] == 0)
            {
                this.Log(LogLevel.Warning, "bank {0} access to unmodelled register 0x{1:X2}", bank, reg);
                return 0;
            }
            if(bank == 0)
            {
                if(reg >= 0x1D && reg <= 0x2A)
                {
                    // Before its first sample a word reads -32768 (0x8000), or 0 when
                    // INTF_CONFIG0.FIFO_HOLD_LAST_DATA_EN is set: DS-000347 p.59, 12.8.
                    var at = (reg - 0x1D) / 2;
                    var word = sampled[at] ? words[at] : ((regs[0][0x4C] & 0x80) != 0 ? (ushort)0 : words[at]);
                    var high = ((reg - 0x1D) % 2) == 0;
                    var bigEndian = (regs[0][0x4C] & 0x10) != 0; // SENSOR_DATA_ENDIAN
                    var first = (byte)(word >> 8);
                    var second = (byte)(word & 0xFF);
                    if(UiDrdyClearMode() >= 2 && intAsserted && !pulseActive)
                    {
                        intAsserted = false;
                        UpdatePin(true);
                    }
                    return (high == bigEndian) ? first : second;
                }
                if(reg == 0x2D)
                {
                    var status = regs[0][0x2D];
                    regs[0][0x2D] = 0;
                    var mode = UiDrdyClearMode();
                    if((mode & 1) != 0 || mode == 0)
                    {
                        if(intAsserted && !pulseActive)
                        {
                            intAsserted = false;
                            UpdatePin(true);
                        }
                    }
                    return status;
                }
            }
            return regs[bank][reg];
        }

        private void WriteRegister(int bank, byte reg, byte value)
        {
            if(reg == 0x76)
            {
                bankSel = (byte)(value & 0x07); // BANK_SEL bits 2:0, DS p.88
                return;
            }
            var k = bank >= Banks ? (byte)0 : kind[bank][reg];
            if(k == 0)
            {
                this.Log(LogLevel.Warning, "bank {0} access to unmodelled register 0x{1:X2}", bank, reg);
                return;
            }
            if(k == RO)
            {
                this.Log(LogLevel.Warning, "bank {0} write to read-only register 0x{1:X2} ignored", bank, reg);
                return;
            }
            if(bank != 0)
            {
                regs[bank][reg] = value;
                return;
            }
            switch(reg)
            {
            case 0x11: // DEVICE_CONFIG
                if((value & 0x01) != 0)
                {
                    ResetRegisters(); // SOFT_RESET_CONFIG; the part is ready again at once here
                    return;
                }
                regs[0][reg] = value;
                break;
            case 0x4B: // SIGNAL_PATH_RESET is write-to-clear; the FIFO is not modelled, nothing to flush
                break;
            case 0x14:
                regs[0][reg] = value;
                UpdatePin(true);
                break;
            case 0x4E:
                regs[0][reg] = value;
                OnPowerManagement();
                break;
            case 0x4F:
            case 0x50:
                regs[0][reg] = value;
                OnOutputDataRate();
                break;
            default:
                regs[0][reg] = value;
                break;
            }
        }

        private int UiDrdyClearMode()
        {
            return (regs[0][0x63] >> 4) & 3; // INT_CONFIG0.UI_DRDY_INT_CLEAR, DS p.84
        }

        // ---- sampling --------------------------------------------------------------------------

        private ulong Now()
        {
            if(machine.SystemBus.TryGetCurrentCPU(out var cpu))
            {
                cpu.SyncTime();
            }
            return machine.LocalTimeSource.ElapsedVirtualTime.Ticks;
        }

        private ulong AccelPeriod()
        {
            return AccelPeriods[regs[0][0x50] & 0x0F];
        }

        private ulong GyroPeriod()
        {
            return GyroPeriods[regs[0][0x4F] & 0x0F];
        }

        private void OnPowerManagement()
        {
            var now = Now();
            var pwr = regs[0][0x4E];
            var wantAccel = (pwr & 0x03) >= 2;      // LP (0b10) or LN (0b11), DS p.76
            var wantGyro = ((pwr >> 2) & 0x03) == 3; // LN only; standby gives no data
            if(wantAccel && !accelOn)
            {
                accelOn = true;
                accelValidAt = now + AccelStartNs;
                accelNext = AccelPeriod() == 0 ? Never : accelValidAt;
            }
            else if(!wantAccel)
            {
                accelOn = false;
                accelNext = Never;
            }
            if(wantGyro && !gyroOn)
            {
                gyroOn = true;
                gyroValidAt = now + GyroStartNs;
                gyroNext = GyroPeriod() == 0 ? Never : gyroValidAt;
            }
            else if(!wantGyro)
            {
                gyroOn = false;
                gyroNext = Never;
            }
            Arm(now);
        }

        private void OnOutputDataRate()
        {
            var now = Now();
            if(accelOn)
            {
                var p = AccelPeriod();
                accelNext = p == 0 ? Never : Math.Max(accelValidAt, now + p);
            }
            if(gyroOn)
            {
                var p = GyroPeriod();
                gyroNext = p == 0 ? Never : Math.Max(gyroValidAt, now + p);
            }
            Arm(now);
        }

        private void Arm(ulong now)
        {
            var target = Never;
            if(accelOn && accelNext < target)
            {
                target = accelNext;
            }
            if(gyroOn && gyroNext < target)
            {
                target = gyroNext;
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
            RunEventsUpTo(t);
            Arm(t);
        }

        private void RunEventsUpTo(ulong t)
        {
            while(true)
            {
                var when = Never;
                var which = -1;
                if(accelOn && accelNext <= t && accelNext < when)
                {
                    when = accelNext;
                    which = 0;
                }
                if(gyroOn && gyroNext <= t && gyroNext < when)
                {
                    when = gyroNext;
                    which = 1;
                }
                if(pulseActive && pulseEnd <= t && pulseEnd < when)
                {
                    when = pulseEnd;
                    which = 2;
                }
                if(which < 0)
                {
                    return;
                }
                if(which == 0)
                {
                    SampleAccel(when);
                }
                else if(which == 1)
                {
                    SampleGyro(when);
                }
                else
                {
                    pulseActive = false;
                    intAsserted = false;
                    UpdatePin(true);
                }
            }
        }

        private bool AccelSampling(ulong t)
        {
            return accelOn && accelNext != Never && t >= accelValidAt;
        }

        private bool GyroSampling(ulong t)
        {
            return gyroOn && gyroNext != Never && t >= gyroValidAt;
        }

        private void SampleAccel(ulong t)
        {
            words[1] = unchecked((ushort)Raw16(AccelerationX * AccelSensitivity()));
            words[2] = unchecked((ushort)Raw16(AccelerationY * AccelSensitivity()));
            words[3] = unchecked((ushort)Raw16(AccelerationZ * AccelSensitivity()));
            sampled[1] = sampled[2] = sampled[3] = true;
            // Temperature is updated at max(accel ODR, gyro ODR), DS p.66.
            if(!GyroSampling(t) || AccelPeriod() <= GyroPeriod())
            {
                SampleTemperature();
            }
            accelNext = AccelPeriod() == 0 ? Never : t + AccelPeriod();
            DataReady(t);
        }

        private void SampleGyro(ulong t)
        {
            var s = GyroSensitivity();
            words[4] = unchecked((ushort)Raw16(AngularRateX * s));
            words[5] = unchecked((ushort)Raw16(AngularRateY * s));
            words[6] = unchecked((ushort)Raw16(AngularRateZ * s));
            sampled[4] = sampled[5] = sampled[6] = true;
            if(!AccelSampling(t) || GyroPeriod() < AccelPeriod())
            {
                SampleTemperature();
            }
            gyroNext = GyroPeriod() == 0 ? Never : t + GyroPeriod();
            DataReady(t);
        }

        private void SampleTemperature()
        {
            if((regs[0][0x4E] & 0x20) == 0) // PWR_MGMT0.TEMP_DIS, DS p.76
            {
                words[0] = unchecked((ushort)Raw16((Temperature - 25.0) * 132.48));
                sampled[0] = true;
            }
        }

        private void DataReady(ulong t)
        {
            regs[0][0x2D] |= 0x08; // INT_STATUS.DATA_RDY_INT
            if((regs[0][0x65] & 0x08) == 0) // INT_SOURCE0.UI_DRDY_INT1_EN
            {
                return;
            }
            intAsserted = true;
            if((regs[0][0x14] & 0x04) != 0) // INT1_MODE latched
            {
                pulseActive = false;
            }
            else
            {
                pulseActive = true;
                pulseEnd = t + (((regs[0][0x64] & 0x40) != 0) ? 8000UL : 100000UL); // INT_TPULSE_DURATION, DS p.85
            }
            UpdatePin(true);
        }

        private void UpdatePin(bool log)
        {
            var activeHigh = (regs[0][0x14] & 0x01) != 0; // INT1_POLARITY
            var level = intAsserted == activeHigh;
            if(INT1.IsSet == level)
            {
                return;
            }
            INT1.Set(level);
            if(log)
            {
                this.Log(LogLevel.Noisy, level ? "INT1 set" : "INT1 cleared");
            }
        }

        private short Raw16(double value)
        {
            if(double.IsNaN(value))
            {
                return 0;
            }
            var r = Math.Round(value, MidpointRounding.AwayFromZero);
            // FIFO_HOLD_LAST_DATA_EN = 0: -32768 and -32767 are replaced with -32766 (DS p.74)
            var low = (regs[0][0x4C] & 0x80) != 0 ? -32768.0 : -32766.0;
            if(r < low)
            {
                r = low;
            }
            if(r > 32767.0)
            {
                r = 32767.0;
            }
            return (short)r;
        }

        private double AccelSensitivity()
        {
            switch((regs[0][0x50] >> 5) & 0x07)
            {
            case 1: return 4096.0;
            case 2: return 8192.0;
            case 3: return 16384.0;
            default: return 2048.0; // 0 (+-16 g); 4 to 7 are reserved and read as +-16 g
            }
        }

        private double GyroSensitivity()
        {
            return GyroSensitivities[(regs[0][0x4F] >> 5) & 0x07];
        }

        // ---- state -----------------------------------------------------------------------------

        private const byte RW = 1;
        private const byte RO = 2;
        private const int Banks = 5;
        private const ulong Never = ulong.MaxValue;
        private const ulong AccelStartNs = 10000000UL; // Table 2 p.12, typ 10 ms
        private const ulong GyroStartNs = 30000000UL;  // Table 1 p.11, typ 30 ms

        // bank, address, reset, kind
        private static readonly int[][] Defs = new int[][]
        {
            new[] { 0, 0x11, 0x00, RW }, // DEVICE_CONFIG DS-000347 p.64
            new[] { 0, 0x13, 0x05, RW }, // DRIVE_CONFIG p.64
            new[] { 0, 0x14, 0x00, RW }, // INT_CONFIG p.65
            new[] { 0, 0x16, 0x00, RW }, // FIFO_CONFIG p.65
            // 0x1D-0x2A TEMP/ACCEL/GYRO data, read only, reset 0x8000 per pair: p.65-68
            new[] { 0, 0x2B, 0x00, RO }, // TMST_FSYNCH p.68
            new[] { 0, 0x2C, 0x00, RO }, // TMST_FSYNCL p.69
            new[] { 0, 0x2D, 0x10, RO }, // INT_STATUS, read-to-clear p.69
            new[] { 0, 0x2E, 0x00, RO }, // FIFO_COUNTH p.69
            new[] { 0, 0x2F, 0x00, RO }, // FIFO_COUNTL p.70
            new[] { 0, 0x30, 0xFF, RO }, // FIFO_DATA p.70
            new[] { 0, 0x37, 0x00, RO }, // INT_STATUS2 p.73
            new[] { 0, 0x38, 0x00, RO }, // INT_STATUS3 p.73
            new[] { 0, 0x4B, 0x00, RW }, // SIGNAL_PATH_RESET, write-to-clear p.73
            new[] { 0, 0x4C, 0x30, RW }, // INTF_CONFIG0 p.74-75
            new[] { 0, 0x4D, 0x91, RW }, // INTF_CONFIG1 p.75
            new[] { 0, 0x4E, 0x00, RW }, // PWR_MGMT0 p.76
            new[] { 0, 0x4F, 0x06, RW }, // GYRO_CONFIG0 p.77
            new[] { 0, 0x50, 0x06, RW }, // ACCEL_CONFIG0 p.78
            new[] { 0, 0x51, 0x16, RW }, // GYRO_CONFIG1 p.79
            new[] { 0, 0x52, 0x11, RW }, // GYRO_ACCEL_CONFIG0 p.80
            new[] { 0, 0x53, 0x0D, RW }, // ACCEL_CONFIG1 p.81
            new[] { 0, 0x54, 0x23, RW }, // TMST_CONFIG p.81
            new[] { 0, 0x5F, 0x00, RW }, // FIFO_CONFIG1 p.83
            new[] { 0, 0x60, 0x00, RW }, // FIFO_CONFIG2 p.83
            new[] { 0, 0x61, 0x00, RW }, // FIFO_CONFIG3 p.83
            new[] { 0, 0x62, 0x10, RW }, // FSYNC_CONFIG p.84
            new[] { 0, 0x63, 0x00, RW }, // INT_CONFIG0 p.84
            new[] { 0, 0x64, 0x10, RW }, // INT_CONFIG1 p.85
            new[] { 0, 0x65, 0x10, RW }, // INT_SOURCE0 p.85
            new[] { 0, 0x66, 0x00, RW }, // INT_SOURCE1 p.86
            new[] { 0, 0x68, 0x00, RW }, // INT_SOURCE3 p.86
            new[] { 0, 0x70, 0x00, RW }, // SELF_TEST_CONFIG p.88 (stored, no self test)
            new[] { 0, 0x75, 0x47, RO }, // WHO_AM_I p.88
            new[] { 1, 0x03, 0x80, RW }, // SENSOR_CONFIG0 p.89
            new[] { 1, 0x0B, 0xA0, RW }, // GYRO_CONFIG_STATIC2 p.89
            new[] { 1, 0x0C, 0x0D, RW }, // GYRO_CONFIG_STATIC3 p.89
            new[] { 1, 0x0D, 0xAA, RW }, // GYRO_CONFIG_STATIC4 p.90
            new[] { 1, 0x0E, 0x80, RW }, // GYRO_CONFIG_STATIC5 p.90
            new[] { 2, 0x03, 0x30, RW }, // ACCEL_CONFIG_STATIC2 p.95
            new[] { 2, 0x04, 0x40, RW }, // ACCEL_CONFIG_STATIC3 p.95
            new[] { 2, 0x05, 0x62, RW }, // ACCEL_CONFIG_STATIC4 p.95
        };

        // Gyro Table 1 p.11, GYRO_FS_SEL 0..7, LSB per deg/s
        private static readonly double[] GyroSensitivities = { 16.4, 32.8, 65.5, 131.0, 262.0, 524.3, 1048.6, 2097.2 };

        // Period in ns by ODR code, 0 = reserved (no samples). Accel p.78, gyro p.77.
        private static readonly ulong[] AccelPeriods =
        {
            0, 31250, 62500, 125000, 250000, 500000, 1000000, 5000000,
            10000000, 20000000, 40000000, 80000000, 160000000, 320000000, 640000000, 2000000
        };

        private static readonly ulong[] GyroPeriods =
        {
            0, 31250, 62500, 125000, 250000, 500000, 1000000, 5000000,
            10000000, 20000000, 40000000, 80000000, 0, 0, 0, 2000000
        };

        private readonly IMachine machine;
        private readonly LimitTimer timer;
        private readonly byte[][] regs;
        private readonly byte[][] kind;
        private readonly ushort[] words = new ushort[7]; // temp, ax, ay, az, gx, gy, gz
        private readonly bool[] sampled = new bool[7]; // whether each word has had a sample since reset
        private readonly List<byte> txBytes = new List<byte>();

        private byte bankSel;
        private bool txStarted;
        private bool txRead;
        private byte txReg;
        private byte txStartReg;
        private int txBank;

        private bool accelOn;
        private bool gyroOn;
        private ulong accelValidAt;
        private ulong gyroValidAt;
        private ulong accelNext = Never;
        private ulong gyroNext = Never;
        private bool intAsserted;
        private bool pulseActive;
        private ulong pulseEnd;
        private ulong timerTarget;
    }
}
