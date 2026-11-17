# aethelgard-twin

Simulator platform descriptions, the analog model, and the lock-step bridge for the Aethelgard digital twin (`docs/architecture.md` §3). Its register facts come from `aethelgard-svd-hal`. Each SPI controller it adds carries its receive DMA request line, numbered by RM0433 Rev 9 Table 124 as transcribed, with the citation, in `data/dmamux1-spi-rx.tsv` (`AET-WO-4400c2`); SPI6's request is on DMAMUX2, which Renode 1.17.0 does not declare, and its entry says so.

Most devices are Renode 1.17.0's own models. The ICM-42688-P and the BMP581 are this project's (`models/renode/`, the `aethelgard-renode-models` crate, `AET-WO-4220`). `model::OWN` pins each by sha256, and the emitted platform's header names each one it declares with that pin. The CLI includes only a copy that hashes to it, and `platform::own_models` refuses any other (`ModelUnpinned`).
