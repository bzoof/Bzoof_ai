# Phase 4: WebSocket Server Implementation

## Overview

Phase 4 implements a WebSocket server that allows remote clients (editor plugins, web clients) to communicate with bwb_ai over a network connection.

**Goals**:
- ✅ Accept WebSocket connections from remote clients
- ✅ Route messages (chat, PDF, shell) to appropriate handlers
- ✅ Stream responses back to clients in real-time
- ✅ Handle multiple concurrent connections
- ✅ Support authentication (Phase 4.5)
- ✅ Error handling and graceful disconnects

---

## Architecture

### Message Flow

```
Editor Plugin (Neovim/VSCode)
    ↓ WebSocket Connection
WSS://localhost:8080
    ↓
┌─────────────────────────────────────┐
│     WebSocket Server (axum)         │
│  • Handle connections                │
│  • Route messages                    │
│  • Broadcast responses               │
└─────────────────────────────────────┘
    ↓
┌─────────────────────────────────────┐
│   Message Handlers (LLM, Shell, PDF) │
│  • Chat → LlamaClient                │
│  • Shell → ShellRunner               │
│  • PDF → PdfReader                   │
└─────────────────────────────────────┘
    ↓
    Response (tokens/errors/results)
    ↓
Editor Plugin (real-time updates)
```

### Protocol

**Client → Server**:
```json
{
  "type": "chat",
  "content": "What is Rust?"
}

{
  "type": "load_pdf",
  "path": "/path/to/document.pdf"
}

{
  "type": "run",
  "command": "ls -la"
}
```

**Server → Client**:
```json
{
  "type": "token",
  "content": "Rust"
}

{
  "type": "token",
  "content": " is"
}

{
  "type": "done",
  "content": "Complete response text..."
}

{
  "type": "error",
  "content": "Error message"
}

{
  "type": "shell",
  "content": "Command output"
}
```

---

## Implementation Plan

### 1. Dependencies (Update Cargo.toml)

```toml
[dependencies]
# Existing...
tokio-tungstenite = "0.23"  # WebSocket protocol
uuid = { version = "1.0", features = ["v4", "serde"] }  # Client IDs
dashmap = "5.5"  # Concurrent hashmap for connections
```

### 2. Core WebSocket Handler (ws_server.rs)

**Structure**:
```rust
pub struct WsServer {
    addr: SocketAddr,
    llama_client: Arc<Mutex<LlamaClient>>,
    clients: Arc<DashMap<String, WebSocketSender>>,
}

impl WsServer {
    pub async fn run(addr: SocketAddr, config: LlamaConfig) -> Result<()> {
        // 1. Create TCP listener
        // 2. Accept connections
        // 3. Handle each connection in spawn task
        // 4. Parse messages and route
    }

    async fn handle_connection(&self, ws: WebSocket) -> Result<()> {
        // 1. Generate client ID
        // 2. Store client sender
        // 3. Listen for messages
        // 4. Route to handlers
    }

    async fn handle_chat(&self, client_id: &str, content: String) -> Result<()> {
        // 1. Lock LlamaClient
        // 2. Stream tokens
        // 3. Send ServerMessage::Token for each
        // 4. Send ServerMessage::Done when complete
    }

    async fn handle_load_pdf(&self, client_id: &str, path: String) -> Result<()> {
        // 1. Extract PDF text
        // 2. Inject context
        // 3. Send status updates
    }

    async fn handle_run(&self, client_id: &str, command: String) -> Result<()> {
        // 1. Validate command
        // 2. Execute
        // 3. Send output
    }
}
```

### 3. Connection Management

**Per-Client State**:
- Client ID (UUID)
- WebSocket sender
- Active request (optional)
- Last activity timestamp

**Global State**:
- Active connections (DashMap)
- Shared LlamaClient (Arc<Mutex>)
- Request queue (optional, for rate limiting)

### 4. Message Parsing & Routing

```rust
enum ClientMessage {
    Chat { content: String },
    LoadPdf { path: String },
    Run { command: String },
}

enum ServerMessage {
    Token { content: String },
    Done { content: String },
    Error { content: String },
    Shell { content: String },
}

// Routing logic
match message {
    ClientMessage::Chat { content } => handle_chat(content).await,
    ClientMessage::LoadPdf { path } => handle_load_pdf(path).await,
    ClientMessage::Run { command } => handle_run(command).await,
}
```

### 5. Error Handling

**Connection Errors**:
- Send `ServerMessage::Error` to client
- Log error
- Continue listening

**Handler Errors**:
- Send `ServerMessage::Error` with description
- Don't disconnect client
- Allow retry

**Graceful Shutdown**:
- Store all client senders
- When server shuts down, send goodbye message
- Close all connections

### 6. Testing Strategy

**Unit Tests**:
```rust
#[tokio::test]
async fn test_chat_streaming() {
    // 1. Create server
    // 2. Connect client
    // 3. Send chat message
    // 4. Verify tokens received
    // 5. Verify done message
}

#[tokio::test]
async fn test_pdf_loading() {
    // 1. Send load_pdf message
    // 2. Verify status updates
    // 3. Verify context injected
}

#[tokio::test]
async fn test_shell_command() {
    // 1. Send run message
    // 2. Verify output
    // 3. Verify blocked commands rejected
}

#[tokio::test]
async fn test_concurrent_clients() {
    // 1. Connect multiple clients
    // 2. Send simultaneous requests
    // 3. Verify isolation (no cross-talk)
}
```

