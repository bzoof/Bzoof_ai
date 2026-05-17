# Deployment Guide: bwb_ai with Fine-Tuned Models

This guide covers deploying bwb_ai with fine-tuned LoRA adapters in production.

---

## Local Deployment (Development)

### Setup Base Model
```bash
# Ensure Ollama is running
ollama serve &

# Pull base model
ollama pull qwen2.5-coder:3b

# Verify
ollama list  # Should show qwen2.5-coder:3b
```

### Deploy Fine-Tuned Model (LoRA)
```bash
# 1. Extract LoRA weights (from training)
unzip lora_adapter.zip -d ~/.ollama/models/lora/qwen-finetuned

# 2. Create Modelfile
cat > Modelfile << 'EOF'
FROM qwen2.5-coder:3b

# Fine-tuning metadata
PARAMETER temperature 0.7
PARAMETER top_p 0.9
PARAMETER repeat_penalty 1.1

# Optional: System prompt for fine-tuned model
SYSTEM """You are a helpful AI coding assistant specialized in Rust and shell scripting. \
You have been fine-tuned on bwb_ai use cases including PDF Q&A, code generation, and safe shell commands. \
Always prioritize safety when recommending commands."""
EOF

# 3. Register with Ollama (requires LoRA support)
# Note: As of 2026, Ollama integration with separate LoRA adapters is in development
# For now, use the combined model approach below
```

### Combined Model Approach (Current Best Practice)
```bash
# After training, export full fine-tuned model
# From Google Colab: Download the full merged model

# Convert to GGUF (if not already in GGUF format)
# python3 scripts/fine_tuning/convert_to_gguf.py finetuned_model finetuned_qwen.gguf

# Place in Ollama models directory
cp finetuned_qwen.gguf ~/.ollama/models/

# Create Modelfile
cat > Modelfile << 'EOF'
FROM ./finetuned_qwen.gguf

PARAMETER temperature 0.7
PARAMETER top_p 0.9
PARAMETER repeat_penalty 1.1
PARAMETER num_ctx 2048
EOF

# Register with Ollama
ollama create qwen2.5-coder:3b-finetuned -f Modelfile

# Verify
ollama list | grep finetuned
```

### Test with bwb_ai
```bash
# Build release binary
cd /home/bwb/Dev/Ai/bwb_ai
cargo build --release

# Run with fine-tuned model
./target/release/bwb_ai --model qwen2.5-coder:3b-finetuned

# Test commands:
# > Write a Rust function to parse JSON
# > :run ls -la
# > :load docs/ai_agent_architecture.pdf
# > What are the main components?
```

---

## Docker Deployment (Production)

### Dockerfile
```dockerfile
# Multi-stage build
FROM rust:1.80 as builder

WORKDIR /app

# Copy source
COPY . .

# Build release binary
RUN cargo build --release

# Runtime image
FROM debian:bookworm-slim

# Install dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /app/target/release/bwb_ai /usr/local/bin/bwb_ai

# Create non-root user
RUN useradd -m -u 1000 ai && \
    mkdir -p /home/ai/.ollama && \
    chown -R ai:ai /home/ai

USER ai

# Default to interactive TUI mode
ENTRYPOINT ["bwb_ai"]
CMD ["--help"]

EXPOSE 11434
VOLUME ["/home/ai/.ollama"]
```

### docker-compose.yml
```yaml
version: '3.8'

services:
  ollama:
    image: ollama/ollama:latest
    container_name: ollama
    environment:
      OLLAMA_HOST: 0.0.0.0:11434
    ports:
      - "11434:11434"
    volumes:
      - ollama_data:/root/.ollama
      - ./models:/models
    networks:
      - bwb_ai_network

  bwb_ai:
    build:
      context: .
      dockerfile: Dockerfile
    container_name: bwb_ai
    environment:
      RUST_LOG: info
    volumes:
      - bwb_ai_data:/home/ai/.bwb_ai
      - ./history:/home/ai/.bwb_ai/history
    depends_on:
      - ollama
    networks:
      - bwb_ai_network
    stdin_open: true
    tty: true
    entrypoint: /bin/bash
    command: -c "ollama pull qwen2.5-coder:3b && bwb_ai"

volumes:
  ollama_data:
  bwb_ai_data:

networks:
  bwb_ai_network:
    driver: bridge
```

### Build and Run Docker
```bash
# Build image
docker build -t bwb_ai:latest .

# Run with compose
docker-compose up -d

# View logs
docker-compose logs -f bwb_ai

# Interactive shell
docker-compose run --rm bwb_ai bwb_ai --help

# Stop
docker-compose down
```

---

## WebSocket Server Deployment (Phase 4)

