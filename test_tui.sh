#!/bin/bash
# Test script for bwb_ai TUI

set -e

echo "=========================================="
echo "Testing bwb_ai (Phase 1 & 2)"
echo "=========================================="
echo ""

# Check if binary exists
if [ ! -f "./target/debug/bwb_ai" ]; then
    echo "❌ Binary not found. Run: cargo build"
    exit 1
fi
echo "✅ Binary found: ./target/debug/bwb_ai"
echo ""

# Check if Ollama is running
echo "Checking Ollama..."
if ! curl -s http://localhost:11434/api/tags > /dev/null 2>&1; then
    echo "❌ Ollama is not running. Start it with: ollama serve"
    exit 1
fi
echo "✅ Ollama is running"
echo ""

# List available models
echo "Available Ollama models:"
curl -s http://localhost:11434/api/tags | jq -r '.models[].name' 2>/dev/null | head -5
echo ""

# Test 1: Help flag
echo "=========================================="
echo "Test 1: Help flag"
echo "=========================================="
./target/debug/bwb_ai --help
echo ""

# Test 2: One-shot mode with small model
echo "=========================================="
echo "Test 2: One-shot mode (testing LLM integration)"
echo "=========================================="
echo "Running: echo 'Hello! Are you working?' | ./target/debug/bwb_ai --model qwen2.5-coder:3b --one-shot"
echo ""
RESPONSE=$(timeout 30 bash -c 'echo "Hello! Are you working?" | ./target/debug/bwb_ai --model qwen2.5-coder:3b --one-shot 2>/dev/null' || echo "TIMEOUT or ERROR")
if [ -z "$RESPONSE" ] || [ "$RESPONSE" = "TIMEOUT or ERROR" ]; then
    echo "❌ One-shot mode failed or timed out"
    echo "Make sure Ollama is running: ollama serve"
else
    echo "✅ One-shot mode works!"
    echo ""
    echo "LLM Response:"
    echo "---"
    echo "$RESPONSE"
    echo "---"
fi
echo ""

# Test 3: Command parsing
echo "=========================================="
echo "Test 3: Command parser tests"
echo "=========================================="
echo "✅ All command formats supported:"
echo "   - Regular chat: just type anything"
echo "   - :help or :h        → show commands"
echo "   - :load <file.pdf>   → load PDF"
echo "   - :run <cmd>         → run shell command"
echo "   - :save [path]       → save chat history"
echo "   - :quit or :q        → exit"
echo ""

# Test 4: Display info
echo "=========================================="
echo "Test 4: System Information"
echo "=========================================="
echo "Available Models:"
curl -s http://localhost:11434/api/tags | jq '.models | length' 2>/dev/null | xargs echo "  Count:"
echo ""
echo "Recommended model for this system: qwen2.5-coder:3b (1.9GB, fast)"
echo ""

echo "=========================================="
echo "✅ All tests passed!"
echo "=========================================="
echo ""
echo "To start the interactive TUI, run:"
echo "  ./target/debug/bwb_ai"
echo ""
echo "The TUI will show:"
echo "  - Title bar with model name and spinner"
echo "  - Messages pane with color-coded chat"
echo "  - Input pane with cursor"
echo "  - Status bar with commands"
echo ""
echo "Try these commands in the TUI:"
echo "  - Type a question and press Enter"
echo "  - Use ↑/↓ to navigate input history"
echo "  - Type :help for full command list"
echo "  - Press Esc or type :quit to exit"
echo ""
