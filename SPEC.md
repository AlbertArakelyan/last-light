# Last Light — Spec

## Concept

Last Light is a post-apocalyptic survival toolkit for the desktop. The scenario: the internet is gone, the cloud is gone, and all that's left is a laptop. Last Light bundles a set of simple, dependable mini apps that run **fully offline** on that one machine.

## Principles

- **Offline first, offline only.** No feature may need a network connection. Nothing phones home, checks for updates or loads remote assets.
- **Self-contained.** One native binary (Rust + egui). Data and models live on local disk.
- **Lightweight.** It should run well on an old or low-power laptop, where battery may be scarce.
- **Simple and reliable.** Each mini app does one job well. Plain file formats are preferred so data stays readable without the app.

## Layout

- **Header:** "Last Light" title at the top, aligned left.
- **Main screen:** a grid of **cards**, one per mini app. Each card shows the app's name, a short description and (later) an icon. Clicking a card opens that app.
- **Inside an app:** the app takes over the main area, and there is a way to get back to the card grid (for example a back button in the header).

## Mini apps

### Planned

| App | Description |
|---|---|
| **Text Editor** | Create, open, edit and save plain-text notes on local disk. |
| **AI Chat** | Chat with a lightweight language model that runs locally. The model file is downloaded ahead of time and loaded from disk, with no network calls. |

### Ideas (not committed)

- Notes / journal with dates
- Calculator and unit converter
- Offline reference library (e.g. survival guides, first aid)
- Inventory / supplies tracker
- Calendar and day counter
- Map viewer for pre-downloaded maps

## Architecture (initial)

- Each mini app implements a common trait (name, description, `ui` method), so adding an app means adding one module and registering it.
- The app shell keeps the current screen: either the card grid or one open mini app.
- App data is stored under a local data directory.

## Building

End users install nothing: the exe statically links llama.cpp and the C runtime, and depends only on DLLs that ship with Windows. Ship `last-light.exe` together with its `assets/` folder.

Developers building from source need, once per machine:
- MSVC and CMake (both come with Visual Studio's C++ workload).
- LLVM for libclang (`scoop install llvm`), used to generate the llama.cpp bindings.
- Environment variables `LIBCLANG_PATH` (LLVM's `bin` folder) and `CMAKE` (path to `cmake.exe`) if CMake isn't on `PATH`.
- The model at `assets/models/qwen3.5-0.8b-q8_0.gguf` (not in git).

`.cargo/config.toml` and `cmake/static-crt.cmake` force the static runtime. If you change them, run `cargo clean -p llama-cpp-sys-2` so llama.cpp is rebuilt.

## Open questions

- Which local model and inference runtime the AI Chat will use (e.g. a GGUF model via llama.cpp bindings).
- Where app data and models are stored (next to the binary vs. the user data directory).
- Visual theme (e.g. a dark, worn "terminal" look to fit the setting).