### Configuration
```bash
# Environment variables for WebSocket mode
export BWB_AI_WS_HOST=0.0.0.0
export BWB_AI_WS_PORT=8080
export BWB_AI_MODEL=qwen2.5-coder:3b-finetuned

# Run with WebSocket server
bwb_ai --ws

# Connect from client
wscat -c ws://localhost:8080
```

### Nginx Reverse Proxy
```nginx
upstream bwb_ai_backend {
    server localhost:8080;
}

server {
    listen 443 ssl;
    server_name ai.example.com;

    ssl_certificate /etc/letsencrypt/live/ai.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/ai.example.com/privkey.pem;

    location / {
        proxy_pass http://bwb_ai_backend;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    }
}
```

---

## Monitoring & Metrics

### Prometheus Metrics
```rust
// Add metrics collection (future work)
use prometheus::{Counter, Histogram};

lazy_static::lazy_static! {
    pub static ref CHAT_REQUESTS: Counter = Counter::new("bwb_ai_chat_requests_total", "Total chat requests").unwrap();
    pub static ref TOKENS_GENERATED: Counter = Counter::new("bwb_ai_tokens_total", "Total tokens generated").unwrap();
    pub static ref INFERENCE_DURATION: Histogram = Histogram::new("bwb_ai_inference_seconds", "Inference latency").unwrap();
}
```

### Health Check Endpoint
```bash
# Add to WebSocket server
GET /health → 200 OK { "status": "ready", "model": "qwen2.5-coder:3b-finetuned" }

# Test
curl http://localhost:8080/health
```

### Logging
```bash
# Set log level
export RUST_LOG=debug

# View logs
docker-compose logs -f --tail=100 bwb_ai
```

---

## Backup & Recovery

### Model Backup
```bash
# Backup fine-tuned model
tar -czf bwb_ai_model_backup.tar.gz ~/.ollama/models/finetuned_qwen.gguf

# Backup conversation history
tar -czf bwb_ai_history_backup.tar.gz ~/.bwb_ai/history/

# Upload to cloud storage
aws s3 cp bwb_ai_model_backup.tar.gz s3://my-backups/bwb_ai/
```

### Recovery
```bash
# Restore from backup
tar -xzf bwb_ai_model_backup.tar.gz -C ~/.ollama/models/

# Verify
ollama list | grep finetuned
```

---

## Performance Tuning

### CPU Optimization
```bash
# Set thread count (default 6)
bwb_ai --model qwen2.5-coder:3b --num-threads 8

# Monitor performance
time bwb_ai --model qwen2.5-coder:3b --one-shot "Hello, world"
```

### Memory Management
```bash
# Monitor memory usage
docker stats bwb_ai

# Limit memory
docker-compose.yml:
  bwb_ai:
    mem_limit: 4g
    memswap_limit: 4g
```

### Context Window Optimization
```bash
# Larger context for more complex queries
bwb_ai --model qwen2.5-coder:3b --context 4096

# Smaller context for faster inference
bwb_ai --model qwen2.5-coder:3b --context 1024
```

---

## Security Considerations

### Shell Command Safety
- Always use ALLOWED_COMMANDS whitelist
- Never auto-execute shell commands
- Log all shell command attempts
- Review command output for safety

### PDF Handling
- Validate PDF files before processing
- Limit file size (max 50MB recommended)
- Scan for malicious content
- Store in secure directory

### Model Safety
- Use quantized models (GGUF) to prevent prompt injection
- Validate model source before deployment
- Keep Ollama and dependencies updated
- Monitor for anomalous outputs

### Network Security
- Use HTTPS/WSS in production
- Require authentication (Phase 5+)
- Rate limit API endpoints
- Log all requests

---

## Troubleshooting

### Model Not Found
```bash
# Error: "Model not found"
ollama pull qwen2.5-coder:3b
ollama list
```

### Connection Refused
```bash
# Error: "Failed to connect to Ollama"
# Check Ollama is running
ollama serve &

# Check port is accessible
curl http://localhost:11434/api/tags
```

### Memory Issues
```bash
# Error: "Out of memory"
# Reduce context window
bwb_ai --context 1024

# Use smaller model
bwb_ai --model qwen2.5-coder:3b  # (already using smallest)
```

### Slow Inference
```bash
# Check thread count
bwb_ai --model qwen2.5-coder:3b --num-threads 6

# Profile inference time
time bwb_ai --one-shot "test"
```

---

## Next Steps

1. **Immediate**: Deploy base model locally
2. **Week 1**: Train fine-tuned model (Phase 7)
3. **Week 2**: Deploy fine-tuned model
4. **Month 1**: Docker containerization (Phase 5)
5. **Month 2**: WebSocket server (Phase 4)
6. **Month 3**: Editor plugins (Phase 6)

---

**Document Version**: 1.0  
**Last Updated**: 2026-05-18  
**Status**: Ready for production deployment
