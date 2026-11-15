//
// AET-WO-4220 spike S1 (P1.5, P1.6): a deliberate fault. Line 13 lacks its `;`, and line 14
// names a type that does not exist. Spike only.
//
using Antmicro.Renode.Peripherals.SPI;

namespace Antmicro.Renode.Peripherals.Sensors
{
    public class AetProbeFault : ISPIPeripheral
    {
        public byte Transmit(byte data)
        {
            byte reply = 0
            NoSuchType unused = null;
            return reply;
        }

        public void FinishTransmission()
        {
        }

        public void Reset()
        {
        }
    }
}
