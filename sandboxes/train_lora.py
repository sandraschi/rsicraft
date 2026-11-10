#!/usr/bin/env python3
"""
RSICraft LoRA post-training harness.

Trains a LoRA adapter on a chat-format JSONL dataset (one {"messages": [...]}
object per line, as written by `rsi_train_step`) and saves it with PEFT.

Nothing here is simulated. The script either writes
<output-dir>/adapter_model.safetensors and reports status SUCCESS, or it
reports a failure status and exits non-zero. The last stdout line is always
RSI_TRAIN_METRIC={...}.

Requires: torch (CUDA build), transformers, peft, datasets, and bitsandbytes
for --load-in-4bit. The base model must be a Hugging Face id or a local
safetensors directory, not a GGUF file.

STATUS: written against the standard transformers/PEFT API but NOT yet run end
to end on Goliath. Treat the first run as a test of this script.
"""

import argparse
import json
import os
import sys
import time


def report(status, code, **extra):
    print("RSI_TRAIN_METRIC=" + json.dumps({"status": status, **extra}))
    sys.exit(code)


def main():
    p = argparse.ArgumentParser(description="RSICraft LoRA fine-tuner")
    p.add_argument("--dataset", required=True)
    p.add_argument("--base-model", required=True)
    p.add_argument("--output-dir", required=True)
    p.add_argument("--epochs", type=int, default=3)
    p.add_argument("--learning-rate", type=float, default=2e-4)
    p.add_argument("--rank", type=int, default=16)
    p.add_argument("--max-seq-len", type=int, default=2048)
    p.add_argument("--load-in-4bit", action="store_true")
    args = p.parse_args()

    if not os.path.exists(args.dataset):
        report("DATASET_NOT_FOUND", 1, dataset=args.dataset)
    with open(args.dataset, "r", encoding="utf-8") as f:
        rows = [json.loads(line) for line in f if line.strip()]
    if not rows:
        report("DATASET_EMPTY", 1)

    try:
        import torch
        from datasets import Dataset
        from peft import LoraConfig, get_peft_model
        from transformers import (
            AutoModelForCausalLM,
            AutoTokenizer,
            DataCollatorForLanguageModeling,
            Trainer,
            TrainingArguments,
        )
    except ImportError as e:
        report("DEPS_MISSING", 2, error=str(e))

    if not torch.cuda.is_available():
        report("NO_CUDA", 2, error="torch reports no CUDA device")

    start = time.time()
    tok = AutoTokenizer.from_pretrained(args.base_model)
    if tok.pad_token is None:
        tok.pad_token = tok.eos_token

    load_kwargs = {"device_map": "auto", "torch_dtype": torch.bfloat16}
    if args.load_in_4bit:
        from transformers import BitsAndBytesConfig

        load_kwargs["quantization_config"] = BitsAndBytesConfig(
            load_in_4bit=True,
            bnb_4bit_quant_type="nf4",
            bnb_4bit_compute_dtype=torch.bfloat16,
        )
    model = AutoModelForCausalLM.from_pretrained(args.base_model, **load_kwargs)
    model = get_peft_model(
        model,
        LoraConfig(
            r=args.rank,
            lora_alpha=args.rank * 2,
            lora_dropout=0.05,
            target_modules="all-linear",
            task_type="CAUSAL_LM",
        ),
    )

    def encode(row):
        text = tok.apply_chat_template(row["messages"], tokenize=False)
        return tok(text, truncation=True, max_length=args.max_seq_len)

    ds = Dataset.from_list(rows).map(encode, remove_columns=["messages"])
    trainer = Trainer(
        model=model,
        args=TrainingArguments(
            output_dir=os.path.join(args.output_dir, "_checkpoints"),
            num_train_epochs=args.epochs,
            learning_rate=args.learning_rate,
            per_device_train_batch_size=1,
            gradient_accumulation_steps=8,
            logging_steps=1,
            save_strategy="no",
            report_to=[],
            bf16=True,
        ),
        train_dataset=ds,
        data_collator=DataCollatorForLanguageModeling(tok, mlm=False),
    )
    out = trainer.train()
    losses = [h["loss"] for h in trainer.state.log_history if "loss" in h]
    model.save_pretrained(args.output_dir)

    adapter = os.path.join(args.output_dir, "adapter_model.safetensors")
    if not os.path.exists(adapter):
        report("ADAPTER_NOT_WRITTEN", 1)
    report(
        "SUCCESS",
        0,
        samples_trained=len(rows),
        epochs=args.epochs,
        first_logged_loss=losses[0] if losses else None,
        last_logged_loss=losses[-1] if losses else None,
        mean_train_loss=out.training_loss,
        adapter_path=args.output_dir,
        duration_secs=round(time.time() - start, 1),
    )


if __name__ == "__main__":
    main()
