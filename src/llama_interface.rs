use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct LlamaConfig {
    pub model_name: String,
    pub context_tokens: u32,
    pub temperature: f32,
    pub num_threads: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct OllamaRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
    options: OllamaOptions,
}

#[derive(Debug, Serialize, Deserialize)]
struct OllamaMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct OllamaOptions {
    num_ctx: u32,
    num_thread: usize,
    temperature: f32,
}

#[derive(Debug, Deserialize)]
struct OllamaResponse {
    message: OllamaMessage,
    done: bool,
}

pub struct LlamaClient {
    config: LlamaConfig,
    history: VecDeque<ChatMessage>,
    http_client: reqwest::Client,
}

impl LlamaClient {
    pub async fn new(config: LlamaConfig) -> Result<Self> {
        tracing::info!("Initializing LLM client for model: {}", config.model_name);
        Ok(Self {
            config,
            history: VecDeque::with_capacity(20),
            http_client: reqwest::Client::new(),
        })
    }

    pub async fn chat(&mut self, user_input: &str) -> Result<String> {
        self.add_message("user", user_input.to_string());

        let request = self.build_request();
        let response = self
            .http_client
            .post("http://localhost:11434/api/chat")
            .json(&request)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to connect to Ollama: {}", e))?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Ollama returned error: {}",
                response.status()
            ));
        }

        let mut assistant_message = String::new();
        let mut stream = response.bytes_stream();
        let mut line_buffer = String::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            let text = String::from_utf8(chunk.to_vec())?;
            line_buffer.push_str(&text);

            while let Some(newline_pos) = line_buffer.find('\n') {
                let line = line_buffer[..newline_pos].to_string();
                line_buffer = line_buffer[newline_pos + 1..].to_string();

                if !line.is_empty() {
                    if let Ok(parsed) = serde_json::from_str::<OllamaResponse>(&line) {
                        assistant_message.push_str(&parsed.message.content);

                        if parsed.done {
                            self.add_message("assistant", assistant_message.clone());
                            return Ok(assistant_message);
                        }
                    }
                }
            }
        }

        if !line_buffer.is_empty() {
            if let Ok(parsed) = serde_json::from_str::<OllamaResponse>(&line_buffer) {
                assistant_message.push_str(&parsed.message.content);
            }
        }

        self.add_message("assistant", assistant_message.clone());
        Ok(assistant_message)
    }

    pub async fn chat_streaming(
        &mut self,
        user_input: &str,
        tx: tokio::sync::mpsc::Sender<String>,
    ) -> Result<()> {
        self.add_message("user", user_input.to_string());

        let request = self.build_request();
        let response = self
            .http_client
            .post("http://localhost:11434/api/chat")
            .json(&request)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to connect to Ollama: {}", e))?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Ollama returned error: {}",
                response.status()
            ));
        }

        let mut assistant_message = String::new();
        let mut stream = response.bytes_stream();
        let mut line_buffer = String::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            let text = String::from_utf8(chunk.to_vec())?;
            line_buffer.push_str(&text);

            while let Some(newline_pos) = line_buffer.find('\n') {
                let line = line_buffer[..newline_pos].to_string();
                line_buffer = line_buffer[newline_pos + 1..].to_string();

                if !line.is_empty() {
                    if let Ok(parsed) = serde_json::from_str::<OllamaResponse>(&line) {
                        let token = &parsed.message.content;
                        assistant_message.push_str(token);
                        let _ = tx.send(token.to_string()).await;

                        if parsed.done {
                            self.add_message("assistant", assistant_message);
                            return Ok(());
                        }
                    }
                }
            }
        }

        if !line_buffer.is_empty() {
            if let Ok(parsed) = serde_json::from_str::<OllamaResponse>(&line_buffer) {
                let token = &parsed.message.content;
                assistant_message.push_str(token);
                let _ = tx.send(token.to_string()).await;
            }
        }

        self.add_message("assistant", assistant_message);
        Ok(())
    }

    pub fn save_history(&self, path: &Path) -> Result<()> {
        let parent = path.parent();
        if let Some(parent) = parent {
            fs::create_dir_all(parent)?;
        }

        let history_vec: Vec<ChatMessage> = self.history.iter().cloned().collect();
        let json = serde_json::to_string_pretty(&history_vec)?;
        fs::write(path, json)?;

        tracing::info!("Saved chat history to {}", path.display());
        Ok(())
    }

    pub fn load_history(&mut self, path: &Path) -> Result<()> {
        let json = fs::read_to_string(path)?;
        let messages: Vec<ChatMessage> = serde_json::from_str(&json)?;

        self.history.clear();
        for msg in messages {
            self.history.push_back(msg);
        }

        tracing::info!(
            "Loaded {} messages from {}",
            self.history.len(),
            path.display()
        );
        Ok(())
    }

    pub fn clear_history(&mut self) {
        self.history.clear();
    }

    pub fn inject_context(&mut self, content: String) {
        let msg = ChatMessage {
            role: "system".to_string(),
            content,
            timestamp: Utc::now(),
        };
        self.history.push_front(msg);

        while self.history.len() > 20 {
            if let Some(pos) = self.history.iter().position(|m| m.role != "system") {
                self.history.remove(pos);
            } else {
                self.history.pop_back();
            }
        }

        tracing::info!("Context injected, history size: {}", self.history.len());
    }

    fn add_message(&mut self, role: &str, content: String) {
        let msg = ChatMessage {
            role: role.to_string(),
            content,
            timestamp: Utc::now(),
        };

        self.history.push_back(msg);
        if self.history.len() > 20 {
            self.history.pop_front();
        }
    }

    fn build_request(&self) -> OllamaRequest {
        let messages: Vec<OllamaMessage> = self
            .history
            .iter()
            .map(|m| OllamaMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect();

        OllamaRequest {
            model: self.config.model_name.clone(),
            messages,
            stream: true,
            options: OllamaOptions {
                num_ctx: self.config.context_tokens,
                num_thread: self.config.num_threads,
                temperature: self.config.temperature,
            },
        }
    }
}
