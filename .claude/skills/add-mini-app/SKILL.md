---
name: add-mini-app
description: Scaffold a new Last Light mini app (a card on the main screen), wire it into the app list, and give it SQLite storage if it keeps data. Use when the user asks to add, build or start a new mini app such as a text editor, journal, calculator or supplies tracker.
---

# Add a mini app

Every mini app is a type implementing `MiniApp` (`src/apps/mod.rs`) and registered in `apps::all()`. The shell in `src/main.rs` handles everything else: the card on the grid, opening the app, the Back button and the header title ("<App> | Last Light"). Don't touch the shell to add an app.

## Steps

1. **Pick the module shape.**
   - Simple app: `src/apps/<name>.rs`.
   - App with storage or a worker thread: a folder `src/apps/<name>/` with `mod.rs` (UI and state), plus `store.rs` (SQLite) and/or `worker.rs` as needed. Mirror `src/apps/ai_chat/`.
2. **Implement `MiniApp`**: `name()` is the card and header title, `description()` is one short sentence for the card, and `ui(&mut self, ui)` draws the app inside the central area.
3. **Register it** in `apps::all()` in `src/apps/mod.rs`. If a `Placeholder::new("<Name>", ...)` already exists for it, replace that line rather than adding a second card.
4. **Persistence (if the app keeps data):**
   - Open the shared DB with `crate::storage::open_db()`. It's one `last-light.db` in the per-user data folder, shared by all apps.
   - Create the app's own tables with `CREATE TABLE IF NOT EXISTS`, prefixing table names with the app (e.g. `journal_entries`).
   - Copy `Store::open()` from `src/apps/ai_chat/store.rs`: fall back to an in-memory DB and surface a warning instead of panicking.
   - Add store unit tests against `Connection::open_in_memory()` (see `ai_chat/store.rs` tests).
5. **Long-running work** (inference, big file I/O) goes on a worker thread that talks to the UI over `std::sync::mpsc` and calls `ctx.request_repaint()` after sending. Never block `ui()`.
6. **Update `SPEC.md`**: move the app from "Ideas" to "Planned", or describe it.
7. **Verify**: `cargo build`, `cargo test` and `cargo clippy`. Then check the app runs (`cargo run`) or tell the user what to check, since GUI changes can't be fully verified headless.

## Rules that apply to every mini app

- **Offline only.** No network calls, no crates that phone home, no remote assets. Bundled files go under `assets/` (copied next to the binary by `build.rs`) and are found relative to `std::env::current_exe()`.
- **Zero install for end users.** Don't add dependencies that need system libraries or runtime DLLs. Prefer pure Rust, or crates that statically compile their C code (as with `rusqlite`'s `bundled` feature). If you add a C/C++ dependency, run the `verify-zero-install` skill afterwards.
- **egui 0.37 API.** See the `egui-ui` skill before writing UI code; many online examples use older APIs.
