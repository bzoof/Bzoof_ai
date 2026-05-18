use anyhow::{anyhow, Result};
use futures::stream::StreamExt;
use futures::sink::SinkExt;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tokio_tungstenite::WebSocketStream;
use uuid::Uuid;

use crate::llama_interface::{LlamaClient, LlamaConfig};
use crate::ws_messages::{ClientMessage, ServerMessage};

type WsSender = Arc<Mutex<futures::stream::SplitSink<WebSocketStream<TcpStream>, WsMessage>>>;

/// WebSocket server for remote client connections
pub struct WsServer {
    addr: SocketAddr,
    config: LlamaConfig,
}

impl WsServer {
    /// Create a new WebSocket server
    pub fn new(addr: SocketAddr, config: LlamaConfig) -> Self {
        Self { addr, config }
    }

    /// Run the WebSocket server
    pub async fn run(addr: SocketAddr, config: LlamaConfig) -> Result<()> {
        let server = Self::new(addr, config);
        server.start().await
    }

    /// Start listening for connections
    async fn start(self) -> Result<()> {
        let listener = TcpListener::bind(self.addr).await?;
        tracing::info!("WebSocket server listening on {}", self.addr);

        loop {
            let (stream, peer_addr) = listener.accept().await?;
            tracing::debug!("New connection from {}", peer_addr);

            let config = self.config.clone();
            tokio::spawn(async move {
                if let Err(e) = Self::handle_connection(stream, config).await {
                    tracing::error!("Connection error from {}: {}", peer_addr, e);
                }
            });
        }
    }

