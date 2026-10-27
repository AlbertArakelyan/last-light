---
name: egui-ui
description: The Last Light visual style guide plus patterns and pitfalls for egui 0.37: theme colors and helpers, panel and screen layout conventions, cards, truncation, focus, and headless layout and screenshot tests. Use before writing or changing any egui UI code or styling, or when a widget, panel or layout misbehaves (wrong size, growing panels, swallowed clicks).
---

# egui 0.37 UI in Last Light

egui 0.37 renamed several core APIs, and most examples online (and in model memory) are older. **Don't guess APIs:** check the real signatures in the local crate source at `~/.cargo/registry/src/*/egui-0.37.0/src/`, or query the Context7 MCP for `egui`.

## Last Light style guide

The app has one look, defined in `src/theme.rs`: warm charcoal surfaces lit by a single amber accent, like the last lamp still burning. It is always dark. Keep every screen in this style.

**Colors.** Use only the `theme` constants, never egui's defaults (`ui.visuals().…_color`) or a new `Color32`. If something genuinely needs a new color, add a named constant to `theme.rs` first.

| Constant | Use for |
|---|---|
| `BG` | The darkest layer: the card grid, editor and chat areas, toolbars, text fields |
| `PANEL` | Header and sidebars, and raised "pages" such as the Markdown preview |
| `RAISED` / `HOVER` | Cards, buttons and AI chat bubbles, at rest and hovered |
| `BORDER` / `BORDER_STRONG` | 1 px outlines and separators; the strong one on hover |
| `TEXT_STRONG` / `TEXT` / `TEXT_MUTED` / `TEXT_FAINT` | Titles / body / secondary info / hints and disabled |
| `ACCENT` | The one highlight: selection, focus, the primary action, the lamp. Use it sparingly. |
| `ACCENT_SOFT` | Tinted backgrounds: selected rows, badges, the user's chat bubbles |
| `WARN` / `ERROR` / `DANGER` | Warning text / error text / fill of destructive buttons such as Delete |

**Helpers.** Reuse these instead of styling widgets by hand:
- `theme::primary_button(text)`: the single main action on a screen (Send). Solid amber; at most one per screen.
- `theme::quiet_button(text)`: frameless secondary actions ("+", "...", "Hide sidebar", "x").
- `theme::segmented(ui, enabled, current, &options)`: mutually exclusive modes (Split / Edit / View).
- `theme::section_label(text)` / `widgets::section_header(ui, title, hint, can_add)`: small upper-case sidebar headings with a "+".
- `widgets::list_row` / `widgets::inline_input`: sidebar rows (left-aligned, amber when selected, "..." menu, inline rename).
- `theme::lamp(ui, size)`: the app's mark, for the header and empty states. Its circles live in `theme::LAMP_LAYERS`. The files in `branding/` are generated from that table: after changing it, run `cargo test render_logo -- --ignored`.

**Screen layout conventions.**
- **Panel frames:** every panel sets its own `Frame` with an explicit fill and margin instead of egui's default. Sidebars use `PANEL` with `Margin::symmetric(10, 4)`. A second sidebar column, like the Text Editor's folders, uses `BG` so the two read as levels. Toolbars and central areas use `BG` with 16–20 px side margins.
- **Empty states:** center a `lamp`, a short title in `TEXT`/`TEXT_STRONG`, and one muted sentence saying what to do next (see `TextEditor::editor`, `AiChat::message_list`, `Placeholder::ui`).
- **Status and notices:** small text, `TEXT_FAINT` when idle and `ACCENT` while busy ("Saved" / "Saving..."). Warnings go in `WARN` with a quiet "x" to dismiss.
- **Destructive actions:** always confirm in an `egui::Modal`, with the action as a `DANGER`-filled button next to a plain Cancel.
- **Readable width:** long-form content (chat messages, the composer) sits in a centered column of at most ~760 px (`reading_column` in `src/apps/ai_chat/mod.rs`).
- **Radii:** 6 px for widgets (set by the theme), 8–12 px for cards, pages and bubbles.

**Check it visually.** After any UI change, run `cargo test render_screenshots -- --ignored` and look at `target/screenshots/*.png` before calling the work done. If you add an app or a new state, extend `apps::demo()` and that app's `demo()` constructor so the change appears there.

## API differences from older egui

- **App entry point:** `eframe::App::ui(&mut self, ui: &mut egui::Ui, frame)`. There is no `update(ctx, frame)`.
- **Panels:** `egui::Panel::top/bottom/left/right(id)`. `TopBottomPanel` and `SidePanel` are gone. Size methods are `default_size`, `size_range` and `exact_size`, not `default_width`.
- **Showing panels:** panels and `CentralPanel` take a `&mut Ui`, as in `.show(ui, |ui| ...)`. Panels nest: an app's `ui()` can add its own `Panel::left` and `CentralPanel` inside the shell's central panel.
- **Text edit alignment:** use `TextEdit::align(Align2)`; `vertical_align` is deprecated.
- **Buttons:** `egui::Button::selectable(selected, text)` gives a selectable row, `.truncate()` elides long text, and `.min_size(vec2)` sets a minimum size.
- **Child UIs:** `ui.scope_builder(UiBuilder::new().layout(...), ...)` and `ui.allocate_ui_with_layout(size, layout, ...)`.

## Pitfalls already hit in this codebase

- **Nested layouts stretch.** `ui.with_layout(...)` or `ui.vertical(...)` inside a `right_to_left` row (or inside a `Frame` within one) takes the whole remaining width. This stretched the Split/Edit/View pill across the toolbar and made chat bubbles full width. In a right-to-left row, add the items in reverse order instead (see `theme::segmented`). To size a box to its text, lay the text out first with `ui.painter().layout(...)` and use the galley's width (see `message_ui` in `src/apps/ai_chat/mod.rs`).

- **Panels grow when content overflows.** If a row is wider than the panel, egui widens the panel every frame, ignoring `size_range` and the user's resizing. Lay out rows so they can never overflow: put fixed-size widgets first in a `Layout::right_to_left` row, then give the flexible widget exactly `ui.available_width()` with `.truncate()`. Reuse `list_row` in `src/apps/widgets.rs` for sidebar rows; it already does this.
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

To *see* a screen, run `cargo test render_screenshots -- --ignored` and open `target/screenshots/*.png`; it renders the real shell and apps with demo data through `egui_kittest`.

Render several frames (sizes settle over frames) and confirm the test fails on the buggy layout before trusting it. See `long_titles_do_not_widen_the_panel` in `src/apps/widgets.rs`. To measure a panel nested inside an app, load its `PanelState` (id = parent `ui.scope_id().with_salt(IdSalt::new(name))`), as in `long_names_do_not_widen_the_sidebar_in_any_mode` in `src/apps/text_editor/mod.rs`. Measuring the root `ui.min_rect()` does not work: panels clip overflow instead of growing the root.
