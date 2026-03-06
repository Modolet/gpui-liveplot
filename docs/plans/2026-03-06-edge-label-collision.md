# Edge Label Collision Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Improve axis edge tick-label collision handling by using clipped visible bounds instead of full off-screen label bounds.

**Architecture:** Keep tick label anchor positions unchanged, continue relying on axis clip rectangles for rendering, and only change collision/overlap decisions to use the portion of a label that is actually visible inside the axis region. This preserves the current center-anchor semantics while reducing over-conservative hiding near edges.

**Tech Stack:** Rust, cargo test/check/clippy, existing GPUI backend geometry utilities.

---

### Task 1: Add failing tests for visible-bounds collision semantics

**Files:**
- Modify: `src/gpui_backend/frame.rs`

**Step 1: Write the failing test**
- Add unit tests for intersecting a tick label rect with an axis rect.
- Add a test proving a partially visible label produces a smaller visible rect.
- Add a test proving a fully off-screen label produces no visible rect.

**Step 2: Run test to verify it fails**
- Run: `cargo test x_tick_label_visible_rect --lib`
- Expected: FAIL because helper logic does not exist yet.

### Task 2: Implement minimal visible-rect helpers

**Files:**
- Modify: `src/gpui_backend/frame.rs`

**Step 1: Write minimal implementation**
- Add helpers to build full label rects and intersect them with the axis clip rect.
- Return `None` when the visible area is empty.

**Step 2: Run focused tests**
- Run: `cargo test tick_label_visible_rect --lib`
- Expected: PASS

### Task 3: Switch axis collision checks to visible rects

**Files:**
- Modify: `src/gpui_backend/frame.rs`

**Step 1: Update x-axis collision logic**
- Use visible rect for title overlap and previous-label overlap bookkeeping.

**Step 2: Update y-axis collision logic**
- Use visible rect for title overlap and previous-label overlap bookkeeping.

**Step 3: Run focused tests**
- Run: `cargo test frame --lib`
- Expected: PASS

### Task 4: Verify and commit

**Files:**
- Modify: `docs/plans/2026-03-06-edge-label-collision.md`
- Modify: `src/gpui_backend/frame.rs`

**Step 1: Run verification**
- Run: `RUSTC_WRAPPER= cargo check`
- Run: `RUSTC_WRAPPER= cargo clippy --all-targets`

**Step 2: Commit**
- Run: `git add docs/plans/2026-03-06-edge-label-collision.md src/gpui_backend/frame.rs && git commit -m "fix: 优化边缘刻度标签避碰"`