    /// Handle a single client connection
    async fn handle_connection(stream: TcpStream, config: LlamaConfig) -> Result<()> {
        let client_id = Uuid::new_v4().to_string();
        tracing::debug!("Upgrading connection to WebSocket: {}", client_id);

        // Upgrade to WebSocket
        let ws_stream = tokio_tungstenite::accept_async(stream)
            .await
            .map_err(|e| anyhow!("WebSocket upgrade failed: {}", e))?;

        tracing::info!("WebSocket connection established: {}", client_id);

        // Create LLM client for this connection
        let llama_client = Arc::new(Mutex::new(LlamaClient::new(config).await?));

        // Send ready message
        let ready_msg = ServerMessage::ready(
            &client_id,
            "Connected to bwb_ai. Ready to chat, load PDFs, or run commands.",
        );
        let ready_json = ready_msg.to_json()?;
        let ready_ws = WsMessage::Text(ready_json.into());

        let (tx, mut rx) = ws_stream.split();
        let tx: WsSender = Arc::new(Mutex::new(tx));

        // Send ready message
        tx.lock()
            .await
            .send(ready_ws)
            .await
            .map_err(|e| anyhow!("Failed to send ready message: {}", e))?;

        // Listen for messages from client
        while let Some(result) = rx.next().await {
            let msg = match result {
                Ok(msg) => msg,
                Err(e) => {
                    tracing::error!("WebSocket error: {}", e);
                    break;
                }
            };

            match msg {
                WsMessage::Text(text) => {
                    if let Err(e) = Self::handle_message(&text, &client_id, &llama_client, &tx)
                        .await
                    {
                        tracing::error!("Message handling error: {}", e);
                        let error_msg = ServerMessage::error(e.to_string());
                        let _ = tx
                            .lock()
                            .await
                            .send(WsMessage::Text(error_msg.to_json()?.into()))
                            .await;
                    }
                }
                WsMessage::Close(_) => {
                    tracing::info!("Client disconnected: {}", client_id);
                    break;
                }
                WsMessage::Ping(data) => {
                    let pong = WsMessage::Pong(data);
                    let _ = tx.lock().await.send(pong).await;
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Handle a message from a client
    async fn handle_message(
        text: &str,
        client_id: &str,
        llama_client: &Arc<Mutex<LlamaClient>>,
        tx: &WsSender,
    ) -> Result<()> {
        let msg = ClientMessage::from_text(text)?;
        tracing::debug!("Received message from {}: {:?}", client_id, msg);

        match msg {
            ClientMessage::Chat { content } => {
                Self::handle_chat(client_id, &content, llama_client, tx).await?;
            }
            ClientMessage::LoadPdf { path } => {
                Self::handle_load_pdf(client_id, &path, llama_client, tx).await?;
            }
            ClientMessage::Run { command } => {
                Self::handle_run(client_id, &command, tx).await?;
            }
            ClientMessage::Ping => {
                let pong = ServerMessage::Pong;
                let msg = WsMessage::Text(pong.to_json()?.into());
                tx.lock().await.send(msg).await?;
            }
        }

        Ok(())
    }

    /// Handle chat message
    async fn handle_chat(
        client_id: &str,
        content: &str,
        llama_client: &Arc<Mutex<LlamaClient>>,
        tx: &WsSender,
    ) -> Result<()> {
        tracing::debug!("Chat request from {}: {}", client_id, content);

        let (token_tx, mut token_rx) = tokio::sync::mpsc::channel(256);
        let tx_clone = tx.clone();

        // Spawn task to forward tokens
        let forward_tokens = tokio::spawn(async move {
            while let Some(token) = token_rx.recv().await {
                let msg = ServerMessage::token(&token);
                if let Ok(json) = msg.to_json() {
                    let _ = tx_clone
                        .lock()
                        .await
                        .send(WsMessage::Text(json.into()))
                        .await;
                }
            }
        });

        // Execute chat with streaming
        let mut client = llama_client.lock().await;
        match client.chat_streaming(content, token_tx).await {
            Ok(()) => {
                let _ = forward_tokens.await;
                let done_msg = ServerMessage::Done {
                    content: "Chat completed".to_string(),
                };
                let msg = WsMessage::Text(done_msg.to_json()?.into());
                tx.lock().await.send(msg).await?;
            }
            Err(e) => {
                let error_msg = ServerMessage::error(e.to_string());
                let msg = WsMessage::Text(error_msg.to_json()?.into());
                tx.lock().await.send(msg).await?;
            }
        }

        Ok(())
    }

    /// Handle PDF load message
    async fn handle_load_pdf(
        client_id: &str,
        path: &str,
        llama_client: &Arc<Mutex<LlamaClient>>,
        tx: &WsSender,
    ) -> Result<()> {
        use crate::pdf_reader::PdfReader;
        use std::path::PathBuf;

        tracing::debug!("PDF load request from {}: {}", client_id, path);

        let status_msg = ServerMessage::pdf_status("Loading PDF...");
        tx.lock()
            .await
            .send(WsMessage::Text(status_msg.to_json()?.into()))
            .await?;

        let path_buf = PathBuf::from(path);

        // Extract PDF text on blocking thread
        let text = match tokio::task::spawn_blocking(move || {
            PdfReader::extract_text(&path_buf)
        })
        .await
        {
            Ok(Ok(t)) => t,
            Ok(Err(e)) => {
                let error_msg = ServerMessage::error(format!("PDF error: {}", e));
                let _ = tx
                    .lock()
                    .await
                    .send(WsMessage::Text(error_msg.to_json()?.into()))
                    .await;
                return Ok(());
            }
            Err(e) => {
                let error_msg = ServerMessage::error(format!("PDF task error: {}", e));
                let _ = tx
                    .lock()
                    .await
                    .send(WsMessage::Text(error_msg.to_json()?.into()))
                    .await;
                return Ok(());
            }
        };

        let token_count = PdfReader::estimate_tokens(&text);

        if token_count <= 1800 {
            // Fits in context — inject directly
            let mut client = llama_client.lock().await;
            client.inject_context(format!("The following document has been loaded:\n\n{}", text));
            let status_msg = ServerMessage::pdf_status(format!(
                "PDF loaded (~{} tokens). Ask me anything about it.",
                token_count
            ));
            let _ = tx
                .lock()
                .await
                .send(WsMessage::Text(status_msg.to_json()?.into()))
                .await;
        } else {
            // Too large — summarize in chunks
            let chunks = PdfReader::chunk_text(&text, 3200);
            let total = chunks.len();

            let status_msg = ServerMessage::pdf_status(format!(
                "PDF is large (~{} tokens, {} chunks). Summarizing...",
                token_count, total
            ));
            let _ = tx
                .lock()
                .await
                .send(WsMessage::Text(status_msg.to_json()?.into()))
                .await;

            let mut summary = String::new();
            for (i, chunk) in chunks.iter().enumerate() {
                let prompt = if i == 0 {
                    format!("Summarize this document section:\n\n{}", chunk)
                } else {
                    format!(
                        "Continue summarizing. So far: {}\n\nNext section:\n\n{}",
                        summary, chunk
                    )
                };

                let status_msg = ServerMessage::pdf_status(format!(
                    "Summarizing chunk {}/{}...",
                    i + 1,
                    total
                ));
                let _ = tx
                    .lock()
                    .await
                    .send(WsMessage::Text(status_msg.to_json()?.into()))
                    .await;

                let mut client = llama_client.lock().await;
                match client.chat(&prompt).await {
                    Ok(r) => {
                        summary = r;
                    }
                    Err(e) => {
                        let error_msg = ServerMessage::error(format!("Summary error: {}", e));
                        let _ = tx
                            .lock()
                            .await
                            .send(WsMessage::Text(error_msg.to_json()?.into()))
                            .await;
                        return Ok(());
                    }
                }
            }

            let mut client = llama_client.lock().await;
            client.inject_context(format!("Document summary:\n\n{}", summary));
            let status_msg =
                ServerMessage::pdf_status(format!("PDF summarized ({} chunks).", total));
            let _ = tx
                .lock()
                .await
                .send(WsMessage::Text(status_msg.to_json()?.into()))
                .await;
        }

        Ok(())
    }

    /// Handle shell run message
    async fn handle_run(
        client_id: &str,
        command: &str,
        tx: &WsSender,
    ) -> Result<()> {
        use crate::shell_runner::ShellRunner;

        tracing::debug!("Shell run request from {}: {}", client_id, command);

        let tx_clone = tx.clone();
        let cmd = command.to_string();

        // Spawn shell command execution
        tokio::spawn(async move {
            match ShellRunner::sanitize_command(&cmd) {
                Ok((prog, args)) => {
                    let output = match tokio::time::timeout(
                        std::time::Duration::from_secs(10),
                        tokio::process::Command::new(&prog)
                            .args(&args)
                            .stdout(std::process::Stdio::piped())
                            .stderr(std::process::Stdio::piped())
                            .kill_on_drop(true)
                            .current_dir("/tmp")
                            .env_clear()
                            .env("PATH", "/usr/bin:/bin")
                            .output(),
                    )
                    .await
                    {
                        Ok(Ok(o)) => o,
                        Ok(Err(e)) => {
                            let error_msg = ServerMessage::error(format!("Execution error: {}", e));
                            if let Ok(json) = error_msg.to_json() {
                                let _ = tx_clone
                                    .lock()
                                    .await
                                    .send(WsMessage::Text(json.into()))
                                    .await;
                            }
                            return;
                        }
                        Err(_) => {
                            let error_msg = ServerMessage::error("Command execution timeout (10s)");
                            if let Ok(json) = error_msg.to_json() {
                                let _ = tx_clone
                                    .lock()
                                    .await
                                    .send(WsMessage::Text(json.into()))
                                    .await;
                            }
                            return;
                        }
                    };

                    let stdout_str =
                        String::from_utf8_lossy(&output.stdout[..output.stdout.len().min(4096)]);
                    let stderr_str =
                        String::from_utf8_lossy(&output.stderr[..output.stderr.len().min(512)]);

                    let result_msg = if output.status.success() {
                        ServerMessage::shell(stdout_str.to_string())
                    } else {
                        ServerMessage::shell(format!("Exit {}: {}", output.status, stderr_str))
                    };

                    if let Ok(json) = result_msg.to_json() {
                        let _ = tx_clone
                            .lock()
                            .await
                            .send(WsMessage::Text(json.into()))
                            .await;
                    }
                }
                Err(e) => {
                    let error_msg = ServerMessage::error(format!("Command validation failed: {}", e));
                    if let Ok(json) = error_msg.to_json() {
                        let _ = tx_clone
                            .lock()
                            .await
                            .send(WsMessage::Text(json.into()))
                            .await;
                    }
                }
            }
        });

        Ok(())
    }
}
