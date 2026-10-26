---
name: release
description: Build and package a Last Light release folder (binary plus assets including the model) for the current OS, verify it needs nothing installed, and zip it. Use when the user asks to package, ship, bundle, or make a release or distributable build.
---

# Package a release

The distributable is a folder holding the release binary and `assets/` (with the model). Releases are built per OS on that OS; there is no cross-compiling.

## Steps

1. **Pre-flight.** Make sure the working tree is committed (`git status`) and the tests pass:
   ```sh
   cargo test
   cargo test -- --ignored        # model-backed tests; needs the model file
   cargo clippy
   ```
2. **Build:**
   ```sh
   cargo build --release
   ```
   `build.rs` copies `assets/` to `target/release/assets/`. Confirm `target/release/assets/models/qwen3.5-0.8b-q8_0.gguf` exists.
3. **Verify zero install** with the `verify-zero-install` skill. Don't package a build that fails it.
4. **Assemble** `dist/last-light-<version>-<os>-<arch>/`, taking the version from `Cargo.toml`:
   ```
   last-light(.exe)
   assets/
   ```
   Copy only the binary and `assets/`, never the rest of `target/release/` (`.d`, `.pdb`, `deps/`, `build/`).
5. **Zip it** as `dist/last-light-<version>-<os>-<arch>.zip`. On Windows use PowerShell `Compress-Archive`; on macOS and Linux use `zip -r`.
6. **Report** the zip path, its size (the model is about 774 MB), and the dependency check result.

## Notes

- `dist/` is a build output. Make sure it's in `.gitignore` before creating it.
- **macOS:** the binary is unsigned, so Gatekeeper will warn users. Tell the user. Signing and notarizing need an Apple Developer account and are out of scope unless asked.
- **Version bumps:** don't change the version in `Cargo.toml` unless the user asks.
