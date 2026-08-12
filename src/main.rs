use std::io;
use std::time::Duration;

mod BigNum;
mod game;
mod ui;
mod upgrade;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::DefaultTerminal;

use game::Game;
use ui::ui;

fn init_logging() {
    logging::root().add_handler(logging::FileHandler::new("game.log"));
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
                self.game.input(c);
            }
        }
    }
}

fn run(terminal: &mut DefaultTerminal) -> io::Result<()> {
    let mut app = App::new();

    loop {
        let now = std::time::Instant::now();

        app.game.update(now);
        terminal.draw(|f| ui(f, &app.game))?;

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
