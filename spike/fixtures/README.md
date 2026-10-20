# Tool-calibration inputs

These files exercise the tools in the image on known inputs: a board with one deliberate clearance violation between `NET_A` and `NET_B` (two tracks 0.05 mm apart against the default 0.2 mm), the same board without the tracks, and an RC step whose rise time is known in closed form. They are not board artifacts and no specification produces them, so `INV-01` (`docs/northstar.md` §4) does not govern them.
