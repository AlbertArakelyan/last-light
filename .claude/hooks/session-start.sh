#!/usr/bin/env bash
# SessionStart hook: anything printed here is added to Claude's context.
# Flags local setup gaps that would otherwise surface as confusing errors.

cd "$CLAUDE_PROJECT_DIR" || exit 0

if [ ! -f assets/models/qwen3.5-0.8b-q8_0.gguf ]; then
  echo "Note: assets/models/qwen3.5-0.8b-q8_0.gguf is missing. AI Chat and the model-backed (ignored) tests need it; see README.md \"Get the model\"."
fi

if [ ! -f .mcp.json ] && [ -f .mcp.example.json ]; then
  echo "Note: .mcp.json is missing; copy .mcp.example.json to .mcp.json to enable the Context7 docs MCP."
fi
exit 0
