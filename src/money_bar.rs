use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::game::Game;
use crate::pane_module::PaneModule;

pub const MONEY_BAR_HEIGHT: u16 = 3;
const MONEY_GOLD: Color = Color::Rgb(255, 215, 0);

pub struct MoneyBar;

impl MoneyBar {
    pub fn ui(frame: &mut Frame, area: Rect, game: &Game) {
        let mut money_text = format!("Money: {}", game.game_state.money);
        if game.game_state.trust_level != 0 || game.game_state.streaks_unlocked {
            money_text.push_str(&format!(
                " (Trust: {}%)",
                game.game_state.trust_level
            ));
        }
        if game.game_state.seniority_level > 0 {
            let numerals: Vec<&str> = vec!["I", "II", "III", "IV", "V", "VI"];
            money_text.push_str(&format!(
                " Typer {}",
                numerals[game.game_state.seniority_level as usize]
            ));
        }
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                money_text,
                Style::default()
                    .fg(MONEY_GOLD)
                    .add_modifier(Modifier::BOLD),
            )))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(MONEY_GOLD)),
            ),
            area,
        );
    }

    pub fn height_constraint() -> Constraint {
        Constraint::Length(MONEY_BAR_HEIGHT)
    }
}

impl PaneModule for MoneyBar {
    fn title() -> &'static str {
        "Money"
    }

    fn render(frame: &mut Frame, area: Rect, game: &Game, _focused: bool) {
        Self::ui(frame, area, game);
    }
}
