# Landscape Design

## Goal

Add an optional ground reference layer so the terminal sky can show either no landscape, a minimal horizon, or a bearing-oriented observation ground.

## Approved Behavior

- Default landscape mode is `horizon`.
- Users can switch between `off`, `horizon`, and `bearings`.
- The settings panel exposes the landscape mode and sky orientation.
- `g` remains reserved for the globe / sky flip.
- Sky orientation defaults to observer view, with east on the left and west on the right. A map-style option keeps east on the right when desired.
- The landscape overlay follows the selected sky orientation so bearing cues, labels, stars, planets, and pointer hit-testing stay aligned.

## Rendering Direction

`off` keeps the existing plain sky. `horizon` keeps the visual weight low by emphasizing the Alt=0 horizon and a dim ground band. `bearings` adds stronger N/E/S/W cues and small bearing ticks near the horizon so the feature feels like an observing aid rather than decoration.
