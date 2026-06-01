# X3 Ollama Model Pack

Five specialist Ollama models for X3 Chain engineering, built on Qwen2.5-Coder.

## Models

| Model | Role | Temperature | Best For |
|---|---|---|---|
| `lojak/cryptomaster` | Main architect / production coder | 0.15 | Architecture, implementation, review |
| `lojak/x3-auditor` | Security & mainnet-readiness reviewer | 0.05 | Audits, exploit analysis, release gates |
| `lojak/x3-rust-runtime` | Substrate/Rust runtime specialist | 0.12 | Pallets, runtime, consensus, weights |
| `lojak/x3-solidity-guard` | Solidity/EVM contract security | 0.08 | Contracts, vaults, DEX, launchpads |
| `lojak/x3-cline-finisher` | Repo completion / anti-slop agent | 0.10 | TODO killing, stub removal, test fixing |

## Quick Start

### 1. Pull the base model

```bash
ollama pull qwen2.5-coder:14b
# For stronger hardware:
# ollama pull qwen2.5-coder:32b
```

### 2. Build all five models

```bash
cd ollama-models/x3-pack
./build_x3_models.sh
```

For 32B base:

```bash
BASE_MODEL=qwen2.5-coder:32b ./build_x3_models.sh
```

### 3. Smoke test

```bash
ollama run lojak/cryptomaster
ollama run lojak/x3-auditor
ollama run lojak/x3-rust-runtime
ollama run lojak/x3-solidity-guard
ollama run lojak/x3-cline-finisher
```

### 4. Push to registry

```bash
ollama login
./push_x3_models.sh
```

## Use in Cline

| Setting | Value |
|---|---|
| Provider | `Ollama` |
| Base URL | `http://localhost:11434` |
| Context Window | `32768` |
| Model | One of the `lojak/...` models |

### Routing

| Job | Model |
|---|---|
| Big architecture / general coding | `lojak/cryptomaster` |
| Security review / audit | `lojak/x3-auditor` |
| Runtime / Substrate / Rust | `lojak/x3-rust-runtime` |
| Solidity / contracts | `lojak/x3-solidity-guard` |
| Finish TODOs / repair repo | `lojak/x3-cline-finisher` |

### Starter Prompts

**cryptomaster:**
```
You are CryptoMaster inside Cline.

Inspect this X3 repo for production readiness.
Do not edit files yet.

Return:
1. Project map
2. Build commands
3. Test commands
4. Runtime entrypoints
5. Critical path files
6. Mainnet blockers
7. First 10 safe patches
8. Exact patch order
```

**x3-auditor:**
```
You are X3-Auditor inside Cline.

Audit this repo as if public capital may touch it.
Do not modify files.

Find:
1. Fund-loss risks
2. Supply-invariant risks
3. Replay/double-spend risks
4. Cross-VM atomicity failures
5. Finality assumption bugs
6. Dangerous TODOs/stubs
7. Tests that are missing or cheating
8. Minimum patch order before testnet
```

**x3-rust-runtime:**
```
You are X3 Rust Runtime inside Cline.

Inspect the Rust/Substrate/runtime pieces.
Do not modify files yet.

Return:
1. Crates and pallets map
2. Runtime APIs
3. Storage items
4. Dispatchables
5. Invariants
6. Weight/benchmark problems
7. Determinism risks
8. Tests to add
9. First safe patch
```

**x3-solidity-guard:**
```
You are X3 Solidity Guard inside Cline.

Audit all Solidity contracts.
Do not edit files yet.

Return:
1. Contract map
2. Access-control risks
3. Reentrancy/callback risks
4. Accounting bugs
5. Slippage/deadline/replay gaps
6. Unsafe token assumptions
7. Foundry/Hardhat tests missing
8. Patch order
```

**x3-cline-finisher:**
```
You are X3 Cline Finisher.

Your job is to finish incomplete repo work without cheating.

Rules:
- Do not change tests to hide bugs.
- Do not add fake mocks/stubs/placeholders.
- Do not stop after only planning.
- Inspect TODO/FIXME/HACK/stub/unimplemented/todo!/panic/unwrap in critical paths.
- Make small patches.
- Run relevant tests after each patch.
- Continue until the selected task is actually complete or blocked by missing external information.
```

## Fine-Tuning (Advanced)

These Modelfile models are prompt-specialized, not fine-tuned. For deeper X3 specialization, fine-tune LoRA adapters and import them into Ollama.

### Fine-Tuning Ladder

| Stage | What it does | Ready now? |
|---|---|---|
| Modelfile | Role/personality/rules via system prompt | ✅ Yes |
| Repo RAG | Model + searchable X3 knowledgebase | ✅ Yes |
| SFT LoRA | Trains on X3 patterns/examples | Later — needs dataset |
| DPO/RLAIF | Preference tuning: good patch > bad patch | Later |
| Eval harness | Prevents the model from getting dumber | Mandatory before any fine-tune |

