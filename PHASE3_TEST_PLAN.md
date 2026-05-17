# Phase 3a + 3b Test Verification

## Build Status ✅
- [x] `cargo build` — succeeds with expected warnings about unused Phase 4 code
- [x] `cargo test` — all 17 tests pass (8 parse_command + 6 shell_runner + 2 pdf_reader + 1 other)

## Architecture Changes
- **main.rs**: Refactored `handle_tui()` to use `tokio::select!` for async event loop
- **llama_interface.rs**: Added `inject_context()` to inject PDF content as system message
- **chat_ui.rs**: Fixed scroll offset with `.scroll()` on Paragraph widget and `scroll_to_bottom()` helper
- **pdf_reader.rs**: Implemented `extract_text()` using `pdf-extract` crate
- **Token streaming**: Separated token channel from UiEvent channel with forwarding task

## Manual Tests

### Test 1: Shell Runner (Phase 3a)
Launch the TUI and test these commands:

```bash
./target/debug/bwb_ai
```

In the TUI, execute:
1. `:run ls -la` → Should show current directory listing
2. `:run git status` → Should show git status
3. `:run pwd` → Should show current working directory
4. `:run rm -rf /` → Should be **blocked** with "not allowed" error
5. `:run cat /etc/shadow` → Should be **blocked** with "not allowed" error
6. `:run sleep 100 &` → Should be **blocked** with "not allowed" (sleep not in ALLOWED_COMMANDS)

**What to verify:**
- [ ] Output appears in a "Shell" message
- [ ] Allowed commands show real output
- [ ] Dangerous/blocked commands show error: "command not allowed"
- [ ] Status updates to "Ready" after execution

### Test 2: PDF Reader (Phase 3b)
In the same TUI session, execute:

```
:load docs/ai_agent_architecture.pdf
```

**What to verify:**
- [ ] Status shows "Loading PDF..."
- [ ] A "PDF" message appears with either:
  - `"PDF loaded (~XXX tokens). Ask me anything about it."` (if small <1800 tokens)
  - `"PDF is large (~XXX tokens, N chunks). Summarizing..."` then `"PDF summarized (N chunks)."` (if large)
- [ ] Can then ask questions about the PDF:
  ```
  What is the architecture of this AI agent?
  How does the TUI work?
  ```
- [ ] AI responses show knowledge of the PDF content

### Test 3: Real-time Token Streaming
Ask a longer question to verify tokens stream in real-time:

```
Write a 200-word essay about Rust programming
```

**What to verify:**
- [ ] Tokens appear one-by-one (not all at once)
- [ ] You can watch the response building in real-time
- [ ] Spinner rotates (⠙ → ⠹ → ⠸ → ⠼) while AI is thinking
- [ ] Message completes and status shows "Ready"

### Test 4: Input History
Test command history navigation:

```
Type: hello world
Press Enter
Type: :run ls -la
Press Enter
Type: something random
Press ↑ twice
```

**What to verify:**
- [ ] ↑ arrow shows `:run ls -la`
- [ ] ↑ arrow again shows `hello world`
- [ ] ↓ arrow navigates forward through history
- [ ] Clearing input and pressing ↑ from empty shows most recent

### Test 5: Combined Flow
Test the full workflow:

1. Load PDF: `:load docs/ai_agent_architecture.pdf`
2. Wait for summary to complete
3. Ask question: `Summarize the components of this system`
4. Run command: `:run git log --oneline | head -5`
5. Continue conversation based on PDF
6. Exit: Press Esc

**What to verify:**
- [ ] All features work together without conflicts
- [ ] Channel communication is clean (no message corruption)
- [ ] UI remains responsive during long operations

## Expected Output Examples

### PDF Loading (small PDF):
```
PDF: PDF loaded (~150 tokens). Ask me anything about it.
```

### PDF Loading (large PDF):
```
PDF: PDF is large (~2500 tokens, 2 chunks). Summarizing...
PDF: Summarizing chunk 1/2...
PDF: Summarizing chunk 2/2...
PDF: PDF summarized (2 chunks).
```

### Shell Command:
```
You: :run ls -la
Shell: total 128
drwxr-xr-x 5 user user 4096 May 18 14:32 .
drwxr-xr-x 10 user user 4096 May 18 10:00 ..
-rw-r--r-- 1 user user 1234 May 18 14:00 Cargo.toml
...
```

### Blocked Command:
```
You: :run rm -rf /
Shell Error: command not allowed
```

## Troubleshooting

| Issue | Solution |
|-------|----------|
| "Connection refused" | Start Ollama: `ollama serve` in another terminal |
| TUI looks corrupted | Resize terminal window or clear screen (`clear` then run again) |
| PDF summary takes too long | Model is slow. Try smaller model: `--model qwen2.5-coder:3b` |
| Tokens don't stream | Check Ollama is running with `curl localhost:11434/api/tags` |
| Shell command hangs | It likely hit the 10-second timeout. Press Ctrl+C to exit and try again. |

## Sign-Off

Once all tests above pass, Phase 3a + 3b is complete and ready for:
- **Phase 4**: WebSocket server for editor plugins
- **Phase 5**: Docker multi-stage build
- **Phase 6**: Neovim + VSCode integrations
