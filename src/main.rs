mod llama_interface;
mod chat_ui;
mod pdf_reader;
mod shell_runner;
mod ws_server;
mod ws_messages;

use anyhow::{anyhow, Result};
use chat_ui::TuiApp;
use clap::Parser;
use llama_interface::{LlamaClient, LlamaConfig};
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(name = "bwb_ai")]
#[command(about = "Local Rust AI agent with TUI, PDF reading, and shell execution")]
struct Args {
    #[arg(long, default_value = "qwen2.5-coder:3b")]
    model: String,

    #[arg(long, default_value = "2048")]
    context: u32,

    #[arg(long, default_value = "0.7")]
    temperature: f32,

    #[arg(long)]
    lora_adapter: Option<PathBuf>,

    #[arg(long)]
    ws: bool,

    #[arg(long, default_value = "127.0.0.1:8080")]
    ws_addr: String,

    #[arg(long)]
    one_shot: bool,
}

#[derive(Debug)]
pub enum Command {
    LoadPdf(PathBuf),
    RunShell(String),
    Save,
    SaveAs(PathBuf),
    Chat(String),
    Quit,
    Help,
}

pub fn parse_command(input: &str) -> Command {
    let trimmed = input.trim();

    match trimmed {
        s if s.starts_with(":load ") => Command::LoadPdf(PathBuf::from(&s[6..])),
        s if s.starts_with(":run ") => Command::RunShell(s[5..].to_string()),
        ":save" => Command::Save,
        s if s.starts_with(":save ") => Command::SaveAs(PathBuf::from(&s[6..])),
        ":q" | ":quit" | ":exit" => Command::Quit,
        ":help" | ":h" => Command::Help,
        s if !s.is_empty() => Command::Chat(s.to_string()),
        _ => Command::Help,
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::WARN.into()),
        )
        .init();

    if let Some(ref lora) = args.lora_adapter {
        tracing::info!("Starting bwb_ai with model: {} + LoRA: {}", args.model, lora.display());
    } else {
        tracing::info!("Starting bwb_ai with model: {}", args.model);
    }

    let config = LlamaConfig {
        model_name: args.model.clone(),
        context_tokens: args.context,
        temperature: args.temperature,
        num_threads: 6,
    };

    if args.one_shot {
        let client = LlamaClient::new(config).await?;
        handle_one_shot(client).await?;
    } else if args.ws {
        let addr = args.ws_addr.parse::<std::net::SocketAddr>()?;
        ws_server::WsServer::run(addr, config).await?;
    } else {
        let client = LlamaClient::new(config).await?;
        handle_tui(client, &args.model).await?;
    }

    Ok(())
}

async fn handle_one_shot(mut client: LlamaClient) -> Result<()> {
    let stdin = io::stdin();
    let mut input = String::new();

    stdin.read_line(&mut input)?;
    let response = client.chat(input.trim()).await?;
    println!("{}", response);

    Ok(())
}

