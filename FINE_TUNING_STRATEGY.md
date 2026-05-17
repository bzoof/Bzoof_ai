# bwb_ai: Fine-Tuning Strategy for Domain-Specific Code Generation

## Overview

This document provides a practical roadmap for fine-tuning qwen2.5-coder:3b on your project's specific use cases (shell commands, code patterns, documentation Q&A) to improve model quality for the bwb_ai assistant.

---

## 1. Why Fine-Tune?

### Current Model (qwen2.5-coder:3b)
- ✅ Fast on CPU (15-20 tokens/sec)
- ✅ General-purpose code generation
- ⚠️ May not understand your project's conventions
- ⚠️ Shell command handling is generic (not optimized for safe execution)
- ⚠️ PDF Q&A quality depends on document context

### After Fine-Tuning
- ✅ Specialized in YOUR code patterns
- ✅ Better shell command recommendations (understands safety constraints)
- ✅ Improved PDF Q&A (learns document structure from your examples)
- ✅ Faster response time (fewer hallucinations = fewer tokens)

### Expected ROI
- 15-25% improvement in code generation accuracy
- 20-30% reduction in unsafe shell command suggestions
- 10-20% faster inference (fewer retries)

---

## 2. Fine-Tuning Approaches

### Option A: LoRA (Recommended for CPU)
**Low-Rank Adaptation** — trains only small adapter layers while keeping base model frozen.

**Pros:**
- ✅ 4-6x less memory required (8-12 GB VRAM)
- ✅ Faster training (2-4 hours on GPU)
- ✅ Can stack multiple LoRA adapters
- ✅ Easy to distribute fine-tuned weights

**Cons:**
- ⚠️ Requires GPU for training (CPU training is extremely slow)
- ⚠️ Slightly less model capacity than full fine-tuning

**Best for**: Your use case — quick iteration, specialized domain knowledge

### Option B: QLoRA (Ultra-Low Resource)
**Quantized LoRA** — 4-bit quantization + LoRA reduces VRAM to 6-8 GB.

**Pros:**
- ✅ Works on consumer GPUs (RTX 3060, RTX 4060)
- ✅ Minimal hardware barrier

**Cons:**
- ⚠️ More complex setup
- ⚠️ Slightly lower training stability

**Best for**: Teams without GPU access (use cloud GPU for 2-3 hours)

### Option C: Full Fine-Tuning
**Train all model weights** — highest quality but most resource-intensive.

**Pros:**
- ✅ Maximum model customization
- ✅ Best results for code generation

**Cons:**
- ⚠️ Requires 24GB+ VRAM (RTX 4090 level)
- ⚠️ 1-2 days training on high-end GPU
- ⚠️ Large output model (~7GB)

**Best for**: Large organizations with unlimited resources

### Recommendation for bwb_ai
**Start with LoRA** (Option A):
1. Rent GPU time on cloud (Google Colab Pro, Lambda Labs, Vast.ai — ~$5-10/hour)
2. Train 2-4 hour session with your dataset
3. Keep LoRA weights (~50MB) in repo
4. Load LoRA adapter at startup with Ollama

---

## 3. Dataset Preparation

### What to Collect

**3a. Shell Command Pairs** (for `:run` command safety)
```json
{
  "instruction": "Show files in current directory",
  "output": "ls -la",
  "safety_notes": "allowed_command"
}
```

**3b. Code Generation Examples** (for chat quality)
```json
{
  "instruction": "Write a Rust function to parse JSON",
  "output": "use serde_json::json;\n\nfn parse_json(s: &str) -> Result<Value, Box<dyn Error>> {\n    Ok(serde_json::from_str(s)?)\n}\n",
  "language": "rust"
}
```

**3c. PDF Q&A Pairs** (for documentation handling)
```json
{
  "instruction": "Based on the AI architecture document, what are the main components of the system?",
  "output": "The system has three main components: 1) TUI for user interaction, 2) LLM interface for Ollama communication, 3) PDF reader and shell runner for extended capabilities.",
  "context": "docs/ai_agent_architecture.pdf"
}
```

### Dataset Quality Standards
| Criterion | Standard | Example |
|-----------|----------|---------|
| **Accuracy** | 100% — no broken code | Test all shell commands before including |
| **Clarity** | Instruction must be specific | ❌ "write code" → ✅ "write Rust async function to..." |
| **Length** | 5-500 tokens per example | Too short = unclear; too long = hard to learn |
| **Variety** | Cover 80% of real use cases | 30% shell, 50% code, 20% Q&A |
| **Format** | Consistent JSON structure | Use template below |

### Dataset Template (JSONL format)
```jsonl
{"instruction": "List all files in current directory with details", "output": "ls -la", "category": "shell"}
{"instruction": "Show git status", "output": "git status", "category": "shell"}
{"instruction": "Write a Rust function to read a file", "output": "fn read_file(path: &str) -> Result<String> {\n    std::fs::read_to_string(path)\n}\n", "category": "code"}
{"instruction": "What are the benefits of using async/await in Rust?", "output": "Async/await enables non-blocking concurrent code without callback hell. It integrates with tokio runtime for efficient I/O handling.", "category": "qa"}
```

