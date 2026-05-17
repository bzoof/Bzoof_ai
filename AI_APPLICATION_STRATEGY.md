# bwb_ai: AI Application Strategy

## 1. AI Problem Definition & Product Value

### Use Case
**Local AI Code Assistant with Document & Shell Integration**
- Enable developers to interact with a local LLM for code questions, documentation analysis, and safe command execution
- Provide offline-first AI assistance without cloud dependencies or API costs
- Support integration with developer workflows: PDF documentation, git repositories, shell commands

### Product Value
- **For the developer**: Real-time code suggestions, documentation Q&A, safe command execution in isolated environments
- **For the organization**: Offline capability, data privacy (no cloud transmission), reduced LLM API costs
- **For the industry**: Demonstrates viability of local LLM integration in developer tools

### Success Metrics
1. **Functional**: TUI responsiveness <200ms, streaming tokens in real-time, PDF loads complete in <30s
2. **Accuracy**: Model responses match intent (verified through manual testing with qwen2.5-coder:3b)
3. **Safety**: Shell commands only execute from allowlist; no path traversal or destructive operations
4. **Adoption**: User can configure and run locally without external dependencies

### Key Assumptions
- User has Ollama running locally with GGUF quantized models
- CPU-only inference is acceptable (i7-10710U, 15GB RAM)
- PDF documents fit in available context window via summarization
- User trusts qwen2.5-coder for code generation tasks

---

## 2. Data & Model Requirements

### Model Selection
**Current**: `qwen2.5-coder:3b` (1.9GB GGUF quantized)
- Optimized for code generation and reasoning
- Fast inference on CPU (~15-20 tokens/sec)
- Trade-off: Limited context window (2048 tokens default) vs. speed

**Alternatives to test**:
- `gemma3:4b` (3.3GB) — balanced quality/speed
- `qwen2.5:14b` (9GB) — higher quality, slower (5-8 tokens/sec)

### Data Strategy
1. **Conversation History**
   - Store locally in memory (VecDeque, max 20 messages)
   - Save to JSON on disk (Phase 1 extended)
   - Privacy: Zero transmission to external services

2. **PDF Documents**
   - Extract text via `pdf-extract` crate
   - Estimate tokens: `len / 4` heuristic
   - Split large docs into 3200-char chunks
   - Summarize chunks before context injection
   - Maximum 1800 tokens per injected summary

3. **Shell Command History**
   - Maintain ALLOWED_COMMANDS whitelist (27 commands)
   - Log execution in `/tmp` (ephemeral, auto-cleanup)
   - Output capped at 4096 bytes stdout + 512 stderr

### Model Behavior & Requirements
| Requirement | Target | Rationale |
|-------------|--------|-----------|
| Context window | 2048 tokens | Fits most code snippets + PDF summaries |
| Temperature | 0.7 (default) | Balance creativity (docs) vs. precision (code) |
| Token limit | 1000 tokens/response | Responsive UI, avoid timeout |
| Code quality | Syntactically valid | Test with simple prompts first |
| Document Q&A | Topic relevance >80% | Verify PDF context injection works |

---

## 3. AI Solution Design & Integration

### Architecture
```
┌─────────────────────────────────────────────────────┐
│                  TUI (Ratatui)                       │
│  Title │ Messages │ Input │ Status (ratatui layout) │
└──────────────────┬──────────────────────────────────┘
                   │
         ┌─────────┴──────────┬──────────────┐
         │                    │              │
    ┌────▼─────┐      ┌───────▼────┐   ┌────▼──────┐
    │  Chat    │      │   Shell    │   │ PDF       │
    │ Streaming│      │   Runner   │   │ Reader    │
    └────┬─────┘      └───────┬────┘   └────┬──────┘
         │                    │             │
    ┌────▼────────────────────▼─────────────▼────┐
    │         Llama Client (Ollama HTTP API)     │
    │  - chat() for non-streaming                │
    │  - chat_streaming() for token forwarding   │
    │  - inject_context() for PDF/system msgs    │
    └────┬─────────────────────────────────────┘
         │
    ┌────▼────────────────────────────────────┐
    │  Ollama Server (localhost:11434)         │
    │  - qwen2.5-coder:3b (default)            │
    │  - 6 CPU threads                         │
    └─────────────────────────────────────────┘
```

