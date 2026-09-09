use std::collections::HashMap;

use crossterm::event::{KeyCode};
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use serde::{Deserialize, Serialize};

use crate::game::Game;
use crate::pane_module::{focus_border_color, PaneModule, ModuleId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutoTyperSelector {
    pub auto_typer_counts: HashMap<char, u32>,
    pub max_auto_typers: u32,
}

impl Default for AutoTyperSelector {
    fn default() -> Self {
        let mut auto_typer_counts: HashMap<char, u32> = HashMap::new();
        for c in 'a'..='z' {
            auto_typer_counts.insert(c, 0);
        }
        Self {
            auto_typer_counts: auto_typer_counts,
            max_auto_typers: 0,
        }
    }
}

impl AutoTyperSelector {
    pub fn get_letter_counts(&self, letter: char) -> u32 {
        *self.auto_typer_counts.get(&letter).unwrap_or(&0)
    }

    pub fn handle_input(&mut self, key: KeyCode) -> bool {
        match key {
            KeyCode::Enter => {
                if self.max_auto_typers == 0 {
                    return true
                }
            }
            KeyCode::Char(c) if c.is_lowercase() => {
                if self.max_auto_typers > 0 {
                    self.max_auto_typers -= 1;
                    *self.auto_typer_counts.entry(c).or_insert(0) += 1;
                }
            }
            KeyCode::Char(c) if c.is_uppercase() => {
                if let Some(count) = self.auto_typer_counts.get_mut(&c.to_ascii_lowercase()) {
                    if *count > 0 {
                        *count -= 1;
                        self.max_auto_typers += 1;
                    }
                }
            }
            _ => {}
        }
        return false
    }

    pub fn render_lines(
        &self,
    ) -> Vec<Line<'static>> {
        let mut lines = Vec::new();
        let auto_typer_color = if self.max_auto_typers > 0 {
            Color::Green
        } else {
            Color::Red
        };
        lines.push(Line::from(Span::styled(
            format!("Available auto typers: {}", self.max_auto_typers),
            Style::default().bold().bg(auto_typer_color).fg(Color::Black),
        )).centered());
        for letter in 'a'..='z' {
            let count = self.get_letter_counts(letter);
            let color = if count > 0 {
                Color::Green
            } else {
                Color::Red
            };
            lines.push(Line::from(Span::styled(
                format!("    {}: {}", letter, "█".repeat(2 * count as usize)),
                Style::default().fg(color),
            )));
        }

        lines.push(Line::from(Span::styled(
            "lowercase: increase count, uppercase: decrease count",
            Style::default().bold().fg(Color::Cyan),
        )).centered());
        lines.push(Line::from(Span::styled(
            "Press Enter to confirm",
            Style::default().bold().fg(Color::Cyan),
        )).centered());

        lines
    }

    pub fn ui(
        &self,
        frame: &mut Frame,
        area: Rect,
        is_focused: bool,
    ) {
        let lines = self.render_lines();
        frame.render_widget(
            Paragraph::new(lines).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(Self::title())
                    .border_style(Style::default().fg(focus_border_color(is_focused))),
            ),
            area,
        );
    }
}

impl PaneModule for AutoTyperSelector {
    fn title() -> &'static str {
        "Auto Typers Selector"
    }

    fn handle_input(_game: &mut Game, _key: KeyCode) {
        let ret = _game.game_state.auto_queue.auto_typer_selector.handle_input(_key);
        if ret {
            _game.remove_floating_pane(ModuleId::AutoTyperSelector);
        }
    }

    fn render(frame: &mut Frame, area: Rect, _game: &Game, focused: bool) {
        _game.game_state.auto_queue.auto_typer_selector.ui(frame, area, focused);
    }
}
