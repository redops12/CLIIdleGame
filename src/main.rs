use std::io;
use std::time::Duration;

mod big_num;
mod game;
mod save;
mod ui;
mod upgrade;

use crossbeam_channel::unbounded;
use crossterm::event::{self, Event, KeyEvent};
use ratatui::DefaultTerminal;

use game::Game;
use save::{prompt_save, prompt_startup};
use ui::ui;

const FPS: u32 = 60;

fn init_logging() {
    logging::root().add_handler(logging::FileHandler::new("game.log"));
}

fn run(terminal: &mut DefaultTerminal, game: &mut Game) -> io::Result<()> {
    loop {
        let now = std::time::Instant::now();

        game.update(now);
        terminal.draw(|f| ui(f, game))?;

        if game.should_quit {
            return Ok(());
        }

        let duration = std::time::Instant::now() - now;
        if duration < Duration::from_millis(1000 / FPS as u64) {
            std::thread::sleep(Duration::from_millis(1000 / FPS as u64) - duration);
        } else {
            logging::warn("Frame took longer than expected");
        }
    }
}

fn main() -> io::Result<()> {
    init_logging();
    logging::info("Starting Game");

    let startup = prompt_startup()?;
    let mut save_path = startup.save_path;

    let mut terminal = ratatui::init();

    let (input_tx, input_rx) = unbounded::<KeyEvent>();
    let mut game = match startup.game_state {
        Some(game_state) => Game::from_state(input_rx, game_state),
        None => Game::new(input_rx),
    };

    std::thread::spawn(move || {
        loop {
            match event::poll(Duration::from_millis(100)) {
                Ok(true) => {
                    if let Ok(Event::Key(key)) = event::read() {
                        if input_tx.send(key).is_err() {
                            break;
                        }
                    }
                }
                Ok(false) => {}
                Err(e) => {
                    logging::error(&format!("Error in input loop: {e}"));
                    break;
                }
            }
        }
    });

    let result = run(&mut terminal, &mut game);
    ratatui::restore();
    prompt_save(&game, &mut save_path)?;
    logging::info("Exiting Game");
    result
}
