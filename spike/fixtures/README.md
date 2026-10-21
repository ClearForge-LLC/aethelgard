# Spike calibration inputs (AET-WO-0300)

Hand-written schematics for measuring `kicad-cli sch erc`: one with nothing on it, and three candidates for a single deliberate violation (a dangling wire, a dangling label, an unconnected input pin). They are tool-calibration inputs, not board artifacts; no specification produces them, so `INV-01` does not govern them. The build half moves the clean one and one violating one into `container/fixtures/` and deletes this directory.
