# X3 AI Command System

An X3-specialized omnichain engineering and trading model pack for Ollama, Cline, and local AI development.

## What This Is

CryptoMaster is an X3-specialized omnichain engineering and trading model pack built on open coding models and customized with X3-specific Modelfiles, system behavior, role specialization, safety rules, and production workflows.

It supports:
- X3 Chain architecture
- X3-Lang / X3VM
- EVM / Solidity
- SVM / Solana / Anchor
- Substrate / Rust runtime
- BTC UTXO / Taproot / PSBT
- CosmWasm / IBC
- Cross-VM atomic routing
- Canonical asset accounting
- Arbitrage architecture
- Flashloan route design
- Route scoring and finality risk
- MEV defense
- PnL / risk modeling
- Production DevOps
- Testing / fuzzing / invariant review

## The 20 Specialist Models

| Model | Role | Temperature |
|---|---|---|
| `lojak/cryptomaster` | Omnichain architect / final judge | 0.15 |
| `lojak/x3-auditor` | Security / mainnet-readiness reviewer | 0.05 |
| `lojak/x3-rust-runtime` | Substrate / Rust / X3VM runtime | 0.12 |
| `lojak/x3-solidity-guard` | EVM / Solidity security | 0.08 |
| `lojak/x3-svm-guard` | Solana / SVM / Anchor | 0.08 |
| `lojak/x3-cosmwasm-guard` | Cosmos / CosmWasm / IBC | 0.08 |
| `lojak/x3-btc-guard` | BTC / UTXO / Taproot / PSBT | 0.08 |
| `lojak/x3-arb-king` | Arbitrage / trading architect | 0.12 |
| `lojak/x3-flashloan-executor` | Flashloan route builder | 0.08 |
| `lojak/x3-route-oracle` | Route scoring / finality risk | 0.05 |
| `lojak/x3-quant-risk` | PnL / risk / volatility modeling | 0.05 |
| `lojak/x3-trade-ops` | Live trading infrastructure | 0.10 |
| `lojak/x3-mev-defense` | MEV protection / anti-sandwich | 0.05 |
| `lojak/x3-data-engineer` | Indexers / ETL / price feeds | 0.10 |
| `lojak/x3-devops-commander` | Infrastructure / Docker / secrets | 0.10 |
| `lojak/x3-testsmith` | Testing / fuzzing / invariants | 0.10 |
| `lojak/x3-docsmith` | Documentation / model cards | 0.15 |
| `lojak/x3-compliance-ops` | Grants / audit / compliance | 0.10 |
| `lojak/x3-eval-judge` | Model quality scoring | 0.05 |
| `lojak/x3-cline-finisher` | Repo completion / TODO killer | 0.10 |

## Quick Start

### 1. Pull the base model

```bash
ollama pull qwen2.5-coder:14b
# For stronger hardware:
# ollama pull qwen2.5-coder:32b
```

### 2. Build all 20 models

```bash
cd x3-ai-command-system
./build_all_models.sh
```

### 3. Smoke test

```bash
ollama run lojak/cryptomaster
ollama run lojak/x3-auditor
# ... test any model
```

### 4. Push to registry

```bash
ollama login
./push_all_models.sh
```

### 5. Use in Cline

| Setting | Value |
|---|---|
| Provider | `Ollama` |
| Base URL | `http://localhost:11434` |
| Context Window | `32768` |
| Model | One of the `lojak/...` models |

See `prompts/cline/starters.md` for task-specific prompts.

### 6. Use in Roo Code

1. Start the router:
   ```bash
   cd x3-ai-command-system/router
   ./x3_router.sh start
   ```

2. Copy the modes file to your project:
   ```bash
   cp x3-ai-command-system/roo-config/.roomodes /path/to/your/project/
   ```

3. In VS Code, open Roo Code settings:

   | Setting | Value |
   |---|---|
   | Provider | `OpenAI Compatible` |
   | Base URL | `http://localhost:11435/v1` |
   | API Key | (leave empty) |
   | Model | `lojak/cryptomaster` |
   | Context Window | `32768` |

4. Switch between X3 specialist modes in Roo Code's mode selector.

See `prompts/roo/starters.md` for task-specific prompts per mode.

## Directory Layout

