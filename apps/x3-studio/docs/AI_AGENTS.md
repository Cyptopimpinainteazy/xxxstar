# AI Agents

## Status: ✅ FULL

### Configurable Providers
- **Ollama** — `http://localhost:11434`
- **LM Studio** — Custom endpoint
- **OpenAI-compatible** — Any OpenAI API
- **Anthropic-compatible** — Claude API

### Agent Modes
| Mode | Description |
|------|-------------|
| Architect | System design and architecture |
| Builder | Feature implementation |
| Auditor | Code review and verification |
| Security Reviewer | Security audit |
| x3-lang Specialist | X3 language development |
| Cross-VM Adapter Specialist | Adapter implementation |
| Relayer Specialist | Relayer configuration and ops |
| Validator Specialist | Validator operations |
| Mainnet Gatekeeper | Launch readiness |

### Built-in Prompts
Located in `prompts/` directory. Each agent mode has a system prompt that includes:
- Role definition
- Hard rules against fake completion
- Workspace context injection
- Mode-specific instructions

### API Chat Endpoint
The AI panel sends requests to the configured provider using the OpenAI-compatible chat completions format.
