use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    #[serde(rename = "chat")]
    Chat { content: String },
    #[serde(rename = "load_pdf")]
    LoadPdf { path: String },
    #[serde(rename = "run")]
    Run { command: String },
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    #[serde(rename = "token")]
    Token { content: String },
    #[serde(rename = "done")]
    Done { content: String },
    #[serde(rename = "error")]
    Error { content: String },
    #[serde(rename = "shell")]
    Shell { content: String },
}

pub struct WsServer;

impl WsServer {
    pub async fn run(_addr: std::net::SocketAddr) -> Result<()> {
        todo!("Phase 4: Implement WebSocket server")
    }
}
