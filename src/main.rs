use std::io;

mod BigNum;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::{DefaultTerminal, Frame};

use std::collections::HashMap;
use std::time::Duration;

use logging::{info, warn, error, debug};

fn init_logging() {
    logging::root().add_handler(logging::FileHandler::new("game.log"));
}

struct Game {
    counts: HashMap<String, u32>,

    // currently typed string
    typed: String,

    // Text variables
    current_text: Vec<&'static str>,
    current_line: usize,

    // displayed stats
    money: i64,
    incorrect_penalty: i64,
    correct_increment: i64,
}

const WASTELAND: &str = include_str!("../assets/wasteland.txt");

impl Game {
    fn new() -> Self {
        Self {
            counts: HashMap::new(),
            typed: String::new(),
            current_text: WASTELAND.split('\n').collect(),
            current_line: 0,
            money: 0,
            incorrect_penalty: 5,
            correct_increment: 1,
        }
    }

    fn increment(&mut self, key: &str) {
        *self.counts.entry(key.to_string()).or_insert(0) += 1;
    }

    fn input(&mut self, key: KeyCode) {
        logging::debug(&format!("Input received: {key}"));
        match key {
            KeyCode::Char(c) => {
                self.typed.push(c);
            }
            KeyCode::Enter => {
                let mut money_change: i64 = 0;
                let current_line: &'static str = self.current_text[self.current_line];
                let typed_chars: Vec<char> = self.typed.chars().collect();
                let ref_chars: Vec<char> = current_line.chars().collect();
                let len = typed_chars.len().max(ref_chars.len());

                for i in 0..len {
                    match (typed_chars.get(i), ref_chars.get(i)) {
                        (Some(&c), Some(&t)) if c == t => {
                            money_change += self.correct_increment;
                        }
                        // Wrong char, missing char, or extra typed char
                        _ => {
                            money_change -= self.incorrect_penalty;
                        }
                    }
                }

                logging::debug(&format!(
                    "Scored line {}: money_change={money_change} (typed={} chars, ref={} chars)",
                    self.current_line,
                    typed_chars.len(),
                    ref_chars.len()
                ));
                self.money += money_change;
                self.typed = String::new();
                self.current_line += 1;
            }
            KeyCode::Backspace => {
                self.typed.pop();
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

fn key_count(counts: &HashMap<String, u32>, key: &str) -> u32 {
    *counts.get(key).unwrap_or(&0)
}

fn keyboard_count_lines(counts: &HashMap<String, u32>) -> Vec<Line<'static>> {
    const ROWS: [&str; 4] = [
        "1234567890",
        "qwertyuiop",
        "asdfghjkl",
        "zxcvbnm",
    ];

    let mut lines = Vec::new();
    for (row_idx, row) in ROWS.iter().enumerate() {
        let indent = " ".repeat(row_idx.saturating_sub(1));
        let mut spans = vec![Span::raw(indent)];
        for (i, c) in row.chars().enumerate() {
            if i > 0 {
                spans.push(Span::raw("  "));
            }
            let key = c.to_string();
            let count = key_count(counts, &key);
            spans.push(Span::styled(
                format!("{c}:{count}"),
                Style::default().fg(Color::Cyan),
            ));
        }
        lines.push(Line::from(spans));
    }

    let space_count = key_count(counts, " ");
    lines.push(Line::from(Span::styled(
        format!("    [space]:{space_count}"),
        Style::default().fg(Color::Cyan),
    )));
    lines
}

fn typing_zone(app: &App, width: u16) -> Vec<Line<'static>> {
    let reference = app
        .game
        .current_text
        .get(app.game.current_line)
        .copied()
        .unwrap_or("");
    let next1 = app
        .game
        .current_text
        .get(app.game.current_line + 1)
        .copied()
        .unwrap_or("");
    let next2 = app
        .game
        .current_text
        .get(app.game.current_line + 2)
        .copied()
        .unwrap_or("");
    let typed_chars: Vec<char> = app.game.typed.chars().collect();
    let ref_chars: Vec<char> = reference.chars().collect();
    let mut line = vec![];

    let mut correct = 0usize;
    let mut checked = ref_chars.len().max(typed_chars.len());
    for (i, e) in ref_chars.iter().enumerate() {
        let span = match typed_chars.get(i) {
            Some(&c) if c == *e => {
                correct += 1;
                Span::styled(c.to_string(), Style::default().fg(Color::Green))
            }
            Some(&c) if c != *e && !c.is_whitespace() => Span::styled(c.to_string(), Style::default().fg(Color::Red)),
            Some(&c) if c != *e && c.is_whitespace() => Span::styled(c.to_string(), Style::default().bg(Color::Red)),
            Some(&_) => Span::styled(e.to_string(), Style::default().fg(Color::White)),
            None => Span::styled(e.to_string(), Style::default().fg(Color::White)),
        };
        line.push(span);
    }
    for c in typed_chars.iter().skip(ref_chars.len()) {
        checked += 1;
        if c.is_whitespace() {
            line.push(Span::styled(c.to_string(), Style::default().bg(Color::Red)));
        } else {
            line.push(Span::styled(c.to_string(), Style::default().fg(Color::Red)));
        }
    }

    let pct = if checked == 0 {
        100
    } else {
        correct * 100 / checked
    };
    let pct_text = format!("{pct:>3}%");
    let text_width: usize = line.iter().map(Span::width).sum();
    let pad = (width as usize).saturating_sub(text_width + pct_text.chars().count());
    if pad > 0 {
        line.push(Span::raw(" ".repeat(pad)));
    }
    let pct_color = if pct >= 90 {
        Color::Green
    } else if pct >= 70 {
        Color::Yellow
    } else {
        Color::Red
    };
    line.push(Span::styled(pct_text, Style::default().fg(pct_color)));

    const GRAY_DIM: Color = Color::Rgb(80, 80, 80);
    const GRAY_MID: Color = Color::Rgb(140, 140, 140);

    vec![
        Line::from(Span::styled(next2, Style::default().fg(GRAY_DIM))),
        Line::from(Span::styled(next1, Style::default().fg(GRAY_MID))),
        Line::from(line),
    ]
}

fn ui(frame: &mut Frame, app: &App) {
    let columns = Layout::horizontal([
        Constraint::Percentage(25),
        Constraint::Percentage(50),
        Constraint::Percentage(25),
    ])
    .split(frame.area());

    let middle = Layout::vertical([
        Constraint::Min(5),
        Constraint::Length(5),
    ])
    .split(columns[1]);

    frame.render_widget(
        Paragraph::new("Esc / Ctrl-C to quit")
            .block(Block::default().borders(Borders::ALL).title("Left")),
        columns[0],
    );

    let stats = format!(
        "Money: {}\nPenalty: {}\nIncome: {}",
        app.game.money, app.game.incorrect_penalty, app.game.correct_increment
    );
    frame.render_widget(
        Paragraph::new(stats).block(Block::default().borders(Borders::ALL).title("Stats")),
        middle[0],
    );

    // Inner width accounts for left/right borders.
    let text_width = middle[1].width.saturating_sub(2);
    frame.render_widget(
        Paragraph::new(typing_zone(app, text_width))
            .block(Block::default().borders(Borders::ALL).title("Text")),
        middle[1],
    );

    frame.render_widget(
        Paragraph::new(keyboard_count_lines(&app.game.counts))
            .block(Block::default().borders(Borders::ALL).title("Keys")),
        columns[2],
    );
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
