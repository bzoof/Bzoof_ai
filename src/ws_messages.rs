use serde::{Deserialize, Serialize};

/// Messages sent from WebSocket clients to the server
#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum ClientMessage {
    #[serde(rename = "chat")]
    Chat { content: String },

    #[serde(rename = "load_pdf")]
    LoadPdf { path: String },

    #[serde(rename = "run")]
    Run { command: String },

    #[serde(rename = "ping")]
    Ping,
}

/// Messages sent from the server back to WebSocket clients
#[derive(Debug, Serialize, Clone)]
#[serde(tag = "type")]
pub enum ServerMessage {
    /// Token from LLM streaming response
    #[serde(rename = "token")]
    Token { content: String },

    /// Completion of chat message
    #[serde(rename = "done")]
    Done { content: String },

    /// Error response
    #[serde(rename = "error")]
    Error {
        #[serde(rename = "content")]
        message: String,
    },

    /// Shell command output
    #[serde(rename = "shell")]
    Shell { content: String },

    /// PDF loading status
    #[serde(rename = "pdf_status")]
    PdfStatus { content: String },

    /// Pong response
    #[serde(rename = "pong")]
    Pong,

    /// Connection acknowledged
    #[serde(rename = "ready")]
    Ready {
        client_id: String,
        message: String,
    },
}

impl ClientMessage {
    /// Parse a text message into a ClientMessage
    pub fn from_text(text: &str) -> anyhow::Result<Self> {
        serde_json::from_str(text).map_err(|e| {
            anyhow::anyhow!("Failed to parse message: {}", e)
        })
    }

    /// Check if message type is chat
    pub fn is_chat(&self) -> bool {
        matches!(self, ClientMessage::Chat { .. })
    }

    /// Check if message type is PDF load
    pub fn is_load_pdf(&self) -> bool {
        matches!(self, ClientMessage::LoadPdf { .. })
    }

    /// Check if message type is shell run
    pub fn is_run(&self) -> bool {
        matches!(self, ClientMessage::Run { .. })
    }
}

impl ServerMessage {
    /// Convert ServerMessage to JSON text
    pub fn to_json(&self) -> anyhow::Result<String> {
        serde_json::to_string(self).map_err(|e| {
            anyhow::anyhow!("Failed to serialize message: {}", e)
        })
    }

    /// Create a token message
    pub fn token(content: impl Into<String>) -> Self {
        ServerMessage::Token {
            content: content.into(),
        }
    }

    /// Create a done message
    pub fn done(content: impl Into<String>) -> Self {
        ServerMessage::Done {
            content: content.into(),
        }
    }

    /// Create an error message
    pub fn error(message: impl Into<String>) -> Self {
        ServerMessage::Error {
            message: message.into(),
        }
    }

    /// Create a shell output message
    pub fn shell(content: impl Into<String>) -> Self {
        ServerMessage::Shell {
            content: content.into(),
        }
    }

    /// Create a PDF status message
    pub fn pdf_status(content: impl Into<String>) -> Self {
        ServerMessage::PdfStatus {
            content: content.into(),
        }
    }

    /// Create a ready message
    pub fn ready(client_id: impl Into<String>, message: impl Into<String>) -> Self {
        ServerMessage::Ready {
            client_id: client_id.into(),
            message: message.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_chat_message() {
        let json = r#"{"type": "chat", "content": "Hello"}"#;
        let msg = ClientMessage::from_text(json).unwrap();
        assert!(msg.is_chat());
    }

    #[test]
    fn test_parse_load_pdf_message() {
        let json = r#"{"type": "load_pdf", "path": "/path/to/file.pdf"}"#;
        let msg = ClientMessage::from_text(json).unwrap();
        assert!(msg.is_load_pdf());
    }

    #[test]
    fn test_parse_run_message() {
        let json = r#"{"type": "run", "command": "ls -la"}"#;
        let msg = ClientMessage::from_text(json).unwrap();
        assert!(msg.is_run());
    }

    #[test]
    fn test_serialize_token_message() {
        let msg = ServerMessage::token("Hello");
        let json = msg.to_json().unwrap();
        assert!(json.contains("token"));
        assert!(json.contains("Hello"));
    }

    #[test]
    fn test_serialize_error_message() {
        let msg = ServerMessage::error("Something went wrong");
        let json = msg.to_json().unwrap();
        assert!(json.contains("error"));
        assert!(json.contains("Something went wrong"));
    }

    #[test]
    fn test_invalid_json() {
        let json = r#"{"invalid": "message"}"#;
        assert!(ClientMessage::from_text(json).is_err());
    }
}
