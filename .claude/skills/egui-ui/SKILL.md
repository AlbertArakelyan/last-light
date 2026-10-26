---
name: egui-ui
description: Patterns and pitfalls for writing egui 0.37 UI in Last Light: panels, layouts, cards, truncation, focus, and headless layout tests. Use before writing or changing any egui UI code, or when a widget, panel or layout misbehaves (wrong size, growing panels, swallowed clicks).
---

# egui 0.37 UI in Last Light

egui 0.37 renamed several core APIs, and most examples online (and in model memory) are older. **Don't guess APIs:** check the real signatures in the local crate source at `~/.cargo/registry/src/*/egui-0.37.0/src/`, or query the Context7 MCP for `egui`.

## API differences from older egui

- **App entry point:** `eframe::App::ui(&mut self, ui: &mut egui::Ui, frame)`. There is no `update(ctx, frame)`.
- **Panels:** `egui::Panel::top/bottom/left/right(id)`. `TopBottomPanel` and `SidePanel` are gone. Size methods are `default_size`, `size_range` and `exact_size`, not `default_width`.
- **Showing panels:** panels and `CentralPanel` take a `&mut Ui`, as in `.show(ui, |ui| ...)`. Panels nest: an app's `ui()` can add its own `Panel::left` and `CentralPanel` inside the shell's central panel.
- **Text edit alignment:** use `TextEdit::align(Align2)`; `vertical_align` is deprecated.
- **Buttons:** `egui::Button::selectable(selected, text)` gives a selectable row, `.truncate()` elides long text, and `.min_size(vec2)` sets a minimum size.
- **Child UIs:** `ui.scope_builder(UiBuilder::new().layout(...), ...)` and `ui.allocate_ui_with_layout(size, layout, ...)`.

## Pitfalls already hit in this codebase

- **Panels grow when content overflows.** If a row is wider than the panel, egui widens the panel every frame, ignoring `size_range` and the user's resizing. Lay out rows so they can never overflow: put fixed-size widgets first in a `Layout::right_to_left` row, then give the flexible widget exactly `ui.available_width()` with `.truncate()`. See `chat_row` in `src/apps/ai_chat/mod.rs`.
- **Labels swallow clicks.** Labels are selectable by default, so a label on top of a clickable area steals hover and clicks. For custom clickable surfaces, allocate with `Sense::click()` and paint text with `ui.painter().layout(...)` plus `painter.galley(...)` instead of adding label widgets. See `card` in `src/main.rs`.
- **Spacing leaks into children.** Changing `ui.spacing_mut().item_spacing` affects every child UI too, which caused oversized line gaps in cards.
- **Rename-style text fields:** check `lost_focus()` first (Escape cancels, anything else commits), and only `request_focus()` while the field doesn't have focus. Otherwise clicking away never commits.
- **Background updates:** threads must call `ctx.request_repaint()` after sending a UI update, or the UI won't refresh until the mouse moves.

## Headless layout tests

Layout bugs can be caught without a window. Render a few frames with `Context::run_ui` and assert on response rects:

```rust
let ctx = egui::Context::default();
for _ in 0..20 {
    let output = ctx.run_ui(Default::default(), |ui| {
        let panel = egui::Panel::left("id").default_size(220.0).show(ui, |ui| { /* widgets */ });
        widths.push(panel.response.rect.width());
    });
    output.drop_without_applying_deltas(); // otherwise epaint panics on unapplied texture deltas
}
```

Render several frames (sizes settle over frames) and confirm the test fails on the buggy layout before trusting it. See `long_titles_do_not_widen_the_chat_list` in `src/apps/ai_chat/mod.rs`.
