# Companion project: Tviburi

Additive note to the 2026-11-21 Grok review. This document is not a work order and does not change Aethelgard's invariants or process. It records the shape of a sibling project that Aethelgard can consume but does not own.

**Name:** Tviburi (Old Norse *tví-buri*, "twin").  
**Header for work orders and builders:** TVI.  
**Repository:** https://github.com/ClearForge-LLC/tviburi (public, created 2026-11-21).  
**Relationship:** Aethelgard drives it the same way it drives Renode today. Tviburi owns the models and the gap contract; Aethelgard owns the specification-to-board-and-firmware path and the gates.

---

## Why a separate project

Aethelgard's frame is deliberately narrow. The twin is a means. A high-fidelity, citation-backed digital twin for commercial microcontrollers is a different product. Putting the models inside Aethelgard would expand the process surface and couple the generation cadence to the slower model-improvement cadence. The clean seam is the one already used with Renode: treat the twin as an external tool that produces a replayable trace under a stricter contract.

## Starting point

Start from Renode. Do not begin with a clean-room simulator.

Renode already supplies the CPU model, bus, virtual time, and existing peripheral scaffolding. The gaps Aethelgard has measured (incomplete error flags, silent success on unmodeled paths) are gaps in an existing system. The first usable version of Tviburi is a forked STM32H7 platform whose unmodeled paths refuse by name, plus a versioned gap-ledger artifact.

A later optional step is to re-express stable models in a form more easily audited by Aethelgard's Rust builders. That is not a prerequisite.

## Highest-leverage peripherals for explicit refusals

Beyond the already-scarred SPI and I2C paths, the three that give the most leverage for a flight-controller-class system are:

1. **Timers / advanced-control timers (TIM1/TIM8 style).**  
   Update-event timing under preload, break-input behavior, and the exact conditions that set the break flags. Safety-critical for motor PWM and fault response.

2. **DMA controllers (memory-to-peripheral and circular modes).**  
   Half-transfer vs full-transfer interrupt ordering, the exact moment the transfer-complete flag is set relative to the last data word, and bus-error mid-transfer behavior. Silent success here is dangerous.

3. **Brown-out / voltage-detector and associated reset sources.**  
   Any path where the model cannot yet express the relationship between a declared supply droop and the BOR/PDR flags or the reset behavior that follows.

These cover the majority of the "the twin passed what silicon would have faulted" surface once simple polled communication is left behind.

## Gap ledger

Separate artifact emitted by the twin, recorded by hash in Aethelgard.

- The ledger is a property of the model version, not of any particular campaign.
- Tviburi emits a small, versioned CBOR or JSONL document: model hash, list of (peripheral, register/bit, condition) that are currently refusals, and a citation to the reference-manual section that is not yet implemented.
- Aethelgard stores that document's hash next to the trace. The campaign archive stays focused on what the firmware and board did.

## First deliverable

A forked STM32H7 platform in which unmodeled paths refuse by name, plus the gap-ledger artifact described above. Aethelgard continues to drive it exactly as it drives Renode today (same sandbox, same campaign archive, same attribute step).

---

This note exists so the co-architect and builders can see the boundary and the first concrete steps without having to reconstruct them from conversation. It does not authorize work inside Aethelgard.
