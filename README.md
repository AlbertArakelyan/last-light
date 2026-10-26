# Last Light

An offline survival toolkit for the desktop. When the internet is gone and all that's left is a laptop, Last Light gives you a handful of dependable mini apps that run entirely on that machine. They include a text editor and an AI chat powered by a small language model running locally.

Built with Rust and [egui](https://github.com/emilk/egui). The AI runs on [llama.cpp](https://github.com/ggml-org/llama.cpp) through the [`llama-cpp-2`](https://crates.io/crates/llama-cpp-2) bindings. See [`SPEC.md`](SPEC.md) for the product concept.

## For users

Nothing to install. Download the app folder and run it:

```
last-light(.exe)
assets/
  models/
    qwen3.5-0.8b-q8_0.gguf
```

Keep `assets/` next to the executable. Chats and other data are saved in your user data folder:

| OS | Data folder |
|---|---|
| Windows | `%APPDATA%\Last Light` |
| macOS | `~/Library/Application Support/Last Light` |
| Linux | `~/.local/share/Last Light` |

## For developers

Building from source compiles llama.cpp (C/C++) and SQLite into the binary. You need a Rust toolchain, a C/C++ compiler, CMake, and **libclang**, which is used to generate the Rust bindings to llama.cpp. These are needed only to build; the finished app needs none of them.

### 1. Install the prerequisites

<details open>
<summary><strong>Windows</strong></summary>

1. **Visual Studio 2022 or newer** (Community is fine) with the **"Desktop development with C++"** workload. This provides MSVC, the Windows SDK and CMake.
2. **Rust** via [rustup](https://rustup.rs) with the default `x86_64-pc-windows-msvc` toolchain.
3. **LLVM** for libclang:
   ```powershell
   scoop install llvm
   # or: winget install LLVM.LLVM
   ```
4. Point the build at libclang, and at CMake if `cmake` isn't on your `PATH`. Then open a new terminal.
   ```powershell
   # Scoop install path; for winget use "C:\Program Files\LLVM\bin"
   [Environment]::SetEnvironmentVariable('LIBCLANG_PATH', "$env:USERPROFILE\scoop\apps\llvm\current\bin", 'User')
   # Visual Studio's bundled CMake (adjust "18\Community" to your VS version/edition)
   [Environment]::SetEnvironmentVariable('CMAKE', 'C:\Program Files\Microsoft Visual Studio\18\Community\Common7\IDE\CommonExtensions\Microsoft\CMake\CMake\bin\cmake.exe', 'User')
   ```
   You can skip `CMAKE` if you install CMake separately and it's on `PATH` (`winget install Kitware.CMake`).

</details>

<details open>
<summary><strong>macOS</strong></summary>

1. **Xcode Command Line Tools**. These provide clang, libclang and the macOS SDK:
   ```sh
   xcode-select --install
   ```
2. **CMake**:
   ```sh
   brew install cmake
   ```
3. **Rust** via [rustup](https://rustup.rs):
   ```sh
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

The build normally finds libclang in the Command Line Tools automatically. If it doesn't, set `LIBCLANG_PATH="$(xcode-select -p)/usr/lib"`.

</details>

<details open>
<summary><strong>Linux (Ubuntu / Debian)</strong></summary>

1. **Compiler and CMake**:
   ```sh
   sudo apt update
   sudo apt install build-essential cmake pkg-config
   ```
2. **LLVM (clang and libclang)**. On Ubuntu, LLVM comes as the `clang` and `libclang-dev` packages, which install Ubuntu's default LLVM version:
   ```sh
   sudo apt install clang libclang-dev
   ```
   The build normally finds libclang automatically. If it doesn't (`Unable to find libclang`), point it at the LLVM `lib` folder, replacing `18` with your installed version (`ls /usr/lib/llvm-*`):
   ```sh
   export LIBCLANG_PATH=/usr/lib/llvm-18/lib
   ```
3. **Libraries for the egui window** (X11/Wayland, keyboard, OpenGL):
   ```sh
   sudo apt install libxkbcommon-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev \
     libwayland-dev libgl1-mesa-dev
   ```
4. **Rust** via [rustup](https://rustup.rs). Distro packages of Rust are usually too old:
   ```sh
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

</details>

**Rust version:** 1.95 or newer is required, because eframe 0.37 needs it. Run `rustup update` if your toolchain is older.

### 2. Get the model

The AI model isn't stored in git because of its size (774 MB). The rest of the app works without it; only AI Chat needs it.

1. Download [`Qwen3.5 0.8B Q8_0 (2026)[Romarchive].gguf`](https://huggingface.co/Romarchive/Qwen3.5-0.8B-GGUF/blob/main/Qwen3.5%200.8B%20Q8_0%20(2026)%5BRomarchive%5D.gguf) from Hugging Face (the download button is on that page).
2. Put it in the `assets/models/` folder (create the folder if it doesn't exist).
3. Rename it to `qwen3.5-0.8b-q8_0.gguf`. This is the file name the app looks for.

The result should be `assets/models/qwen3.5-0.8b-q8_0.gguf`.

Or do all three steps from a terminal:

```sh
mkdir -p assets/models
curl -L -o assets/models/qwen3.5-0.8b-q8_0.gguf \
  "https://huggingface.co/Romarchive/Qwen3.5-0.8B-GGUF/resolve/main/Qwen3.5%200.8B%20Q8_0%20(2026)%5BRomarchive%5D.gguf"
```

On Windows, `curl.exe` ships with Windows 10 and later; in PowerShell call it as `curl.exe` so you don't get the `Invoke-WebRequest` alias.

> [!NOTE]
> You can download and use any model for development, for example a smaller one to save disk space or a bigger one to try out. The model must be a GGUF that llama.cpp supports and has a chat template. Either save it under the name above, or point `MODEL_FILE` in `src/apps/ai_chat/worker.rs` at your file. The `<think>` prefill in `worker.rs` is meant for Qwen-style models, and other models may need prompt tweaks; see `.claude/skills/local-llm/SKILL.md`. Don't commit a changed `MODEL_FILE` unless the shipped model is changing too.

### 3. Build and run

```sh
cargo run                  # debug build + launch
cargo build --release      # optimized build in target/release/
```

The first build takes a few minutes while llama.cpp compiles. Later builds take seconds.

`build.rs` copies `assets/` next to the binary (`target/<profile>/assets/`) on every build, so the model is found at runtime.

### Tests

```sh
cargo test                                   # fast unit tests
cargo test -- --include-ignored --nocapture  # also tests that load the model
cargo test remembers_earlier_messages -- --ignored --nocapture   # a single test
```

### Claude Code

The repo includes a shared Claude Code setup in `.claude/`, with skills, agents, hooks and permissions. See the "Claude Code setup" section of [`CLAUDE.md`](CLAUDE.md). To enable the Context7 docs MCP, copy the example config:

```sh
cp .mcp.example.json .mcp.json
```

`.mcp.json` and `.claude/settings.local.json` are gitignored. Put API keys and personal overrides there, never in the committed files.

### Packaging a release

Copy the release binary and the `assets/` folder from `target/release/` together:

- **Windows:** the C runtime is linked statically (see `.cargo/config.toml` and `cmake/static-crt.cmake`). The `.exe` depends only on DLLs that ship with Windows, so no VC++ Redistributable is needed. You can verify this with `dumpbin /dependents last-light.exe`.
- **macOS:** the binary uses only system frameworks.
- **Linux:** the binary uses the system's graphics and windowing libraries (OpenGL, X11/Wayland, xkbcommon), which every desktop install already has.

## Troubleshooting

- **`Unable to find libclang`:** libclang isn't installed or `LIBCLANG_PATH` doesn't point at the folder containing `libclang.dll`, `libclang.dylib` or `libclang.so`.
- **`is cmake not installed?` / CMake not found:** install CMake or set the `CMAKE` environment variable to its full path.
- **Windows: "failed to remove file ... last-light.exe":** the app is still running. Close it and build again.
- **Windows: unresolved `__imp_*` symbols when linking:** llama.cpp was built with the wrong C runtime, usually after changing `.cargo/config.toml`. Run `cargo clean -p llama-cpp-sys-2` and rebuild.
- **AI Chat says "Model not found":** the model file is missing from `assets/models/` or is named differently. See [Get the model](#2-get-the-model).
