use std::io;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::{DefaultTerminal, Frame};
use std::collections::HashMap;

struct Game {
    counts: HashMap<String, u32>,

}

impl Game {
    fn new() -> Self {
        Self {
            counts: HashMap::new(),
        }
    }

    fn increment(&mut self, key: &str) {
        *self.counts.entry(key.to_string()).or_insert(0) += 1;
    }

    fn reset(&mut self) {
        self.counts.clear();
    }
}

struct App {
    should_quit: bool,
    game: Game,
    action_map: HashMap<String, Box<dyn Fn(&mut Game)>>,
}

impl App {
    fn new() -> Self {
        Self {
            should_quit: false,
            game: Game::new(),
            action_map: HashMap::new(),
        }
    }

    fn on_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc => self.should_quit = true,
            KeyCode::Char(c) => {
                self.game.increment(&c.to_string());
            }
            _ => {}
        }
    }
}

fn ui(frame: 

fn run(terminal: &mut DefaultTerminal) -> io::Result<()> {
    let mut app = App::new();

    loop {
        terminal.draw(|f| ui(f, &app))?;

        if app.should_quit {
            return Ok(());
        }

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                app.on_key(key.code);
            }
        }
    }
}

fn main() {
    let mut terminal = ratatui::init();
    let result = run(&mut terminal);
    ratatui::restore();
    result
}