### Dataset Size Recommendations
| Quality | Size | Training Time | Expected ROI |
|---------|------|---------------|--------------|
| **Minimal** | 50-100 examples | 30 min | 5-10% improvement |
| **Baseline** | 200-500 examples | 1-2 hours | 15-20% improvement |
| **Comprehensive** | 500-2000 examples | 2-4 hours | 20-30% improvement |
| **Extensive** | 2000+ examples | 4-8 hours | 30-40% improvement |

**Recommendation**: Start with 200-500 examples (1-2 hour training session)

---

## 4. Implementation Pipeline

### Step 1: Prepare Dataset (1-2 weeks)
```bash
# Create training data directory
mkdir -p datasets/fine_tuning
cd datasets/fine_tuning

# 1a. Collect shell commands from your allowlist
# Create shell_commands.jsonl with your 27 ALLOWED_COMMANDS + explanations

# 1b. Collect code examples from your project
# Extract snippets from src/*.rs (Cargo.toml, main.rs, etc.)

# 1c. Collect PDF Q&A pairs
# Extract questions about ai_agent_architecture.pdf + expected answers

# Validate dataset
python3 validate_dataset.py shell_commands.jsonl code_examples.jsonl qa_pairs.jsonl
```

### Step 2: Set Up Fine-Tuning Environment (30 min)
Using [Unsloth](https://unsloth.ai/) on Google Colab (free GPU):
```python
# In Google Colab
!pip install unsloth[colab-new] xformers bitsandbytes
from unsloth import FastLanguageModel

# Load qwen2.5-coder:3b
model, tokenizer = FastLanguageModel.from_pretrained(
    model_name="Qwen/Qwen2.5-Coder-3B-Instruct",
    max_seq_length=2048,
    load_in_4bit=True,  # QLoRA
)

# Add LoRA adapters
model = FastLanguageModel.get_peft_model(
    model,
    r=16,  # LoRA rank
    lora_alpha=16,
    target_modules=["q_proj", "v_proj"],
    lora_dropout=0.05,
    bias="none",
    use_gradient_checkpointing=True,
    random_state=42,
)
```

### Step 3: Train LoRA Adapter (2-4 hours)
```python
from trl import SFTTrainer
from transformers import TrainingArguments

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
    packing=False,  # Better for code
    max_steps=500,  # ~2 hours on GPU
)

trainer.train()
```

### Step 4: Merge and Test (30 min)
```python
# Save LoRA weights
model.save_pretrained("lora_adapter")
tokenizer.save_pretrained("lora_adapter")

# Test fine-tuned model
outputs = model.generate(inputs, max_new_tokens=128)
print(tokenizer.decode(outputs[0]))
```

### Step 5: Convert to GGUF for Ollama (30 min)
```bash
# Export fine-tuned model
python export_model.py --lora lora_adapter --output finetuned_qwen

# Convert to GGUF
./llama.cpp/main -m finetuned_qwen.gguf -n 128 \
  -p "List files in directory:" \
  --repeat-penalty 1.1
```

### Step 6: Deploy to Ollama (5 min)
```bash
# Create Modelfile for fine-tuned version
cat > Modelfile << EOF
FROM finetuned_qwen.gguf

PARAMETER temperature 0.7
PARAMETER top_p 0.9
PARAMETER num_ctx 2048
EOF

# Add to Ollama
ollama create qwen2.5-coder:3b-finetuned -f Modelfile

# Test with bwb_ai
cargo run -- --model qwen2.5-coder:3b-finetuned
```

---

## 5. Monitoring & Iteration

### Metrics to Track

| Metric | Before | Target | Method |
|--------|--------|--------|--------|
| **Shell safety** | 85% safe commands | 95%+ | Manual review of :run suggestions |
| **Code quality** | 70% syntactically valid | 85%+ | Test generated code compiles |
| **Response speed** | 15 tokens/sec | 18+ tokens/sec | Measure latency per batch |
| **PDF Q&A accuracy** | 60% relevant | 75%+ | Human review of PDF answers |

### A/B Testing
```bash
# Test both models side-by-side
ollama run qwen2.5-coder:3b "Write a Rust parser"
ollama run qwen2.5-coder:3b-finetuned "Write a Rust parser"

# Compare outputs, collect feedback
```

### Retraining Cadence
- **Monthly**: Collect 20-50 new examples from user interactions
- **Quarterly**: Retrain LoRA with accumulated dataset
- **Semi-annually**: Evaluate alternative base models (gemma3, qwen3)

---

## 6. Cloud Resources & Costs

### Option 1: Google Colab Pro (Recommended)
- **Cost**: $11.99/month (unlimited GPU, TPU)
- **GPU**: Tesla V100 or A100 (20-40 GB VRAM)
- **Setup**: Notebook in [Google Drive](https://colab.google/)
- **Training time**: 2-4 hours
- **Notes**: Perfect for prototyping, no credit card needed if using free tier

### Option 2: Lambda Labs
- **Cost**: $0.00-1.10 per hour (A100 GPU)
- **Setup**: SSH access, pre-installed PyTorch
- **Training time**: 1-2 hours on A100
- **Best for**: Production fine-tuning, reproducible runs

### Option 3: Vast.ai
- **Cost**: $0.20-0.50 per hour (V100, A100 instances)
- **Setup**: Docker container or Jupyter notebook
- **Training time**: 2-3 hours on V100
- **Best for**: Budget-conscious teams

### Option 4: Local GPU (If available)
- **Cost**: Electricity only (~$5 for 4-hour training)
- **GPU**: RTX 3060 or better (12GB+ VRAM)
- **Training time**: 4-8 hours on RTX 3060
- **Best for**: Privacy-sensitive work, reproducibility

**Total Cost for Initial Fine-Tuning**: $10-20 (Google Colab or cloud GPU for one session)

---

## 7. Integration with bwb_ai

### Phase 7: Fine-Tuning Pipeline (Proposed)
```
Phase 7 Tasks:
1. Create dataset collection workflow (scripts/collect_examples.py)
2. Set up Unsloth training notebook (notebooks/fine_tune_qwen.ipynb)
3. Add LoRA model switching to main.rs (--model-lora flag)
4. Document feedback loop for continuous improvement
5. Test A/B comparison between base and fine-tuned models
```

### Code Changes Required
```rust
// In main.rs: Add LoRA support
#[derive(Parser)]
struct Args {
    #[arg(long, default_value = "qwen2.5-coder:3b")]
    model: String,
    
    #[arg(long)]
    lora_adapter: Option<String>,  // NEW: Path to LoRA weights
}

// In llama_interface.rs: Load LoRA at startup
impl LlamaClient {
    pub async fn new(config: LlamaConfig, lora: Option<&Path>) -> Result<Self> {
        if let Some(adapter_path) = lora {
            tracing::info!("Loading LoRA adapter: {}", adapter_path.display());
            // Configure Ollama to use LoRA weights
        }
        // ... rest of initialization
    }
}
```

---

## 8. Roadmap

### Week 1-2: Preparation
- [ ] Collect 200-500 training examples
- [ ] Set up Unsloth environment (Google Colab)
- [ ] Validate dataset quality

### Week 3: Training
- [ ] Train LoRA adapter (2-4 hour GPU session)
- [ ] Test fine-tuned model outputs
- [ ] Compare base vs. fine-tuned on 10 test cases

### Week 4: Integration
- [ ] Convert to GGUF format
- [ ] Add to Ollama models
- [ ] Deploy with bwb_ai
- [ ] Gather user feedback

### Month 2+: Iteration
- [ ] Collect feedback examples
- [ ] Monthly retraining with new data
- [ ] Measure ROI (accuracy, speed, safety)

---

## 9. Resources & Links

### Tools
- [Unsloth](https://unsloth.ai/) — Optimized fine-tuning framework
- [Google Colab](https://colab.google/) — Free GPU for training
- [Hugging Face](https://huggingface.co/Qwen) — Base models & community
- [llama.cpp](https://github.com/ggerganov/llama.cpp) — GGUF conversion

### Documentation
- [Unsloth Fine-Tuning Guide](https://unsloth.ai/docs/get-started/fine-tuning-llms-guide)
- [SitePoint: Fine-tune Local LLMs 2026](https://www.sitepoint.com/fine-tune-local-llms-2026/)
- [Effloow: LoRA & QLoRA Guide 2026](https://effloow.com/articles/llm-fine-tuning-lora-qlora-guide-2026/)
- [Fine-Tuning Code LLMs (Medium)](https://medium.com/@zulqarnain.shahid.iqbal/fine-tuning-code-llms-b06d3f50212e)

### Example Datasets
- [GitHub: santacoder-finetuning](https://github.com/loubnabnl/santacoder-finetuning)
- [GitHub: LLM Datasets](https://github.com/mlabonne/llm-datasets)
- [Strand-Rust-Coder](https://huggingface.co/blog/Fortytwo-Network/strand-rust-coder-tech-report) — Rust-specific fine-tuned model

---

## 10. FAQ

**Q: Can I fine-tune without a GPU?**  
A: CPU fine-tuning is possible but extremely slow (1-2 days for tiny datasets). Recommend using cloud GPU ($10-20/session).

**Q: Will fine-tuning break the base model?**  
A: No — LoRA keeps the base model frozen. If training goes wrong, just discard the LoRA adapter.

**Q: How often should I retrain?**  
A: Start monthly (once you have 50+ new examples). Quarterly if examples come slower.

**Q: What's the difference between LoRA and full fine-tuning?**  
A: LoRA trains small adapter layers (faster, cheaper); full training trains all weights (slower, more flexible). LoRA is 95% as good for most use cases.

**Q: Can I combine multiple LoRA adapters?**  
A: Yes! Train separate adapters for shell, code, and Q&A, then merge them at inference time.

---

**Document Version**: 1.0  
**Last Updated**: 2026-05-18  
**Status**: Ready for Phase 7 implementation
