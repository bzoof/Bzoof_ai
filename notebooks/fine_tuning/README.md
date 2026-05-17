# Fine-Tuning Qwen2.5-Coder with Unsloth

This directory contains instructions and resources for fine-tuning qwen2.5-coder:3b on your domain-specific data.

## Quick Start (Google Colab)

### Step 1: Open Colab Notebook
1. Go to [Google Colab](https://colab.google/)
2. Click "New Notebook"
3. Copy-paste the code from **fine_tune_qwen_colab.py** into the cells

### Step 2: Install Dependencies
```python
!pip install unsloth[colab-new] xformers bitsandbytes
```

### Step 3: Load and Prepare Dataset
```python
# Upload your combined datasets
from google.colab import files
files.upload()  # Select shell_commands.jsonl, code_examples.jsonl, qa_examples.jsonl

# Combine datasets
import json
dataset = []
for filename in ['shell_commands.jsonl', 'code_examples.jsonl', 'qa_examples.jsonl']:
    with open(filename) as f:
        for line in f:
            record = json.loads(line)
            dataset.append(record)

# Save combined dataset
with open('combined_dataset.jsonl', 'w') as f:
    for record in dataset:
        f.write(json.dumps(record) + '\n')

print(f"Combined {len(dataset)} examples")
```

### Step 4: Train LoRA Adapter
```python
from unsloth import FastLanguageModel
from trl import SFTTrainer
from transformers import TrainingArguments
import pandas as pd

# Load model with LoRA
model, tokenizer = FastLanguageModel.from_pretrained(
    model_name="Qwen/Qwen2.5-Coder-3B-Instruct",
    max_seq_length=2048,
    load_in_4bit=True,
)

# Add LoRA adapters
model = FastLanguageModel.get_peft_model(
    model,
    r=16,
    lora_alpha=16,
    target_modules=["q_proj", "v_proj"],
    lora_dropout=0.05,
    bias="none",
    use_gradient_checkpointing=True,
    random_state=42,
)

# Prepare dataset
df = pd.read_json('combined_dataset.jsonl', lines=True)
dataset = df.sample(frac=1).reset_index(drop=True)  # Shuffle

# Train
trainer = SFTTrainer(
    model=model,
    tokenizer=tokenizer,
    train_dataset=dataset,
    dataset_text_field="instruction",
    max_seq_length=2048,
    dataset_num_proc=2,
    args=TrainingArguments(
        per_device_train_batch_size=2,
        gradient_accumulation_steps=4,
        learning_rate=2e-4,
        num_train_epochs=3,
        fp16=True,
        logging_steps=10,
        output_dir="outputs",
        optim="adamw_8bit",
        warmup_steps=5,
        seed=42,
    ),
    packing=False,
    max_steps=500,
)

trainer.train()
```

### Step 5: Save LoRA Weights
```python
model.save_pretrained("lora_adapter")
tokenizer.save_pretrained("lora_adapter")

# Download weights
from google.colab import files
import shutil
shutil.make_archive('lora_adapter', 'zip', '.', 'lora_adapter')
files.download('lora_adapter.zip')
```

## Expected Training Time
- **GPU**: V100/A100 (Colab): 2-3 hours
- **GPU**: RTX 4090: 2-4 hours
- **GPU**: RTX 3060: 6-8 hours
- **CPU**: Not recommended (days)

## Expected Results
- Improvement: 15-25% on domain-specific tasks
- Model size: ~50MB (LoRA weights)
- Inference time: Same as base model (5-10% faster on repeated patterns)

## Next Steps After Training

1. **Convert to GGUF** (using llama.cpp):
   ```bash
   python3 convert_lora_to_gguf.py
   ```

2. **Add to Ollama**:
   ```bash
   ollama create qwen2.5-coder:3b-finetuned -f Modelfile
   ```

3. **Test with bwb_ai**:
   ```bash
   cargo run -- --model qwen2.5-coder:3b-finetuned
   ```

## Troubleshooting

### Out of Memory (OOM)
- Reduce batch size: `per_device_train_batch_size=1`
- Use QLoRA instead of LoRA
- Use free cloud GPU (Colab)

### Slow Training
- GPU not in use? Check with `!nvidia-smi`
- Use Colab Pro for faster GPU (A100)
- Reduce max_steps or num_train_epochs

### Poor Results
- More training data (200-500 examples minimum)
- Higher learning rate: `3e-4`
- More epochs: `num_train_epochs=5`

## Resources
- [Unsloth Documentation](https://unsloth.ai/)
- [SitePoint: Fine-tune LLMs 2026](https://www.sitepoint.com/fine-tune-local-llms-2026/)
- [Hugging Face: Qwen2.5-Coder](https://huggingface.co/Qwen/Qwen2.5-Coder-3B-Instruct)
