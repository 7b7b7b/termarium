# Interaction Simplification Design

## Goal

Make Termarium's controls feel calmer by showing only the primary interaction paths in the footer and moving less frequent shortcuts into help.

## Approved Behavior

- The sky footer shows only the core loop: search, location, globe/sky, settings, and help.
- The globe footer mirrors that hierarchy with city search, sky, save preview, settings, and help.
- Time controls, pointer mode, constellation zoom/cycling, magnitude, visual toggles, and panels remain available but live in the help panel and settings flow instead of the persistent footer.
- The mouse line stays short and spatial: left opens search, center flips sky/globe, and right opens settings.
- The help overlay is grouped by intent: common actions first, time second, advanced sky or globe operations next, and settings/display last.

## Design Direction

The underlying keyboard behavior stays intact for experienced users. This change is primarily an information architecture pass: reduce what is always visible, clarify what each screen expects, and reserve the full shortcut catalog for `?`.