```
x3-ai-command-system/
├── models/                          # 20 specialist Modelfiles
│   ├── cryptomaster/
│   ├── x3-auditor/
│   ├── x3-rust-runtime/
│   ├── x3-solidity-guard/
│   ├── x3-svm-guard/
│   ├── x3-cosmwasm-guard/
│   ├── x3-btc-guard/
│   ├── x3-arb-king/
│   ├── x3-flashloan-executor/
│   ├── x3-route-oracle/
│   ├── x3-quant-risk/
│   ├── x3-trade-ops/
│   ├── x3-mev-defense/
│   ├── x3-data-engineer/
│   ├── x3-devops-commander/
│   ├── x3-testsmith/
│   ├── x3-docsmith/
│   ├── x3-compliance-ops/
│   ├── x3-eval-judge/
│   └── x3-cline-finisher/
├── knowledge-core/                  # Shared X3 doctrine
├── rag/                             # RAG memory (future)
├── roo-config/                      # Roo Code configuration
│   └── .roomodes                    # 20 X3 specialist modes
├── router/                          # Model routing registry
│   ├── classifier.py                # Keyword classifier
│   ├── config.yaml                  # Router configuration
│   ├── model_registry.yaml          # 20 model definitions
│   ├── x3_router.py                 # HTTP proxy server
│   ├── x3_router.sh                 # Start/stop/test script
│   └── x3_router_test.py           # Test suite (32 cases)
├── evals/                           # Eval harness
│   ├── eval_cases.jsonl
│   ├── run_evals.sh
│   ├── score_output.py
│   └── reports/
├── fine-tune/                       # LoRA training pipeline
│   ├── train_x3_lora.py
│   └── data/
│       └── x3_sft_template.jsonl
├── prompts/                         # Agent-specific prompts
│   ├── cline/
│   ├── roo/
│   ├── codex/
│   ├── claude/
│   └── roo/
├── hooks/                           # Safety hooks (future)
├── safety/                          # Trading safety kernel
│   ├── TRADING_LIMITS.md
│   ├── SECURITY_BOUNDARIES.md
│   └── RELEASE_GATES.md
├── docs/                            # Documentation
│   ├── README.md (this file)
│   ├── MODEL_CARD.md
│   ├── EVALS.md
│   ├── CHANGELOG.md
│   └── LICENSE_NOTES.md
├── build_all_models.sh              # Build all 20 models
└── push_all_models.sh               # Push all models to registry
```

## Routing

Tasks are routed to the right model. See `router/model_registry.yaml` for the full routing table.

| Task | Primary Model | Reviewer |
|---|---|---|
| Architecture / planning | cryptomaster | auditor |
| Rust / Substrate | x3-rust-runtime | auditor |
| Solidity / EVM | x3-solidity-guard | auditor |
| Solana / SVM | x3-svm-guard | auditor |
| CosmWasm / IBC | x3-cosmwasm-guard | auditor |
| Bitcoin / UTXO | x3-btc-guard | route-oracle |
| Arbitrage strategy | x3-arb-king | quant-risk |
| Flashloan execution | x3-flashloan-executor | mev-defense |
| Route scoring | x3-route-oracle | quant-risk |
| Risk modeling | x3-quant-risk | route-oracle |
| Trading ops | x3-trade-ops | mev-defense |
| MEV defense | x3-mev-defense | auditor |
| Data pipelines | x3-data-engineer | devops-commander |
| Infrastructure | x3-devops-commander | auditor |
| Testing | x3-testsmith | auditor |
| Documentation | x3-docsmith | cryptomaster |
| Compliance | x3-compliance-ops | cryptomaster |
| Eval scoring | x3-eval-judge | cryptomaster |
| Repo completion | x3-cline-finisher | auditor |

## Fine-Tuning

Current version: **Modelfile-customized** (v0.1)

### Fine-Tuning Ladder

| Stage | What | Ready? |
|---|---|---|
| Modelfile | Role/personality/rules via system prompt | ✅ Now |
| RAG | Model + searchable X3 knowledgebase | ✅ Now (see knowledge-core/) |
| SFT LoRA | Train on X3 patterns/examples | Later — needs dataset |
| DPO/RLAIF | Preference tuning | Later |
| Eval harness | Prevent model regression | ✅ Now (see evals/) |

### LoRA Training

```bash
cd x3-ai-command-system/fine-tune
python3 -m venv .venv
source .venv/bin/activate
pip install -U torch transformers datasets accelerate trl peft bitsandbytes

# Prepare data in data/x3_sft.jsonl
# Train
python train_x3_lora.py

# For 14B:
MODEL_NAME=Qwen/Qwen2.5-Coder-14B-Instruct OUTPUT_DIR=outputs/x3-14b-lora python train_x3_lora.py
```

### Import LoRA Adapter into Ollama

```
FROM qwen2.5-coder:14b
ADAPTER /path/to/outputs/x3-coder-lora
PARAMETER temperature 0.1
PARAMETER num_ctx 32768
SYSTEM """You are CryptoMaster-Finetuned..."""
```

```bash
ollama create lojak/cryptomaster-ft -f Modelfile
```

## Evals

```bash
# Run evals against all models
cd x3-ai-command-system/evals
./run_evals.sh

# Run against specific model
./run_evals.sh lojak/cryptomaster

# Score results
python3 score_output.py --reports-dir reports/
```

Every model must score 4+ average with zero dangerous outputs before being called production-ready.

## Trading Safety

All trading models enforce the X3 Trading Safety Kernel. See `safety/TRADING_LIMITS.md`.

Key rules:
- No mainnet execution without dry-run mode
- No execution without simulation
- Max loss, max gas, max failed tx limits
- Circuit breakers and kill switches required
- PnL logging mandatory
- Profitable ≠ safe

