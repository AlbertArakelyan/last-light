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
cargo test replies_to_a_prompt -- --ignored --nocapture   # run one test by name
```

- **Locked exe:** if `cargo build` fails with "failed to remove file ... last-light.exe", the app is still running. Close it, or use `cargo check`.
- **First build is slow:** about 2 minutes, because `llama-cpp-sys-2` compiles llama.cpp from source. It needs MSVC, CMake and libclang. `LIBCLANG_PATH` (LLVM `bin`, installed via `scoop install llvm`) and `CMAKE` (Visual Studio's `cmake.exe`) must be set in the environment.

## Hard constraint: zero install for end users

The shipped app is `last-light.exe` plus its `assets/` folder. It must run with nothing installed: no VC++ redistributable, no OpenMP and no network access.
- **Static C runtime:** `.cargo/config.toml` adds `+crt-static` for Rust. It also points `CMAKE_TOOLCHAIN_FILE` at `cmake/static-crt.cmake`, which forces `/MT` for llama.cpp, because llama.cpp's CMake ignores cmake-rs's `static_crt` setting.
- **No default features:** `llama-cpp-2` is used with `default-features = false`, which drops OpenMP (`vcomp140.dll`). Don't re-enable its default features.
- **Rebuilding llama.cpp:** after changing any of the above, run `cargo clean -p llama-cpp-sys-2`. Its build script doesn't watch these variables, and a mixed `/MD`/`/MT` build fails at link time with `__imp_*` unresolved symbols.
- **Verifying:** check the result with `dumpbin /dependents target/debug/last-light.exe`. It should list only Windows system DLLs.

## Architecture

- **App shell (`src/main.rs`):** `LastLightApp` holds `Vec<Box<dyn MiniApp>>` and `open: Option<usize>`. `None` shows the card grid. `Some(i)` renders that app, and the header shows a Back button. Cards are hand-painted (painter + galleys rather than label widgets) so their text can't steal clicks from the card.
- **Mini apps (`src/apps/`):** every app implements the `MiniApp` trait (`name`, `description`, `ui`) and is registered in `apps::all()`. Unbuilt apps use `Placeholder`.
- **AI Chat (`src/apps/ai_chat.rs`):**
  - **Worker thread:** a thread owns the `LlamaBackend` and `LlamaModel`. It is spawned lazily on the first `ui` call. The UI sends prompts over an mpsc channel. The worker replies with `Event`s (`Ready`/`Token`/`Done`/`Error`) and calls `ctx.request_repaint()`. The UI drains events in `poll_events` each frame.
  - **Per-message context:** each prompt currently builds a fresh `LlamaContext`, so there's no conversation history yet. Context, history and a database are the planned next steps.
  - **No thinking:** the prompt has an empty `<think>\n\n</think>\n\n` block appended so Qwen3.5 answers without thinking.
  - **UTF-8 streaming:** token bytes are buffered until they form valid UTF-8, because tokens can split multi-byte characters.
- **egui 0.37 API:** most online examples use older APIs. Here, apps implement `fn ui(&mut self, ui: &mut egui::Ui, ...)` rather than `update(ctx)`. Panels are `egui::Panel::top/bottom(...).show(ui, ...)` (`TopBottomPanel` is gone), and `CentralPanel::show` takes `ui`.

## Assets and model

- **Bundling:** `build.rs` copies `assets/` next to the binary (`target/<profile>/assets`) and skips unchanged files.
- **Runtime lookup:** at runtime, assets are resolved relative to `current_exe()`.
- **Model file:** `assets/models/qwen3.5-0.8b-q8_0.gguf` is gitignored (`*.gguf`), so it must be present locally to run AI Chat or the ignored tests.

## Repo conventions

Commit directly on `main`; this is a solo, local repo, so don't create feature branches.
