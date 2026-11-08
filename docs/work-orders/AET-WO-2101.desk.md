# OQ-02 desk spike: read at source, 2026-11-08 (the architect, via a research subagent). Nothing run.
Pins: Renode v1.17.0 = ab721d88; src/Infrastructure gitlink 066a7f13c052215632d469c995c89aea37c573b1.
RI = https://github.com/renode/renode-infrastructure/blob/066a7f13c052215632d469c995c89aea37c573b1
RN = https://github.com/renode/renode/blob/v1.17.0
ngspice tag ngspice-44.2 = 80d22fc8 (2025-01-11); NG = https://sourceforge.net/p/ngspice/ngspice/ci/ngspice-44.2/tree
Manual: ngspice-44-manual.pdf (sourceforge old-releases/44.2).

Q1 ngspice sync stepping
- NG/src/include/ngspice/sharedspice.h L386: int ngSpice_Init_Sync(GetVSRCData *vsrcdat, GetISRCData *isrcdat, GetSyncData *syncdat, int *ident, void *userData); NGSPICE_PACKAGE_VERSION "44.2".
- L299 typedef int (GetVSRCData)(double*, double, char*, int, void*);  "return voltage value / actual time / node name" — supplies value of a `V... external` source.
- L309 GetISRCData same shape for current sources.
- L319 typedef int (GetSyncData)(double, double*, double, int, int, int, void*); "actual time (ckt->CKTtime) / delta time (ckt->CKTdelta) / old delta time / redostep / ident / location of call ... in dctran.c" — caller may rewrite next delta via double*.
- NG/src/spicelib/devices/vsrc/vsrcload.c L428: case EXTERNAL: { value = getvsrcval(time, here->VSRCname); (inside #ifdef SHARED_MODULE); getvsrcval -> getvdat (sharedspice.c L2330). Netlist: `Vex 1 0 dc 0 external` (manual §4.1.9).
- NG/src/sharedspice.c L2389-2431 sharedsync: retval = getsync(...); "never move beyond final time"; "user has decided to redo the step" if (retval) { *pckttime -= olddelta; ... }. Called from dctran.c L660, 719, 1046, 1050.
- Manual §15.6.1: return 0 -> next time step; return 1 -> olddelta subtracted, redo with new deltatime; "Typically the user provided deltatime should be smaller than the value suggested by ngspice."
- Manual §15.6.2: sync functions useful with a single library "if you want to use external voltage or current sources or 'play' with advancing simulation time."
- Manual §15.6.3.1 lists GetSyncData with 5 params; header has 7. Follow the header.
- sharedspice.c L649: only `bg_` commands start the background thread (pthread_create); others go to cp_evloop(buf) on the caller's thread (L717, 723, 726). Manual §15.4.2: "run" or "resume" (no bg_) run in the main thread; caller waits; 'halt' not available.
- Inversion of control: ngSpice_Command("run") blocks; GetSyncData fires per accepted/rejected step, GetVSRCData per source load, on the same thread. Lock-step from inside GetSyncData: advance the other simulator to CKTtime, set *delta.
- Debian libngspice0 44.2+ds-1 built --with-ngshared (salsa debian/rules DEB_LIBNGSPICE_CONFIGURE_OPTS), which defines SHARED_MODULE (NG/configure.ac L450).

Q2 Renode I2C models at 066a7f13 (II2CPeripheral)
- Peripherals/Sensors/: ADXL345, ADXL372, AK0991x, AS6221, ArduCAMMini2MPPlus, BMP180, HM01B0, HS3001, ICM20948, ICP_101xx, IIS2MDC, INA228, LIS2DS12, LIS2DW12, LPS25HB, LSM303DLHC_*, LSM330_*, LSM6DSO_IMU, LSM9DS1_*, MAX30208, MAX77818, MC3635, OB1203, OV2640, PAC1934, SI70xx, SI7210, ST_I2CSensorBase, TMP103 (+TMP108), VEML7700, ZMOD4xxx.
- Peripherals/I2C/: BMA180, BMC050, BME280, LC709205F, LM95245, MB85RC1MT, SHT21, SHT45, TAS2572, TCA6416, PCA9548.
- Elsewhere: Memory/GenericI2cEeprom.cs, Timers/ABRTCMC.cs, Timers/RV8803_RTC.cs, Input/FT5336|FT5x06|AR1021|FusionF0710A, Mocks/DummyI2CSlave.cs, Mocks/EchoI2CDevice.cs.
- TMP117: absent. TMP102: absent. TMP108: stub (RI/.../Sensors/TMP108.cs L9-10 "This model is a stub, roughly compatible with TMP103"; class TMP108 : TMP103). BME280: RI/.../I2C/BME280.cs L17 class BME280 : II2CPeripheral, ..., ITemperatureSensor. W25Q: no model; RI/.../SPI/GenericSpiFlash.cs L22 ISPIPeripheral, ctor L24 (manufacturerId, memoryType, capacityCode) — W25Q32 JEDEC 0xEF/0x40/0x16 unverified. Also Micron_MT25Q, Macronix_MX25R, ISSI_IS25WP, Cypress_S25H.

Q3 project-written I2C device without compiling C# into the project
(a) PythonPeripheral is bus-mapped only (RI/src/Emulator/Main/Peripherals/Python/PythonPeripheral.cs L43: IBytePeripheral, IWordPeripheral, IDoubleWordPeripheral, IQuadWordPeripheral, IKnownSize, IAbsoluteAddressAware; not II2CPeripheral).
    Workable: Mocks.DummyI2CSlave + Python handlers. RI/.../Mocks/DummyI2CSlave.cs L51-58 Read(count): ReadRequested?.Invoke(count); dataToReturn = buffer.DequeueRange(count); ... "filling rest with zeros." Events L103 DataReceived(byte[]), L109 ReadRequested(int) ("before any access to the internal buffer allowing to enqueue a response"), L114 TransmissionFinished.
    Precedent: RN/tests/platforms/Renesas_DA14592.robot L44-46 `dummy: Mocks.DummyI2CSlave @ i2c 0x75`, include "${echo_i2c_peripheral}", setup_echo_i2c_peripheral "sysbus.i2c.dummy"; RN/tests/platforms/echo-i2c-peripheral.py: dummy.DataReceived += EchoI2CPeripheral(dummy).write -> self.dummy.EnqueueResponseBytes(data).
(b) C# at run time: RI/src/Emulator/Extensions/UserInterface/Commands/IncludeFileCommand.cs L85 case ".cs": CsharpExecutor -> Monitor.TryCompilePlugin -> AdHocCompiler -> Assembly.LoadFrom (Monitor.cs L395-444). AdHocCompiler.cs L25 ".NET Core and above ... .dll extension"; L49 Roslyn CSharpCompilation.Create. Infrastructure.csproj net6.0, Microsoft.CodeAnalysis.CSharp 5.0.0. CI: RN/tests/tests.yaml L95 unit-tests/AdHocCompiler/adhoc-compiler.robot; virtual-time.robot L4 include @${TEST_DIR}/TestPeripheral.cs.
(c) External Control: TCP (RN/src/Renode/Network/ExternalControl/ExternalControlSocket.cs L166 Socket(InterNetwork, Stream, Tcp); L179 bind IPAddress.Any; L174 client to Loopback); `emulation CreateExternalControlServer "<NAME>" <PORT>`; C client tools/external_control_client/include/renode_api.h. Commands (ICommand.cs L10-22): CheckVersion, RunFor, GetTime, GetMachine, ADC, GPIOPort, SystemBus, TimeElapsedCallback, SPI, CANBus — no I2C. ExternalControlSPIPeripheral.cs (ISPIPeripheral, Func<byte,byte> OnTransmit); Bus/ExternalControlBusPeripheral.cs bus-mapped.
    CoSimulationPlugin: TCP (RN/src/Plugins/CoSimulationPlugin/Connection/SocketConnection.cs L569-570) or in-process native library (CoSimulationConnection.cs L474 LibraryConnection via NativeBinder, LibraryConnection.cs L182). External side C++/SystemVerilog, bus adapters apb3/axi/axilite/wishbone/cfu. Bus-mapped only (CoSimulatedPeripheral.cs L22); no I2C.
(d) Hooks: SystemBusHooksExtensions.cs L16 SetHookAfterPeripheralRead, L25 SetHookBeforePeripheralWrite, L34 AddWatchpointHook — register-level on bus peripherals only; CpuHooksExtensions.cs L16 AddHook on PC. No I2C-transaction hook.

Q4 blocking
- External Control SPI: blocks (SPI.cs L44-52; CommunicationHandler.cs L39-55 WaitHandle.WaitAny).
- External Control RunFor: RunFor.cs L33 -> Emulation.RunFor (Emulation.cs L143-152 InnerStartAll; MasterTimeSource.RunFor; PauseAll; refuses if already started).
- CoSim bus access blocks (CoSimulationConnection.cs L166-178); time sync quantum-based, LimitTimer, TickClock + WaitOne(timeout 3000 ms default L53); frequency 0 never syncs (L477).
- DummyI2CSlave + Python: handlers run synchronously inside Read()/Write(), in-process.

Q5 STM32F7_I2C.cs at 066a7f13
- Absent child: StartTransfer L347-351 TryGetByAddress fails -> Log Warning "Unknown slave at address {0}." return; nothing enqueued, TC not set, masterMode stays true, TXIS false. NACKF L250 WithTag; NACKCF L278 WithTag; L465 "(nackReceivedInterruptEnabled.Value && false) //TODO: implement NACKF". Later TXDR write L412-415: currentSlave == null -> Log "Trying to send byte {0} to an unknown slave"; dropped.
- Short read: L356-360 data = currentSlave.Read(n); enqueue only what returned. ReceiveDataRead L382-386: rxData.Count == 0 after dequeue -> SetTransferCompleteFlags() (TC/TCR, STOPF with AUTOEND) early. Empty queue L394-395 "Receive buffer underflow!" return 0; RXNE false (L248). Child returns 0 bytes -> TC never set.

Unverified: IronPython in a DummyI2CSlave handler opening IPC to an external process; a .py class implementing II2CPeripheral directly; GenericSpiFlash ctor values for W25Q32; ngspice stop/resume per-step loop in non-bg mode; GetVSRCData call count per time point (Newton iterations, rejected steps); ngSpice_Command re-entry from a callback; whether Debian build enables THREADS.
