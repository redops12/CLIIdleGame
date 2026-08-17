use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

use crate::game::{Game, GameState};

pub struct StartupOptions {
    pub save_path: Option<PathBuf>,
    pub game_state: Option<GameState>,
}

fn read_line(prompt: &str) -> io::Result<String> {
    print!("{prompt}");
    io::stdout().flush()?;
    let mut line = String::new();
    io::stdin().lock().read_line(&mut line)?;
    Ok(line.trim().to_string())
}

fn read_yes_no(prompt: &str, default_yes: bool) -> io::Result<bool> {
    let default = if default_yes { "Y/n" } else { "y/N" };
    let line = read_line(&format!("{prompt} [{default}]: "))?;
    match line.to_ascii_lowercase().as_str() {
        "" => Ok(default_yes),
        "y" | "yes" => Ok(true),
        "n" | "no" => Ok(false),
        _ => {
            eprintln!("Please answer y or n.");
            read_yes_no(prompt, default_yes)
        }
    }
}

fn read_path(prompt: &str) -> io::Result<PathBuf> {
    loop {
        let line = read_line(prompt)?;
        if line.is_empty() {
            eprintln!("Path cannot be empty.");
            continue;
        }
        return Ok(PathBuf::from(line));
    }
}

pub fn prompt_startup() -> io::Result<StartupOptions> {
    if !read_yes_no("Load a saved game?", false)? {
        return Ok(StartupOptions {
            save_path: None,
            game_state: None,
        });
    }

    loop {
        let path = read_path("Enter save file path: ")?;
        match Game::load_state(&path) {
            Ok(game_state) => {
                println!("Loaded save from {}.", path.display());
                return Ok(StartupOptions {
                    save_path: Some(path),
                    game_state: Some(game_state),
                });
            }
            Err(err) => {
                eprintln!("Failed to load {}: {err}", path.display());
                if !read_yes_no("Try another path?", true)? {
                    return Ok(StartupOptions {
                        save_path: None,
                        game_state: None,
                    });
                }
            }
        }
    }
}

enum ExitSaveChoice {
    Save,
    SaveAs,
    Skip,
}

fn read_exit_save_choice(path: &Path) -> io::Result<ExitSaveChoice> {
    let line = read_line(&format!(
        "Save to {}? [Y/n/a for save as]: ",
        path.display()
    ))?;
    match line.to_ascii_lowercase().as_str() {
        "" | "y" | "yes" => Ok(ExitSaveChoice::Save),
        "n" | "no" => Ok(ExitSaveChoice::Skip),
        "a" | "s" | "save as" | "saveas" => Ok(ExitSaveChoice::SaveAs),
        _ => {
            eprintln!("Please answer y, n, or a.");
            read_exit_save_choice(path)
        }
    }
}

pub fn prompt_save(game: &Game, save_path: &mut Option<PathBuf>) -> io::Result<()> {
    if !read_yes_no("Save game before exiting?", false)? {
        return Ok(());
    }

    loop {
        let path = match save_path {
            Some(existing) => match read_exit_save_choice(existing)? {
                ExitSaveChoice::Save => existing.clone(),
                ExitSaveChoice::SaveAs => read_path("Enter save file path: ")?,
                ExitSaveChoice::Skip => return Ok(()),
            },
            None => read_path("Enter save file path: ")?,
        };

        match game.save_state(&path) {
            Ok(()) => {
                println!("Game saved to {}.", path.display());
                *save_path = Some(path);
                return Ok(());
            }
            Err(err) => {
                eprintln!("Failed to save to {}: {err}", path.display());
                if !read_yes_no("Try again?", true)? {
                    return Ok(());
                }
            }
        }
    }
}
