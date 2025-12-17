// src/tui.rs
use std::io;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Terminal,
};
use tokio::sync::mpsc;
use anyhow::Result;

// --- 1. THE MESSAGE TYPES ---
// These are the signals sent from the Brain to the Face
#[derive(Debug, Clone)]
pub enum UiMessage {
    User(String),      // User typed something
    Ai(String),        // AI replied
    Log(String),       // System event (tool call, security check)
    Error(String),     // Something broke
}

// --- 2. APP STATE ---
pub struct App {
    pub input: String,
    pub chat_history: Vec<UiMessage>, // Structured history
    pub logs: Vec<String>,
    pub should_quit: bool,
    // The mailbox to send user input TO the brain
    pub tx_agent: mpsc::UnboundedSender<String>, 
}

impl App {
    pub fn new(tx_agent: mpsc::UnboundedSender<String>) -> Self {
        Self {
            input: String::new(),
            chat_history: Vec::new(),
            logs: Vec::new(),
            should_quit: false,
            tx_agent,
        }
    }

    pub fn on_key(&mut self, c: char) {
        self.input.push(c);
    }

    pub fn on_enter(&mut self) {
        if !self.input.trim().is_empty() {
            // 1. Show it in UI immediately
            self.chat_history.push(UiMessage::User(self.input.clone()));
            // 2. Send it to the Brain
            let _ = self.tx_agent.send(self.input.clone());
            // 3. Clear input
            self.input.clear();
        }
    }
}

// --- 3. THE MAIN LOOP ---
pub async fn run_tui(mut app: App, mut rx_ui: mpsc::UnboundedReceiver<UiMessage>) -> Result<()> {
    // Setup Terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    loop {
        // A. DRAW UI
        terminal.draw(|f| ui_builder(f, &app))?;

        // B. CHECK FOR UI MESSAGES (From Brain)
        // We use try_recv to not block the loop
        while let Ok(msg) = rx_ui.try_recv() {
            match msg {
                UiMessage::Log(text) => app.logs.push(text),
                UiMessage::Error(text) => {
                    app.logs.push(format!("ERROR: {}", text));
                    app.chat_history.push(UiMessage::Error(text));
                }
                other => app.chat_history.push(other),
            }
        }

        // C. CHECK FOR USER INPUT (Keyboard)
        // Wait up to 50ms for a key
        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Esc => app.should_quit = true,
                    KeyCode::Enter => app.on_enter(),
                    KeyCode::Char(c) => app.on_key(c),
                    KeyCode::Backspace => { app.input.pop(); }
                    _ => {}
                    }
                
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    // Cleanup
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    Ok(())
}

// --- 4. THE RENDERER (Making it pretty) ---
fn ui_builder(f: &mut ratatui::Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Min(1), Constraint::Length(3)].as_ref())
        .split(f.size());

    let top_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)].as_ref())
        .split(chunks[0]);

    // WIDGET 1: CHAT
    let messages: Vec<Line> = app.chat_history.iter().map(|m| {
        match m {
            UiMessage::User(txt) => Line::from(Span::styled(format!("YOU: {}", txt), Style::default().fg(Color::Cyan))),
            UiMessage::Ai(txt) => Line::from(Span::styled(format!("AI: {}", txt), Style::default().fg(Color::Green))),
            UiMessage::Error(txt) => Line::from(Span::styled(format!("ERR: {}", txt), Style::default().fg(Color::Red))),
            _ => Line::from(""),
        }
    }).collect();

    let chat_block = Paragraph::new(messages)
        .block(Block::default().borders(Borders::ALL).title(" AETHER TERMINAL "))
        .wrap(Wrap { trim: true });
    f.render_widget(chat_block, top_chunks[0]);

    // WIDGET 2: LOGS
    let log_lines: Vec<Line> = app.logs.iter().rev() // Show newest at top
        .take(20) // Only last 20 logs
        .map(|s| Line::from(Span::styled(s, Style::default().fg(Color::DarkGray))))
        .collect();
    
    let logs_block = Paragraph::new(log_lines)
        .block(Block::default().borders(Borders::ALL).title(" SYSTEM CORE "));
    f.render_widget(logs_block, top_chunks[1]);

    // WIDGET 3: INPUT
    let input_block = Paragraph::new(app.input.as_str())
        .block(Block::default().borders(Borders::ALL).title(" COMMAND INPUT (Esc to Quit) "))
        .style(Style::default().fg(Color::Yellow));
    f.render_widget(input_block, chunks[1]);
}