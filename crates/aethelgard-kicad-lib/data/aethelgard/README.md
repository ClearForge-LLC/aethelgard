# Aethelgard's own symbols and footprints

**Licence: `MIT OR Apache-2.0`,** the same terms as the engine (`RUL-11`; the texts are `LICENSE-MIT` and `LICENSE-APACHE` at the repository root). These files are our own work, authored from the makers' datasheets for parts KiCad 10.0.7's libraries do not carry (`AET-WO-4220` D5, ruled on PR #140). The vendored KiCad files beside them, under `../kicad-10.0.7/`, keep their own CC-BY-SA 4.0 licence. Designs emitted from either carry no obligation from this project.

Each file was written from the cited pages, then re-saved by KiCad 10.0.7's own `kicad-cli fp upgrade --force` or `kicad-cli sym upgrade --force`, so that its bytes are KiCad's canonical form. `aethelgard-eda` pins every file's sha256 and checks each pad and pin against its citation table, which it holds apart from these files (`AET-WO-4220b`).

| File | Part | Cited |
|---|---|---|
| `symbols/Aethelgard_Sensor.kicad_symdir/ICM-42688-P.kicad_sym` | TDK InvenSense ICM-42688-P | DS-000347 v1.9, Table 10 "Signal Descriptions", p. 20 |
| `footprints/Aethelgard_Sensor.pretty/TDK_LGA-14_3x2.5mm_P0.5mm.kicad_mod` | ICM-42688-P, LGA-14 2.5 × 3 × 0.91 mm | DS-000347 v1.9, §10.2 pp. 54–55 (top view, centres `2X 1.1625` and `2X 0.9125`, lead L 0.475 × W 0.25 nominal); TDK AN-000393 v2.4 §2.2 Fig. 1 (land = pin length × pin width), §2.1 (no vias, traces or copper pour under the device) |
| `symbols/Aethelgard_Sensor.kicad_symdir/BMP581.kicad_sym` | Bosch Sensortec BMP581 | BST-BMP581-DS004-13 v1.13, Table 28 "Pin description", p. 46 |
| `footprints/Aethelgard_Sensor.pretty/Bosch_LGA-10_2x2mm_P0.5mm.kicad_mod` | BMP581, LGA-10 2.0 × 2.0 × 0.75 mm | BST-BMP581-DS004-13 v1.13, §8.1.2 Fig. 30 p. 68 (bottom view, mirrored to the top view), §8.2 and Fig. 32 p. 69 (land = package pad + 25 µm per side; no vias or traces under the BMP581) |

Each footprint carries two keepout zones: on `F.Cu`, the area inside its ring of pads (no track, via or copper pour); on `B.Cu`, the body at its maximum outline (no track or via).
