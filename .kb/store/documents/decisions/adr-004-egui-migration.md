---
id: 019e278a-f885-72a3-b5d4-3be20e4a88ae
slug: decisions/adr-004-egui-migration
title: "ADR-004: Replace piston_window with egui/eframe for visualization"
type: brief
status: active
tags: [visualization, gui, architecture]
---

# ADR-004: Replace piston_window with egui/eframe for visualization

## Status
Active — implemented in PR #57 (branch `feat/40-egui-migration`)

## Context

The TSP solver renders a live route visualization while solving. This was
implemented using `piston_window 0.147` / `piston 1.0`, which:

- Required a bundled font file (`assets/FiraSans-Light.ttf`)
- Had minimal maintenance activity (161 stars, infrequent releases)
- Caused cross-platform build headaches (wgpu/OpenGL backend churn)
- Forced a specific main-thread workaround for the event loop

## Decision

Replace `piston_window` with `eframe 0.33` + `egui 0.29` (re-exported via eframe).

`eframe` is the de-facto standard for immediate-mode native GUIs in Rust
(29k stars, updated daily). It handles fonts internally, supports x11 and
Wayland on Linux via feature flags, and uses the same winit/wgpu stack.

## Key implementation details

- `eframe::App` trait replaces the piston event loop; `fn update()` is called each frame
- `eframe::run_native()` takes ownership of the app (`self`, not `&mut self`)
- Internal message handler renamed `update()` → `handle_message()` to avoid collision with `eframe::App::update`
- `ctx.request_repaint_after(Duration::from_millis(16))` throttles redraws to ~60 fps (bare `request_repaint()` would spin CPU at 100%)
- `egui::Frame::NONE.fill(WHITE)` for white background (`Frame::none()` was removed in egui 0.28)
- Color constants use `Color32::from_rgba_premultiplied` (const fn); `from_rgba_unmultiplied` is not const in egui 0.33
- `build_edge` is a free function (not `&self` method) to avoid simultaneous borrow of `city_table` and `cities_bounding_box` in `add_path`

## Visualization colour scheme

| Colour | Meaning |
|--------|---------|
| Black nodes | All cities (default) |
| Red node | Active city (`CityChange` message) |
| Blue edges (1.5px) | Current route being explored |
| Green edges (2.5px) | Best route found — shown only after `Done` |
| Orange edge (3px) | Prev→current city step (B&B active step) |

## Channel/messaging system

`send_progress`, `ProgressMessage`, `init_channels`, `try_retrieve_message`
and the global `LazyLock<Mutex<Option<…>>>` channels are **unchanged**.
Only the rendering layer was replaced.

## Consequences

- `assets/FiraSans-Light.ttf` deleted; no font assets needed
- `Cargo.toml`: removed `piston`, `piston_window`; added `eframe = { version = "0.33", features = ["x11", "wayland"] }`
- `src/main.rs`: `let mut progress_display` → `let progress_display` (ownership, not mutable ref)
- All existing tests continue to pass unchanged
