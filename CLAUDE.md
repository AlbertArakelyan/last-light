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
```

- **Locked exe:** if `cargo build` fails with "failed to remove file ... last-light.exe", the app is still running. Close it, or use `cargo check`.
- **First build is slow:** about 2 minutes, because `llama-cpp-sys-2` compiles llama.cpp from source. It needs MSVC, CMake and libclang. `LIBCLANG_PATH` (LLVM `bin`, installed via `scoop install llvm`) and `CMAKE` (Visual Studio's `cmake.exe`) must be set in the environment.

## Hard constraint: zero install for end users

The shipped app is `last-light.exe` plus its `assets/` folder. It must run with nothing installed: no VC++ redistributable, no OpenMP and no network access.
- **Static C runtime:** `.cargo/config.toml` adds `+crt-static` for Rust. It also sets `CMAKE_TOOLCHAIN_FILE_x86_64_pc_windows_msvc` (target-suffixed, so it applies to Windows only) to `cmake/static-crt.cmake`, which forces `/MT` for llama.cpp, because llama.cpp's CMake ignores cmake-rs's `static_crt` setting.
- **No default features:** `llama-cpp-2` is used with `default-features = false`, which drops OpenMP (`vcomp140.dll`). Don't re-enable its default features.
- **Rebuilding llama.cpp:** after changing any of the above, run `cargo clean -p llama-cpp-sys-2`. Its build script doesn't watch these variables, and a mixed `/MD`/`/MT` build fails at link time with `__imp_*` unresolved symbols.
- **Verifying:** check the result with `dumpbin /dependents target/debug/last-light.exe`. It should list only Windows system DLLs.

## Architecture

- **App shell (`src/main.rs`):** `LastLightApp` holds `Vec<Box<dyn MiniApp>>` and `open: Option<usize>`. `None` shows the card grid. `Some(i)` renders that app, and the header shows a Back button. Cards are hand-painted (painter + galleys rather than label widgets) so their text can't steal clicks from the card.
- **Mini apps (`src/apps/`):** every app implements the `MiniApp` trait (`name`, `description`, `ui`) and is registered in `apps::all()`. Unbuilt apps use `Placeholder`.
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
- **egui 0.37 API:** most online examples use older APIs. Here, apps implement `fn ui(&mut self, ui: &mut egui::Ui, ...)` rather than `update(ctx)`. Panels are `egui::Panel::top/bottom(...).show(ui, ...)` (`TopBottomPanel` is gone), and `CentralPanel::show` takes `ui`.

## Assets and model

- **Bundling:** `build.rs` copies `assets/` next to the binary (`target/<profile>/assets`) and skips unchanged files.
- **Runtime lookup:** at runtime, assets are resolved relative to `current_exe()`.
- **Model file:** `assets/models/qwen3.5-0.8b-q8_0.gguf` is gitignored (`*.gguf`), so it must be present locally to run AI Chat or the ignored tests.

## Repo conventions

Commit directly on `main`; this is a solo, local repo, so don't create feature branches.
