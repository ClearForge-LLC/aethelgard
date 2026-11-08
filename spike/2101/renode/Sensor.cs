// Route ii: a run-time-compiled BME280-ID sensor. Fault: 0 ok, 1 no bytes, 2 corrupt (0x61), 3 short (count-1 bytes).
using Antmicro.Renode.Peripherals.I2C;
namespace Antmicro.Renode.Peripherals.I2C
{
    public class S3Sensor : II2CPeripheral
    {
        public int Fault { get; set; }
        public void Write(byte[] data) { if (data.Length > 0) reg = data[0]; }
        public byte[] Read(int count = 1)
        {
            if (Fault == 1) return new byte[0];
            var n = Fault == 3 ? count - 1 : count;
            var b = new byte[n];
            for (var i = 0; i < n; i++) b[i] = (byte)(reg == 0xD0 && i == 0 ? (Fault == 2 ? 0x61 : 0x60) : 0);
            return b;
        }
        public void FinishTransmission() { }
        public void Reset() { }
        private byte reg;
    }
}
