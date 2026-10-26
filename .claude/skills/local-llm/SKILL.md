---
name: local-llm
description: How Last Light runs its local LLM (llama-cpp-2 / llama.cpp with a Qwen3.5 GGUF), and how to change the model, prompt, sampling, context size or streaming safely. Use when working on AI Chat inference, swapping or upgrading the model, debugging odd model output, or upgrading llama-cpp-2.
---

# Local LLM in Last Light

All inference lives in `src/apps/ai_chat/worker.rs`. A single background thread owns the `LlamaBackend` and `LlamaModel`, and processes `Request`s in order (`Reply` and `Title`). It streams `Event`s back over mpsc and calls `ctx.request_repaint()` after each event.

## Invariants

- **One backend per process.** `LlamaBackend::init()` fails if called twice in one process. Tests share a single model through a `OnceLock` (`test_model()`). Never init a second backend.
- **Fresh context per request.** Each request creates a new `LlamaContext` and re-prefills the whole prompt. Qwen3.5 is a hybrid (partly recurrent) architecture, so KV-cache prefix reuse isn't straightforward. Measure before "optimizing" this.
- **No thinking.** Qwen3.5 emits a `<think>` block. `fit_prompt` appends an empty `<think>\n\n</think>\n\n` after the chat template so the model answers directly. Keep it when changing prompts; drop it only for a model without reasoning tags.
- **History trimming.** `fit_prompt` drops the oldest messages (never the system prompt or the latest message) until the prompt fits `CONTEXT_SIZE - max_tokens`. Trimmed messages stay in SQLite; only the model's view shrinks.
- **UTF-8 streaming.** Tokens can split multi-byte characters. Bytes are buffered and only the valid UTF-8 prefix is emitted. Don't convert each token to a `String` on its own.
- **Chat template.** `model.chat_template(None)` plus `apply_chat_template` uses llama.cpp's built-in template matching (not Jinja). A new model whose template llama.cpp doesn't recognize will fail here.

## Changing the model

1. Prefer Q8_0 for small (≤1B) models; lower quants hurt small models a lot. Use Q4_K_M or Q5_K_M only for 3B and larger.
2. Put the file in `assets/models/` with a simple lowercase name, and update `MODEL_FILE` in `worker.rs`.
3. Update the model name and download command in `README.md`, `CLAUDE.md` and `SPEC.md`. `.gguf` files are gitignored.
4. Run the model-backed tests and read the printed output:
   ```sh
   cargo test -- --ignored --nocapture
   ```
   `remembers_earlier_messages` checks context, and `names_a_chat` checks auto-titling.

## Tuning knobs (worker.rs)

- **Context and lengths:** `CONTEXT_SIZE` (8192), `MAX_REPLY_TOKENS` (1024) and `MAX_TITLE_TOKENS` (24).
- **Sampler chain:** top_k 20, top_p 0.8, temp 0.7, which is Qwen's recommended non-thinking setting.
- **System prompt:** `SYSTEM_PROMPT`.

## Upgrading llama-cpp-2

- **Build requirements:** it builds llama.cpp from source and needs libclang, CMake and a C++ compiler (see README).
- **Features:** keep `default-features = false`. The defaults enable OpenMP, which adds a runtime DLL dependency on Windows.
- **Verification:** after upgrading, run `cargo clean -p llama-cpp-sys-2`, rebuild, run the model tests, then run the `verify-zero-install` skill.
- **API changes:** check the crate source at `~/.cargo/registry/src/*/llama-cpp-2-<version>/src/` or Context7. The API changes between versions.
