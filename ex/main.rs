use std::io;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::{DefaultTerminal, Frame};

struct Game {
    target: i32,
    prev_guess: Option<i32>,
    won: bool,
}

impl Game {
    fn new() -> Self {
        Self {
            target: rand::random_range(0..100),
            prev_guess: None,
            won: false,
        }
    }

    /// Apply a guess and return a status message for the log.
    fn guess(&mut self, guess: i32) -> String {
        if !(0..=100).contains(&guess) {
            return "Please enter a number between 0 and 100.".into();
        }

        if self.prev_guess == Some(guess) {
            return "You guessed the same number as before!".into();
        }

        let diff = self
            .prev_guess
            .map(|prev| (prev - guess).abs())
            .unwrap_or(10)
            .max(1);
        self.prev_guess = Some(guess);

        let walk = rand::random_range((-diff / 2)..(diff / 2).max(1));
        self.target = (self.target + walk).clamp(0, 100);

        if self.target < guess {
            "Too high!".into()
        } else if self.target > guess {
            "Too low!".into()
        } else {
            self.won = true;
            "You guessed it! Press Esc to quit, or type 'r' + Enter to restart.".into()
        }
    }

    fn reset(&mut self) {
        *self = Self::new();
    }
}

struct App {
    game: Game,
    input: String,
    messages: Vec<String>,
    should_quit: bool,
}

impl App {
    fn new() -> Self {
        Self {
            game: Game::new(),
            input: String::new(),
            messages: vec![
                "Welcome to Moving Target!".into(),
                "Guess a number from 0–100. The target moves after each guess.".into(),
                "Type a number and press Enter. Esc quits.".into(),
            ],
            should_quit: false,
        }
    }

    fn submit(&mut self) {
        let raw = self.input.trim().to_string();
        self.input.clear();

        if raw.is_empty() {
            return;
        }

        if raw.eq_ignore_ascii_case("r") && self.game.won {
            self.game.reset();
            self.messages.push("New game started.".into());
            return;
        }

        if self.game.won {
            self.messages
                .push("Game over — type 'r' to restart, or Esc to quit.".into());
            return;
        }

        match raw.parse::<i32>() {
            Ok(n) => {
                let msg = self.game.guess(n);
                self.messages.push(format!("Guess {n}: {msg}"));
            }
            Err(_) => self.messages.push(format!("Not a number: {raw:?}")),
        }

        // Keep the log from growing forever.
        if self.messages.len() > 50 {
            self.messages.drain(0..self.messages.len() - 50);
        }
    }

    fn on_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc => self.should_quit = true,
            KeyCode::Enter => self.submit(),
            KeyCode::Backspace => {
                self.input.pop();
            }
            KeyCode::Char(c) => self.input.push(c),
            _ => {}
        }
    }
}

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let result = run(&mut terminal);
    ratatui::restore();
    result
}

fn run(terminal: &mut DefaultTerminal) -> io::Result<()> {
    let mut app = App::new();

    loop {
        terminal.draw(|frame| ui(frame, &app))?;

        // Block until a key event; redraw afterward.
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                app.on_key(key.code);
            }
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}

fn ui(frame: &mut Frame, app: &App) {
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(5),
        Constraint::Length(3),
        Constraint::Length(1),
    ])
    .split(frame.area());

    let bold = Style::default().add_modifier(Modifier::BOLD);
    let dim = Style::default().add_modifier(Modifier::DIM);
    let win = Style::default()
        .fg(Color::Green)
        .add_modifier(Modifier::BOLD);

    let status = if app.game.won {
        Line::from(vec![
            Span::styled("Status: ", bold),
            Span::styled("YOU WIN", win),
        ])
    } else {
        Line::from(vec![
            Span::styled("Status: ", bold),
            Span::raw("Guessing…"),
            Span::raw("   |   "),
            Span::styled("Last guess: ", dim),
            Span::raw(
                app.game
                    .prev_guess
                    .map(|g| g.to_string())
                    .unwrap_or_else(|| "—".into()),
            ),
        ])
    };

    frame.render_widget(
        Paragraph::new(status).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Moving Target Bin Search "),
        ),
        chunks[0],
    );

    let items: Vec<ListItem> = app
        .messages
        .iter()
        .rev()
        .take(chunks[1].height.saturating_sub(2) as usize)
        .map(|m| ListItem::new(m.as_str()))
        .collect();

    frame.render_widget(
        List::new(items).block(Block::default().borders(Borders::ALL).title(" Log ")),
        chunks[1],
    );

    let input_line = Line::from(vec![
        Span::styled(
            "> ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(&app.input),
        Span::styled("█", Style::default().fg(Color::Cyan)),
    ]);

    frame.render_widget(
        Paragraph::new(input_line)
            .style(Style::default().add_modifier(Modifier::BOLD))
            .block(Block::default().borders(Borders::ALL).title(" Input ")),
        chunks[2],
    );

    frame.render_widget(
        Paragraph::new("Enter submit · Backspace delete · Esc quit · r restart after win")
            .style(Style::default().fg(Color::DarkGray)),
        chunks[3],
    );
}