### Input/Output Flows

**Chat Flow** → User types message → LLM streams tokens in real-time → displayed in pending message
- Input: `String` (user message)
- Output: `UiEvent::LlmToken(String)` streamed, then `UiEvent::LlmDone`
- Error handling: `UiEvent::LlmError(String)`

**PDF Flow** → User `:load docs.pdf` → Extract + estimate tokens → Summarize if large → Inject as system context
- Input: `PathBuf` (file path)
- Output: `UiEvent::PdfLoaded(summary)` + injected system message in history
- Safety: spawn_blocking to avoid blocking TUI

**Shell Flow** → User `:run ls -la` → Validate whitelist → Execute with 10s timeout → Capture output
- Input: `String` (command)
- Output: `UiEvent::ShellOutput(result)` or `UiEvent::ShellError(reason)`
- Safety: Blocklist (rm -rf, cat /etc/shadow), metacharacter rejection, path traversal checks

### Safety & Control Mechanisms

| Risk | Mitigation |
|------|-----------|
| **Prompt injection** | System message injected as context, not user input |
| **Shell exploitation** | ALLOWED_COMMANDS whitelist (27 safe commands only) |
| **Path traversal** | Detect ".." in arguments, reject |
| **Command length** | Max 512 chars, max 20 args |
| **Infinite loops** | 10s timeout per command |
| **Memory bloat** | History capped at 20 messages, PDFs summarized |
| **Data leakage** | No cloud transmission; all local via localhost:11434 |

### User Interactions

**Command Syntax**
```
:load <path>           # Load PDF and inject context
:run <command>         # Execute whitelisted shell command
:save [file]           # Save conversation (Phase 1 extended)
:help                  # Show command list
:quit / Esc            # Exit gracefully
```

**Real-time Feedback**
- Spinner animates (⠙→⠹→⠸→⠼) while LLM thinks
- Tokens appear one-by-one as they generate
- Status bar updates: "Ready", "Loading PDF...", "Running...", "Error"

---

## 4. Validation & Monitoring

### Test Plan (Phase 3a + 3b Manual Testing)
- [ ] **Chat**: Verify tokens stream in real-time
- [ ] **PDF**: Load architecture doc, ask questions about content
- [ ] **Shell**: `:run ls -la` works, `:run rm -rf /` blocked
- [ ] **Input History**: ↑/↓ arrows navigate command history
- [ ] **Responsiveness**: No freezing during LLM inference
- [ ] **Error Handling**: Graceful recovery from Ollama disconnect

### Monitoring & Metrics
**Production (post-Phase 6 deployment)**
1. **Performance**
   - Token throughput (tokens/sec)
   - TUI input latency (ms to first response)
   - PDF processing time (sec per page)

2. **Quality**
   - Model response relevance (spot-check samples)
   - Shell command success rate (safe vs. blocked)
   - PDF Q&A accuracy (manual evaluation)

3. **Safety**
   - Blocked command attempts (log + alert)
   - Memory usage over time (leak detection)
   - Error rates per module (chat, shell, PDF)

4. **User Behavior**
   - Most-used commands (`:help`, `:run`, `:load`)
   - Session duration
   - Error recovery success rate

### Retraining & Iteration
- **Short-term** (monthly): Adjust temperature/context window based on user feedback
- **Medium-term** (quarterly): Evaluate alternative models (gemma3, qwen2.5:14b) on feature branches
- **Long-term** (annually): Fine-tune qwen2.5 on domain-specific code snippets if applicable

---

## 5. Implementation & Operational Readiness

