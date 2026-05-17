use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use ratatui::backend::{Backend, CrosstermBackend};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::io;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone)]
pub enum UiEvent {
    LlmToken(String),
    LlmDone,
    LlmError(String),
    PdfLoaded(String),
    ShellOutput(String),
    ShellError(String),
}

pub struct AppState {
    pub messages: Vec<DisplayMessage>,
    pub input_buffer: String,
    pub scroll_offset: usize,
    pub is_loading: bool,
    pub pending_assistant_msg: String,
    pub input_history: VecDeque<String>,
    pub history_index: Option<usize>,
    pub last_status: String,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            input_buffer: String::new(),
            scroll_offset: 0,
            is_loading: false,
            pending_assistant_msg: String::new(),
            input_history: VecDeque::with_capacity(50),
            history_index: None,
            last_status: String::from("Ready"),
        }
    }

    pub fn add_message(&mut self, role: String, content: String) {
        self.messages.push(DisplayMessage { role, content });
        self.scroll_to_bottom();
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = usize::MAX;
    }

    pub fn add_status(&mut self, status: String) {
        self.last_status = status;
    }

    pub fn clear_input(&mut self) {
        self.input_buffer.clear();
        self.history_index = None;
    }

    pub fn add_to_history(&mut self, input: String) {
        if !input.is_empty() {
            self.input_history.push_back(input);
            if self.input_history.len() > 50 {
                self.input_history.pop_front();
            }
        }
        self.history_index = None;
    }

    pub fn history_up(&mut self) {
        let len = self.input_history.len();
        if len == 0 {
            return;
        }

        match self.history_index {
            None => {
                self.history_index = Some(len - 1);
                if let Some(cmd) = self.input_history.get(len - 1) {
                    self.input_buffer = cmd.clone();
                }
            }
            Some(idx) => {
                if idx > 0 {
                    let new_idx = idx - 1;
                    self.history_index = Some(new_idx);
                    if let Some(cmd) = self.input_history.get(new_idx) {
                        self.input_buffer = cmd.clone();
                    }
                }
            }
        }
    }

    pub fn history_down(&mut self) {
        match self.history_index {
            Some(idx) => {
                if idx < self.input_history.len() - 1 {
                    let new_idx = idx + 1;
                    self.history_index = Some(new_idx);
                    if let Some(cmd) = self.input_history.get(new_idx) {
                        self.input_buffer = cmd.clone();
                    }
                } else {
                    self.history_index = None;
                    self.input_buffer.clear();
                }
            }
            None => {}
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct TuiApp<B: Backend> {
    terminal: ratatui::Terminal<B>,
    pub state: AppState,
    model_name: String,
}

impl TuiApp<CrosstermBackend<io::Stdout>> {
    pub fn new(model_name: String) -> Result<Self> {
        enable_raw_mode()?;
        let stdout = io::stdout();
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = ratatui::Terminal::new(backend)?;
        terminal.clear()?;

        Ok(Self {
            terminal,
            state: AppState::new(),
            model_name,
        })
    }

    pub fn draw(&mut self) -> Result<()> {
        let model_name = self.model_name.clone();
        let state = &self.state;
        self.terminal.draw(|f| {
            Self::ui_static(f, state, &model_name);
        })?;
        Ok(())
    }

    fn ui_static(f: &mut ratatui::Frame, state: &AppState, model_name: &str) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(3),
                    Constraint::Min(10),
                    Constraint::Length(4),
                    Constraint::Length(1),
                ]
                .as_ref(),
            )
            .split(f.area());

        Self::draw_title_static(f, chunks[0], state, model_name);
        Self::draw_messages_static(f, chunks[1], state);
        Self::draw_input_static(f, chunks[2], state);
        Self::draw_status_static(f, chunks[3], state);
    }

    fn draw_title_static(f: &mut ratatui::Frame, area: Rect, state: &AppState, model_name: &str) {
        let spinner = if state.is_loading {
            match (std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
                / 250)
                % 4
            {
                0 => "⠙",
                1 => "⠹",
                2 => "⠸",
                _ => "⠼",
            }
        } else {
            " "
        };

        let title = Line::from(vec![
            Span::styled("bwb_ai ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw("— Local AI Agent"),
            Span::raw("  ["),
            Span::styled(
                model_name,
                Style::default().fg(Color::Cyan),
            ),
            Span::raw("]"),
            Span::raw(" "),
            Span::styled(
                spinner,
                Style::default().fg(Color::Yellow),
            ),
        ]);

        let block = Block::default()
            .borders(Borders::BOTTOM)
            .style(Style::default().bg(Color::Black));

        let paragraph = Paragraph::new(title)
            .block(block)
            .alignment(Alignment::Left);

        f.render_widget(paragraph, area);
    }

    fn draw_messages_static(f: &mut ratatui::Frame, area: Rect, state: &AppState) {
        let mut lines: Vec<Line> = Vec::new();

        for msg in &state.messages {
            let role_style = if msg.role == "user" {
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            };

            lines.push(Line::from(vec![
                Span::styled(format!("{}: ", msg.role), role_style),
            ]));

            for line in msg.content.lines() {
                lines.push(Line::from(vec![Span::raw(format!("  {}", line))]));
            }

            lines.push(Line::from("")); // blank line
        }

        // Add pending assistant message
        if !state.pending_assistant_msg.is_empty() || state.is_loading {
            let role_style = Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD);
            lines.push(Line::from(vec![
                Span::styled("AI: ", role_style),
            ]));

            for line in state.pending_assistant_msg.lines() {
                lines.push(Line::from(vec![Span::raw(format!("  {}", line))]));
            }

            if state.is_loading && state.pending_assistant_msg.is_empty() {
                lines.push(Line::from(vec![Span::raw("  ⠀")])); // subtle placeholder
            }
        }

        let block = Block::default()
            .borders(Borders::ALL)
            .title("Messages")
            .title_alignment(Alignment::Left);

        let paragraph = Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: true })
            .scroll((state.scroll_offset as u16, 0));

        f.render_widget(paragraph, area);
    }

    fn draw_input_static(f: &mut ratatui::Frame, area: Rect, state: &AppState) {
        let input_text = &state.input_buffer;
        let cursor_pos = input_text.len() as u16;

        let block = Block::default()
            .borders(Borders::ALL)
            .title("Input")
            .title_alignment(Alignment::Left);

        let paragraph = Paragraph::new(input_text.clone())
            .block(block)
            .style(Style::default().fg(Color::White));

        f.render_widget(paragraph, area);

        // Draw cursor
        if area.height > 1 && area.width > 2 {
            let line_width = (area.width - 2) as u16;
            let cursor_x = area.x + 1 + (cursor_pos % line_width);
            let cursor_y = area.y + 1 + (cursor_pos / line_width);
            f.set_cursor_position((cursor_x, cursor_y));
        }
    }

    fn draw_status_static(f: &mut ratatui::Frame, area: Rect, state: &AppState) {
        let status_text = format!(" {}  [:help for commands] [Esc=quit]", state.last_status);
        let paragraph = Paragraph::new(status_text)
            .style(Style::default().fg(Color::DarkGray));

        f.render_widget(paragraph, area);
    }

    pub fn handle_input(&mut self, timeout: Duration) -> Result<Option<String>> {
        if crossterm::event::poll(timeout)? {
            match event::read()? {
                Event::Key(key) => return self.handle_key(key),
                Event::Resize(_, _) => {
                    // Terminal was resized, will redraw anyway
                }
                _ => {}
            }
        }
        Ok(None)
    }

    fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> Result<Option<String>> {
        match key.code {
            KeyCode::Esc => {
                return Err(anyhow::anyhow!("User requested exit"));
            }
            KeyCode::Enter => {
                let input = self.state.input_buffer.trim().to_string();
                if !input.is_empty() {
                    self.state.add_to_history(input.clone());
                    self.state.clear_input();
                    return Ok(Some(input));
                }
                self.state.clear_input();
                return Ok(None);
            }
            KeyCode::Backspace => {
                self.state.input_buffer.pop();
                self.state.history_index = None;
            }
            KeyCode::Char(c) => {
                self.state.input_buffer.push(c);
                self.state.history_index = None;
            }
            KeyCode::Up => {
                self.state.history_up();
            }
            KeyCode::Down => {
                self.state.history_down();
            }
            _ => {}
        }
        Ok(None)
    }

    pub fn cleanup(mut self) -> Result<()> {
        disable_raw_mode()?;
        self.terminal.show_cursor()?;
        Ok(())
    }
}