## Security Boundaries

See `safety/SECURITY_BOUNDARIES.md`.

The model pack must never produce: theft tools, phishing, rug mechanics, malicious MEV, unauthorized exploits, DAO hijacking, deceptive contracts.

Allowed: defensive audits, testnet research, invariant testing, simulation, MEV defense, legal arbitrage, protocol-permitted liquidations.

## Proof Loop

```
Model writes patch
  → Tests run
  → Auditor reviews
  → Eval judge scores
  → Good output → accepted training data
  → Bad output → rejected training data
  → Fine-tune later on accepted data
```

## Auto-Routing (Model Router)

The X3 Router automatically sends your prompts to the right specialist model. Works with Cline, Roo Code, and any OpenAI-compatible client.

### Routing Priority

1. **X-X3-Model header** → forced model (no classification)
2. **Model passthrough** → if model is already `lojak/x3-*`, pass through directly (Roo Code modes)
3. **Keyword classification** → auto-route based on prompt content
4. **Fallback** → `lojak/cryptomaster`

### How It Works

```
Cline (configured once with lojak/cryptomaster)
  │
  ▼
X3 Router (port 11435)
  │  Classifies prompt by keywords
  │  Routes to best specialist model
  │
  ▼
Ollama (port 11434)
  └── lojak/x3-solidity-guard  (for Solidity questions)
  └── lojak/x3-auditor          (for audit questions)
  └── lojak/cryptomaster       (for everything else)
  └── ... (20 specialist models)
```

### Setup

```bash
# Start the router
cd x3-ai-command-system/router
./x3_router.sh start

# Check status
./x3_router.sh status

# Stop
./x3_router.sh stop
```

### Cline Configuration

| Setting | Value |
|---|---|
| Provider | `Ollama` |
| Base URL | `http://localhost:11435` |
| Model | `lojak/cryptomaster` (router overrides per-request) |
| Context Window | `32768` |

### Roo Code Configuration

| Setting | Value |
|---|---|
| Provider | `OpenAI Compatible` |
| Base URL | `http://localhost:11435/v1` |
| API Key | (leave empty) |
| Model | `lojak/cryptomaster` (auto-routed by keyword) |
| Context Window | `32768` |

Roo Code can also use specialist modes directly. Copy the mode file to your project:

```bash
cp x3-ai-command-system/roo-config/.roomodes /path/to/your/project/
```

Then switch between 20 X3 specialist modes in Roo Code's mode selector. Each mode sends the specialist model name directly — the router recognizes `lojak/x3-*` models and passes them through without re-classifying.

See `prompts/roo/starters.md` for task-specific prompts per mode.

### Routing Examples

| You type | Router sends to |
|---|---|
| "Audit this Solidity contract" | `lojak/x3-solidity-guard` |
| "Fix the Rust pallet tests" | `lojak/x3-rust-runtime` |
| "Design an arbitrage strategy" | `lojak/x3-arb-king` |
| "Build a flashloan executor" | `lojak/x3-flashloan-executor` |
| "Score this cross-chain route" | `lojak/x3-route-oracle` |
| "Check MEV exposure" | `lojak/x3-mev-defense` |
| "Review mainnet readiness" | `lojak/x3-auditor` |
| "What is X3?" (generic) | `lojak/cryptomaster` |

### Force a Specific Model

Add the `X-X3-Model` header to any request:

```bash
curl -H "X-X3-Model: lojak/x3-auditor" http://localhost:11435/v1/chat/completions \
  -d '{"model":"lojak/cryptomaster","messages":[{"role":"user","content":"hello"}]}'
```

### Run Classifier Tests

```bash
cd x3-ai-command-system/router
python3 classifier.py --test     # Test keyword classification
python3 x3_router_test.py         # Test classifier + live router
```

### Configuration

See `router/config.yaml` for:
- Router port (default: 11435)
- Ollama host (default: http://localhost:11434)
- Default model (default: lojak/cryptomaster)
- Routing mode (keyword or future LLM-based)

## Version Status

| Version | Description | Status |
|---|---|---|
| v0.1 | Modelfile-customized specialist models | ✅ Current |
| v0.2 | Knowledge-core enhanced | 🔄 In progress |
| v0.3 | RAG/repo-memory enhanced | 📋 Planned |
| v0.4 | Eval-gated | 📋 Planned |
| v0.5 | LoRA fine-tuned | 📋 Planned |
| v1.0 | X3 production assistant suite | 📋 Planned |

## Base Model Attribution

This model pack is built on Qwen2.5-Coder via Ollama and customized for X3 development using Modelfiles, system prompts, parameters, and role-specific behavior.

X3-specific prompts, workflows, model roles, and documentation are authored for the X3 ecosystem. Base model rights remain with their upstream authors and licenses (Apache-2.0).

## License Notes

- Modelfile customizations: MIT
- X3-specific system prompts and documentation: MIT
- Base model (Qwen2.5-Coder): Apache-2.0
- Fine-tuned adapters: TBD