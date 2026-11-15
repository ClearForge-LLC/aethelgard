# aethelgard-twin

Simulator platform descriptions, the analog model, and the lock-step bridge for the Aethelgard digital twin (`docs/architecture.md` §3). Its register facts come from `aethelgard-svd-hal`.

Most devices are Renode 1.17.0's own models. The ICM-42688-P and the BMP581 are this project's (`models/renode/`, the `aethelgard-renode-models` crate, `AET-WO-4220`). `model::OWN` pins each by sha256, and the emitted platform's header names each one it declares with that pin. The CLI includes only a copy that hashes to it, and `platform::own_models` refuses any other (`ModelUnpinned`).