### LoRA Fine-Tuning

```bash
cd ollama-models/x3-pack/finetune

# Set up environment
python3 -m venv .venv
source .venv/bin/activate
pip install -U torch transformers datasets accelerate trl peft bitsandbytes

# Prepare your dataset in data/x3_sft.jsonl
# See data/x3_sft_template.jsonl for format

# Train (7B default, adjust MODEL_NAME for larger)
python train_x3_lora.py

# For 14B:
MODEL_NAME=Qwen/Qwen2.5-Coder-14B-Instruct OUTPUT_DIR=outputs/x3-coder-14b-lora python train_x3_lora.py
```

### Import LoRA Adapter into Ollama

After training, create a Modelfile:

```
FROM qwen2.5-coder:14b

ADAPTER /path/to/outputs/x3-coder-lora

PARAMETER temperature 0.1
PARAMETER top_p 0.9
PARAMETER repeat_penalty 1.12
PARAMETER num_ctx 32768

SYSTEM """
You are CryptoMaster-Finetuned, lojak's X3-specialized model.
Use learned X3 patterns. Never override safety, tests, or determinism.
"""
```

Then:

```bash
ollama create lojak/cryptomaster-ft -f Modelfile
ollama run lojak/cryptomaster-ft
```

### GGUF Export (Unsloth Path)

For use with llama.cpp, Jan, Open WebUI:

```python
# In Unsloth training script:
model.save_pretrained_gguf(
    "x3-cryptomaster-gguf",
    tokenizer,
    quantization_method="q4_k_m",
)
```

Then create an Ollama model from the GGUF:

```
FROM /path/to/x3-cryptomaster.gguf

PARAMETER temperature 0.1
PARAMETER num_ctx 32768

SYSTEM """
You are CryptoMaster-Finetuned.
"""
```

### Dataset Guidelines

**Good training examples:**
- Accepted patches and bug fixes from the X3 repo
- Real audit reports with findings
- Before/after code diffs
- Failed test → correct fix pairs
- Router invariant examples
- Substrate pallet patterns
- Solidity safe contract patterns
- Cline task completions
- Mainnet-readiness reviews

**Bad training examples:**
- Old broken code
- Half-finished TODO dumps
- Fake deploy logs
- Low-quality chat rambling
- Malicious contract patterns
- Tests changed only to pass

### Hardware Notes

| Model Size | Minimum GPU | Recommended | Notes |
|---|---|---|---|
| 7B QLoRA | 1× RTX 3090 | 1× RTX 4090 | Fast iteration |
| 14B QLoRA | 1× RTX 4090 | 2× RTX 4090 | Sweet spot |
| 32B QLoRA | 2× A100 40GB | 4× A100 80GB | Cloud/H100 territory |

Start with 7B. Move to 14B when the dataset is clean. 32B is for when you have real infrastructure and a real dataset.

## Directory Layout

```
x3-pack/
├── build_x3_models.sh          # Build all five models
├── push_x3_models.sh           # Push all models to Ollama registry
├── README.md                   # This file
├── cryptomaster/
│   └── Modelfile
├── x3-auditor/
│   └── Modelfile
├── x3-rust-runtime/
│   └── Modelfile
├── x3-solidity-guard/
│   └── Modelfile
├── x3-cline-finisher/
│   └── Modelfile
└── finetune/
    ├── train_x3_lora.py        # SFT LoRA training script
    └── data/
        └── x3_sft_template.jsonl  # Example dataset format
```

## The Real Architecture

```
Cline (or any agent runner)
 ├── lojak/x3-cline-finisher       # edits files
 ├── lojak/x3-auditor              # reviews patches
 ├── lojak/x3-rust-runtime         # handles Substrate/Rust
 ├── lojak/x3-solidity-guard       # handles contracts
 └── lojak/cryptomaster            # architect / final judge
```

Later, add fine-tuned LoRA adapters:

```
Base models
 ├── qwen2.5-coder:14b
 └── qwen2.5-coder:32b

Fine-tuned adapters
 ├── x3-architecture-lora
 ├── x3-audit-lora
 ├── x3-rust-runtime-lora
 ├── x3-solidity-lora
 └── x3-finisher-lora
```

Each specialist gets its own dataset. Do not mix everything into one soup.

## Build Order

1. **Now:** Build the five Modelfile models and use them in Cline
2. **Now:** Start collecting clean X3 examples (patches, audits, reviews)
3. **1–2 weeks later:** Fine-tune LoRA adapters when the dataset is clean
4. **Mandatory:** Run eval harness before and after every fine-tune

Grease in, grease out. Fine-tune dirty data and you get a dirty model.