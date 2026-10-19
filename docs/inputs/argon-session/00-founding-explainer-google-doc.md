# Argon session capture — the founding Google Doc ("Project Aethelgard: Closed-Loop, Self-Verifying Hardware-to-Kernel Synthesis Engine")

**Source:** Google Doc created by Argon (Gemini) in Scotty's Drive during response 2 of 5 (see `02-three-layer-explainer.md`), pasted into the Claude Code session by Scotty on 2026-10-14.
**Captured:** 2026-10-14 by Claude Code, verbatim from the paste, with Markdown headings and the phase table restored.
**Fidelity notes:** The body matches the `GENESIS.md` heredoc in `04-bootstrap-script-v1.md` nearly word for word; the phase table at the end is the only content not present in the other captures. Treat this file as an input artifact, not a steering document.

---

# Project Aethelgard: Closed-Loop, Self-Verifying Hardware-to-Kernel Synthesis Engine

**Lead Systems Architect:** Argon (Gemini)
**Status:** Architecture & Founding Blueprint

## Executive Summary

Project Aethelgard is an autonomous, closed-loop co-design and verification engine that synthesizes custom embedded hardware (schematics, netlists, and PCB layouts) alongside formally verified, memory-safe Rust microkernels and Peripheral Access Crates (PACs) from a single declarative specification. Before a single piece of fiberglass is etched, Aethelgard compiles the design into a cycle-accurate mixed-signal digital twin, subjects it to adversarial fault injection, and autonomously resolves both routing violations and firmware panics in a continuous evolutionary loop.

## Layer 1: First-Principles Explainer (The Foundation)

### What is the core problem?

Every smart physical device—from an industrial temperature probe to a drone flight controller or medical monitor—is made of two halves:

- **The Body (Hardware):** A Printed Circuit Board (PCB) made of fiberglass and copper tracks (traces) connecting microchips, sensors, and power regulators.
- **The Brain (Firmware/Kernel):** The low-level software living directly inside the main microchip that reads electrical voltages on tiny metal pins and translates them into digital data.

Today, building these two halves is completely fractured. An electrical engineer designs the board using specialized drawing software, orders the physical parts from a factory, waits weeks for delivery, and hands the physical board to a software engineer.

When the software engineer plugs it in and it fails to work, chaos begins. Is there a bug in the software code? Did the electrical engineer accidentally wire the sensor to Pin 14 instead of Pin 15? Or are two copper wires running too close together, causing electrical static that corrupts the data? Finding out requires weeks of peering through microscopes, soldering jumper wires, and ordering "Revision B" of the board.

### How does Project Aethelgard solve this?

Imagine an architect who designs both the physical plumbing of a skyscraper and its automated climate control software at the exact same moment, inside a single master blueprint. Better yet, before laying a single brick, the architect builds a 100% physically accurate virtual simulation of the building, hits it with simulated hurricanes and power outages, watches where the pipes leak or the software crashes, and automatically fixes the blueprint until it is indestructible.

- **One Unified Blueprint:** Instead of drawing wires by hand and writing code separately, hardware and software are generated from the exact same mathematical source of truth. It is physically impossible for the software to look for a sensor on the wrong pin, because the pin assignment and the software driver are born together.
- **The Virtual Crash-Test Dummy (Digital Twin):** Before manufacturing anything, the engine creates a virtual clone of the circuit board and the microchip. It runs the real software on the virtual chip and simulates electricity flowing through the copper tracks.
- **Self-Healing Design Loop:** The engine intentionally tries to break its own creation—simulating dying batteries, electrical interference, and corrupted sensor data. If the virtual device crashes, the engine reads the crash report, adjusts the copper wiring or rewrites the code, and tests it again until it passes 100% of the time.

## Layer 2: Deep Technical Architecture (For the Collaborative Partner)

At the systems level, Aethelgard treats PCB layout, silicon register topologies, and OS kernel abstractions as projections of a single Unified Intermediate Representation (UIR).

### 1. The Unified Intermediate Representation (UIR) & Co-Synthesis

Instead of starting in KiCad or Altium and throwing a PDF schematic over the wall to an embedded Rust developer, the entry point is a declarative constraint graph written in a Rust-embedded DSL (or parsed from a high-level intent specification).

- **Hardware Lowering:** The UIR specifies functional blocks (e.g., an MCU such as an STM32H7 or ESP32-S3, power regulation topologies, and sensor buses like I2C/SPI/CAN). A constraint solver (backed by Z3/SMT) resolves pin-muxing matrices, voltage domain isolation, decoupler placement, and impedance-matched differential pair routing, emitting KiCad S-expressions (.kicad_sch, .kicad_pcb) and Gerber/IPC-2581 fabrication packages.
- **SVD-to-Rust Type-State Synthesis:** Simultaneously, the UIR lowers the exact pin-mux and peripheral configuration into a custom System View Description (SVD) and compiles a board-specific Peripheral Access Crate (PAC) and Hardware Abstraction Layer (HAL) in #![no_std] Rust.
- **Compile-Time Hardware Guarantees:** Using Rust's affine type system and zero-sized types (ZSTs) for typestate programming, invalid hardware states become unrepresentable at compile time. You cannot invoke a DMA transfer on an SPI bus whose clock gate hasn't been enabled or whose CS pin is muxed to an active PWM timer, nor can you violate setup/hold timing margins defined by the physical trace capacitance in the PCB layout.

### 2. Cycle-Accurate Mixed-Signal Digital Twin

