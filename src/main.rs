use std::io;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::{DefaultTerminal, Frame};

use std::collections::HashMap;
use std::time::Duration;

struct Game {
    counts: HashMap<String, u32>,
}

const WASTELAND: &str = include_str!("../assets/wasteland.txt");

impl Game {
    fn new() -> Self {
        Self {
            counts: HashMap::new(),
        }
    }

    fn increment(&mut self, key: &str) {
        *self.counts.entry(key.to_string()).or_insert(0) += 1;
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
            KeyCode::Char(c) => {
                self.game.increment(&c.to_string());
            }
            _ => {}
        }
    }
}

fn ui(frame: &mut Frame, app: &App) {
    let chunks = Layout::horizontal([
        Constraint::Length(26),
        Constraint::Min(5),
        Constraint::Length(3),
        Constraint::Length(1),
    ])
    .split(frame.area());
    frame.render_widget(
        Paragraph::new(format!("{}", WASTELAND))
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
    let mut terminal = ratatui::init();
    let result = run(&mut terminal);
    ratatui::restore();
    result
}
