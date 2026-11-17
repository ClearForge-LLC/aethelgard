# aethelgard-kicad-lib

KiCad 10.0.7's symbols, footprints and licence text that Aethelgard vendors, and Aethelgard's own symbols and footprints for parts KiCad lacks, as one table of bytes (`FILES`). It holds data only: no hashes, no parsing, no dependencies. `aethelgard-eda` pins each file's sha256 and refuses a file that is missing, unpinned or altered.

**Licence:** CC-BY-SA 4.0 for KiCad's files, with KiCad's design exception; the libraries' own licence text sits beside them in `data/kicad-10.0.7/LICENSE.md`. Our own files, under `data/aethelgard/`, are `MIT OR Apache-2.0` (`AET-WO-4220b`; their `README.md` cites each one).

**Designs emitted from it carry no obligation.** Schematics and boards built from these files fall under KiCad's design exception (`RUL-11`).
