use std::io;
use std::time::Duration;

mod auto_queue;
mod big_num;
mod game;
mod save;
mod test_mode;
mod text_sources;
mod ui;
mod upgrade;

use crossbeam_channel::unbounded;
use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event};
use ratatui::DefaultTerminal;

use game::{Game, InputEvent};
use save::{prompt_save, prompt_startup, StartupOptions};
use test_mode::{has_flag, parse_save_path_from_args, parse_test_mode_from_args};
use ui::ui;

const FPS: u32 = 60;

fn init_logging() {
    logging::root().add_handler(logging::FileHandler::new("game.log"));
}

fn run(terminal: &mut DefaultTerminal, game: &mut Game) -> io::Result<()> {
    loop {
        let now = std::time::Instant::now();

        game.update(now);
        terminal.draw(|f| {
            game.pane_rects = ui(f, game);
        })?;

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

    let args: Vec<String> = std::env::args().collect();
    let test_mode = parse_test_mode_from_args(&args);
    let arg_save_path = parse_save_path_from_args(&args);
    let force_prompt = has_flag(&args, "--prompt");

    let startup = if let Some(ref path) = arg_save_path {
        let game_state = Game::load_state(path)?;
        StartupOptions {
            save_path: Some(path.clone()),
            game_state: Some(game_state),
        }
    } else if test_mode.is_some() && !force_prompt {
        StartupOptions {
            save_path: None,
            game_state: None,
        }
    } else {
        prompt_startup()?
    };
    let mut save_path = startup.save_path;

    if let Some(ref tm) = test_mode {
        logging::info(&format!("Running in test mode: {tm}"));
    }

    let mut terminal = ratatui::init();
    crossterm::execute!(std::io::stdout(), EnableMouseCapture)?;

    let (input_tx, input_rx) = unbounded::<InputEvent>();
    let mut game = match startup.game_state {
        Some(game_state) => Game::from_state_with_test_mode(input_rx, game_state, test_mode.clone()),
        None => Game::new_with_test_mode(input_rx, test_mode.clone()),
    };

    std::thread::spawn(move || {
        loop {
            match event::poll(Duration::from_millis(100)) {
                Ok(true) => {
                    if let Ok(event) = event::read() {
                        match event {
                        Event::Key(key) => {
                            if input_tx.send(InputEvent::Key(key)).is_err() {
                                break;
                            }
                        }
                        Event::Mouse(mouse) => {
                            if input_tx.send(InputEvent::Mouse(mouse)).is_err() {
                                break;
                            }
                        }
                        _ => {}
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
    crossterm::execute!(std::io::stdout(), DisableMouseCapture)?;
    ratatui::restore();

    if test_mode.is_none() || force_prompt || save_path.is_some() {
        prompt_save(&game, &mut save_path)?;
    }
    logging::info("Exiting Game");
    result
}