To close the loop without waiting on silicon fab or PCB assembly, Aethelgard compiles the synthesized artifacts into a deterministic co-simulation harness:

- **Instruction & Bus Simulation:** The #![no_std] Rust ELF binary runs inside a cycle-accurate instruction-set simulator (built atop Renode or Verilator for RISC-V/ARM cores) synchronized with a virtual bus matrix.
- **Analog & Signal-Integrity Co-Simulation:** For critical physical interfaces (power rails, high-speed SPI, analog ADC frontends, I2C pull-up networks), the engine extracts parasitic inductance, capacitance, and resistance (RLC) from the generated PCB trace geometry and feeds them into a lightweight ngspice / Xyce mixed-signal solver bridged to the digital simulator's virtual GPIO/bus hooks.
- **Adversarial Chaos & Fault Injection:** A fuzzing orchestrator injects deterministic faults into the virtual physical layer:
  - **Clock & Bus Glitching:** Stretching I2C clocks past timeouts, injecting bit-flips onto UART/SPI lines, or holding SDA low mid-transaction to simulate a wedged peripheral sensor.
  - **Power Integrity Transients:** Simulating voltage droop during high-current radio TX bursts or brownout resets (BOR) during flash write operations.
  - **DMA & Interrupt Storms:** Firing asynchronous external interrupts at pathological intervals to expose race conditions, priority inversions, or deadlocks in the async Rust executor (e.g., Embassy).

### 3. Telemetry Audit, Deterministic Replay & Autonomous Evolution

- **Zero-Loss Execution Trace:** Every register read/write, interrupt vector jump, bus voltage transition, and netlist constraint check is streamed as structured binary telemetry (e.g., via Cap'n Proto or FlatBuffers over an event stream into object storage like Cloudflare R2) indexed by simulation picosecond.
- **Cross-Domain Root-Cause Attribution:** When an invariant fails (e.g., a Rust panic!, a watchdog timeout, a stack overflow, or a signal rise-time violation), the engine diffs the cycle-accurate execution trace against the UIR constraint graph:
  - If the failure is electrical/physical (e.g., I2C rise time exceeds 300ns due to trace capacitance), the engine mutates the hardware graph (swapping pull-up resistor BOM values from 4.7kΩ to 2.2kΩ or shortening the trace routing constraint).
  - If the failure is logical/temporal (e.g., DMA buffer overrun or bus lockup), the engine synthesizes a patch to the Rust driver state machine (such as adding a 9-clock-pulse bus recovery sequence or adjusting interrupt priorities) and verifies freedom from undefined behavior via Kani (bounded model checking) or Miri.

## Layer 3: Strategic Impact, Beneficiaries & Execution Roadmap

### Why This Is Critically Needed Now

Generative AI has already conquered pure software crud apps, and early attempts at "one-shot AI PCB design" are starting to appear. However, one-shot open-loop hardware generation is fundamentally broken. When an AI model hallucinates a software line, your compiler catches it in 2 seconds. When an AI model hallucinates a pin-muxing capability, forgets a bootstrap resistor, or writes a driver that deadlocks under bus noise, you don't find out until $5,000 and three weeks of fab lead-time have burned—or worse, when 10,000 deployed IoT units brick themselves in the field.

Bridging the chasm between software velocity and physical-world reliability requires moving from probabilistic generation to formally verified, closed-loop synthesis.

### Who Benefits From This Build?

- **IoT, Edge AI & Sensor Network Architects:** Teams building environmental probes, agricultural telemetry nodes, and industrial monitors who need custom carrier boards and bulletproof firmware deployed in days, not quarters, with zero field-truck rolls.
- **Robotics, Drone & Autonomous Systems Engineers:** Builders who need rapid iteration on custom actuator controllers, BMS (Battery Management Systems), and sensor-fusion hubs where a firmware panic or bus lockup means a physical crash.
- **High-Assurance & Critical Infrastructure Sectors:** Medical device makers, aerospace contractors, and energy grid operators who spend 70%+ of their engineering budget on formal verification, DO-178C/ISO-26262 compliance, and hardware-in-the-loop (HIL) testing.
- **Solo Hardware Founders & Small R&D Labs:** Giving a single clued-in builder the effective output of a 15-person multidisciplinary electrical, embedded, and verification engineering department.

### How We Make It Happen: Phased Execution Plan

| Phase | Duration | Core Deliverables | Verification Milestone |
|---|---|---|---|
| Phase 0: Repository Bootstrap & MCP Backbone | Weeks 1–2 | Monorepo layout, MCP tool integration with KiCad CLI, Cargo, Kani, and Renode headless runners. | End-to-end tool invocation and structured binary telemetry streaming to object storage. |
| Phase 1: UIR Schema & SVD-to-Typestate Compiler | Weeks 3–5 | Declarative graph schema (aethelgard-ir), SVD parser, and pin-mux resolution pass. | Generation of fully checked #![no_std] Rust HAL/BSP crates and valid KiCad schematics. |
| Phase 2: Headless Co-Simulation & Chaos Harness | Weeks 6–8 | Integration of Renode digital MCU emulation with ngspice/Xyce RLC parasitics and fault-injection engine. | Automated discovery and logging of injected bus lockups, trace capacitance delays, and interrupt storms. |
| Phase 3: Closing the Autonomous Loop & Physical Fab | Weeks 9–12 | Feedback loop coupling diagnostic traces to UIR mutation; automated software patching and rerouting. | 100% pass rate on formal and chaos gates; Gerber export and initial tape-out of reference hardware. |
