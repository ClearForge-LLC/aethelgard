# Argon session capture — response 7 ("follow-up: STM32F7_I2C, headless routing, CBOR red-team")

**Source:** Argon (Gemini), answering the architect's three follow-up asks after capture 06: the Renode I2C model, headless place and route, and the CBOR profile.
**Captured:** 2026-10-29. Scotty pasted it into the architect's Claude Code session, and Argon's text is reproduced verbatim. The paste had lost its Markdown heading markers, its bullet nesting and its math delimiters, and it appears here as it arrived.
**Fidelity notes:** These are Argon's claims, not measurements.
- **§1:** Argon reports it could not read the source. The architect then read it at Renode v1.17.0's pin. The model is transaction-level, as inferred here, but `BERR` and `ARLO` are `WithTag` placeholders. At 1.17.0 so is `NACKF`, and the error interrupt is never set (`docs/architecture.md` §2.7).
- **§2's routing leads** are recorded as unverified in `docs/roadmap.md` (`AET-WO-1302`).
- **§3:** the equal-timestamp sort key was declined, because it would reorder cause and effect (`AET-WO-0400` §1.2).

Treat this file as an input artifact, not a steering document.

---

1. Confirming STM32F7_I2C in Renode Source
Path in Renode's renode-infrastructure repository:
src/Emulator/Peripherals/Peripherals/I2C/STM32F7_I2C.cs
Honest epistemic boundary (per your rule): I cannot fetch the raw line-by-line body of STM32F7_I2C.cs from GitHub through web search indexes today, so I will not invent line numbers. However, you already have the exact C# source file sitting inside your published image (container/IMAGE)!
Renode's portable Linux tarball ships peripheral sources (or the unstripped Emulator/Peripherals.dll) right under /opt/renode. Have Claude or Furnace run this one-liner against the image:
docker run --rm --network none "$(cat container/IMAGE)" \
  find /opt/renode -name "*STM32F7_I2C*" -o -name "Peripherals.dll"

What I am inferring from Renode's II2CPeripheral architecture (to verify in that file):
 * Every Renode I2C host controller inherits from SimpleContainer<II2CPeripheral> and talks to child devices via II2CPeripheral.Write(byte[] data) and II2CPeripheral.Read(int count)—never by bit-banging GPIO pins.
 * In STM32F7_I2C.cs, check the Registers enum (usually at the bottom of the file) and the field definitions for InterruptsStatus (ISR, offset 0x18): specifically whether BusError (BERR, bit 8) and ArbitrationLost (ARLO, bit 9) are defined as FlagRegisterFields or Ignored/Reserved bits, and whether setting them triggers ErrorInterrupt (ER_IRQ).
2. Headless Place-and-Route for P1 (AET-WO-1300)
Catching unconnected_items (crates/aethelgard-cli/src/kicad.rs:76) as a P1 DRC blocker was a great catch by you and Claude.
Fact check on kicad-cli and Specctra DSN/SES:
 * kicad-cli CANNOT export .dsn or import .ses. In KiCad 10.0.7, kicad-cli pcb export supports gerbers, drill, pos, ipc2581, odb, gencad, svg, pdf, dxf, and vrml/step—not Specctra DSN/SES.
 * Headless Python (pcbnew) does support DSN/SES inside your existing container: Because container/Dockerfile builds FROM kicad/kicad:10.0.7, the image already contains KiCad's python3 and the SWIG pcbnew module! You can run python3 -c "import pcbnew; b = pcbnew.LoadBoard('in.kicad_pcb'); pcbnew.ExportSpecctraDSN(b, 'out.dsn')" and pcbnew.ImportSpecctraSES(b, 'routed.ses'); pcbnew.SaveBoard('out.kicad_pcb', b) completely headless with no X11 display. (Note: KiCad has deprecated SWIG pcbnew for removal in KiCad 11, where it moves to the IPC API, but it works in your pinned KiCad 10.0.7 image).
 * Why I recommend AGAINST Freerouting for P1:
   * Freerouting requires adding a Java runtime (JRE 21) + Freerouting .jar to container/Dockerfile.
   * Freerouting's multi-pass Simulated Annealing autorouter is frequently non-deterministic across runs (violating the P1 exit gate: "Regenerating from the specification twice produces byte-identical board files", docs/roadmap.md §5 P1).
   * Calling python3 -c "import pcbnew..." violates RUL-12 ("Hooks that run inside Renode are Python... C++ is not written here") unless amended.
