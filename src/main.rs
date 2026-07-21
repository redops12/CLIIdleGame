use std::io;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::{DefaultTerminal, Frame};

use std::collections::HashMap;
use std::time::Duration;

use logging::{info, warn, error, debug};

fn init_logging() {
    logging::root().add_handler(logging::FileHandler::new("game.log"));
}

struct Game {
    counts: HashMap<String, u32>,
    typed: Vec<String>,
}

const WASTELAND: &str = include_str!("../assets/wasteland.txt");

impl Game {
    fn new() -> Self {
        Self {
            counts: HashMap::new(),
            typed: Vec::new(),
        }
    }

    fn increment(&mut self, key: &str) {
        *self.counts.entry(key.to_string()).or_insert(0) += 1;
    }

    fn input(&mut self, key: KeyCode) {
        logging::debug(&format!("Input received: {key}"));
        match key {
            KeyCode::Char(c) => {
                if let Some(last) = self.typed.last_mut() {
                    last.push(c);
                } else {
                    self.typed.push(c.to_string());
                }
            }
            KeyCode::Enter => {
                self.typed.push(String::new());
            }
            KeyCode::Backspace => {
                if let Some(last) = self.typed.last_mut() {
                    last.pop();
                }
            }
            _ => {}
        }
    }
}

struct App {
    should_quit: bool,
    game: Game,
}

impl App {
    fn new() -> Self {
        Self {
            should_quit: false,
            game: Game::new(),
        }
    }

    fn on_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.should_quit = true,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
            }
            c => {
                self.game.increment(&c.to_string());
                self.game.input(c);
            }
            _ => {}
        }
    }
}

fn ui(frame: &mut Frame, app: &App) {
    let chunks = Layout::horizontal([
        Constraint::Percentage(50),
        Constraint::Min(2),
    ])
    .split(frame.area());

    let mut rendered_text = vec![];

    for (reference_idx, reference) in WASTELAND.lines().enumerate() {
        let temp_line = String::new();
        let typed_line = app.game.typed.get(reference_idx).unwrap_or(&temp_line);
        let typed_chars: Vec<char> = typed_line.chars().collect();
        let mut line = vec![];
        for (i, e) in reference.chars().enumerate() {
            let span = match typed_chars.get(i) {
                Some(&c) if c == e => Span::styled(c.to_string(), Style::default().fg(Color::Green)),
                Some(&c) if c != e => Span::styled(c.to_string(), Style::default().fg(Color::Red)),
                Some(&_) => Span::styled(e.to_string(), Style::default().fg(Color::White)),
                None => Span::styled(e.to_string(), Style::default().fg(Color::White)),
            };
            line.push(span);
        }
        rendered_text.push(Line::from(line));
    }

    frame.render_widget(
        Paragraph::new(rendered_text)
        .block(Block::default().borders(Borders::ALL).title("Key Counter")),
        chunks[0],
    );
    frame.render_widget(
        Paragraph::new(format!("{:?}", app.game.counts))
        .block(Block::default().borders(Borders::ALL).title("Key Counter")),
        chunks[1],
    )
}

fn run(terminal: &mut DefaultTerminal) -> io::Result<()> {
    let mut app = App::new();

    loop {
        terminal.draw(|f| ui(f, &app))?;

        if app.should_quit {
            return Ok(());
        }

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                app.on_key(key);
            }
        }
    }
}

fn main() -> io::Result<()> {
    init_logging();
    logging::info("Starting Game");
    let mut terminal = ratatui::init();
    let result = run(&mut terminal);
    ratatui::restore();
    logging::info("Exiting Game");
    result
}