### Architecture Decisions
| Decision | Rationale |
|----------|-----------|
| **Ollama HTTP API** | Vendor-agnostic, works with any GGUF model, simple REST protocol |
| **Streaming tokens** | Real-time UX, responsive feel like ChatGPT |
| **Local context injection** | PDF summaries in system message avoid prompt injection |
| **Whitelist shell cmds** | Safe default; users can request additions in Phase 6 |
| **Async event loop (tokio::select!)** | Non-blocking TUI, handles LLM + shell + input concurrently |

### Deployment Readiness
**Phase 4 (WebSocket Server)**
- Expose bwb_ai over WebSocket for editor plugins (Neovim, VSCode)
- Message format: `{cmd: "chat" | "load" | "run", payload: ...}`
- Auth: Local socket only (no remote auth needed yet)

**Phase 5 (Docker)**
- Multi-stage build: compile Rust → minimal runtime image
- Base: `rust:1.80` for build, `debian:bookworm-slim` for runtime
- Include Ollama service container (optional, or expect user-provided Ollama)
- Volume mounts: `/app/models`, `/app/history`

**Phase 6 (Editor Plugins)**
- Neovim: Integration via Lua plugin + WebSocket client
- VSCode: Extension with sidebar for chat, command palette for `:run`
- Requires WebSocket server from Phase 4

### Success Metrics (Implementation)
- [ ] All 17 unit tests pass (parse_command, shell_runner, pdf_reader)
- [ ] TUI builds without warnings (`cargo build --release`)
- [ ] Manual test suite: 5/5 phases working (chat, PDF, shell, history, exit)
- [ ] Documentation: PHASE3_TEST_PLAN.md + AI_APPLICATION_STRATEGY.md complete
- [ ] No unsafe code blocks (review with `cargo clippy`)

### Known Limitations & Future Work
1. **Token limit**: 2048 default may truncate long conversations
   - Mitigation: Implement chat history summarization (Phase 1 extended)
   
2. **Single-user local-only**: No multi-user or remote access yet
   - Mitigation: WebSocket server (Phase 4) enables remote clients
   
3. **No model fine-tuning**: Using base quantized model only
   - Mitigation: Evaluate domain-specific fine-tuning (Phase 6+)
   
4. **PDF summarization quality**: Depends entirely on LLM capability
   - Mitigation: Test with multiple models, document trade-offs

---

## 6. Stakeholder Alignment

### Product
- ✅ Offline-first, zero cloud dependency
- ✅ Privacy: No data leaves local machine
- ✅ Cost: One-time model download (~2GB), zero API spend

### Engineering
- ✅ Pure Rust implementation, no external runtime
- ✅ Async architecture scales to multiple concurrent tasks
- ✅ Comprehensive test coverage (17 unit tests)

### Operations
- ✅ Self-contained binary, no DevOps overhead (Phase 5)
- ✅ Model caching in ~/.ollama/models (standard Ollama dir)
- ✅ Logs to stderr/stdout (easily redirected)

### Users
- ✅ Familiar TUI (ratatui) similar to vim/tmux
- ✅ Local keyboard shortcuts (Esc to quit, ↑/↓ for history)
- ✅ Clear error messages and help text

---

## Next Steps

1. **Immediate** (This session):
   - ✅ Complete Phase 3a + 3b manual testing
   - ✅ Document AI strategy (this file)
   - [ ] Address any failing tests or TUI bugs

2. **Near-term** (Next 1-2 weeks):
   - [ ] Phase 4: Implement WebSocket server
   - [ ] Phase 5: Create Docker Dockerfile + docker-compose
   - [ ] Phase 6: Start Neovim plugin skeleton

3. **Medium-term** (Monthly):
   - [ ] Gather user feedback on model quality
   - [ ] Evaluate alternative models (gemma3, larger quantizations)
   - [ ] Implement conversation summarization (history management)

4. **Long-term** (Quarterly+):
   - [ ] Fine-tune model on code snippets (if dataset available)
   - [ ] Add RAG (retrieval-augmented generation) for documentation
   - [ ] Integrate with popular code repos (GitHub, internal git servers)

---

**Document Version**: 1.0  
**Last Updated**: 2026-05-18  
**Status**: Implementation phase (Phase 3 complete, Phases 4-6 planned)