The Cleaner P1 Path: Pure-Rust Deterministic Grid/Channel Router in aethelgard-eda
For a 4-layer reference probe (1 MCU, 2 I2C sensors, 1 SPI flash, passives, 1 rail — docs/northstar.md §3), you have ~15–25 nets!
 * Layer stack: F.Cu (components + horizontal signal escape), In1.Cu (solid GND plane with filled (zone ...)), In2.Cu (solid VDD_3V3 plane with filled (zone ...)), B.Cu (vertical signal trunks).
 * How GND and VDD_3V3 route: Every GND and VDD_3V3 pad drops a short stub + (via ...) straight to the inner plane! However, note one KiCad quirk: kicad-cli pcb drc does not automatically refill (zone ...) polygons unless the (filled_polygon ...) S-expression is already present in .kicad_pcb. Even simpler: route GND and VDD_3V3 as a deterministic Manhattan spine of (segment ...) tracks on In1.Cu and In2.Cu (or emit the pre-computed rectangular (filled_polygon ...) for the two inner planes).
 * How the ~10 signal nets route: A 150-line deterministic A* (or Lee wave) Manhattan grid router on a 0.25 mm grid in pure Rust inside aethelgard-eda routes F.Cu (horizontal) + B.Cu (vertical) with (via ...) transitions in < 5 ms, is 100% byte-deterministic (INV-01), needs zero Java/SWIG/X11, and gives you exact trace lengths (segment (start ...) (end ...) (width ...) (layer ...) (net ...)) so aethelgard-eda can compute R, L, and C parasitics directly from the segments it just routed!
3. Red-Team of Your Deterministic CBOR Profile (RUL-06)
Choosing CBOR (RFC 8949) for cross-language spec clarity (Rust + Python + TypeScript viewer) makes complete sense.
If your profile restricts CBOR to Major Types 0 (unsigned int), 1 (negative int), 3 (UTF-8 text string), and 4 (array) in shortest-length encoding, a strict decoder is almost bijection-tight, except for 5 specific edge cases that will let two distinct byte streams decode to the same trace unless your decoder explicitly refuses them:
 * Indefinite-Length Strings and Arrays (additional info = 31):
   * 0x9f ... 0xff (indefinite-length array) and 0x7f ... 0xff (indefinite-length chunked text string) decode to the exact same Rust Vec<T> / String as definite-length 0x80.. / 0x60...
   * Rule: Refuse any initial byte with additional_info == 31 (b & 0x1f == 31).
 * Non-Canonical / Overlong Integer & Length Encodings (RFC 8949 §4.2.1):
   * Value 10 encoded in 1 byte (0x0a), 2 bytes (0x18 0x0a), 3 bytes (0x19 0x00 0x0a), 5 bytes (0x1a ...), or 9 bytes (0x1b ...).
   * Rule: Your strict decoder must enforce bounds per additional_info:
     * 24 (0x18, 1-byte uint): value MUST be \ge 24.
     * 25 (0x19, 2-byte uint): value MUST be \ge 256.
     * 26 (0x1a, 4-byte uint): value MUST be \ge 65536.
     * 27 (0x1b, 8-byte uint): value MUST be \ge 4294967296.
       (Most off-the-shelf CBOR crates like ciborium accept overlong encodings by default when deserializing via serde! Either write a 60-line canonical CBOR validator/decoder in aethelgard-chaos or verify with an adversarial test that 0x18 0x00 is refused).
 * Unicode Normalization & Lone Surrogates in Major Type 3:
   * UTF-8 prevents lone surrogates (which Rust's str::from_utf8 already rejects), but "é" as U+00E9 (NFC) vs "e" + U+0301 (NFD) are two different byte streams that represent the same string visually.
   * Rule: Restrict identifiers/tags inside traces to printable ASCII (0x21..=0x7E, matching RUL-14), where UTF-8 encoding is unique.
 * Unordered Sets Masquerading as Arrays (Application-Layer Canonical Order):
   * Even without CBOR Maps (Major Type 5), if a trace frame contains an array of simultaneous events at the same timestamp_ns (e.g., [[pin_a, 1], [pin_b, 0]] vs [[pin_b, 0], [pin_a, 1]]), two runs could emit the array in different orders.
   * Rule: Specify a total sort key for events sharing a timestamp (e.g., sorted by (timestamp_ns, domain_kind, peripheral_or_net_id, event_kind)), and have the reader refuse any trace whose event stream is not monotonically non-decreasing in that key.
 * Trailing Bytes After the Root Array / Frame:
   * If a file or frame has trailing padding bytes (0x00 or a second CBOR item), a decoder that stops after the first item ignores the tail.
   * Rule: Require exact EOF consumption (or length-prefix each frame and assert the frame slice is completely consumed).
Want me to check whether ciboriumaccepts overlong 0x18 0x00 integers by default, or are you writing the strict 4-type CBOR codec directly in aethelgard-chaos (which is ~80 lines of safe Rust with zero dependencies)?