async fn handle_tui(client: LlamaClient, model_name: &str) -> Result<()> {
    use std::sync::Arc;
    use tokio::sync::Mutex;

    let mut app = TuiApp::new(model_name.to_string())?;
    let client = Arc::new(Mutex::new(client));

    app.state.add_message("System".to_string(),
        "Welcome to bwb_ai! Type a message or use :help for commands.".to_string());
    app.draw()?;

    let (ui_tx, mut ui_rx) = tokio::sync::mpsc::channel::<chat_ui::UiEvent>(256);

    loop {
        tokio::select! {
            input_result = async { app.handle_input(Duration::from_millis(0)) } => {
                match input_result {
                    Ok(Some(input)) => {
                        let command = parse_command(&input);
                        match command {
                            Command::Chat(msg) => {
                                app.state.add_message("You".to_string(), msg.clone());
                                app.state.is_loading = true;
                                app.draw()?;

                                let client_clone = client.clone();
                                let tx = ui_tx.clone();
                                tokio::spawn(async move {
                                    let (token_tx, mut token_rx) = tokio::sync::mpsc::channel(256);
                                    let tx_clone = tx.clone();
                                    let token_forward = tokio::spawn(async move {
                                        while let Some(token) = token_rx.recv().await {
                                            let _ = tx_clone.send(chat_ui::UiEvent::LlmToken(token)).await;
                                        }
                                    });

                                    let mut locked = client_clone.lock().await;
                                    match locked.chat_streaming(&msg, token_tx).await {
                                        Ok(_) => { let _ = tx.send(chat_ui::UiEvent::LlmDone).await; }
                                        Err(e) => { let _ = tx.send(chat_ui::UiEvent::LlmError(e.to_string())).await; }
                                    }
                                    let _ = token_forward.await;
                                });
                            }
                            Command::Help => {
                                let help = "Commands: :load <pdf>, :run <cmd>, :save [path], :q/:quit, :h/:help";
                                app.state.add_message("Help".to_string(), help.to_string());
                                app.state.add_status("Ready".to_string());
                            }
                            Command::Quit => {
                                break;
                            }
                            Command::LoadPdf(path) => {
                                app.state.add_message("You".to_string(), format!(":load {}", path.display()));
                                app.state.add_status("Loading PDF...".to_string());
                                app.draw()?;

                                let tx = ui_tx.clone();
                                let client_clone = client.clone();
                                tokio::spawn(async move {
                                    let text = match tokio::task::spawn_blocking(move || {
                                        pdf_reader::PdfReader::extract_text(&path)
                                    }).await {
                                        Ok(Ok(t)) => t,
                                        Ok(Err(e)) => {
                                            let _ = tx.send(chat_ui::UiEvent::LlmError(format!("PDF error: {}", e))).await;
                                            return;
                                        }
                                        Err(e) => {
                                            let _ = tx.send(chat_ui::UiEvent::LlmError(format!("PDF task error: {}", e))).await;
                                            return;
                                        }
                                    };

                                    let token_count = pdf_reader::PdfReader::estimate_tokens(&text);

                                    if token_count <= 1800 {
                                        let mut locked = client_clone.lock().await;
                                        locked.inject_context(format!(
                                            "The following document has been loaded:\n\n{}", text
                                        ));
                                        let _ = tx.send(chat_ui::UiEvent::PdfLoaded(format!(
                                            "PDF loaded (~{} tokens). Ask me anything about it.", token_count
                                        ))).await;
                                    } else {
                                        let chunks = pdf_reader::PdfReader::chunk_text(&text, 3200);
                                        let total = chunks.len();
                                        let _ = tx.send(chat_ui::UiEvent::PdfLoaded(format!(
                                            "PDF is large (~{} tokens, {} chunks). Summarizing...", token_count, total
                                        ))).await;

                                        let mut summary = String::new();
                                        for (i, chunk) in chunks.iter().enumerate() {
                                            let prompt = if i == 0 {
                                                format!("Summarize this document section:\n\n{}", chunk)
                                            } else {
                                                format!("Continue summarizing. So far: {}\n\nNext section:\n\n{}", summary, chunk)
                                            };

                                            let _ = tx.send(chat_ui::UiEvent::PdfLoaded(format!(
                                                "Summarizing chunk {}/{}...", i + 1, total
                                            ))).await;

                                            let mut locked = client_clone.lock().await;
                                            match locked.chat(&prompt).await {
                                                Ok(r) => { summary = r; }
                                                Err(e) => {
                                                    let _ = tx.send(chat_ui::UiEvent::LlmError(format!("Summary error: {}", e))).await;
                                                    return;
                                                }
                                            }
                                        }

                                        let mut locked = client_clone.lock().await;
                                        locked.inject_context(format!("Document summary:\n\n{}", summary));
                                        let _ = tx.send(chat_ui::UiEvent::PdfLoaded(format!(
                                            "PDF summarized ({} chunks).", total
                                        ))).await;
                                    }
                                });
                            }
                            Command::RunShell(cmd) => {
                                app.state.add_message("You".to_string(), format!(":run {}", cmd));
                                app.state.add_status("Running...".to_string());
                                app.draw()?;

                                let tx = ui_tx.clone();
                                tokio::spawn(async move {
                                    if let Err(e) = shell_runner::ShellRunner::run_command(&cmd, tx.clone()).await {
                                        let _ = tx.send(chat_ui::UiEvent::LlmError(e.to_string())).await;
                                    }
                                });
                            }
                            Command::Save | Command::SaveAs(_) => {
                                app.state.add_message("Info".to_string(),
                                    "History saving will be implemented in Phase 1 extended".to_string());
                                app.state.add_status("Ready".to_string());
                            }
                        }
                    }
                    Ok(None) => {}
                    Err(e) => {
                        if e.to_string().contains("exit") {
                            break;
                        }
                        return Err(e);
                    }
                }
            }

            Some(event) = ui_rx.recv() => {
                match event {
                    chat_ui::UiEvent::LlmToken(token) => {
                        app.state.pending_assistant_msg.push_str(&token);
                        app.state.scroll_to_bottom();
                    }
                    chat_ui::UiEvent::LlmDone => {
                        let msg = std::mem::take(&mut app.state.pending_assistant_msg);
                        app.state.add_message("AI".to_string(), msg);
                        app.state.is_loading = false;
                        app.state.add_status("Ready".to_string());
                    }
                    chat_ui::UiEvent::LlmError(e) => {
                        app.state.pending_assistant_msg.clear();
                        app.state.is_loading = false;
                        app.state.add_message("Error".to_string(), e);
                        app.state.add_status("Error".to_string());
                    }
                    chat_ui::UiEvent::ShellOutput(out) => {
                        app.state.add_message("Shell".to_string(), out);
                        app.state.add_status("Ready".to_string());
                    }
                    chat_ui::UiEvent::ShellError(e) => {
                        app.state.add_message("Shell Error".to_string(), e);
                        app.state.add_status("Ready".to_string());
                    }
                    chat_ui::UiEvent::PdfLoaded(msg) => {
                        app.state.add_message("PDF".to_string(), msg);
                        app.state.add_status("Ready".to_string());
                    }
                }
            }
        }

        app.draw()?;
    }

    app.cleanup()?;
    println!("Goodbye!");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_command_chat() {
        match parse_command("hello world") {
            Command::Chat(msg) => assert_eq!(msg, "hello world"),
            _ => panic!("expected Chat"),
        }
    }

    #[test]
    fn test_parse_command_help() {
        match parse_command(":help") {
            Command::Help => {}
            _ => panic!("expected Help"),
        }
    }

    #[test]
    fn test_parse_command_quit() {
        match parse_command(":quit") {
            Command::Quit => {}
            _ => panic!("expected Quit"),
        }
    }

    #[test]
    fn test_parse_command_load() {
        match parse_command(":load test.pdf") {
            Command::LoadPdf(p) => assert_eq!(p.to_string_lossy(), "test.pdf"),
            _ => panic!("expected LoadPdf"),
        }
    }

    #[test]
    fn test_parse_command_run() {
        match parse_command(":run ls -la") {
            Command::RunShell(cmd) => assert_eq!(cmd, "ls -la"),
            _ => panic!("expected RunShell"),
        }
    }

    #[test]
    fn test_parse_command_save() {
        match parse_command(":save") {
            Command::Save => {}
            _ => panic!("expected Save"),
        }
    }

    #[test]
    fn test_parse_command_save_as() {
        match parse_command(":save myfile.json") {
            Command::SaveAs(p) => assert_eq!(p.to_string_lossy(), "myfile.json"),
            _ => panic!("expected SaveAs"),
        }
    }

    #[test]
    fn test_parse_command_empty() {
        match parse_command("") {
            Command::Help => {}
            _ => panic!("expected Help"),
        }
    }

    #[test]
    fn test_parse_command_whitespace() {
        match parse_command("   ") {
            Command::Help => {}
            _ => panic!("expected Help"),
        }
    }
}
