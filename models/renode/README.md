# aethelgard-renode-models

Aethelgard's own [Renode](https://renode.io) 1.17.0 peripheral models, written for this project in C# (`RUL-12` as amended, `AET-WO-4220`):

| File | Part | Bus | Identity |
|---|---|---|---|
| `ICM42688P.cs` | TDK InvenSense ICM-42688-P (6-axis IMU) | SPI | `WHO_AM_I` 0x75 = 0x47 |
| `BMP581.cs` | Bosch Sensortec BMP581 (barometer) | I2C | `CHIP_ID` 0x01 = 0x50 |

Data only: the crate exports each file's text as `FILES`. `aethelgard-twin` pins every file by sha256, the emitted platform names each model it uses with that sha256, and the CLI writes its own copy, hashes it again and includes it before Renode loads the platform. A changed model is refused until its pin moves with it.

Each model logs one `Noisy` line per transaction with every byte moved, and nothing on an empty finish. It samples on its own timer at the configured output data rate, raises its data-ready status and interrupt pin itself, and rounds a stimulus to register counts half away from zero.

Licensed `MIT OR Apache-2.0`, like the rest of the workspace. No Renode source is copied into these files.
