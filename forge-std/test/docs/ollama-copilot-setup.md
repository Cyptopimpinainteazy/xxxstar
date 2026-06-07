# Ollama + GitHub Copilot LLM Gateway — Setup Guide

This file documents how to connect Copilot LLM Gateway to a local Ollama server and basic troubleshooting.

Prerequisites
- Ollama installed and running locally (default port `11434`).
- GitHub Copilot extension and GitHub Copilot LLM Gateway extension installed in VS Code.

Start Ollama
Run Ollama as you normally do (example):

```bash
ollama serve
```

Verify Ollama is responding (lists models):

```bash
curl http://127.0.0.1:11434/v1/models
```

Ollama-specific notes for Copilot LLM Gateway
- Default HTTP API: Ollama commonly exposes an HTTP API on port `11434` (host `127.0.0.1`). Use that base URL as the gateway `serverUrl` (for example `http://127.0.0.1:11434`).
- If you are using Ollama Desktop, enable the HTTP/API option in the app settings so extensions can connect.
- The gateway expects an OpenAI-compatible inference endpoint (e.g., `/v1/models` for model listing and chat/completions-compatible endpoints). If your Ollama distribution exposes a different path, point `github.copilot.llm-gateway.serverUrl` to the base URL that implements the OpenAI-compatible surface.
- Tip: verify which models are available with `curl http://127.0.0.1:11434/v1/models` and then enable those models in the Copilot model manager.


Workspace settings
- A workspace settings file has been added at `.vscode/settings.json` that points the gateway to Ollama (`http://127.0.0.1:11434`) and enables tool calling.

If you need to edit settings manually, open VS Code Settings JSON and set the following keys:

```json
{
  "github.copilot.llm-gateway.serverUrl": "http://127.0.0.1:11434",
  "github.copilot.llm-gateway.apiKey": "",
  "github.copilot.llm-gateway.enableToolCalling": true,
  "github.copilot.llm-gateway.parallelToolCalling": true,
  "github.copilot.llm-gateway.agentTemperature": 0.0,
  "github.copilot.llm-gateway.defaultMaxTokens": 32768,
  "github.copilot.llm-gateway.defaultMaxOutputTokens": 4096
}
```

Test connection
- From terminal:

```bash
curl http://127.0.0.1:11434/v1/models
```
- In VS Code: open Command Palette and run `GitHub Copilot LLM Gateway: Test Server Connection`.

Select a model in Copilot Chat
- Open Copilot Chat (Ctrl+Alt+I). Use the model selector → Manage Models → choose `LLM Gateway` provider and enable the models reported by your Ollama server.

Pre-enabled models
- The workspace settings now include the models discovered on your local Ollama server and request they be auto-enabled:

```json
{
  "github.copilot.llm-gateway.enabledModels": ["codellama:7b","qwen2.5-coder:7b","llama3.2:1b"],
  "github.copilot.llm-gateway.autoEnableModels": true
}
```

Note: The extension may ignore unknown settings keys; if so, open the Copilot Chat model manager and enable the models manually.

Run the VS Code connection test
- Open Command Palette (Ctrl+Shift+P) → `GitHub Copilot LLM Gateway: Test Server Connection`.
- If it succeeds, open the Copilot Chat model selector and choose one of the `LLM Gateway` models from the dropdown.

Troubleshooting
- If models appear but the agent prints tool descriptions instead of calling tools:
  - Set `agentTemperature` to `0.0`.
  - Disable `parallelToolCalling` temporarily.
  - Ensure Ollama exposes an OpenAI-compatible API at the `serverUrl`.
- If Ollama returns OOM or out-of-memory errors: use a smaller or quantized model, reduce `defaultMaxTokens`, or run a quantized build.

YOLO Mode (aggressive/tool-forward)
- Description: "YOLO mode" enables more creative and aggressive agent behavior and allows parallel tool use. This is useful for rapid experimentation but increases the chance of unexpected tool actions — use only in trusted environments.
- Recommended settings (workspace `/.vscode/settings.json`):

```json
{
  "github.copilot.llm-gateway.enableToolCalling": true,
  "github.copilot.llm-gateway.parallelToolCalling": true,
  "github.copilot.llm-gateway.agentTemperature": 0.9
}
```

- Warning: Do not enable YOLO mode on production workspaces or on projects with sensitive data or automated deployments. Review tool call logs and restrict file/terminal permissions when experimenting.

Make it part of the Copilot UI
- In Copilot Chat open the model selector → Manage Models → choose `LLM Gateway` provider and enable your model(s). The gateway-enabled models will appear in the Copilot model dropdown so you can select them like any other provider.

Next steps
- Run the `curl` test and the VS Code connection test.
- If you want, I can verify models reported by your Ollama server (I cannot access your host — run the `curl` command locally and paste the output here).
