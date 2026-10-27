# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

Last Light is an offline-only desktop toolkit (Rust + egui/eframe 0.37) for a "post-apocalyptic, just a laptop left" scenario: a card grid of mini apps such as a text editor and a local AI chat. See `SPEC.md` for the product concept and planned apps.

## Commands

```sh
cargo run                            # build and launch the app
cargo build --release                # release build (target/release/last-light.exe + assets/)
cargo check                          # type-check; works even while the app is running
cargo test                           # unit tests
cargo test -- --ignored --nocapture  # also run tests that need the model file (prints replies)
cargo test remembers_earlier_messages -- --ignored --nocapture   # run one test by name
cargo test render_screenshots -- --ignored   # render each screen with sample data to target/screenshots/*.png
```

- **Checking UI changes:** run `render_screenshots` and look at the PNGs. It draws the real shell and apps through `egui_kittest` (dev-dependency only), using `apps::demo()`. That gives each app sample data in a temp folder or in-memory DB, with no model loaded, so it never touches the user's notes or chats. Simulated clicks on the real window are unreliable when it isn't focused, so prefer this.

- **Locked exe:** if `cargo build` fails with "failed to remove file ... last-light.exe", the app is still running. Close it, or use `cargo check`.
- **First build is slow:** about 2 minutes, because `llama-cpp-sys-2` compiles llama.cpp from source. It needs MSVC, CMake and libclang. `LIBCLANG_PATH` (LLVM `bin`, installed via `scoop install llvm`) and `CMAKE` (Visual Studio's `cmake.exe`) must be set in the environment.

## Hard constraint: zero install for end users

The shipped app is `last-light.exe` plus its `assets/` folder. It must run with nothing installed: no VC++ redistributable, no OpenMP and no network access.
- **Static C runtime:** `.cargo/config.toml` adds `+crt-static` for Rust. It also sets `CMAKE_TOOLCHAIN_FILE_x86_64_pc_windows_msvc` (target-suffixed, so it applies to Windows only) to `cmake/static-crt.cmake`, which forces `/MT` for llama.cpp, because llama.cpp's CMake ignores cmake-rs's `static_crt` setting.
- **No default features:** `llama-cpp-2` is used with `default-features = false`, which drops OpenMP (`vcomp140.dll`). Don't re-enable its default features.
- **Rebuilding llama.cpp:** after changing any of the above, run `cargo clean -p llama-cpp-sys-2`. Its build script doesn't watch these variables, and a mixed `/MD`/`/MT` build fails at link time with `__imp_*` unresolved symbols.
- **Verifying:** check the result with `dumpbin /dependents target/debug/last-light.exe`. It should list only Windows system DLLs.

## Architecture

- **App shell (`src/main.rs`):** `LastLightApp` holds `Vec<Box<dyn MiniApp>>` and `open: Option<usize>`. `None` shows the card grid. `Some(i)` renders that app, and the header shows a Back button. Cards are hand-painted (painter + galleys rather than label widgets) so their text can't steal clicks from the card. Their width stretches so each row fills the window. Apps with `is_ready() == false` (placeholders) get dimmed cards with a "Soon" badge.
- **Theme (`src/theme.rs`):** one dark look for the whole app, warm charcoal with a single amber accent. `apply()` installs it at startup. Use its color constants (`BG`, `PANEL`, `RAISED`, `TEXT*`, `ACCENT`, ...) and helpers (`lamp`, `section_label`, `quiet_button`, `primary_button`, `segmented`) instead of egui's default colors or hard-coded `Color32`s. The full style guide is in the `egui-ui` skill.
- **Mini apps (`src/apps/`):** every app implements the `MiniApp` trait (`name`, `description`, `ui`, plus optional `on_close`, which the shell calls on Back, and `is_ready`, false only for placeholders). Apps are registered in `apps::all()`. Unbuilt apps live in `placeholders()`. Each real app also has a test-only `demo()` constructor, registered in `apps::demo()`, which feeds the screenshot test. Shared widgets live in `src/apps/widgets.rs`: `section_header` (sidebar heading with "+"), `list_row` (a sidebar row with a truncated title, a "..." menu and inline rename, laid out so it can never widen its panel) and `inline_input`.
- **Storage (`src/storage.rs`):** all mini apps share one SQLite database, `last-light.db`, in the per-user data folder (`%APPDATA%\Last Light`). `rusqlite` uses the `bundled` feature, so SQLite is compiled into the exe. Each app creates its own tables with `CREATE TABLE IF NOT EXISTS`.
- **AI Chat (`src/apps/ai_chat/`):**
  - **`mod.rs` (UI state):** the chat side panel and messages.
    - **Chat saving:** a new chat (`current: None`) is only saved when its first message is sent, using the truncated message as a placeholder title.
    - **One reply at a time:** a single reply streams at once (`streaming`). Its text is saved to the DB on `ReplyDone`, unless the chat was deleted in the meantime.
    - **Error handling:** DB errors go through `check()`, which shows them as a notice in the UI.
  - **`store.rs` (SQLite):** the `chats` and `chat_messages` tables, with messages deleted along with their chat.
    - **Titles:** `auto_titled` stays 1 until the user renames a chat, and `set_auto_title` only overwrites titles while it's 1. That keeps an AI title arriving late from clobbering a rename.
    - **Fallback:** if the on-disk DB can't be opened, it falls back to an in-memory DB and shows a warning.
  - **`worker.rs` (model thread):** a thread owns the `LlamaBackend` and `LlamaModel`. It is spawned lazily on the first `ui` call and processes `Request::Reply` and `Request::Title` in order. It streams `Event`s back and calls `ctx.request_repaint()`.
    - **Full history:** every reply sends the full history (prefixed with `SYSTEM_PROMPT`) and builds a fresh `LlamaContext`.
    - **Trimming:** `fit_prompt` drops the oldest messages, never the system prompt or the latest message, until the prompt fits in `CONTEXT_SIZE - max_tokens`.
    - **Auto-titles:** after the first exchange, a separate short generation names the chat.
  - **No thinking:** the prompt has an empty `<think>\n\n</think>\n\n` block appended so Qwen3.5 answers without thinking.
  - **UTF-8 streaming:** token bytes are buffered until they form valid UTF-8, because tokens can split multi-byte characters.
  - **One backend per process:** llama's backend can only be initialized once per process, so the model-backed tests share one model through a `OnceLock`.
- **Text Editor (`src/apps/text_editor/`):** Markdown notes as plain files, not SQLite, so they stay readable without the app.
  - **`store.rs` (files):** notes live at `<data dir>/notes/<folder>/<note>.md`, exactly one folder level deep. Every path goes through `validate_name`, which applies the same portable file-name rules to folders and notes. Creating or renaming onto an existing name is an error (folders are never merged), except a case-only rename. Saves go through a hidden temp file plus a rename, so a crash can't leave a half-written note. `general` is created only when the notes folder itself is first created, so a deleted `general` doesn't come back.
  - **`mod.rs` (UI):** a toggleable sidebar (folders panel plus the selected folder's notes panel), a toolbar with Split, Edit and View modes, and the editor. The Markdown preview is `egui_commonmark` (default features off, so it never loads images or fetches anything). `show_mut` lets ticking a task checkbox in the preview edit the source.
  - **Saving:** the open note is one `OpenNote` that carries its own folder and name, so a save can never land in the wrong file. Autosave runs 500 ms after typing stops. Ctrl/Cmd+S saves immediately, and so do `on_close` (Back) and `Drop` (window close). Flush with `save()` before any rename or switch, since a pending save targets the old path.
- **egui 0.37 API:** most online examples use older APIs. Here, apps implement `fn ui(&mut self, ui: &mut egui::Ui, ...)` rather than `update(ctx)`. Panels are `egui::Panel::top/bottom(...).show(ui, ...)` (`TopBottomPanel` is gone), and `CentralPanel::show` takes `ui`.

## Assets and model

- **Bundling:** `build.rs` copies `assets/` next to the binary (`target/<profile>/assets`) and skips unchanged files.
- **Runtime lookup:** at runtime, assets are resolved relative to `current_exe()`.
- **Model file:** `assets/models/qwen3.5-0.8b-q8_0.gguf` is gitignored (`*.gguf`), so it must be present locally to run AI Chat or the ignored tests.

## Repo conventions

Commit directly on `main`; this is a solo, local repo, so don't create feature branches.

## Claude Code setup (`.claude/`)

- **Skills:**
  - `add-mini-app`: new mini apps.
  - `egui-ui`: **the visual style guide** (theme colors, helpers, panel and empty-state conventions) plus egui 0.37 APIs and the layout pitfalls already hit here; read it before writing or restyling any UI.
  - `local-llm`: model, prompt and llama-cpp-2 changes.
  - `verify-zero-install`: dependency check after adding native deps.
  - `release`: packaging.
- **Agents:**
  - `rust-reviewer`: read-only review of a diff against the project constraints.
  - `crate-api-researcher`: exact APIs for the locked crate versions, from registry source and Context7.
- **Hooks:**
  - After any edit to a `.rs` file, `cargo fmt` runs. A syntax error comes back as a hook error, so fix the code rather than fighting the formatter.
  - On session start, a check flags a missing model file or a missing `.mcp.json`.
- **MCP:** Context7 (docs lookup) is configured in `.mcp.json`. That file and `.claude/settings.local.json` are gitignored because they may hold tokens. `.mcp.example.json` is the committed, token-free copy, so update it whenever you change `.mcp.json`.
