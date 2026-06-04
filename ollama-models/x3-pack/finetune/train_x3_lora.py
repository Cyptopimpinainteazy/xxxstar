#!/usr/bin/env python3
"""
train_x3_lora.py — SFT LoRA fine-tuning for X3 specialist models

Fine-tunes Qwen2.5-Coder on X3-specific examples to create LoRA adapters
that can be imported into Ollama via the ADAPTER instruction.

Prerequisites:
  pip install -U torch transformers datasets accelerate trl peft bitsandbytes

Usage:
  python train_x3_lora.py

  # Override defaults:
  MODEL_NAME=Qwen/Qwen2.5-Coder-14B-Instruct python train_x3_lora.py
  DATA_FILE=data/x3_sft.jsonl OUTPUT_DIR=outputs/x3-coder-14b-lora python train_x3_lora.py

After training:
  1. The LoRA adapter is saved to OUTPUT_DIR
  2. Create an Ollama Modelfile with:
     FROM qwen2.5-coder:14b
     ADAPTER /path/to/outputs/x3-coder-14b-lora
     PARAMETER temperature 0.1
     PARAMETER num_ctx 32768
     SYSTEM You are CryptoMaster-Finetuned...
  3. Run: ollama create lojak/cryptomaster-ft -f Modelfile

For GGUF export (Unsloth path):
  See README.md for Unsloth export instructions.
"""

import os
import torch
from datasets import load_dataset
from peft import LoraConfig
from transformers import AutoModelForCausalLM, AutoTokenizer, BitsAndBytesConfig
from trl import SFTConfig, SFTTrainer

# --- Configuration ---
MODEL_NAME = os.environ.get("MODEL_NAME", "Qwen/Qwen2.5-Coder-7B-Instruct")
DATA_FILE = os.environ.get("DATA_FILE", "data/x3_sft.jsonl")
OUTPUT_DIR = os.environ.get("OUTPUT_DIR", "outputs/x3-coder-lora")

# LoRA hyperparameters
LORA_R = int(os.environ.get("LORA_R", "16"))
LORA_ALPHA = int(os.environ.get("LORA_ALPHA", "32"))
LORA_DROPOUT = float(os.environ.get("LORA_DROPOUT", "0.05"))

# Training hyperparameters
NUM_EPOCHS = int(os.environ.get("NUM_EPOCHS", "2"))
BATCH_SIZE = int(os.environ.get("BATCH_SIZE", "1"))
GRAD_ACCUM = int(os.environ.get("GRAD_ACCUM", "8"))
LEARNING_RATE = float(os.environ.get("LEARNING_RATE", "2e-4"))
MAX_LENGTH = int(os.environ.get("MAX_LENGTH", "4096"))
LOGGING_STEPS = int(os.environ.get("LOGGING_STEPS", "10"))
SAVE_STEPS = int(os.environ.get("SAVE_STEPS", "100"))


def main():
    print(f"Model:    {MODEL_NAME}")
    print(f"Data:     {DATA_FILE}")
    print(f"Output:   {OUTPUT_DIR}")
    print(f"LoRA r={LORA_R}, alpha={LORA_ALPHA}, dropout={LORA_DROPOUT}")
    print(f"Epochs:   {NUM_EPOCHS}")
    print(f"Batch:    {BATCH_SIZE} x {GRAD_ACCUM} accum")
    print(f"LR:       {LEARNING_RATE}")
    print()

    # --- Quantization ---
    bnb_config = BitsAndBytesConfig(
        load_in_4bit=True,
        bnb_4bit_quant_type="nf4",
        bnb_4bit_compute_dtype=torch.float16,
    )

    # --- Tokenizer ---
    tokenizer = AutoTokenizer.from_pretrained(
        MODEL_NAME,
        trust_remote_code=True,
    )
    if tokenizer.pad_token is None:
        tokenizer.pad_token = tokenizer.eos_token

    # --- Model ---
    print("Loading model...")
    model = AutoModelForCausalLM.from_pretrained(
        MODEL_NAME,
        quantization_config=bnb_config,
        device_map="auto",
        trust_remote_code=True,
    )

    # --- Dataset ---
    print(f"Loading dataset from {DATA_FILE}...")
    dataset = load_dataset("json", data_files=DATA_FILE, split="train")
    print(f"Loaded {len(dataset)} examples")

    # --- LoRA config ---
    lora_config = LoraConfig(
        r=LORA_R,
        lora_alpha=LORA_ALPHA,
        lora_dropout=LORA_DROPOUT,
        bias="none",
        task_type="CAUSAL_LM",
        target_modules=[
            "q_proj",
            "k_proj",
            "v_proj",
            "o_proj",
            "gate_proj",
            "up_proj",
            "down_proj",
        ],
    )

    # --- Training config ---
    training_args = SFTConfig(
        output_dir=OUTPUT_DIR,
        num_train_epochs=NUM_EPOCHS,
        per_device_train_batch_size=BATCH_SIZE,
        gradient_accumulation_steps=GRAD_ACCUM,
        learning_rate=LEARNING_RATE,
        logging_steps=LOGGING_STEPS,
        save_steps=SAVE_STEPS,
        max_length=MAX_LENGTH,
        packing=True,
        report_to="none",
    )

    # --- Trainer ---
    trainer = SFTTrainer(
        model=model,
        tokenizer=tokenizer,
        train_dataset=dataset,
        peft_config=lora_config,
        args=training_args,
    )

    # --- Train ---
    print("Starting training...")
    trainer.train()

    # --- Save ---
    trainer.save_model(OUTPUT_DIR)
    tokenizer.save_pretrained(OUTPUT_DIR)
    print(f"\nSaved LoRA adapter to {OUTPUT_DIR}")
    print("\nNext steps:")
    print(f"  1. Create Modelfile with: ADAPTER {os.path.abspath(OUTPUT_DIR)}")
    print("  2. Run: ollama create lojak/cryptomaster-ft -f Modelfile")
    print("  3. Run: ollama run lojak/cryptomaster-ft")


if __name__ == "__main__":
    main()