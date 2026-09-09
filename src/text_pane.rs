use crossterm::event::KeyCode;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use serde::{Deserialize, Serialize};

use crate::game::Game;
use crate::pane_module::{focus_border_color, PaneModule};
use crate::text_sources::{get_lines_from_source, TextSource};

const GRAY_DIM: Color = Color::Rgb(80, 80, 80);
const GRAY_MID: Color = Color::Rgb(140, 140, 140);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TextPane {
    pub current_text: TextSource,
    pub current_line: usize,
    pub typed: String,
}

impl Default for TextPane {
    fn default() -> Self {
        Self {
            current_text: TextSource::Intro,
            current_line: 0,
            typed: String::new(),
        }
    }
}

impl TextPane {
    pub fn get_line(&self, offset: Option<usize>) -> &str {
        get_lines_from_source(
            self.current_text,
            self.current_line + offset.unwrap_or(0),
        )
    }

    fn render_lines(&self, game: &Game) -> Vec<Line<'_>> {
        let reference = self.get_line(None);
        let next1 = self.get_line(Some(1));
        let next2 = self.get_line(Some(2));
        let typed_chars: Vec<char> = self.typed.chars().collect();
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
                Some(&c) if c != *e && !c.is_whitespace() => Span::styled(
                    c.to_string(),
                    Style::default()
                        .fg(Color::Red)
                        .add_modifier(Modifier::CROSSED_OUT),
                ),
                Some(&c) if c != *e && c.is_whitespace() => Span::styled(
                    c.to_string(),
                    Style::default()
                        .bg(Color::Red)
                        .add_modifier(Modifier::CROSSED_OUT),
                ),
                Some(&_) => Span::styled(e.to_string(), Style::default().fg(Color::White)),
                None if i == typed_chars.len() => Span::styled(
                    e.to_string(),
                    Style::default().fg(Color::White).bg(GRAY_DIM),
                ),
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

        let pct = if checked == 0 || game.game_state.seniority_level >= 5 {
            100
        } else {
            (correct * 100
                / (checked as f64 * (1.0 - game.game_state.seniority_level as f64 * 0.2)).round()
                    as usize)
                .min(100)
        };
        let money_change = game.calc_money_change(&self.typed, reference);
        let pct_text = format!("{pct:>3}% {money_change:+}");

        let pct_color = if pct >= 90 {
            Color::Green
        } else if pct >= 70 {
            Color::Yellow
        } else {
            Color::Red
        };

        vec![
            Line::from(Span::styled(pct_text, Style::default().fg(pct_color))),
            Line::from(line),
            Line::from(Span::styled(next1, Style::default().fg(GRAY_MID))),
            Line::from(Span::styled(next2, Style::default().fg(GRAY_DIM))),
        ]
    }

    pub fn ui(&self, frame: &mut Frame, area: Rect, focused: bool, game: &Game) {
        frame.render_widget(
            Paragraph::new(self.render_lines(game)).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(Self::title())
                    .border_style(Style::default().fg(focus_border_color(focused))),
            ),
            area,
        );
    }
}

impl PaneModule for TextPane {
    fn title() -> &'static str {
        "Text"
    }

    fn main_column_row_constraint() -> Option<Constraint> {
        Some(Constraint::Length(7))
    }

    fn handle_input(game: &mut Game, key: KeyCode) {
        match key {
            KeyCode::Enter => {
                let current_line = game.get_text_line(None).to_string();
                let typed = std::mem::take(&mut game.game_state.text_pane.typed);
                let typed_chars: Vec<char> = typed.chars().collect();
                let ref_chars: Vec<char> = current_line.chars().collect();
                let money_change = game.calc_money_change(&typed, &current_line);

                logging::debug(&format!(
                    "Scored line {}: money_change={money_change} (typed={} chars, ref={} chars)",
                    game.game_state.text_pane.current_line,
                    typed_chars.len(),
                    ref_chars.len()
                ));
                game.increment_money(money_change);
                if game.game_state.streaks_unlocked {
                    game.game_state.trust_level = game.calc_trust(
                        &String::from_iter(ref_chars.clone()),
                        &String::from_iter(typed_chars.clone()),
                        game.game_state.trust_level,
                    );
                }
                game.game_state.text_pane.current_line += 1;
            }
            KeyCode::Char(c) => game.game_state.text_pane.typed.push(c),
            KeyCode::Backspace => {
                game.game_state.text_pane.typed.pop();
            }
            _ => {}
        }
    }

    fn render(frame: &mut Frame, area: Rect, game: &Game, focused: bool) {
        game.game_state.text_pane.ui(frame, area, focused, game);
    }
}
