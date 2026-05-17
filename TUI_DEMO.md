# bwb_ai TUI Demo & Testing Guide

## Test Results ✅

All systems operational:
- ✅ Binary built successfully
- ✅ Ollama integration working  
- ✅ LLM responses verified
- ✅ Command parser functional
- ✅ 15 models available on system

## What the TUI Looks Like

When you run `./target/debug/bwb_ai`, you'll see:

```
┌─────────────────────────────────────────────────────────────┐
│ bwb_ai — Local AI Agent          [qwen2.5-coder:3b] ⠙      │  ← Title with spinner
├─────────────────────────────────────────────────────────────┤
│ You: What is Rust?                                          │
│   (colored green)                                           │
│                                                             │
│ AI: Rust is a systems programming language that focuses... │  ← Messages with formatting
│   (colored cyan)                                            │
│   Tokens arrive real-time as the LLM generates them         │
│                                                             │
│ You: Can I use it for web development?                     │
│                                                             │
│ AI: Yes! Rust has excellent web frameworks like...          │
├─────────────────────────────────────────────────────────────┤
│ > type your message here_                                   │  ← Input with cursor
├─────────────────────────────────────────────────────────────┤
│ Ready  [:help for commands] [Esc=quit]                      │  ← Status bar
└─────────────────────────────────────────────────────────────┘
```

## Features Demonstrated

### ✨ Real-time Token Streaming
- Each token from the LLM appears instantly
- You see the response building in real-time (like ChatGPT)
- No waiting for the full response before seeing anything

### 🎨 Styled Interface
- **User messages**: Green & bold
- **AI messages**: Cyan & bold
- **System messages**: Gray
- **Spinner**: Yellow animated character while AI is thinking

### ⌨️ Input Handling
- **Type normally**: Messages are sent as chat
- **↑/↓ Arrow keys**: Browse through your last 50 inputs
- **Backspace**: Delete characters
- **Enter**: Submit message
- **Esc**: Quit the program

### 📋 Commands
Type these in the chat:
- `:help` → Show available commands
- `:load file.pdf` → Load a PDF (Phase 3b)
- `:run ls -la` → Run a shell command (Phase 3a)
- `:save` → Save conversation history
- `:quit` → Exit cleanly

## How to Test It

### Step 1: Verify Ollama is running
```bash
curl http://localhost:11434/api/tags
# Should return: {"models":[...]}
```

If not running:
```bash
ollama serve  # in a separate terminal
```

### Step 2: Pick a model to test with
```bash
ollama list  # Shows all available models
```

Recommended for testing:
- `qwen2.5-coder:3b` — fastest (1.9GB)
- `gemma3:4b` — balanced (3.3GB)  
- `qwen2.5:14b` — more capable (9GB)

### Step 3: Launch the TUI
```bash
# Default (uses qwen2.5-coder:3b):
./target/debug/bwb_ai

# With specific model:
./target/debug/bwb_ai --model qwen2.5:14b --context 4096

# With custom temperature (0-1, default 0.7):
./target/debug/bwb_ai --temperature 0.9
```

### Step 4: Test the chat
In the TUI, try:
1. **Simple chat**: Type "What is 2+2?" and press Enter
2. **Code generation**: Type "Write a Hello World in Rust"
3. **Input history**: Press ↑ to see your previous messages
4. **Help**: Type `:help` to see all commands
5. **Exit**: Press Esc or type `:quit`

## What to Look For

When testing, verify:
- [ ] **Title bar** shows the model name
- [ ] **Spinner** appears and rotates while AI responds (⠙ → ⠹ → ⠸ → ⠼)
- [ ] **Messages** appear in color (user=green, AI=cyan)
- [ ] **Tokens stream in real-time** (not all at once)
- [ ] **Cursor blinks** in the input box
- [ ] **Up/Down arrows** navigate command history
- [ ] **Esc key** or `:quit` exits cleanly
- [ ] **Status bar** shows hints

## Performance Notes

On your system (i7-10710U, 15GB RAM):
- **qwen2.5-coder:3b**: ~15-20 tokens/second
- **gemma3:4b**: ~12-15 tokens/second
- **qwen2.5:14b**: ~5-8 tokens/second

Larger models are slower but more capable. Start with 3b for responsiveness.

## Troubleshooting

### "Connection refused" error
→ Ollama is not running. Start it: `ollama serve`

### TUI crashes or display issues
→ This is expected if piping input (use interactive mode instead)
→ Run in a proper terminal without pipes

### Response takes forever
→ Model is too large for your CPU
→ Try a smaller model: `--model qwen2.5-coder:3b`

### Input doesn't appear
→ Raw mode is active. Just keep typing. It works.

## Next Steps After Testing

If the TUI works well, we're ready for:
- **Phase 3a**: Integrate shell runner with safe commands
- **Phase 3b**: Add PDF loading and summarization
- **Phase 4**: WebSocket server for editor plugins
- **Phase 5**: Docker containerization
- **Phase 6**: Neovim + VSCode editor plugins

---

**Happy testing!** 🚀
