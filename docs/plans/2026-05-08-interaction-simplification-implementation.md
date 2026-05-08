# Interaction Simplification Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Simplify Termarium's always-visible interaction hints while preserving existing shortcuts.

**Architecture:** Keep `App::handle_key` behavior unchanged. Update translations, footer rendering copy, help grouping, README usage text, and unit tests so the UI presents a smaller primary action surface.

**Tech Stack:** Rust, Ratatui, Crossterm, existing unit tests.

---

### Task 1: Footer Hint Hierarchy

**Files:**
- Modify: `src/i18n.rs`
- Modify: `src/ui.rs`
- Test: `src/ui.rs`

**Step 1: Write failing tests**

- Assert the sky footer includes `/ search`, `s location`, `g globe/sky`, `o settings`, and `? help`.
- Assert the sky footer no longer includes pointer, zoom, magnitude, time, constellation, line, or tonight shortcuts.
- Assert the mouse hint uses the compact left/center/right wording.

**Step 2: Run targeted tests**

Run: `cargo test footer`

Expected: fail because the current footer still lists the long shortcut catalog.

**Step 3: Implement minimal code**

- Replace English and Chinese footer copy with the shorter primary actions.
- Replace the mouse hint copy with the shorter spatial description.

**Step 4: Run targeted tests**

Run: `cargo test footer`

Expected: pass.

### Task 2: Help Overlay Grouping

**Files:**
- Modify: `src/ui.rs`
- Test: `src/ui.rs`

**Step 1: Write failing tests**

- Assert common help starts with search/globe/location/settings/help rather than a dense shortcut list.
- Assert advanced shortcuts such as pointer, zoom, constellation cycling, and magnitude remain present in help.
- Assert globe help keeps rotation and save-preview affordances.

**Step 2: Run targeted tests**

Run: `cargo test help`

Expected: fail until help sections are regrouped.

**Step 3: Implement minimal code**

- Rename help sections around "常用", "时间", "星空进阶", and "显示/设置" in Chinese.
- Apply the same structure as "Common", "Time", "Sky Tools", and "Display" in English.
- Keep existing shortcut behavior and labels.

**Step 4: Run targeted tests**

Run: `cargo test help`

Expected: pass.

### Task 3: README And Full Verification

**Files:**
- Modify: `README.md`

**Step 1: Update docs**

- Make the "Inside the TUI" section match the simplified footer hierarchy.
- Keep the advanced shortcut list below it.

**Step 2: Verify**

Run: `cargo fmt`
Run: `cargo test`

Expected: all tests pass.