**Integration Tests**:
```bash
# Test with real WebSocket client
wscat -c ws://localhost:8080
> {"type": "chat", "content": "Hello"}
< {"type": "token", "content": "Hello"}
< {"type": "token", "content": " "}
...
```

---

## File Structure

```
src/
├── ws_server.rs           ← WebSocket server implementation
├── ws_handlers.rs         ← Message handlers (NEW)
├── ws_messages.rs         ← Message types (NEW - move enums here)
├── main.rs                ← Add --ws flag handling
└── ...existing files

tests/
└── websocket_integration.rs (NEW)
```

---

## Integration with main.rs

**Current**:
```rust
#[arg(long)]
ws: bool,

if args.ws {
    eprintln!("WebSocket server not yet implemented (Phase 4)");
}
```

**After Phase 4**:
```rust
#[arg(long)]
ws: bool,

#[arg(long, default_value = "127.0.0.1:8080")]
ws_addr: String,

if args.ws {
    let addr = args.ws_addr.parse::<SocketAddr>()?;
    WsServer::run(addr, config).await?;
}
```

---

## Security Considerations

### 1. Authentication (Phase 4.5)

**Options**:
- Token-based (header: `Authorization: Bearer TOKEN`)
- API key (first message authentication)
- TLS/WSS (encrypted connection)

**Implementation**:
```rust
// Each connection must send auth before other messages
enum FirstMessage {
    Auth { token: String },
}

// If not authenticated within timeout, disconnect
```

### 2. Rate Limiting

**Per-client limits**:
- Max 10 requests/second
- Max request size: 1MB
- Max connections from same IP: 5

### 3. Command Injection Prevention

- Use existing ShellRunner validation
- Whitelist-only commands
- No command composition over WebSocket

### 4. TLS/WSS

**Future enhancement**:
```rust
// For production use HTTPS/WSS
let listener = TcpListener::bind(addr).await?;
let stream = listener.accept().await?;
let ws_stream = tokio_tungstenite::accept_async(stream).await?;
```

---

## Step-by-Step Implementation

### Step 1: Add Dependencies (10 min)
- [ ] Update Cargo.toml
- [ ] Run `cargo check`

### Step 2: Refactor Message Types (15 min)
- [ ] Create ws_messages.rs
- [ ] Move ClientMessage/ServerMessage enums
- [ ] Import in ws_server.rs

### Step 3: Implement Core Server (1 hour)
- [ ] Create WsServer struct
- [ ] Implement TCP listener
- [ ] Handle WebSocket connections
- [ ] Parse incoming messages

### Step 4: Implement Handlers (1.5 hours)
- [ ] Chat message handler (token streaming)
- [ ] PDF loading handler
- [ ] Shell command handler
- [ ] Error handling for all

### Step 5: Connection Management (30 min)
- [ ] Track active connections
- [ ] Handle disconnects
- [ ] Graceful shutdown
- [ ] Broadcast support (optional)

### Step 6: Testing (1 hour)
- [ ] Unit tests for handlers
- [ ] Integration tests with client
- [ ] Load test with multiple clients
- [ ] Error scenarios

### Step 7: Documentation (30 min)
- [ ] API documentation
- [ ] Example client implementations
- [ ] Deployment guide

### Step 8: Integration & Cleanup (30 min)
- [ ] Update main.rs
- [ ] Handle --ws flag
- [ ] Remove todo!() calls
- [ ] Run full test suite

**Total**: ~5-6 hours development

---

## Expected Outcomes

✅ **After Phase 4**:
- WebSocket server listens on port 8080
- Multiple clients can connect simultaneously
- Real-time token streaming
- Full chat, PDF, shell capabilities over network
- Production-ready error handling
- Comprehensive tests

✅ **Enables Phase 5/6**:
- Editor plugins can connect remotely
- CI/CD integration
- Web-based UI
- Distributed deployments

---

## Testing Commands

```bash
# Start server (Terminal 1)
cargo run -- --ws

# Test with wscat (Terminal 2)
npm install -g wscat
wscat -c ws://127.0.0.1:8080

# Send chat message
> {"type": "chat", "content": "What is Rust?"}

# Receive tokens
< {"type": "token", "content": "Rust"}
< {"type": "token", "content": " is"}
...
< {"type": "done", "content": "Full response..."}

# Test PDF
> {"type": "load_pdf", "path": "docs/ai_agent_architecture.pdf"}
< {"type": "token", "content": "PDF loaded..."}

# Test shell
> {"type": "run", "command": "ls -la"}
< {"type": "shell", "content": "drwxr-xr-x..."}
```

---

## Success Criteria

- [ ] WebSocket server accepts connections
- [ ] Messages are parsed correctly
- [ ] Chat tokens stream in real-time
- [ ] PDF loading works over WebSocket
- [ ] Shell commands execute safely
- [ ] Multiple concurrent clients work
- [ ] Error messages are descriptive
- [ ] Graceful handling of disconnects
- [ ] 95%+ test coverage
- [ ] No unsafe blocks
- [ ] Documentation complete

---

**Estimated Effort**: 5-6 hours  
**Difficulty**: Medium (async networking + message routing)  
**Status**: Ready to start implementation

Let's build Phase 4! 🚀
