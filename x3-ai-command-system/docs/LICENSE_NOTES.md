# License Notes

## X3 AI Command System Customizations

- Modelfile customizations: **MIT License**
- X3-specific system prompts: **MIT License**
- X3-specific documentation: **MIT License**
- X3-specific evaluation cases: **MIT License**
- X3-specific training scripts: **MIT License**
- Router configuration: **MIT License**
- Safety documents: **MIT License**

## Base Model

- **Qwen2.5-Coder** by Alibaba: **Apache-2.0 License**
- Used via Ollama as the base model for all Modelfile customizations
- Base model rights remain with their upstream authors

## Fine-Tuned Adapters

- Fine-tuned LoRA adapters trained on X3-specific data: **TBD**
- License will be determined based on training data composition
- Will inherit Apache-2.0 from the base model at minimum

## Usage Restrictions

Regardless of license, the X3 AI model pack must not be used for:

1. Theft, fraud, or unauthorized exploitation
2. Phishing or social engineering
3. Rug-pull mechanics or deceptive tokens
4. Malicious MEV targeting retail users
5. DAO vote hijacking
6. Unauthorized exploit execution
7. Any activity that drains user funds without consent

See `safety/SECURITY_BOUNDARIES.md` for full usage boundaries.