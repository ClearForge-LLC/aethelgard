//
// AET-WO-4220 spike S1: the smallest C# peripheral pair the pinned Renode (1.17.0) must compile at
// `include` time. Not a sensor model: one register each, answered on SPI and on I2C, with one
// Noisy line per byte so the log shows the transaction. Spike only (spike/4220), MIT OR Apache-2.0.
//
using System.Collections.Generic;

using Antmicro.Renode.Core;
using Antmicro.Renode.Logging;
using Antmicro.Renode.Peripherals.I2C;
using Antmicro.Renode.Peripherals.SPI;

namespace Antmicro.Renode.Peripherals.Sensors
{
    // SPI: the first byte of a transfer is the address (bit 7 = read); each later byte of a read
    // returns the register and increments the address. Register 0x00 reads 0xA5; others read 0.
    public class AetProbeSpi : ISPIPeripheral
    {
        public byte Transmit(byte data)
        {
            byte reply = 0;
            if(first)
            {
                address = (byte)(data & 0x7F);
                reading = (data & 0x80) != 0;
                first = false;
            }
            else if(reading)
            {
                reply = address == 0x00 ? (byte)0xA5 : (byte)0;
                address++;
            }
            this.Log(LogLevel.Noisy, "AetProbeSpi byte 0x{0:X2} -> 0x{1:X2}", data, reply);
            return reply;
        }

        public void FinishTransmission()
        {
            first = true;
            this.Log(LogLevel.Noisy, "AetProbeSpi finish");
        }

        public void Reset()
        {
            first = true;
            reading = false;
            address = 0;
        }

        private bool first = true;
        private bool reading;
        private byte address;
    }

    // I2C: a write sets the register pointer (first byte); a read returns from it. Register 0xD0
    // reads 0x5A; others read 0.
    public class AetProbeI2C : II2CPeripheral
    {
        public void Write(byte[] data)
        {
            if(data.Length > 0)
            {
                pointer = data[0];
            }
            this.Log(LogLevel.Noisy, "AetProbeI2C write {0} byte(s), pointer 0x{1:X2}", data.Length, pointer);
        }

        public byte[] Read(int count = 1)
        {
            var result = new List<byte>();
            for(var i = 0; i < count; i++)
            {
                result.Add(pointer == 0xD0 ? (byte)0x5A : (byte)0);
                pointer++;
            }
            this.Log(LogLevel.Noisy, "AetProbeI2C read {0} byte(s), first 0x{1:X2}", count, result.Count > 0 ? result[0] : 0);
            return result.ToArray();
        }

        public void FinishTransmission()
        {
            this.Log(LogLevel.Noisy, "AetProbeI2C finish");
        }

        public void Reset()
        {
            pointer = 0;
        }

        private byte pointer;
    }
}

namespace Aethelgard.Models
{
    // P1.4: the same SPI probe in a namespace of our own; the .repl must name it in full.
    public class AetProbeOwnNs : Antmicro.Renode.Peripherals.Sensors.AetProbeSpi
    {
    }
}
