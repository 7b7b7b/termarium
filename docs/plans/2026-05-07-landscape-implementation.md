# Landscape Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a configurable landscape overlay with `off`, `horizon`, and `bearings` modes, defaulting to `horizon`.

**Architecture:** Store the landscape mode and sky orientation in `DisplayConfig` as serializable enums. Cycle them from the settings panel. Render the overlay in `sky_lines` before stars so stars, planets, and labels remain readable.

**Tech Stack:** Rust, Ratatui, Crossterm, Serde, existing unit tests.

---

### Task 1: Configuration And Input

**Files:**
- Modify: `src/config.rs`
- Modify: `src/app.rs`
- Test: `src/config.rs`
- Test: `src/app.rs`

**Step 1: Write failing tests**

- Assert `Config::default().display.landscape` is `LandscapeMode::Horizon`.
- Assert deserializing old config JSON without `landscape` defaults to `Horizon`.
- Assert the settings row cycles `horizon -> bearings -> off -> horizon`.

**Step 2: Run targeted tests**

Run: `cargo test landscape`

Expected: fail because `LandscapeMode` and settings support do not exist.

**Step 3: Implement minimal code**

- Add `LandscapeMode` enum with `Off`, `Horizon`, and `Bearings`.
- Add `DisplayConfig.landscape` with a default helper.
- Add `LandscapeMode::next`.
- Handle the settings row and save config.

**Step 4: Run targeted tests**

Run: `cargo test landscape`

Expected: pass.

### Task 2: Settings And Text

**Files:**
- Modify: `src/app.rs`
- Modify: `src/ui.rs`
- Modify: `src/i18n.rs`
- Test: `src/ui.rs`

**Step 1: Write failing tests**

- Assert `settings_count()` includes the landscape row.
- Assert settings rendering includes the landscape row.

**Step 2: Run targeted tests**

Run: `cargo test settings landscape footer`

Expected: fail until settings rows and translations exist.

**Step 3: Implement minimal code**

- Add the landscape row before limiting magnitude.
- Teach `adjust_setting` to cycle landscape mode.
- Add English and Chinese labels and help text.

**Step 4: Run targeted tests**

Run: `cargo test settings landscape footer`

Expected: pass.

### Task 3: Canvas Overlay

**Files:**
- Modify: `src/ui.rs`
- Test: `src/ui.rs`

**Step 1: Write failing tests**

- Assert horizon mode draws a ground/horizon marker on a normal sky buffer.
- Assert off mode does not draw the ground marker.
- Assert bearings mode draws bearing tick cues.

**Step 2: Run targeted tests**

Run: `cargo test landscape`

Expected: fail because overlay rendering does not exist.

**Step 3: Implement minimal code**

- Draw overlay before stars in `sky_lines`.
- Keep the existing Alt=0 horizon points.
- For `horizon`, fill a dim bottom band and mark the center horizon.
- For `bearings`, add bearing ticks near N/E/S/W projected horizon points.

**Step 4: Verify**

Run: `cargo fmt`
Run: `cargo test`

Expected: all tests pass.

### Task 4: Sky Orientation

**Files:**
- Modify: `src/config.rs`
- Modify: `src/astro.rs`
- Modify: `src/app.rs`
- Modify: `src/ui.rs`
- Modify: `src/i18n.rs`
- Test: `src/astro.rs`
- Test: `src/config.rs`
- Test: `src/app.rs`
- Test: `src/ui.rs`

**Step 1: Write failing tests**

- Assert the default projection places east on the left and west on the right.
- Assert the map projection places east on the right and west on the left.
- Assert `Config::default().display.sky_orientation` is `SkyOrientation::Observer`.
- Assert the settings row cycles `observer -> map -> observer`.
- Assert cardinal labels flip east/west when the setting changes.

**Step 2: Implement minimal code**

- Add `SkyOrientation` with `Observer` default and `Map` alternate.
- Add `DisplayConfig.sky_orientation` with serde default.
- Route stars, deep-sky objects, planets, landscape cues, selected labels, pointer hit-testing, and zoom projection through the selected orientation.
- Add English and Chinese labels.

**Step 3: Verify**

Run: `cargo test sky_orientation`
Run: `cargo test projection`
Run: `cargo test`
