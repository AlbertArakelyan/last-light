---
name: verify-zero-install
description: Check that the built Last Light binary still runs with nothing installed, with no VC++ redistributable, OpenMP or other non-system shared libraries. Use after adding or upgrading a dependency (especially one with C/C++ code), changing .cargo/config.toml or cmake/, or before a release.
---

# Verify zero install

End users get the binary plus `assets/` and nothing else. Every library the binary loads must ship with the OS.

## 1. Build

```sh
cargo build --release
```

If you changed `.cargo/config.toml`, `cmake/static-crt.cmake` or llama-cpp-2 features, first run `cargo clean -p llama-cpp-sys-2`. Its build script doesn't watch those settings, so a stale llama.cpp build would hide the problem.

## 2. List dynamic dependencies

**Windows:** `dumpbin` ships with Visual Studio. Find it with `vswhere`, or use the path under `VC\Tools\MSVC\<ver>\bin\Hostx64\x64\`.
```sh
dumpbin //dependents target/release/last-light.exe     # Git Bash needs // for the flag
```
- Allowed: Windows system DLLs, such as `kernel32`, `user32`, `gdi32`, `shell32`, `advapi32`, `ole32`, `oleaut32`, `combase`, `opengl32`, `dwmapi`, `dxgi`, `dwrite`, `uxtheme`, `imm32`, `setupapi`, `shlwapi`, `uiautomationcore`, `bcryptprimitives`, `ntdll` and `api-ms-win-*`.
- **Not allowed:** `vcruntime*.dll`, `msvcp*.dll`, `vcomp*.dll`, `ucrtbased.dll`, or any DLL that isn't part of Windows.

**macOS:**
```sh
otool -L target/release/last-light
```
Only `/usr/lib/*` and `/System/Library/Frameworks/*` are allowed.

**Linux:**
```sh
ldd target/release/last-light
```
Only libc and the libraries every desktop already has are allowed: libm, libgcc_s, libstdc++, OpenGL/EGL, X11/xcb, wayland and xkbcommon. Flag anything like `libgomp` or `libssl`.

## 3. If something extra shows up

- **`vcruntime`/`msvcp` (Windows):** something compiled C/C++ with the DLL runtime (`/MD`).
  - Rust code and `cc`-built crates follow `+crt-static` from `.cargo/config.toml`.
  - CMake-built crates need `CMAKE_MSVC_RUNTIME_LIBRARY=MultiThreaded`, which `cmake/static-crt.cmake` provides through `CMAKE_TOOLCHAIN_FILE_x86_64_pc_windows_msvc`.
  - Link errors with unresolved `__imp_*` symbols mean the runtimes are mixed. Clean the offending crate and rebuild.
- **`vcomp`/`libgomp`:** OpenMP is enabled. Keep `llama-cpp-2` at `default-features = false`.
- **Any other library:** find which crate pulls it in with `cargo tree -i <crate>`. Then switch to a pure-Rust alternative or the crate's bundled/static feature.

Report the full dependency list to the user, and state whether it passes.
